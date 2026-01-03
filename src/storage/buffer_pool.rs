//! 缓冲池管理器
//!
//! 实现内存页面缓存和 LRU 淘汰策略

use crate::error::{Error, Result};
use crate::metrics;
use crate::storage::disk::DiskStorage;
use crate::storage::page::{Page, PageType};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;

/// 默认缓冲池大小（页面数）
const DEFAULT_POOL_SIZE: usize = 1024;

/// LRU 替换器
struct LRUReplacer {
    /// 可淘汰页面队列
    queue: VecDeque<u64>,
    /// 页面位置映射
    position: HashMap<u64, usize>,
}

impl LRUReplacer {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            position: HashMap::new(),
        }
    }

    /// 记录页面访问（移到队尾）
    fn access(&mut self, page_id: u64) {
        if let Some(&pos) = self.position.get(&page_id) {
            if pos < self.queue.len() {
                self.queue.remove(pos);
            }
        }
        self.queue.push_back(page_id);
        self.position.insert(page_id, self.queue.len() - 1);
        self.rebuild_positions();
    }

    /// 移除页面
    fn remove(&mut self, page_id: u64) {
        if let Some(&pos) = self.position.get(&page_id) {
            if pos < self.queue.len() {
                self.queue.remove(pos);
            }
            self.position.remove(&page_id);
            self.rebuild_positions();
        }
    }

    /// 选择淘汰页面
    fn evict(&mut self) -> Option<u64> {
        let page_id = self.queue.pop_front()?;
        self.position.remove(&page_id);
        self.rebuild_positions();
        Some(page_id)
    }

    /// 重建位置映射
    fn rebuild_positions(&mut self) {
        self.position.clear();
        for (i, &page_id) in self.queue.iter().enumerate() {
            self.position.insert(page_id, i);
        }
    }

    /// 可淘汰页面数量
    #[allow(dead_code)]
    fn size(&self) -> usize {
        self.queue.len()
    }
}

/// 缓冲池页面帧
struct Frame {
    page: Option<Page>,
    is_dirty: bool,
    pin_count: u32,
}

impl Frame {
    fn new() -> Self {
        Self {
            page: None,
            is_dirty: false,
            pin_count: 0,
        }
    }
}

/// 缓冲池管理器
pub struct BufferPool {
    /// 磁盘存储
    disk: Arc<DiskStorage>,
    /// 页面帧
    frames: Vec<RwLock<Frame>>,
    /// 页面 ID 到帧索引的映射
    page_table: Mutex<HashMap<u64, usize>>,
    /// LRU 替换器
    replacer: Mutex<LRUReplacer>,
    /// 空闲帧列表
    free_list: Mutex<VecDeque<usize>>,
    /// 缓冲池大小
    pool_size: usize,
}

impl BufferPool {
    /// 创建缓冲池
    pub fn new<P: AsRef<Path>>(data_dir: P, pool_size: Option<usize>) -> Result<Arc<Self>> {
        let pool_size = pool_size.unwrap_or(DEFAULT_POOL_SIZE);
        let disk = DiskStorage::open(data_dir, false)?;

        let mut frames = Vec::with_capacity(pool_size);
        let mut free_list = VecDeque::with_capacity(pool_size);

        for i in 0..pool_size {
            frames.push(RwLock::new(Frame::new()));
            free_list.push_back(i);
        }

        Ok(Arc::new(Self {
            disk,
            frames,
            page_table: Mutex::new(HashMap::new()),
            replacer: Mutex::new(LRUReplacer::new()),
            free_list: Mutex::new(free_list),
            pool_size,
        }))
    }

    /// 创建新页面
    pub fn new_page(&self, page_type: PageType) -> Result<PageHandle<'_>> {
        let frame_id = self.find_free_frame()?;
        let page = self.disk.allocate_page(page_type)?;
        let page_id = page.page_id;

        {
            let mut frame = self.frames[frame_id].write();
            frame.page = Some(page);
            frame.is_dirty = true;
            frame.pin_count = 1;
        }

        self.page_table.lock().insert(page_id, frame_id);

        Ok(PageHandle {
            page_id,
            frame_id,
            pool: self,
        })
    }

    /// 获取页面
    pub fn fetch_page(&self, page_id: u64) -> Result<PageHandle<'_>> {
        // 检查是否已在缓冲池中
        {
            let page_table = self.page_table.lock();
            if let Some(&frame_id) = page_table.get(&page_id) {
                let mut frame = self.frames[frame_id].write();
                frame.pin_count += 1;
                self.replacer.lock().remove(page_id);

                // 记录缓存命中
                metrics::global_metrics().record_buffer_hit();

                return Ok(PageHandle {
                    page_id,
                    frame_id,
                    pool: self,
                });
            }
        }

        // 记录缓存未命中
        metrics::global_metrics().record_buffer_miss();

        // 从磁盘加载
        let frame_id = self.find_free_frame()?;
        let page = self.disk.read_page(page_id)?;

        {
            let mut frame = self.frames[frame_id].write();
            frame.page = Some(page);
            frame.is_dirty = false;
            frame.pin_count = 1;
        }

        self.page_table.lock().insert(page_id, frame_id);

        Ok(PageHandle {
            page_id,
            frame_id,
            pool: self,
        })
    }

    /// 查找空闲帧
    fn find_free_frame(&self) -> Result<usize> {
        // 先从空闲列表获取
        {
            let mut free_list = self.free_list.lock();
            if let Some(frame_id) = free_list.pop_front() {
                return Ok(frame_id);
            }
        }

        // 使用 LRU 淘汰
        let victim_page_id = {
            let mut replacer = self.replacer.lock();
            replacer.evict()
        };

        if let Some(victim_page_id) = victim_page_id {
            let frame_id = {
                let page_table = self.page_table.lock();
                *page_table
                    .get(&victim_page_id)
                    .ok_or_else(|| Error::StorageError("LRU 淘汰页面不在页表中".to_string()))?
            };

            // 记录页面驱逐
            metrics::global_metrics().record_eviction();

            // 如果脏页，写回磁盘
            {
                let frame = self.frames[frame_id].read();
                if frame.is_dirty {
                    if let Some(ref page) = frame.page {
                        self.disk.write_page(page)?;
                        // 记录脏页写回
                        metrics::global_metrics().record_dirty_write();
                    }
                }
            }

            // 从页表移除
            self.page_table.lock().remove(&victim_page_id);

            return Ok(frame_id);
        }

        Err(Error::StorageError("缓冲池已满且无可淘汰页面".to_string()))
    }

    /// 释放页面引用
    fn unpin_page(&self, page_id: u64, is_dirty: bool) -> Result<()> {
        let page_table = self.page_table.lock();
        if let Some(&frame_id) = page_table.get(&page_id) {
            let mut frame = self.frames[frame_id].write();
            if is_dirty {
                frame.is_dirty = true;
            }
            if frame.pin_count > 0 {
                frame.pin_count -= 1;
                if frame.pin_count == 0 {
                    self.replacer.lock().access(page_id);
                }
            }
        }
        Ok(())
    }

    /// 刷新页面到磁盘
    pub fn flush_page(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.lock();
        if let Some(&frame_id) = page_table.get(&page_id) {
            let mut frame = self.frames[frame_id].write();
            if let Some(ref page) = frame.page {
                self.disk.write_page(page)?;
                frame.is_dirty = false;
            }
        }
        Ok(())
    }

    /// 刷新所有脏页
    pub fn flush_all(&self) -> Result<()> {
        let page_table = self.page_table.lock();
        for (&_page_id, &frame_id) in page_table.iter() {
            let mut frame = self.frames[frame_id].write();
            if frame.is_dirty {
                if let Some(ref page) = frame.page {
                    self.disk.write_page(page)?;
                    frame.is_dirty = false;
                }
            }
        }
        self.disk.sync()
    }

    /// 删除页面
    pub fn delete_page(&self, page_id: u64) -> Result<()> {
        let mut page_table = self.page_table.lock();
        if let Some(frame_id) = page_table.remove(&page_id) {
            let frame = self.frames[frame_id].read();
            if frame.pin_count > 0 {
                page_table.insert(page_id, frame_id);
                return Err(Error::StorageError("无法删除被引用的页面".to_string()));
            }
            drop(frame);

            self.replacer.lock().remove(page_id);
            self.free_list.lock().push_back(frame_id);

            let mut frame = self.frames[frame_id].write();
            frame.page = None;
            frame.is_dirty = false;
            frame.pin_count = 0;
        }

        self.disk.free_page(page_id)
    }

    /// 获取缓冲池大小
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }

    /// 获取当前缓存页面数
    pub fn cached_pages(&self) -> usize {
        self.page_table.lock().len()
    }

    /// 获取水位信息（用于监控）
    pub fn watermark_info(&self) -> BufferPoolWatermark {
        let cached = self.cached_pages();
        let total = self.pool_size;
        let usage_percent = (cached as f64 / total as f64) * 100.0;

        BufferPoolWatermark {
            cached_pages: cached,
            total_pages: total,
            usage_percent,
            status: if usage_percent >= 90.0 {
                WatermarkStatus::Critical
            } else if usage_percent >= 80.0 {
                WatermarkStatus::Warning
            } else {
                WatermarkStatus::Normal
            },
        }
    }
}

/// 缓冲池水位信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BufferPoolWatermark {
    pub cached_pages: usize,
    pub total_pages: usize,
    pub usage_percent: f64,
    pub status: WatermarkStatus,
}

/// 水位状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WatermarkStatus {
    /// 正常 (< 80%)
    Normal,
    /// 警告 (80-90%)
    Warning,
    /// 危险 (>= 90%)
    Critical,
}

/// 页面句柄（RAII 自动释放）
pub struct PageHandle<'a> {
    page_id: u64,
    frame_id: usize,
    pool: &'a BufferPool,
}

impl<'a> PageHandle<'a> {
    /// 获取页面 ID
    pub fn page_id(&self) -> u64 {
        self.page_id
    }

    /// 获取页面只读访问
    pub fn read(&self) -> PageReadGuard<'a> {
        PageReadGuard {
            guard: self.pool.frames[self.frame_id].read(),
        }
    }

    /// 获取页面可写访问
    pub fn write(&self) -> PageWriteGuard<'a> {
        PageWriteGuard {
            guard: self.pool.frames[self.frame_id].write(),
        }
    }

    /// 标记为脏页
    pub fn mark_dirty(&self) {
        self.pool.frames[self.frame_id].write().is_dirty = true;
    }
}

impl<'a> Drop for PageHandle<'a> {
    fn drop(&mut self) {
        let _ = self.pool.unpin_page(self.page_id, false);
    }
}

/// 页面只读守卫
pub struct PageReadGuard<'a> {
    guard: parking_lot::RwLockReadGuard<'a, Frame>,
}

impl<'a> PageReadGuard<'a> {
    pub fn page(&self) -> Option<&Page> {
        self.guard.page.as_ref()
    }
}

/// 页面可写守卫
pub struct PageWriteGuard<'a> {
    guard: parking_lot::RwLockWriteGuard<'a, Frame>,
}

impl<'a> PageWriteGuard<'a> {
    pub fn page(&self) -> Option<&Page> {
        self.guard.page.as_ref()
    }

    pub fn page_mut(&mut self) -> Option<&mut Page> {
        self.guard.is_dirty = true;
        self.guard.page.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_buffer_pool_basic() {
        let dir = tempdir().unwrap();
        let pool = BufferPool::new(dir.path(), Some(10)).unwrap();

        // 创建新页面
        let handle = pool.new_page(PageType::Vertex).unwrap();
        let page_id = handle.page_id();

        {
            let mut guard = handle.write();
            if let Some(page) = guard.page_mut() {
                page.append_data(b"hello").unwrap();
            }
        }

        drop(handle);

        // 重新获取
        let handle = pool.fetch_page(page_id).unwrap();
        let guard = handle.read();
        if let Some(page) = guard.page() {
            assert_eq!(&page.data[0..5], b"hello");
        }
    }

    #[test]
    fn test_lru_eviction() {
        let dir = tempdir().unwrap();
        let pool = BufferPool::new(dir.path(), Some(3)).unwrap();

        // 创建 3 个页面填满缓冲池
        let h1 = pool.new_page(PageType::Vertex).unwrap();
        let h2 = pool.new_page(PageType::Vertex).unwrap();
        let h3 = pool.new_page(PageType::Vertex).unwrap();

        let id1 = h1.page_id();

        // 释放所有句柄
        drop(h1);
        drop(h2);
        drop(h3);

        // 创建新页面，应该淘汰最早的页面
        let h4 = pool.new_page(PageType::Vertex).unwrap();
        drop(h4);

        // 页面 1 应该被淘汰，重新获取需要从磁盘加载
        let h1_again = pool.fetch_page(id1).unwrap();
        assert_eq!(h1_again.page_id(), id1);
    }
}
