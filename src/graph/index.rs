//! 图索引
//!
//! 顶点和边的内存索引，支持快速查找

use crate::graph::edge::EdgeId;
use crate::graph::vertex::VertexId;
use crate::types::{Address, EdgeLabel, VertexLabel};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 顶点索引
pub struct VertexIndex {
    /// 地址到顶点 ID 的映射
    address_to_id: RwLock<HashMap<Address, VertexId>>,
    /// 标签到顶点 ID 集合的映射
    label_to_ids: RwLock<HashMap<VertexLabel, HashSet<VertexId>>>,
    /// 顶点 ID 到页面位置的映射
    id_to_location: RwLock<HashMap<VertexId, (u64, u32)>>,
}

impl VertexIndex {
    /// 创建新索引
    pub fn new() -> Self {
        Self {
            address_to_id: RwLock::new(HashMap::new()),
            label_to_ids: RwLock::new(HashMap::new()),
            id_to_location: RwLock::new(HashMap::new()),
        }
    }

    /// 添加地址索引
    pub fn add_address(&self, address: Address, vertex_id: VertexId) {
        self.address_to_id.write().insert(address, vertex_id);
    }

    /// 通过地址查找顶点
    pub fn get_by_address(&self, address: &Address) -> Option<VertexId> {
        self.address_to_id.read().get(address).copied()
    }

    /// 添加标签索引
    pub fn add_label(&self, label: VertexLabel, vertex_id: VertexId) {
        self.label_to_ids
            .write()
            .entry(label)
            .or_insert_with(HashSet::new)
            .insert(vertex_id);
    }

    /// 获取标签下的所有顶点
    pub fn get_by_label(&self, label: &VertexLabel) -> Vec<VertexId> {
        self.label_to_ids
            .read()
            .get(label)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 设置页面位置
    pub fn set_location(&self, vertex_id: VertexId, page_id: u64, offset: u32) {
        self.id_to_location
            .write()
            .insert(vertex_id, (page_id, offset));
    }

    /// 获取页面位置
    pub fn get_location(&self, vertex_id: VertexId) -> Option<(u64, u32)> {
        self.id_to_location.read().get(&vertex_id).copied()
    }

    /// 移除顶点
    pub fn remove(
        &self,
        vertex_id: VertexId,
        address: Option<&Address>,
        label: Option<&VertexLabel>,
    ) {
        if let Some(addr) = address {
            self.address_to_id.write().remove(addr);
        }
        if let Some(lbl) = label {
            if let Some(set) = self.label_to_ids.write().get_mut(lbl) {
                set.remove(&vertex_id);
            }
        }
        self.id_to_location.write().remove(&vertex_id);
    }

    /// 获取顶点数量
    pub fn vertex_count(&self) -> usize {
        self.id_to_location.read().len()
    }
}

impl Default for VertexIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 边索引
pub struct EdgeIndex {
    /// 源顶点到出边的映射
    outgoing: RwLock<HashMap<VertexId, Vec<EdgeId>>>,
    /// 目标顶点到入边的映射
    incoming: RwLock<HashMap<VertexId, Vec<EdgeId>>>,
    /// 边标签到边 ID 集合的映射
    label_to_ids: RwLock<HashMap<EdgeLabel, HashSet<EdgeId>>>,
    /// 边 ID 到 (src, dst) 的映射
    edge_endpoints: RwLock<HashMap<EdgeId, (VertexId, VertexId)>>,
    /// 边 ID 到页面位置的映射
    id_to_location: RwLock<HashMap<EdgeId, (u64, u32)>>,
    /// (src, dst) 到边 ID 列表的映射（支持多重边）
    pair_to_edges: RwLock<HashMap<(VertexId, VertexId), Vec<EdgeId>>>,
}

impl EdgeIndex {
    /// 创建新索引
    pub fn new() -> Self {
        Self {
            outgoing: RwLock::new(HashMap::new()),
            incoming: RwLock::new(HashMap::new()),
            label_to_ids: RwLock::new(HashMap::new()),
            edge_endpoints: RwLock::new(HashMap::new()),
            id_to_location: RwLock::new(HashMap::new()),
            pair_to_edges: RwLock::new(HashMap::new()),
        }
    }

    /// 添加边
    pub fn add_edge(&self, edge_id: EdgeId, src: VertexId, dst: VertexId, label: EdgeLabel) {
        // 出边索引
        self.outgoing
            .write()
            .entry(src)
            .or_insert_with(Vec::new)
            .push(edge_id);

        // 入边索引
        self.incoming
            .write()
            .entry(dst)
            .or_insert_with(Vec::new)
            .push(edge_id);

        // 标签索引
        self.label_to_ids
            .write()
            .entry(label)
            .or_insert_with(HashSet::new)
            .insert(edge_id);

        // 端点映射
        self.edge_endpoints.write().insert(edge_id, (src, dst));

        // 点对映射
        self.pair_to_edges
            .write()
            .entry((src, dst))
            .or_insert_with(Vec::new)
            .push(edge_id);
    }

    /// 获取顶点的出边
    pub fn get_outgoing(&self, vertex_id: VertexId) -> Vec<EdgeId> {
        self.outgoing
            .read()
            .get(&vertex_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取顶点的入边
    pub fn get_incoming(&self, vertex_id: VertexId) -> Vec<EdgeId> {
        self.incoming
            .read()
            .get(&vertex_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取边的端点
    pub fn get_endpoints(&self, edge_id: EdgeId) -> Option<(VertexId, VertexId)> {
        self.edge_endpoints.read().get(&edge_id).copied()
    }

    /// 获取两点之间的所有边
    pub fn get_edges_between(&self, src: VertexId, dst: VertexId) -> Vec<EdgeId> {
        self.pair_to_edges
            .read()
            .get(&(src, dst))
            .cloned()
            .unwrap_or_default()
    }

    /// 获取标签下的所有边
    pub fn get_by_label(&self, label: &EdgeLabel) -> Vec<EdgeId> {
        self.label_to_ids
            .read()
            .get(label)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 设置页面位置
    pub fn set_location(&self, edge_id: EdgeId, page_id: u64, offset: u32) {
        self.id_to_location
            .write()
            .insert(edge_id, (page_id, offset));
    }

    /// 获取页面位置
    pub fn get_location(&self, edge_id: EdgeId) -> Option<(u64, u32)> {
        self.id_to_location.read().get(&edge_id).copied()
    }

    /// 移除边
    pub fn remove(&self, edge_id: EdgeId, label: Option<&EdgeLabel>) {
        if let Some((src, dst)) = self.edge_endpoints.write().remove(&edge_id) {
            // 从出边列表移除
            if let Some(edges) = self.outgoing.write().get_mut(&src) {
                edges.retain(|&id| id != edge_id);
            }
            // 从入边列表移除
            if let Some(edges) = self.incoming.write().get_mut(&dst) {
                edges.retain(|&id| id != edge_id);
            }
            // 从点对映射移除
            if let Some(edges) = self.pair_to_edges.write().get_mut(&(src, dst)) {
                edges.retain(|&id| id != edge_id);
            }
        }

        if let Some(lbl) = label {
            if let Some(set) = self.label_to_ids.write().get_mut(lbl) {
                set.remove(&edge_id);
            }
        }

        self.id_to_location.write().remove(&edge_id);
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edge_endpoints.read().len()
    }

    /// 获取顶点的出度
    pub fn out_degree(&self, vertex_id: VertexId) -> usize {
        self.outgoing
            .read()
            .get(&vertex_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 获取顶点的入度
    pub fn in_degree(&self, vertex_id: VertexId) -> usize {
        self.incoming
            .read()
            .get(&vertex_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 获取邻居（出边指向的顶点）
    pub fn neighbors(&self, vertex_id: VertexId) -> Vec<VertexId> {
        self.get_outgoing(vertex_id)
            .iter()
            .filter_map(|&edge_id| self.get_endpoints(edge_id).map(|(_, dst)| dst))
            .collect()
    }

    /// 获取前驱（入边来源的顶点）
    pub fn predecessors(&self, vertex_id: VertexId) -> Vec<VertexId> {
        self.get_incoming(vertex_id)
            .iter()
            .filter_map(|&edge_id| self.get_endpoints(edge_id).map(|(src, _)| src))
            .collect()
    }
}

impl Default for EdgeIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_index() {
        let index = VertexIndex::new();
        let addr = Address::from_hex("0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0").unwrap();
        let vid = VertexId::new(1);

        index.add_address(addr.clone(), vid);
        index.add_label(VertexLabel::Account, vid);
        index.set_location(vid, 10, 100);

        assert_eq!(index.get_by_address(&addr), Some(vid));
        assert_eq!(index.get_by_label(&VertexLabel::Account), vec![vid]);
        assert_eq!(index.get_location(vid), Some((10, 100)));
    }

    #[test]
    fn test_edge_index() {
        let index = EdgeIndex::new();
        let eid = EdgeId::new(1);
        let src = VertexId::new(100);
        let dst = VertexId::new(200);

        index.add_edge(eid, src, dst, EdgeLabel::Transfer);

        assert_eq!(index.get_outgoing(src), vec![eid]);
        assert_eq!(index.get_incoming(dst), vec![eid]);
        assert_eq!(index.get_endpoints(eid), Some((src, dst)));
        assert_eq!(index.get_edges_between(src, dst), vec![eid]);
        assert_eq!(index.neighbors(src), vec![dst]);
        assert_eq!(index.predecessors(dst), vec![src]);
    }
}
