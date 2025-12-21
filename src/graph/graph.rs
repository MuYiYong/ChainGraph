//! 图数据结构
//!
//! 基于 SSD 存储的图数据库核心

use super::edge::{Edge, EdgeId};
use super::index::{EdgeIndex, VertexIndex};
use super::vertex::{Vertex, VertexId};
use crate::error::{Error, Result};
use crate::storage::BufferPool;
use crate::types::{EdgeLabel, VertexLabel};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 内部存储的 schema 表示（不依赖 query/ast）
#[derive(Debug, Clone)]
pub struct StoredPropertySpec {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StoredGraphSchema {
    /// node label -> properties
    pub node_types: HashMap<String, Vec<StoredPropertySpec>>,
    /// edge label -> properties
    pub edge_types: HashMap<String, Vec<StoredPropertySpec>>,
}

/// 图数据库
pub struct Graph {
    /// 缓冲池
    buffer_pool: Arc<BufferPool>,
    /// 顶点索引
    vertex_index: VertexIndex,
    /// 边索引
    edge_index: EdgeIndex,
    /// 下一个顶点 ID
    next_vertex_id: AtomicU64,
    /// 下一个边 ID
    next_edge_id: AtomicU64,
    /// 顶点缓存（内存中）
    vertex_cache: RwLock<HashMap<VertexId, Vertex>>,
    /// 边缓存（内存中）
    edge_cache: RwLock<HashMap<EdgeId, Edge>>,
    /// 可选的图 schema（由 CREATE GRAPH 保存）
    schema: RwLock<Option<StoredGraphSchema>>,
}

impl Graph {
    /// 打开或创建图数据库
    pub fn open<P: AsRef<Path>>(data_dir: P, buffer_pool_size: Option<usize>) -> Result<Arc<Self>> {
        let buffer_pool = BufferPool::new(data_dir, buffer_pool_size)?;

        Ok(Arc::new(Self {
            buffer_pool,
            vertex_index: VertexIndex::new(),
            edge_index: EdgeIndex::new(),
            next_vertex_id: AtomicU64::new(1),
            next_edge_id: AtomicU64::new(1),
            vertex_cache: RwLock::new(HashMap::new()),
            edge_cache: RwLock::new(HashMap::new()),
            schema: RwLock::new(None),
        }))
    }

    /// 设置图 schema（来自 CREATE GRAPH 的内联 schema）
    pub fn set_schema(&self, s: StoredGraphSchema) {
        *self.schema.write() = Some(s);
    }

    /// 获取当前图的 schema（如果有）
    pub fn get_schema(&self) -> Option<StoredGraphSchema> {
        self.schema.read().clone()
    }

    /// 创建内存图（用于测试）
    pub fn in_memory() -> Result<Arc<Self>> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("chaingraph_{}_{}", std::process::id(), timestamp));
        // 确保创建新目录
        let _ = std::fs::remove_dir_all(&temp_dir);
        Self::open(temp_dir, Some(1024))
    }

    // ==================== 顶点操作 ====================

    /// 添加顶点
    pub fn add_vertex(&self, label: VertexLabel) -> Result<VertexId> {
        let id = VertexId::new(self.next_vertex_id.fetch_add(1, Ordering::SeqCst));
        let vertex = Vertex::new(id, label.clone());

        // 添加到索引
        self.vertex_index.add_label(label, id);

        // 添加到缓存
        self.vertex_cache.write().insert(id, vertex);

        Ok(id)
    }

    /// 添加账户顶点
    pub fn add_account(&self, address: String) -> Result<VertexId> {
        // 检查是否已存在（按字符串地址）
        if let Some(existing_id) = self.vertex_index.get_by_address(&address) {
            return Ok(existing_id);
        }

        let id = VertexId::new(self.next_vertex_id.fetch_add(1, Ordering::SeqCst));
        let vertex = Vertex::new_account(id, address.clone());

        // 添加到索引
        self.vertex_index.add_address(address, id);
        self.vertex_index.add_label(VertexLabel::Account, id);

        // 添加到缓存
        self.vertex_cache.write().insert(id, vertex);

        Ok(id)
    }

    /// 添加合约顶点
    pub fn add_contract(&self, address: String) -> Result<VertexId> {
        if let Some(existing_id) = self.vertex_index.get_by_address(&address) {
            return Ok(existing_id);
        }

        let id = VertexId::new(self.next_vertex_id.fetch_add(1, Ordering::SeqCst));
        let vertex = Vertex::new_contract(id, address.clone());

        self.vertex_index.add_address(address, id);
        self.vertex_index.add_label(VertexLabel::Contract, id);
        self.vertex_cache.write().insert(id, vertex);

        Ok(id)
    }

    /// 获取顶点
    pub fn get_vertex(&self, id: VertexId) -> Option<Vertex> {
        self.vertex_cache.read().get(&id).cloned()
    }

    /// 通过地址获取顶点
    pub fn get_vertex_by_address(&self, address: &str) -> Option<Vertex> {
        let id = self.vertex_index.get_by_address(address)?;
        self.get_vertex(id)
    }

    /// 获取标签下的所有顶点
    pub fn get_vertices_by_label(&self, label: &VertexLabel) -> Vec<Vertex> {
        self.vertex_index
            .get_by_label(label)
            .iter()
            .filter_map(|&id| self.get_vertex(id))
            .collect()
    }

    /// 更新顶点
    pub fn update_vertex(&self, vertex: Vertex) -> Result<()> {
        let id = vertex.id();
        if !self.vertex_cache.read().contains_key(&id) {
            return Err(Error::NotFound(format!("顶点 {:?} 不存在", id)));
        }
        self.vertex_cache.write().insert(id, vertex);
        Ok(())
    }

    /// 删除顶点
    pub fn remove_vertex(&self, id: VertexId) -> Result<()> {
        // 获取顶点信息
        let vertex = self.vertex_cache.write().remove(&id);
        if let Some(v) = vertex {
            self.vertex_index.remove(id, v.address(), Some(v.label()));
        }

        // 删除相关的边
        let outgoing = self.edge_index.get_outgoing(id);
        let incoming = self.edge_index.get_incoming(id);

        for edge_id in outgoing.into_iter().chain(incoming.into_iter()) {
            self.remove_edge(edge_id)?;
        }

        Ok(())
    }

    /// 获取顶点数量
    pub fn vertex_count(&self) -> usize {
        self.vertex_cache.read().len()
    }

    // ==================== 边操作 ====================

    /// 添加边
    pub fn add_edge(&self, label: EdgeLabel, src: VertexId, dst: VertexId) -> Result<EdgeId> {
        // 验证顶点存在
        if !self.vertex_cache.read().contains_key(&src) {
            return Err(Error::NotFound(format!("源顶点 {:?} 不存在", src)));
        }
        if !self.vertex_cache.read().contains_key(&dst) {
            return Err(Error::NotFound(format!("目标顶点 {:?} 不存在", dst)));
        }

        let id = EdgeId::new(self.next_edge_id.fetch_add(1, Ordering::SeqCst));
        let edge = Edge::new(id, label.clone(), src, dst);

        // 添加到索引
        self.edge_index.add_edge(id, src, dst, label);

        // 添加到缓存
        self.edge_cache.write().insert(id, edge);

        Ok(id)
    }

    /// 添加转账边
    pub fn add_transfer(
        &self,
        src: VertexId,
        dst: VertexId,
        amount: crate::types::TokenAmount,
        block_number: u64,
    ) -> Result<EdgeId> {
        if !self.vertex_cache.read().contains_key(&src) {
            return Err(Error::NotFound(format!("源顶点 {:?} 不存在", src)));
        }
        if !self.vertex_cache.read().contains_key(&dst) {
            return Err(Error::NotFound(format!("目标顶点 {:?} 不存在", dst)));
        }

        let id = EdgeId::new(self.next_edge_id.fetch_add(1, Ordering::SeqCst));
        let edge = Edge::new_transfer(id, src, dst, amount, block_number);

        self.edge_index.add_edge(id, src, dst, EdgeLabel::Transfer);
        self.edge_cache.write().insert(id, edge);

        Ok(id)
    }

    /// 获取边
    pub fn get_edge(&self, id: EdgeId) -> Option<Edge> {
        self.edge_cache.read().get(&id).cloned()
    }

    /// 获取两点之间的所有边
    pub fn get_edges_between(&self, src: VertexId, dst: VertexId) -> Vec<Edge> {
        self.edge_index
            .get_edges_between(src, dst)
            .iter()
            .filter_map(|&id| self.get_edge(id))
            .collect()
    }

    /// 获取顶点的所有出边
    pub fn get_outgoing_edges(&self, vertex_id: VertexId) -> Vec<Edge> {
        self.edge_index
            .get_outgoing(vertex_id)
            .iter()
            .filter_map(|&id| self.get_edge(id))
            .collect()
    }

    /// 获取顶点的所有入边
    pub fn get_incoming_edges(&self, vertex_id: VertexId) -> Vec<Edge> {
        self.edge_index
            .get_incoming(vertex_id)
            .iter()
            .filter_map(|&id| self.get_edge(id))
            .collect()
    }

    /// 获取标签下的所有边
    pub fn get_edges_by_label(&self, label: &EdgeLabel) -> Vec<Edge> {
        self.edge_index
            .get_by_label(label)
            .iter()
            .filter_map(|&id| self.get_edge(id))
            .collect()
    }

    /// 更新边
    pub fn update_edge(&self, edge: Edge) -> Result<()> {
        let id = edge.id();
        if !self.edge_cache.read().contains_key(&id) {
            return Err(Error::NotFound(format!("边 {:?} 不存在", id)));
        }
        self.edge_cache.write().insert(id, edge);
        Ok(())
    }

    /// 删除边
    pub fn remove_edge(&self, id: EdgeId) -> Result<()> {
        let edge = self.edge_cache.write().remove(&id);
        if let Some(e) = edge {
            self.edge_index.remove(id, Some(e.label()));
        }
        Ok(())
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edge_cache.read().len()
    }

    // ==================== 邻居查询 ====================

    /// 获取顶点的邻居（出边指向的顶点）
    pub fn neighbors(&self, vertex_id: VertexId) -> Vec<VertexId> {
        self.edge_index.neighbors(vertex_id)
    }

    /// 获取顶点的前驱（入边来源的顶点）
    pub fn predecessors(&self, vertex_id: VertexId) -> Vec<VertexId> {
        self.edge_index.predecessors(vertex_id)
    }

    /// 获取顶点的出度
    pub fn out_degree(&self, vertex_id: VertexId) -> usize {
        self.edge_index.out_degree(vertex_id)
    }

    /// 获取顶点的入度
    pub fn in_degree(&self, vertex_id: VertexId) -> usize {
        self.edge_index.in_degree(vertex_id)
    }

    // ==================== 持久化 ====================

    /// 刷新到磁盘
    pub fn flush(&self) -> Result<()> {
        self.buffer_pool.flush_all()
    }

    /// 获取缓冲池引用
    pub fn buffer_pool(&self) -> &Arc<BufferPool> {
        &self.buffer_pool
    }

    /// 获取边索引引用
    pub fn edge_index(&self) -> &EdgeIndex {
        &self.edge_index
    }

    /// 获取顶点索引引用  
    pub fn vertex_index(&self) -> &VertexIndex {
        &self.vertex_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenAmount;

    #[test]
    fn test_graph_basic() {
        let graph = Graph::in_memory().unwrap();

        // 添加账户
        let addr1 = Address::from_hex("0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0").unwrap();
        let addr2 = Address::from_hex("0x8ba1f109551bD432803012645Ac136ddd64DBA72").unwrap();

        let v1 = graph.add_account(addr1.clone()).unwrap();
        let v2 = graph.add_account(addr2.clone()).unwrap();

        assert_eq!(graph.vertex_count(), 2);

        // 添加转账边
        let amount = TokenAmount::from_u64(1000);
        let e1 = graph.add_transfer(v1, v2, amount, 12345678).unwrap();

        assert_eq!(graph.edge_count(), 1);

        // 查询
        let v = graph.get_vertex_by_address(&addr1).unwrap();
        assert_eq!(v.id(), v1);

        let edges = graph.get_edges_between(v1, v2);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].id(), e1);

        // 邻居
        assert_eq!(graph.neighbors(v1), vec![v2]);
        assert_eq!(graph.predecessors(v2), vec![v1]);
    }

    #[test]
    fn test_graph_degrees() {
        let graph = Graph::in_memory().unwrap();

        let v1 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v2 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v3 = graph.add_vertex(VertexLabel::Account).unwrap();

        graph.add_edge(EdgeLabel::Transfer, v1, v2).unwrap();
        graph.add_edge(EdgeLabel::Transfer, v1, v3).unwrap();
        graph.add_edge(EdgeLabel::Transfer, v2, v3).unwrap();

        assert_eq!(graph.out_degree(v1), 2);
        assert_eq!(graph.in_degree(v3), 2);
    }
}
