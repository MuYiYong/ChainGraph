//! Web3 特定类型和通用类型定义

use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 顶点 ID (64位整数，便于磁盘存储和索引)
pub type VertexId = u64;

/// 边 ID
pub type EdgeId = u64;

/// 页面 ID
pub type PageId = u64;

/// 区块高度
pub type BlockNumber = u64;

/// 以太坊地址 (20 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub H160);

impl Address {
    pub fn from_hex(s: &str) -> Result<Self, crate::Error> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| crate::Error::InvalidAddress(e.to_string()))?;
        if bytes.len() != 20 {
            return Err(crate::Error::InvalidAddress(format!(
                "地址长度应为 20 字节, 实际为 {} 字节",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Address(H160::from(arr)))
    }

    pub fn to_hex(&self) -> String {
        format!("0x{:x}", self.0)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

/// 交易哈希 (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub H256);

impl TxHash {
    pub fn from_hex(s: &str) -> Result<Self, crate::Error> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| crate::Error::InvalidTxHash(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(crate::Error::InvalidTxHash(format!(
                "交易哈希长度应为 32 字节, 实际为 {} 字节",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(TxHash(H256::from(arr)))
    }

    pub fn to_hex(&self) -> String {
        format!("0x{:x}", self.0)
    }
}

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

/// 代币数量 (256位大整数)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAmount(pub U256);

impl TokenAmount {
    pub fn from_u64(v: u64) -> Self {
        TokenAmount(U256::from(v))
    }

    pub fn from_str_radix(s: &str, radix: u32) -> Result<Self, crate::Error> {
        U256::from_str_radix(s, radix)
            .map(TokenAmount)
            .map_err(|e| crate::Error::InternalError(e.to_string()))
    }
}

/// 属性值
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Boolean(bool),
    Int(i64),
    Integer(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Address(Address),
    TxHash(TxHash),
    Amount(TokenAmount),
    TokenAmount(TokenAmount),
    BlockNumber(BlockNumber),
    Bytes(Vec<u8>),
    List(Vec<PropertyValue>),
    Map(HashMap<String, PropertyValue>),
    Timestamp(i64),
}

impl PropertyValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyValue::Null => "null",
            PropertyValue::Bool(_) | PropertyValue::Boolean(_) => "bool",
            PropertyValue::Int(_) | PropertyValue::Integer(_) => "int",
            PropertyValue::UInt(_) => "uint",
            PropertyValue::Float(_) => "float",
            PropertyValue::String(_) => "string",
            PropertyValue::Address(_) => "address",
            PropertyValue::TxHash(_) => "txhash",
            PropertyValue::Amount(_) | PropertyValue::TokenAmount(_) => "amount",
            PropertyValue::BlockNumber(_) => "blocknumber",
            PropertyValue::Bytes(_) => "bytes",
            PropertyValue::List(_) => "list",
            PropertyValue::Map(_) => "map",
            PropertyValue::Timestamp(_) => "timestamp",
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            PropertyValue::Int(v) => Some(*v),
            PropertyValue::UInt(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_address(&self) -> Option<&Address> {
        match self {
            PropertyValue::Address(v) => Some(v),
            _ => None,
        }
    }
}

impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self {
        PropertyValue::Int(v)
    }
}

impl From<u64> for PropertyValue {
    fn from(v: u64) -> Self {
        PropertyValue::UInt(v)
    }
}

impl From<String> for PropertyValue {
    fn from(v: String) -> Self {
        PropertyValue::String(v)
    }
}

impl From<&str> for PropertyValue {
    fn from(v: &str) -> Self {
        PropertyValue::String(v.to_string())
    }
}

impl From<Address> for PropertyValue {
    fn from(v: Address) -> Self {
        PropertyValue::Address(v)
    }
}

impl From<TxHash> for PropertyValue {
    fn from(v: TxHash) -> Self {
        PropertyValue::TxHash(v)
    }
}

/// 属性映射
pub type Properties = HashMap<String, PropertyValue>;

/// 遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::Both
    }
}

/// 顶点类型标签
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertexLabel {
    /// 外部账户 (EOA)
    Account,
    /// 智能合约
    Contract,
    /// 代币合约
    Token,
    /// 交易
    Transaction,
    /// 区块
    Block,
    /// 自定义标签
    Custom(String),
}

impl VertexLabel {
    pub fn as_str(&self) -> &str {
        match self {
            VertexLabel::Account => "Account",
            VertexLabel::Contract => "Contract",
            VertexLabel::Token => "Token",
            VertexLabel::Transaction => "Transaction",
            VertexLabel::Block => "Block",
            VertexLabel::Custom(s) => s,
        }
    }
}

impl fmt::Display for VertexLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 边类型标签
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeLabel {
    /// 转账
    Transfer,
    /// 调用合约
    Call,
    /// 创建合约
    Create,
    /// 授权
    Approve,
    /// 包含在区块中
    InBlock,
    /// 自定义标签
    Custom(String),
}

impl EdgeLabel {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeLabel::Transfer => "Transfer",
            EdgeLabel::Call => "Call",
            EdgeLabel::Create => "Create",
            EdgeLabel::Approve => "Approve",
            EdgeLabel::InBlock => "InBlock",
            EdgeLabel::Custom(s) => s,
        }
    }
}

impl fmt::Display for EdgeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_parsing() {
        let addr = Address::from_hex("0x742d35Cc6634C0532925a3b844Bc9e7595f5bB01").unwrap();
        assert_eq!(
            addr.to_hex().to_lowercase(),
            "0x742d35cc6634c0532925a3b844bc9e7595f5bb01"
        );
    }

    #[test]
    fn test_tx_hash_parsing() {
        let hash =
            TxHash::from_hex("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060")
                .unwrap();
        assert_eq!(
            hash.to_hex().to_lowercase(),
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
        );
    }
}
