//! 顶点定义
//!
//! Web3 场景的顶点类型：账户、合约、代币、交易、区块

use crate::types::{PropertyValue, TxHash, VertexLabel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 顶点 ID（全局唯一）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VertexId(pub u64);

impl VertexId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for VertexId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// 顶点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    /// 顶点 ID
    id: VertexId,
    /// 顶点标签
    label: VertexLabel,
    /// 属性
    properties: HashMap<String, PropertyValue>,
    /// 所在页面 ID（用于持久化）
    page_id: Option<u64>,
    /// 页面内偏移
    page_offset: Option<u32>,
}

impl Vertex {
    /// 创建新顶点
    pub fn new(id: VertexId, label: VertexLabel) -> Self {
        Self {
            id,
            label,
            properties: HashMap::new(),
            page_id: None,
            page_offset: None,
        }
    }

    /// 创建账户顶点
    pub fn new_account(id: VertexId, address: String) -> Self {
        let mut v = Self::new(id, VertexLabel::Account);
        v.properties
            .insert("address".to_string(), PropertyValue::String(address));
        v
    }

    /// 创建合约顶点
    pub fn new_contract(id: VertexId, address: String) -> Self {
        let mut v = Self::new(id, VertexLabel::Contract);
        v.properties
            .insert("address".to_string(), PropertyValue::String(address));
        v
    }

    /// 创建代币顶点
    pub fn new_token(id: VertexId, address: String, symbol: String) -> Self {
        let mut v = Self::new(id, VertexLabel::Token);
        v.properties
            .insert("address".to_string(), PropertyValue::String(address));
        v.properties
            .insert("symbol".to_string(), PropertyValue::String(symbol));
        v
    }

    /// 创建交易顶点
    pub fn new_transaction(id: VertexId, tx_hash: TxHash, block_number: u64) -> Self {
        let mut v = Self::new(id, VertexLabel::Transaction);
        v.properties
            .insert("tx_hash".to_string(), PropertyValue::TxHash(tx_hash));
        v.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        v
    }

    /// 创建区块顶点
    pub fn new_block(id: VertexId, block_number: u64, block_hash: TxHash) -> Self {
        let mut v = Self::new(id, VertexLabel::Block);
        v.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        v.properties
            .insert("block_hash".to_string(), PropertyValue::TxHash(block_hash));
        v
    }

    /// 获取顶点 ID
    pub fn id(&self) -> VertexId {
        self.id
    }

    /// 获取顶点标签
    pub fn label(&self) -> &VertexLabel {
        &self.label
    }

    /// 获取属性
    pub fn property(&self, key: &str) -> Option<&PropertyValue> {
        self.properties.get(key)
    }

    /// 设置属性
    pub fn set_property(&mut self, key: String, value: PropertyValue) {
        self.properties.insert(key, value);
    }

    /// 移除属性
    pub fn remove_property(&mut self, key: &str) -> Option<PropertyValue> {
        self.properties.remove(key)
    }

    /// 获取所有属性
    pub fn properties(&self) -> &HashMap<String, PropertyValue> {
        &self.properties
    }

    /// 获取地址（如果是账户/合约/代币类型）
    pub fn address(&self) -> Option<&str> {
        if let Some(PropertyValue::String(s)) = self.properties.get("address") {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// 设置页面位置
    pub fn set_page_location(&mut self, page_id: u64, offset: u32) {
        self.page_id = Some(page_id);
        self.page_offset = Some(offset);
    }

    /// 获取页面位置
    pub fn page_location(&self) -> Option<(u64, u32)> {
        match (self.page_id, self.page_offset) {
            (Some(page_id), Some(offset)) => Some((page_id, offset)),
            _ => None,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// 从字节反序列化
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }

    /// 估算字节大小
    pub fn size_estimate(&self) -> usize {
        // 基础大小 + 属性大小
        32 + self
            .properties
            .iter()
            .map(|(k, v)| {
                k.len()
                    + match v {
                        PropertyValue::String(s) => s.len() + 8,
                        PropertyValue::Bytes(b) => b.len() + 8,
                        _ => 32,
                    }
            })
            .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_account() {
        let v = Vertex::new_account(VertexId::new(1), "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0".to_string());

        assert_eq!(v.id().as_u64(), 1);
        assert_eq!(v.label(), &VertexLabel::Account);
        assert!(v.address().is_some());
    }

    #[test]
    fn test_vertex_serialization() {
        let v = Vertex::new_account(VertexId::new(1), "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0".to_string());

        let bytes = v.to_bytes();
        let restored = Vertex::from_bytes(&bytes).unwrap();

        assert_eq!(v.id(), restored.id());
        assert_eq!(v.label(), restored.label());
    }
}
