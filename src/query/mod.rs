//! GQL 查询模块
//!
//! 基于 ISO GQL 39075 标准的查询语言解析器和执行器
//!
//! 主要特性:
//! - 路径模式: WALK, TRAIL, SIMPLE, ACYCLIC
//! - 路径搜索前缀: ALL, ANY, SHORTEST
//! - 匹配模式: REPEATABLE ELEMENTS, DIFFERENT EDGES
//! - 图类型: 封闭图 (Web3 场景)
//! - 完整的标签表达式支持
//! - 量化路径模式

mod ast;
mod executor;
mod parser;

// 导出 AST 类型
pub use ast::{
    BinaryOperator,
    CallStatement,
    // DDL 语句
    CreateGraphStatement,
    DeleteStatement,
    DescribeStatement,
    DescribeType,
    DropGraphStatement,
    EdgeDirection,
    EdgePattern,
    // Graph Schema 类型
    EdgeTypeSpec,
    // 表达式
    Expression,
    // 顶级语句
    GqlStatement,
    GraphPattern,
    GraphSchema,
    InsertEdge,
    // DML 语句
    InsertStatement,
    LabelExpression,
    MatchMode,
    // MATCH 语句
    MatchStatement,
    NodePattern,
    NodeTypeSpec,
    OrderByItem,
    PathElement,
    PathMode,
    PathPattern,
    PathSearchPrefix,
    PatternQuantifier,
    PropertySpec,
    RemoveStatement,
    ReturnItem,
    SetStatement,
    // SHOW/DESCRIBE 语句
    ShowStatement,
    ShowType,
    UnaryOperator,
};

// 导出执行器
pub use executor::{QueryExecutor, QueryResult};

// 导出解析器
pub use parser::GqlParser;
