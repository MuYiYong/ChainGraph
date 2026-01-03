//! 存储引擎模块
//!
//! 基于 SSD 优化的磁盘存储引擎，包含：
//! - 页面管理器 (Page Manager)
//! - 缓冲池 (Buffer Pool)
//! - 磁盘页面格式
//! - 外存算法支持

mod buffer_pool;
mod disk;
mod page;

pub use buffer_pool::{BufferPool, BufferPoolWatermark, WatermarkStatus};
pub use disk::DiskStorage;
pub use page::{Page, PageType, PAGE_SIZE};
