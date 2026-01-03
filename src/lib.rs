//! ChainGraph - Web3 区块链链路追踪图数据库
//!
//! 专为 Web3 场景设计的高性能图数据库，支持：
//! - SSD 磁盘存储，处理海量数据
//! - 高效的图算法（链路追踪、最大流等）
//! - ISO GQL 兼容的查询语言
//! - 批量区块链数据导入

pub mod algorithm;
pub mod builtin_types;
pub mod cli;
pub mod error;
pub mod graph;
pub mod import;
pub mod metrics;
pub mod query;
pub mod server;
pub mod storage;
pub mod types;

// 重导出常用类型
pub use builtin_types::{BuiltinEdgeType, BuiltinGraph, BuiltinNodeType};
pub use error::{Error, Result};
pub use graph::{Edge, EdgeId, Graph, GraphCatalog, Vertex, VertexId};
pub use storage::{BufferPool, DiskStorage, Page, PageType, PAGE_SIZE};
pub use types::{Address, EdgeLabel, PropertyValue, TokenAmount, TxHash, VertexLabel};

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
