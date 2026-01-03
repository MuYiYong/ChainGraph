//! 内置类型系统
//!
//! 定义 ChainGraph 内置的 NODE TYPE 和 EDGE TYPE，
//! 以 `__` 前缀标识，用户可以在 CREATE GRAPH 时直接引用。

use crate::query::{EdgeTypeSpec, NodeTypeSpec, PropertySpec};
use std::collections::HashMap;

/// 内置类型前缀
pub const BUILTIN_PREFIX: &str = "__";

// ============================================================================
// Built-in NODE TYPE Definitions
// ============================================================================

/// 内置 NODE TYPE 枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinNodeType {
    /// __Account - 外部账户/地址
    Account,
    /// __Transaction - 链上交易
    Transaction,
    /// __Block - 区块
    Block,
    /// __Token - 代币合约
    Token,
    /// __Contract - 智能合约
    Contract,
}

impl BuiltinNodeType {
    /// 获取内置类型名称（带前缀）
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinNodeType::Account => "__Account",
            BuiltinNodeType::Transaction => "__Transaction",
            BuiltinNodeType::Block => "__Block",
            BuiltinNodeType::Token => "__Token",
            BuiltinNodeType::Contract => "__Contract",
        }
    }

    /// 从名称解析内置类型
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "__Account" => Some(BuiltinNodeType::Account),
            "__Transaction" => Some(BuiltinNodeType::Transaction),
            "__Block" => Some(BuiltinNodeType::Block),
            "__Token" => Some(BuiltinNodeType::Token),
            "__Contract" => Some(BuiltinNodeType::Contract),
            _ => None,
        }
    }

    /// 检查名称是否是内置 NODE TYPE
    pub fn is_builtin_node_type(name: &str) -> bool {
        Self::from_name(name).is_some()
    }

    /// 获取属性定义
    pub fn properties(&self) -> Vec<PropertySpec> {
        match self {
            BuiltinNodeType::Account => vec![
                PropertySpec {
                    name: "address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: true,
                },
                PropertySpec {
                    name: "entity_label".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "risk_label".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "first_seen_block".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "last_active_block".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinNodeType::Transaction => vec![
                PropertySpec {
                    name: "hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: true,
                },
                PropertySpec {
                    name: "block_number".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "block_timestamp".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "value".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "gas_used".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "gas_price".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "status".to_string(),
                    data_type: "INT".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinNodeType::Block => vec![
                PropertySpec {
                    name: "number".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: true,
                },
                PropertySpec {
                    name: "hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "parent_hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "timestamp".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "miner".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "tx_count".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinNodeType::Token => vec![
                PropertySpec {
                    name: "address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: true,
                },
                PropertySpec {
                    name: "name".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "symbol".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "decimals".to_string(),
                    data_type: "INT".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "total_supply".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinNodeType::Contract => vec![
                PropertySpec {
                    name: "address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: true,
                },
                PropertySpec {
                    name: "name".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "creator".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "creation_block".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "is_verified".to_string(),
                    data_type: "BOOL".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "bytecode_hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
            ],
        }
    }

    /// 转换为 NodeTypeSpec
    pub fn to_spec(&self) -> NodeTypeSpec {
        NodeTypeSpec {
            label: self.name().to_string(),
            properties: self.properties(),
            is_builtin_ref: true,
        }
    }

    /// 获取所有内置 NODE TYPE
    pub fn all() -> Vec<Self> {
        vec![
            BuiltinNodeType::Account,
            BuiltinNodeType::Transaction,
            BuiltinNodeType::Block,
            BuiltinNodeType::Token,
            BuiltinNodeType::Contract,
        ]
    }
}

// ============================================================================
// Built-in EDGE TYPE Definitions
// ============================================================================

/// 内置 EDGE TYPE 枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinEdgeType {
    /// __Transfer - 转账关系（聚合）
    Transfer,
    /// __TxRel - 账户与交易关系
    TxRel,
    /// __Call - 合约调用
    Call,
    /// __Approval - ERC20 授权
    Approval,
    /// __TokenTransfer - 单笔代币转账
    TokenTransfer,
}

impl BuiltinEdgeType {
    /// 获取内置类型名称（带前缀）
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinEdgeType::Transfer => "__Transfer",
            BuiltinEdgeType::TxRel => "__TxRel",
            BuiltinEdgeType::Call => "__Call",
            BuiltinEdgeType::Approval => "__Approval",
            BuiltinEdgeType::TokenTransfer => "__TokenTransfer",
        }
    }

    /// 从名称解析内置类型
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "__Transfer" => Some(BuiltinEdgeType::Transfer),
            "__TxRel" => Some(BuiltinEdgeType::TxRel),
            "__Call" => Some(BuiltinEdgeType::Call),
            "__Approval" => Some(BuiltinEdgeType::Approval),
            "__TokenTransfer" => Some(BuiltinEdgeType::TokenTransfer),
            _ => None,
        }
    }

    /// 检查名称是否是内置 EDGE TYPE
    pub fn is_builtin_edge_type(name: &str) -> bool {
        Self::from_name(name).is_some()
    }

    /// 获取源节点类型
    pub fn source_type(&self) -> &'static str {
        match self {
            BuiltinEdgeType::Transfer => "__Account",
            BuiltinEdgeType::TxRel => "__Account",
            BuiltinEdgeType::Call => "__Account",
            BuiltinEdgeType::Approval => "__Account",
            BuiltinEdgeType::TokenTransfer => "__Account",
        }
    }

    /// 获取目标节点类型
    pub fn target_type(&self) -> &'static str {
        match self {
            BuiltinEdgeType::Transfer => "__Account",
            BuiltinEdgeType::TxRel => "__Transaction",
            BuiltinEdgeType::Call => "__Contract",
            BuiltinEdgeType::Approval => "__Account",
            BuiltinEdgeType::TokenTransfer => "__Account",
        }
    }

    /// 获取属性定义
    pub fn properties(&self) -> Vec<PropertySpec> {
        match self {
            BuiltinEdgeType::Transfer => vec![
                PropertySpec {
                    name: "token_address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false, // MULTIEDGE KEY in semantic
                },
                PropertySpec {
                    name: "first_time".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "last_time".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "sum".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "tx_count".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinEdgeType::TxRel => vec![
                PropertySpec {
                    name: "type".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "value".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinEdgeType::Call => vec![
                PropertySpec {
                    name: "method".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "tx_hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "block_number".to_string(),
                    data_type: "INT64".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "success".to_string(),
                    data_type: "BOOL".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinEdgeType::Approval => vec![
                PropertySpec {
                    name: "token_address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "amount".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "tx_hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
            ],
            BuiltinEdgeType::TokenTransfer => vec![
                PropertySpec {
                    name: "token_address".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "amount".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "tx_hash".to_string(),
                    data_type: "STRING".to_string(),
                    is_primary_key: false,
                },
                PropertySpec {
                    name: "log_index".to_string(),
                    data_type: "INT".to_string(),
                    is_primary_key: false,
                },
            ],
        }
    }

    /// 转换为 EdgeTypeSpec
    pub fn to_spec(&self) -> EdgeTypeSpec {
        EdgeTypeSpec {
            label: self.name().to_string(),
            source_label: self.source_type().to_string(),
            target_label: self.target_type().to_string(),
            properties: self.properties(),
            is_builtin_ref: true,
        }
    }

    /// 获取所有内置 EDGE TYPE
    pub fn all() -> Vec<Self> {
        vec![
            BuiltinEdgeType::Transfer,
            BuiltinEdgeType::TxRel,
            BuiltinEdgeType::Call,
            BuiltinEdgeType::Approval,
            BuiltinEdgeType::TokenTransfer,
        ]
    }
}

// ============================================================================
// Built-in Graph Templates
// ============================================================================

/// 内置 Graph 模板枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinGraph {
    /// __ethereum - 以太坊数据模型
    Ethereum,
    /// __tron - 波场链数据模型
    Tron,
    /// __bsc - 币安智能链数据模型
    Bsc,
    /// __polygon - Polygon 数据模型
    Polygon,
}

impl BuiltinGraph {
    /// 获取内置 Graph 名称（带前缀）
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinGraph::Ethereum => "__ethereum",
            BuiltinGraph::Tron => "__tron",
            BuiltinGraph::Bsc => "__bsc",
            BuiltinGraph::Polygon => "__polygon",
        }
    }

    /// 从名称解析内置 Graph
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "__ethereum" => Some(BuiltinGraph::Ethereum),
            "__tron" => Some(BuiltinGraph::Tron),
            "__bsc" => Some(BuiltinGraph::Bsc),
            "__polygon" => Some(BuiltinGraph::Polygon),
            _ => None,
        }
    }

    /// 检查名称是否是内置 Graph
    pub fn is_builtin_graph(name: &str) -> bool {
        Self::from_name(name).is_some()
    }

    /// 获取包含的 NODE TYPE
    pub fn node_types(&self) -> Vec<BuiltinNodeType> {
        match self {
            BuiltinGraph::Ethereum => vec![
                BuiltinNodeType::Account,
                BuiltinNodeType::Transaction,
                BuiltinNodeType::Block,
                BuiltinNodeType::Token,
                BuiltinNodeType::Contract,
            ],
            BuiltinGraph::Tron => vec![
                BuiltinNodeType::Account,
                BuiltinNodeType::Transaction,
                BuiltinNodeType::Block,
                BuiltinNodeType::Token,
            ],
            BuiltinGraph::Bsc => vec![
                BuiltinNodeType::Account,
                BuiltinNodeType::Transaction,
                BuiltinNodeType::Block,
                BuiltinNodeType::Token,
            ],
            BuiltinGraph::Polygon => vec![
                BuiltinNodeType::Account,
                BuiltinNodeType::Transaction,
                BuiltinNodeType::Block,
                BuiltinNodeType::Token,
            ],
        }
    }

    /// 获取包含的 EDGE TYPE
    pub fn edge_types(&self) -> Vec<BuiltinEdgeType> {
        match self {
            BuiltinGraph::Ethereum => vec![
                BuiltinEdgeType::Transfer,
                BuiltinEdgeType::TxRel,
                BuiltinEdgeType::Call,
                BuiltinEdgeType::Approval,
                BuiltinEdgeType::TokenTransfer,
            ],
            BuiltinGraph::Tron => vec![
                BuiltinEdgeType::Transfer,
                BuiltinEdgeType::TxRel,
                BuiltinEdgeType::Call,
            ],
            BuiltinGraph::Bsc => vec![
                BuiltinEdgeType::Transfer,
                BuiltinEdgeType::TxRel,
                BuiltinEdgeType::Call,
            ],
            BuiltinGraph::Polygon => vec![
                BuiltinEdgeType::Transfer,
                BuiltinEdgeType::TxRel,
                BuiltinEdgeType::Call,
            ],
        }
    }

    /// 获取所有内置 Graph
    pub fn all() -> Vec<Self> {
        vec![
            BuiltinGraph::Ethereum,
            BuiltinGraph::Tron,
            BuiltinGraph::Bsc,
            BuiltinGraph::Polygon,
        ]
    }

    /// 生成 CREATE GRAPH GQL 语句
    pub fn to_gql(&self) -> String {
        let node_refs: Vec<String> = self
            .node_types()
            .iter()
            .map(|n| format!("   NODE {}", n.name()))
            .collect();

        let edge_refs: Vec<String> = self
            .edge_types()
            .iter()
            .map(|e| format!("   EDGE {}", e.name()))
            .collect();

        let all_refs: Vec<String> = [node_refs, edge_refs].concat();

        format!(
            "CREATE GRAPH IF NOT EXISTS {} {{\n{}\n}}",
            self.name(),
            all_refs.join(",\n")
        )
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// 检查类型名是否以 __ 前缀开头（内置类型保留前缀）
pub fn is_builtin_name(name: &str) -> bool {
    name.starts_with(BUILTIN_PREFIX)
}

/// 展开内置 NODE TYPE 引用
pub fn expand_builtin_node_type(name: &str) -> Option<NodeTypeSpec> {
    BuiltinNodeType::from_name(name).map(|t| t.to_spec())
}

/// 展开内置 EDGE TYPE 引用
pub fn expand_builtin_edge_type(name: &str) -> Option<EdgeTypeSpec> {
    BuiltinEdgeType::from_name(name).map(|t| t.to_spec())
}

/// 获取所有内置类型信息（用于 SHOW 命令）
pub fn get_builtin_types_info() -> HashMap<String, Vec<(String, String)>> {
    let mut info = HashMap::new();

    // Node types
    let node_info: Vec<(String, String)> = BuiltinNodeType::all()
        .iter()
        .map(|t| {
            let props: Vec<String> = t
                .properties()
                .iter()
                .map(|p| {
                    if p.is_primary_key {
                        format!("{} {} PRIMARY KEY", p.name, p.data_type)
                    } else {
                        format!("{} {}", p.name, p.data_type)
                    }
                })
                .collect();
            (t.name().to_string(), props.join(", "))
        })
        .collect();
    info.insert("node_types".to_string(), node_info);

    // Edge types
    let edge_info: Vec<(String, String)> = BuiltinEdgeType::all()
        .iter()
        .map(|t| {
            let props: Vec<String> = t
                .properties()
                .iter()
                .map(|p| format!("{} {}", p.name, p.data_type))
                .collect();
            (
                t.name().to_string(),
                format!(
                    "({}) -> ({}) [{}]",
                    t.source_type(),
                    t.target_type(),
                    props.join(", ")
                ),
            )
        })
        .collect();
    info.insert("edge_types".to_string(), edge_info);

    info
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_node_type_names() {
        assert_eq!(BuiltinNodeType::Account.name(), "__Account");
        assert_eq!(BuiltinNodeType::Transaction.name(), "__Transaction");
        assert_eq!(BuiltinNodeType::Block.name(), "__Block");
        assert_eq!(BuiltinNodeType::Token.name(), "__Token");
        assert_eq!(BuiltinNodeType::Contract.name(), "__Contract");
    }

    #[test]
    fn test_builtin_node_type_from_name() {
        assert_eq!(
            BuiltinNodeType::from_name("__Account"),
            Some(BuiltinNodeType::Account)
        );
        assert_eq!(BuiltinNodeType::from_name("Account"), None);
        assert_eq!(BuiltinNodeType::from_name("__Unknown"), None);
    }

    #[test]
    fn test_builtin_edge_type_names() {
        assert_eq!(BuiltinEdgeType::Transfer.name(), "__Transfer");
        assert_eq!(BuiltinEdgeType::TxRel.name(), "__TxRel");
        assert_eq!(BuiltinEdgeType::Call.name(), "__Call");
    }

    #[test]
    fn test_builtin_edge_type_from_name() {
        assert_eq!(
            BuiltinEdgeType::from_name("__Transfer"),
            Some(BuiltinEdgeType::Transfer)
        );
        assert_eq!(BuiltinEdgeType::from_name("Transfer"), None);
    }

    #[test]
    fn test_builtin_graph_names() {
        assert_eq!(BuiltinGraph::Ethereum.name(), "__ethereum");
        assert_eq!(BuiltinGraph::Tron.name(), "__tron");
        assert_eq!(BuiltinGraph::Bsc.name(), "__bsc");
        assert_eq!(BuiltinGraph::Polygon.name(), "__polygon");
    }

    #[test]
    fn test_is_builtin_name() {
        assert!(is_builtin_name("__Account"));
        assert!(is_builtin_name("__Transfer"));
        assert!(!is_builtin_name("Account"));
        assert!(!is_builtin_name("MyType"));
    }

    #[test]
    fn test_expand_builtin_node_type() {
        let spec = expand_builtin_node_type("__Account").unwrap();
        assert_eq!(spec.label, "__Account");
        assert!(spec.is_builtin_ref);
        assert!(!spec.properties.is_empty());

        // Check primary key
        let pk = spec.properties.iter().find(|p| p.is_primary_key);
        assert!(pk.is_some());
        assert_eq!(pk.unwrap().name, "address");
    }

    #[test]
    fn test_expand_builtin_edge_type() {
        let spec = expand_builtin_edge_type("__Transfer").unwrap();
        assert_eq!(spec.label, "__Transfer");
        assert_eq!(spec.source_label, "__Account");
        assert_eq!(spec.target_label, "__Account");
        assert!(spec.is_builtin_ref);
    }

    #[test]
    fn test_builtin_graph_to_gql() {
        let gql = BuiltinGraph::Ethereum.to_gql();
        assert!(gql.contains("CREATE GRAPH IF NOT EXISTS __ethereum"));
        assert!(gql.contains("NODE __Account"));
        assert!(gql.contains("EDGE __Transfer"));
    }

    #[test]
    fn test_account_properties() {
        let props = BuiltinNodeType::Account.properties();
        assert!(!props.is_empty());

        // Check address is primary key
        let addr_prop = props.iter().find(|p| p.name == "address");
        assert!(addr_prop.is_some());
        assert!(addr_prop.unwrap().is_primary_key);
    }
}
