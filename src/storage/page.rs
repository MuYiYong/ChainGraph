//! 磁盘页面定义
//!
//! 页面是磁盘 I/O 的基本单位，大小为 4KB（SSD 友好）

use crate::error::{Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};

/// 页面大小：4KB (SSD 友好，对齐扇区)
pub const PAGE_SIZE: usize = 4096;

/// 页面头部大小（实际布局：8+1+1+2+2+8+8+4=34 bytes, 对齐到 36）
pub const PAGE_HEADER_SIZE: usize = 36;

/// 页面数据区大小
pub const PAGE_DATA_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

/// 页面类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PageType {
    /// 空闲页
    Free = 0,
    /// 顶点数据页
    Vertex = 1,
    /// 边数据页
    Edge = 2,
    /// 顶点索引页
    VertexIndex = 3,
    /// 边索引页
    EdgeIndex = 4,
    /// 属性数据页
    Property = 5,
    /// 溢出页（存储大属性）
    Overflow = 6,
    /// 元数据页
    Meta = 7,
}

impl From<u8> for PageType {
    fn from(v: u8) -> Self {
        match v {
            0 => PageType::Free,
            1 => PageType::Vertex,
            2 => PageType::Edge,
            3 => PageType::VertexIndex,
            4 => PageType::EdgeIndex,
            5 => PageType::Property,
            6 => PageType::Overflow,
            7 => PageType::Meta,
            _ => PageType::Free,
        }
    }
}

/// 磁盘页面
///
/// 页面布局:
/// ```text
/// +-------------------+
/// | Header (32 bytes) |
/// |  - page_id (8)    |
/// |  - page_type (1)  |
/// |  - flags (1)      |
/// |  - item_count (2) |
/// |  - free_offset (2)|
/// |  - next_page (8)  |
/// |  - prev_page (8)  |
/// |  - checksum (4)   |
/// +-------------------+
/// | Data (4064 bytes) |
/// +-------------------+
/// ```
#[derive(Clone)]
pub struct Page {
    /// 页面 ID
    pub page_id: u64,
    /// 页面类型
    pub page_type: PageType,
    /// 标志位
    pub flags: u8,
    /// 项目数量
    pub item_count: u16,
    /// 空闲空间偏移
    pub free_offset: u16,
    /// 下一页 ID（用于链表）
    pub next_page: u64,
    /// 上一页 ID
    pub prev_page: u64,
    /// 校验和
    pub checksum: u32,
    /// 数据区
    pub data: Vec<u8>,
    /// 是否脏页
    pub is_dirty: bool,
    /// 引用计数
    pub pin_count: u32,
}

impl Page {
    /// 创建新页面
    pub fn new(page_id: u64, page_type: PageType) -> Self {
        Self {
            page_id,
            page_type,
            flags: 0,
            item_count: 0,
            free_offset: 0,
            next_page: 0,
            prev_page: 0,
            checksum: 0,
            data: vec![0u8; PAGE_DATA_SIZE],
            is_dirty: true,
            pin_count: 0,
        }
    }

    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PAGE_SIZE {
            return Err(Error::StorageError(format!(
                "页面大小错误: 期望 {}, 实际 {}",
                PAGE_SIZE,
                bytes.len()
            )));
        }

        // 解析头部（34 字节实际使用）
        let page_id = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let page_type = PageType::from(bytes[8]);
        let flags = bytes[9];
        let item_count = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
        let free_offset = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let next_page = u64::from_le_bytes(bytes[14..22].try_into().unwrap());
        let prev_page = u64::from_le_bytes(bytes[22..30].try_into().unwrap());
        let stored_checksum = u32::from_le_bytes(bytes[30..34].try_into().unwrap());

        // 读取数据区（从 36 开始）
        let mut data = vec![0u8; PAGE_DATA_SIZE];
        data.copy_from_slice(&bytes[PAGE_HEADER_SIZE..]);

        // 验证校验和
        let calculated_checksum = Self::calculate_checksum(&data);
        if stored_checksum != 0 && stored_checksum != calculated_checksum {
            return Err(Error::ChecksumMismatch {
                expected: stored_checksum,
                actual: calculated_checksum,
            });
        }

        Ok(Self {
            page_id,
            page_type,
            flags,
            item_count,
            free_offset,
            next_page,
            prev_page,
            checksum: stored_checksum,
            data,
            is_dirty: false,
            pin_count: 0,
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; PAGE_SIZE];

        // 写入头部（34 字节实际使用）
        buffer[0..8].copy_from_slice(&self.page_id.to_le_bytes());
        buffer[8] = self.page_type as u8;
        buffer[9] = self.flags;
        buffer[10..12].copy_from_slice(&self.item_count.to_le_bytes());
        buffer[12..14].copy_from_slice(&self.free_offset.to_le_bytes());
        buffer[14..22].copy_from_slice(&self.next_page.to_le_bytes());
        buffer[22..30].copy_from_slice(&self.prev_page.to_le_bytes());

        let checksum = Self::calculate_checksum(&self.data);
        buffer[30..34].copy_from_slice(&checksum.to_le_bytes());

        // 写入数据区（从 36 开始，留 2 字节对齐填充）
        buffer[PAGE_HEADER_SIZE..PAGE_HEADER_SIZE + self.data.len()].copy_from_slice(&self.data);

        buffer
    }

    /// 计算数据校验和
    fn calculate_checksum(data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }

    /// 获取可用空间
    pub fn free_space(&self) -> usize {
        PAGE_DATA_SIZE - self.free_offset as usize
    }

    /// 写入数据到页面
    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > PAGE_DATA_SIZE {
            return Err(Error::StorageError("数据超出页面边界".to_string()));
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.is_dirty = true;
        Ok(())
    }

    /// 读取页面数据
    pub fn read_data(&self, offset: usize, len: usize) -> Result<&[u8]> {
        if offset + len > PAGE_DATA_SIZE {
            return Err(Error::StorageError("读取超出页面边界".to_string()));
        }
        Ok(&self.data[offset..offset + len])
    }

    /// 追加数据
    pub fn append_data(&mut self, data: &[u8]) -> Result<usize> {
        let offset = self.free_offset as usize;
        if offset + data.len() > PAGE_DATA_SIZE {
            return Err(Error::StorageError("页面空间不足".to_string()));
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.free_offset += data.len() as u16;
        self.item_count += 1;
        self.is_dirty = true;
        Ok(offset)
    }

    /// 标记为脏页
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// 增加引用计数
    pub fn pin(&mut self) {
        self.pin_count += 1;
    }

    /// 减少引用计数
    pub fn unpin(&mut self) {
        if self.pin_count > 0 {
            self.pin_count -= 1;
        }
    }

    /// 是否可被淘汰
    pub fn is_evictable(&self) -> bool {
        self.pin_count == 0
    }
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("page_id", &self.page_id)
            .field("page_type", &self.page_type)
            .field("item_count", &self.item_count)
            .field("free_offset", &self.free_offset)
            .field("is_dirty", &self.is_dirty)
            .field("pin_count", &self.pin_count)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_serialization() {
        let mut page = Page::new(1, PageType::Vertex);
        page.append_data(b"hello world").unwrap();

        let bytes = page.to_bytes();
        assert_eq!(bytes.len(), PAGE_SIZE);

        let restored = Page::from_bytes(&bytes).unwrap();
        assert_eq!(restored.page_id, 1);
        assert_eq!(restored.page_type, PageType::Vertex);
        assert_eq!(restored.item_count, 1);
    }

    #[test]
    fn test_page_append() {
        let mut page = Page::new(1, PageType::Vertex);

        let offset1 = page.append_data(b"data1").unwrap();
        assert_eq!(offset1, 0);

        let offset2 = page.append_data(b"data2").unwrap();
        assert_eq!(offset2, 5);

        assert_eq!(page.item_count, 2);
        assert_eq!(page.free_offset, 10);
    }
}
