//! 边定义
//!
//! Web3 场景的边类型：转账、调用、创建、授权

use crate::graph::vertex::VertexId;
use crate::types::{EdgeLabel, PropertyValue, TokenAmount};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 边 ID（全局唯一）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub u64);

impl EdgeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for EdgeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// 边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 边 ID
    id: EdgeId,
    /// 边标签
    label: EdgeLabel,
    /// 源顶点 ID
    src: VertexId,
    /// 目标顶点 ID
    dst: VertexId,
    /// 属性
    properties: HashMap<String, PropertyValue>,
    /// 所在页面 ID
    page_id: Option<u64>,
    /// 页面内偏移
    page_offset: Option<u32>,
}

impl Edge {
    /// 创建新边
    pub fn new(id: EdgeId, label: EdgeLabel, src: VertexId, dst: VertexId) -> Self {
        Self {
            id,
            label,
            src,
            dst,
            properties: HashMap::new(),
            page_id: None,
            page_offset: None,
        }
    }

    /// 创建转账边
    pub fn new_transfer(
        id: EdgeId,
        src: VertexId,
        dst: VertexId,
        amount: TokenAmount,
        block_number: u64,
    ) -> Self {
        let mut e = Self::new(id, EdgeLabel::Transfer, src, dst);
        e.properties
            .insert("amount".to_string(), PropertyValue::TokenAmount(amount));
        e.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        e
    }

    /// 创建合约调用边
    pub fn new_call(
        id: EdgeId,
        src: VertexId,
        dst: VertexId,
        method: String,
        block_number: u64,
    ) -> Self {
        let mut e = Self::new(id, EdgeLabel::Call, src, dst);
        e.properties
            .insert("method".to_string(), PropertyValue::String(method));
        e.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        e
    }

    /// 创建合约创建边
    pub fn new_create(id: EdgeId, src: VertexId, dst: VertexId, block_number: u64) -> Self {
        let mut e = Self::new(id, EdgeLabel::Create, src, dst);
        e.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        e
    }

    /// 创建授权边
    pub fn new_approve(
        id: EdgeId,
        src: VertexId,
        dst: VertexId,
        amount: TokenAmount,
        block_number: u64,
    ) -> Self {
        let mut e = Self::new(id, EdgeLabel::Approve, src, dst);
        e.properties
            .insert("amount".to_string(), PropertyValue::TokenAmount(amount));
        e.properties.insert(
            "block_number".to_string(),
            PropertyValue::Integer(block_number as i64),
        );
        e
    }

    /// 获取边 ID
    pub fn id(&self) -> EdgeId {
        self.id
    }

    /// 获取边标签
    pub fn label(&self) -> &EdgeLabel {
        &self.label
    }

    /// 获取源顶点 ID
    pub fn src(&self) -> VertexId {
        self.src
    }

    /// 获取目标顶点 ID
    pub fn dst(&self) -> VertexId {
        self.dst
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

    /// 获取转账金额
    pub fn amount(&self) -> Option<&TokenAmount> {
        if let Some(PropertyValue::TokenAmount(amt)) = self.properties.get("amount") {
            Some(amt)
        } else {
            None
        }
    }

    /// 获取区块号
    pub fn block_number(&self) -> Option<u64> {
        if let Some(PropertyValue::Integer(n)) = self.properties.get("block_number") {
            Some(*n as u64)
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
        48 + self
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

    /// 获取边的权重（用于最大流算法）
    /// 对于转账边，返回金额；对于其他边返回 1
    pub fn weight(&self) -> f64 {
        if let Some(amt) = self.amount() {
            // 将 U256 转换为 f64（可能会损失精度，但用于最大流算法足够）
            amt.0.low_u64() as f64
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenAmount;

    #[test]
    fn test_edge_transfer() {
        let amount = TokenAmount::from_u64(1000);
        let e = Edge::new_transfer(
            EdgeId::new(1),
            VertexId::new(100),
            VertexId::new(200),
            amount,
            12345678,
        );

        assert_eq!(e.id().as_u64(), 1);
        assert_eq!(e.label(), &EdgeLabel::Transfer);
        assert_eq!(e.src().as_u64(), 100);
        assert_eq!(e.dst().as_u64(), 200);
        assert!(e.amount().is_some());
        assert_eq!(e.block_number(), Some(12345678));
    }

    #[test]
    fn test_edge_serialization() {
        let amount = TokenAmount::from_u64(1000);
        let e = Edge::new_transfer(
            EdgeId::new(1),
            VertexId::new(100),
            VertexId::new(200),
            amount,
            12345678,
        );

        let bytes = e.to_bytes();
        let restored = Edge::from_bytes(&bytes).unwrap();

        assert_eq!(e.id(), restored.id());
        assert_eq!(e.src(), restored.src());
        assert_eq!(e.dst(), restored.dst());
    }

    #[test]
    fn test_edge_weight() {
        let amount = TokenAmount::from_u64(1000);
        let e = Edge::new_transfer(
            EdgeId::new(1),
            VertexId::new(100),
            VertexId::new(200),
            amount,
            12345678,
        );

        assert_eq!(e.weight(), 1000.0);
    }
}
