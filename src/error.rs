//! 错误类型定义

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("顶点不存在: {0}")]
    VertexNotFound(String),

    #[error("边不存在: {0}")]
    EdgeNotFound(String),

    #[error("顶点已存在: {0}")]
    VertexAlreadyExists(String),

    #[error("页面不存在: {0}")]
    PageNotFound(u64),

    #[error("缓冲池已满")]
    BufferPoolFull,

    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("数据校验失败: 期望 CRC {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("查询错误: {0}")]
    QueryError(String),

    #[error("查询解析错误: {0}")]
    QueryParseError(String),

    #[error("查询执行错误: {0}")]
    QueryExecutionError(String),

    #[error("未找到: {0}")]
    NotFound(String),

    #[error("无效的地址格式: {0}")]
    InvalidAddress(String),

    #[error("无效的交易哈希: {0}")]
    InvalidTxHash(String),

    #[error("导入错误: {0}")]
    ImportError(String),

    #[error("服务器错误: {0}")]
    ServerError(String),

    #[error("算法错误: {0}")]
    AlgorithmError(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}
