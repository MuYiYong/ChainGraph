//! 磁盘存储引擎
//!
//! 使用内存映射文件实现高效的 SSD I/O

use crate::error::{Error, Result};
use crate::storage::page::{Page, PageType, PAGE_SIZE};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use memmap2::{MmapMut, MmapOptions};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 数据文件扩展名
const DATA_FILE_EXT: &str = "cgd"; // ChainGraph Data
/// 默认初始文件大小 (64MB)
const DEFAULT_INITIAL_SIZE: u64 = 64 * 1024 * 1024;
/// 文件扩展步长 (16MB)
const EXTEND_SIZE: u64 = 16 * 1024 * 1024;
/// 文件魔数
const MAGIC_NUMBER: u64 = 0x4348_4149_4E47_5248; // "CHAINGR\0"
/// 文件版本
const FILE_VERSION: u32 = 1;

/// 文件头部（第 0 页）
#[derive(Debug)]
struct FileHeader {
    magic: u64,
    version: u32,
    page_count: u64,
    free_page_head: u64,
}

impl FileHeader {
    fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.magic.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.version.to_le_bytes());
        bytes[12..20].copy_from_slice(&self.page_count.to_le_bytes());
        bytes[20..28].copy_from_slice(&self.free_page_head.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(Error::StorageError("文件头部数据不足".to_string()));
        }
        let magic = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        if magic != MAGIC_NUMBER {
            return Err(Error::StorageError("无效的数据文件格式".to_string()));
        }
        Ok(Self {
            magic,
            version: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            page_count: u64::from_le_bytes(bytes[12..20].try_into().unwrap()),
            free_page_head: u64::from_le_bytes(bytes[20..28].try_into().unwrap()),
        })
    }
}

/// 磁盘存储引擎
pub struct DiskStorage {
    /// 数据目录
    data_dir: PathBuf,
    /// 数据文件
    data_file: RwLock<File>,
    /// 内存映射
    mmap: RwLock<MmapMut>,
    /// 当前页数
    page_count: AtomicU64,
    /// 空闲页头
    free_page_head: AtomicU64,
    /// 是否启用压缩
    enable_compression: bool,
    /// 压缩缓存（页面 ID -> 压缩后数据）
    compression_cache: RwLock<HashMap<u64, Vec<u8>>>,
}

impl DiskStorage {
    /// 打开或创建存储
    pub fn open<P: AsRef<Path>>(data_dir: P, enable_compression: bool) -> Result<Arc<Self>> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let data_file_path = data_dir.join(format!("data.{}", DATA_FILE_EXT));
        let is_new = !data_file_path.exists();

        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&data_file_path)?;

        // 初始化或加载文件
        let (page_count, free_page_head) = if is_new {
            // 新文件：设置初始大小
            data_file.set_len(DEFAULT_INITIAL_SIZE)?;
            (1u64, 0u64) // 第 0 页是文件头
        } else {
            // 读取文件头
            let mmap = unsafe { MmapOptions::new().map(&data_file)? };
            let header = FileHeader::from_bytes(&mmap[0..PAGE_SIZE])?;
            (header.page_count, header.free_page_head)
        };

        // 创建可写内存映射
        let mmap = unsafe { MmapOptions::new().map_mut(&data_file)? };

        let storage = Arc::new(Self {
            data_dir,
            data_file: RwLock::new(data_file),
            mmap: RwLock::new(mmap),
            page_count: AtomicU64::new(page_count),
            free_page_head: AtomicU64::new(free_page_head),
            enable_compression,
            compression_cache: RwLock::new(HashMap::new()),
        });

        // 写入文件头
        if is_new {
            storage.write_header()?;
        }

        Ok(storage)
    }

    /// 写入文件头
    fn write_header(&self) -> Result<()> {
        let header = FileHeader {
            magic: MAGIC_NUMBER,
            version: FILE_VERSION,
            page_count: self.page_count.load(Ordering::SeqCst),
            free_page_head: self.free_page_head.load(Ordering::SeqCst),
        };

        let bytes = header.to_bytes();
        let mut mmap = self.mmap.write();
        mmap[0..32].copy_from_slice(&bytes);
        mmap.flush()?;
        Ok(())
    }

    /// 分配新页面
    pub fn allocate_page(&self, page_type: PageType) -> Result<Page> {
        // 先尝试从空闲链表分配
        let free_head = self.free_page_head.load(Ordering::SeqCst);
        if free_head != 0 {
            let free_page = self.read_page(free_head)?;
            self.free_page_head
                .store(free_page.next_page, Ordering::SeqCst);
            self.write_header()?;

            let mut page = Page::new(free_head, page_type);
            page.is_dirty = true;
            return Ok(page);
        }

        // 分配新页
        let page_id = self.page_count.fetch_add(1, Ordering::SeqCst);
        self.ensure_capacity(page_id)?;
        self.write_header()?;

        Ok(Page::new(page_id, page_type))
    }

    /// 确保文件容量足够
    fn ensure_capacity(&self, page_id: u64) -> Result<()> {
        let required_size = (page_id + 1) * PAGE_SIZE as u64;
        let file = self.data_file.read();
        let current_size = file.metadata()?.len();

        if required_size > current_size {
            drop(file);
            let file = self.data_file.write();
            let new_size = ((required_size / EXTEND_SIZE) + 1) * EXTEND_SIZE;
            file.set_len(new_size)?;
            drop(file);

            // 重新映射
            let file = self.data_file.read();
            let new_mmap = unsafe { MmapOptions::new().map_mut(&*file)? };
            *self.mmap.write() = new_mmap;
        }

        Ok(())
    }

    /// 读取页面
    pub fn read_page(&self, page_id: u64) -> Result<Page> {
        if page_id == 0 {
            return Err(Error::StorageError("无法读取文件头页".to_string()));
        }

        let offset = page_id as usize * PAGE_SIZE;
        let mmap = self.mmap.read();

        if offset + PAGE_SIZE > mmap.len() {
            return Err(Error::StorageError(format!(
                "页面 {} 超出文件范围",
                page_id
            )));
        }

        let page_data = &mmap[offset..offset + PAGE_SIZE];

        // 检查是否压缩
        if self.enable_compression {
            if let Some(compressed) = self.compression_cache.read().get(&page_id) {
                let decompressed = decompress_size_prepended(compressed)
                    .map_err(|e| Error::StorageError(format!("解压失败: {}", e)))?;
                return Page::from_bytes(&decompressed);
            }
        }

        Page::from_bytes(page_data)
    }

    /// 写入页面
    pub fn write_page(&self, page: &Page) -> Result<()> {
        if page.page_id == 0 {
            return Err(Error::StorageError("无法写入文件头页".to_string()));
        }

        let offset = page.page_id as usize * PAGE_SIZE;
        self.ensure_capacity(page.page_id)?;

        let page_bytes = page.to_bytes();

        // 可选压缩
        if self.enable_compression {
            let compressed = compress_prepend_size(&page_bytes);
            if compressed.len() < page_bytes.len() {
                self.compression_cache
                    .write()
                    .insert(page.page_id, compressed);
            }
        }

        let mut mmap = self.mmap.write();
        mmap[offset..offset + PAGE_SIZE].copy_from_slice(&page_bytes);
        Ok(())
    }

    /// 释放页面
    pub fn free_page(&self, page_id: u64) -> Result<()> {
        if page_id == 0 {
            return Err(Error::StorageError("无法释放文件头页".to_string()));
        }

        // 加入空闲链表
        let mut page = Page::new(page_id, PageType::Free);
        page.next_page = self.free_page_head.load(Ordering::SeqCst);
        self.write_page(&page)?;

        self.free_page_head.store(page_id, Ordering::SeqCst);
        self.write_header()?;

        // 清除压缩缓存
        self.compression_cache.write().remove(&page_id);

        Ok(())
    }

    /// 同步到磁盘
    pub fn sync(&self) -> Result<()> {
        let mmap = self.mmap.read();
        mmap.flush()?;
        Ok(())
    }

    /// 获取页面数量
    pub fn page_count(&self) -> u64 {
        self.page_count.load(Ordering::SeqCst)
    }

    /// 获取数据目录
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 批量写入页面（优化 SSD 顺序写入）
    pub fn write_pages_batch(&self, pages: &[Page]) -> Result<()> {
        for page in pages {
            self.write_page(page)?;
        }
        self.sync()
    }

    /// 批量读取页面（优化 SSD 顺序读取）
    pub fn read_pages_batch(&self, page_ids: &[u64]) -> Result<Vec<Page>> {
        let mut pages = Vec::with_capacity(page_ids.len());
        for &page_id in page_ids {
            pages.push(self.read_page(page_id)?);
        }
        Ok(pages)
    }
}

impl Drop for DiskStorage {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            eprintln!("警告: 同步数据失败: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_disk_storage_basic() {
        let dir = tempdir().unwrap();
        let storage = DiskStorage::open(dir.path(), false).unwrap();

        // 分配页面
        let page = storage.allocate_page(PageType::Vertex).unwrap();
        assert_eq!(page.page_id, 1);

        // 写入和读取
        let mut page = page;
        page.append_data(b"test data").unwrap();
        storage.write_page(&page).unwrap();

        let loaded = storage.read_page(1).unwrap();
        assert_eq!(&loaded.data[0..9], b"test data");
    }

    #[test]
    fn test_page_allocation_and_free() {
        let dir = tempdir().unwrap();
        let storage = DiskStorage::open(dir.path(), false).unwrap();

        let page1 = storage.allocate_page(PageType::Vertex).unwrap();
        let page2 = storage.allocate_page(PageType::Edge).unwrap();
        assert_eq!(page1.page_id, 1);
        assert_eq!(page2.page_id, 2);

        // 释放页面 1
        storage.free_page(1).unwrap();

        // 重新分配应该复用页面 1
        let page3 = storage.allocate_page(PageType::Vertex).unwrap();
        assert_eq!(page3.page_id, 1);
    }
}
