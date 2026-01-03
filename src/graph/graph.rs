//! 图数据结构
//!
//! 基于 SSD 存储的图数据库核心，支持数据持久化

use super::edge::{Edge, EdgeId};
use super::index::{EdgeIndex, VertexIndex};
use super::vertex::{Vertex, VertexId};
use crate::error::{Error, Result};
use crate::storage::{BufferPool, PageType};
use crate::types::{EdgeLabel, VertexLabel};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Meta 页面 ID（动态分配，存储在图结构中）
/// 也可以存储在 GraphMeta 中

/// 页面数据区可用大小（约 4060 字节）
const PAGE_DATA_SIZE: usize = 4060;

/// 内部存储的 schema 表示（不依赖 query/ast）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPropertySpec {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredGraphSchema {
    /// node label -> properties
    pub node_types: HashMap<String, Vec<StoredPropertySpec>>,
    /// edge label -> properties
    pub edge_types: HashMap<String, Vec<StoredPropertySpec>>,
}

/// 图元数据（存储在 Meta 页面中）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphMeta {
    /// Meta 页面自身的 ID（用于重新加载时定位）
    meta_page_id: u64,
    /// 下一个顶点 ID
    next_vertex_id: u64,
    /// 下一个边 ID
    next_edge_id: u64,
    /// 顶点页面列表
    vertex_pages: Vec<u64>,
    /// 边页面列表
    edge_pages: Vec<u64>,
    /// 图 schema
    schema: Option<StoredGraphSchema>,
}

impl Default for GraphMeta {
    fn default() -> Self {
        Self {
            meta_page_id: 0, // 0 表示尚未分配
            next_vertex_id: 1,
            next_edge_id: 1,
            vertex_pages: Vec::new(),
            edge_pages: Vec::new(),
            schema: None,
        }
    }
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
    /// 顶点页面列表
    vertex_pages: RwLock<Vec<u64>>,
    /// 边页面列表
    edge_pages: RwLock<Vec<u64>>,
    /// 当前顶点页面的剩余空间
    current_vertex_page_space: RwLock<usize>,
    /// 当前边页面的剩余空间
    current_edge_page_space: RwLock<usize>,
    /// 是否有未保存的更改
    dirty: RwLock<bool>,
    /// Meta 页面 ID
    meta_page_id: RwLock<u64>,
}

impl Graph {
    /// 打开或创建图数据库
    pub fn open<P: AsRef<Path>>(data_dir: P, buffer_pool_size: Option<usize>) -> Result<Arc<Self>> {
        let buffer_pool = BufferPool::new(data_dir, buffer_pool_size)?;

        // 尝试加载已有的元数据
        let meta = Self::load_meta_from_pool(&buffer_pool)?;

        let graph = Arc::new(Self {
            buffer_pool,
            vertex_index: VertexIndex::new(),
            edge_index: EdgeIndex::new(),
            next_vertex_id: AtomicU64::new(meta.next_vertex_id),
            next_edge_id: AtomicU64::new(meta.next_edge_id),
            vertex_cache: RwLock::new(HashMap::new()),
            edge_cache: RwLock::new(HashMap::new()),
            schema: RwLock::new(meta.schema),
            vertex_pages: RwLock::new(meta.vertex_pages),
            edge_pages: RwLock::new(meta.edge_pages),
            current_vertex_page_space: RwLock::new(0),
            current_edge_page_space: RwLock::new(0),
            dirty: RwLock::new(false),
            meta_page_id: RwLock::new(meta.meta_page_id),
        });

        // 加载所有顶点和边
        graph.load_all_data()?;

        Ok(graph)
    }

    /// 从缓冲池加载元数据
    fn load_meta_from_pool(buffer_pool: &Arc<BufferPool>) -> Result<GraphMeta> {
        // 扫描前几个页面寻找 Meta 页面
        // 通常 Meta 页面是第一个分配的页面，所以从 1 开始扫描
        for page_id in 1..=16 {
            if let Ok(handle) = buffer_pool.fetch_page(page_id) {
                let guard = handle.read();
                if let Some(page) = guard.page() {
                    if page.page_type == PageType::Meta {
                        // 读取数据长度（前 4 字节）
                        if page.free_offset >= 4 {
                            let len =
                                u32::from_le_bytes(page.data[0..4].try_into().unwrap()) as usize;
                            if len > 0 && len + 4 <= page.data.len() {
                                if let Ok(meta) =
                                    bincode::deserialize::<GraphMeta>(&page.data[4..4 + len])
                                {
                                    return Ok(meta);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Meta 页面不存在，返回默认值
        Ok(GraphMeta::default())
    }

    /// 加载所有顶点和边数据
    fn load_all_data(&self) -> Result<()> {
        // 加载所有顶点
        let vertex_pages = self.vertex_pages.read().clone();
        for &page_id in &vertex_pages {
            self.load_vertices_from_page(page_id)?;
        }

        // 加载所有边
        let edge_pages = self.edge_pages.read().clone();
        for &page_id in &edge_pages {
            self.load_edges_from_page(page_id)?;
        }

        // 更新当前页面的剩余空间
        if let Some(&last_vertex_page) = vertex_pages.last() {
            if let Ok(handle) = self.buffer_pool.fetch_page(last_vertex_page) {
                let guard = handle.read();
                if let Some(page) = guard.page() {
                    *self.current_vertex_page_space.write() =
                        PAGE_DATA_SIZE.saturating_sub(page.free_offset as usize);
                }
            }
        }

        if let Some(&last_edge_page) = edge_pages.last() {
            if let Ok(handle) = self.buffer_pool.fetch_page(last_edge_page) {
                let guard = handle.read();
                if let Some(page) = guard.page() {
                    *self.current_edge_page_space.write() =
                        PAGE_DATA_SIZE.saturating_sub(page.free_offset as usize);
                }
            }
        }

        Ok(())
    }

    /// 从页面加载顶点
    fn load_vertices_from_page(&self, page_id: u64) -> Result<()> {
        let handle = self.buffer_pool.fetch_page(page_id)?;
        let guard = handle.read();

        if let Some(page) = guard.page() {
            let mut offset = 0;
            while offset + 4 <= page.free_offset as usize {
                // 读取条目长度
                let entry_len =
                    u32::from_le_bytes(page.data[offset..offset + 4].try_into().unwrap()) as usize;

                if entry_len == 0 || offset + 4 + entry_len > page.free_offset as usize {
                    break;
                }

                // 反序列化顶点
                if let Some(vertex) =
                    Vertex::from_bytes(&page.data[offset + 4..offset + 4 + entry_len])
                {
                    let id = vertex.id();
                    // 更新索引
                    self.vertex_index
                        .add_label(vertex.label().clone(), id);
                    if let Some(addr) = vertex.address() {
                        self.vertex_index.add_address(addr.to_string(), id);
                    }
                    // 添加到缓存
                    self.vertex_cache.write().insert(id, vertex);
                }

                offset += 4 + entry_len;
            }
        }

        Ok(())
    }

    /// 从页面加载边
    fn load_edges_from_page(&self, page_id: u64) -> Result<()> {
        let handle = self.buffer_pool.fetch_page(page_id)?;
        let guard = handle.read();

        if let Some(page) = guard.page() {
            let mut offset = 0;
            while offset + 4 <= page.free_offset as usize {
                // 读取条目长度
                let entry_len =
                    u32::from_le_bytes(page.data[offset..offset + 4].try_into().unwrap()) as usize;

                if entry_len == 0 || offset + 4 + entry_len > page.free_offset as usize {
                    break;
                }

                // 反序列化边
                if let Some(edge) = Edge::from_bytes(&page.data[offset + 4..offset + 4 + entry_len])
                {
                    let id = edge.id();
                    // 更新索引
                    self.edge_index
                        .add_edge(id, edge.src(), edge.dst(), edge.label().clone());
                    // 添加到缓存
                    self.edge_cache.write().insert(id, edge);
                }

                offset += 4 + entry_len;
            }
        }

        Ok(())
    }

    /// 保存元数据到磁盘
    fn save_meta(&self) -> Result<()> {
        let mut current_meta_page_id = *self.meta_page_id.read();
        
        // 获取或创建 Meta 页面
        let handle = if current_meta_page_id == 0 {
            // 需要分配新的 Meta 页面
            let h = self.buffer_pool.new_page(PageType::Meta)?;
            current_meta_page_id = h.page_id();
            *self.meta_page_id.write() = current_meta_page_id;
            h
        } else {
            // 使用已有的 Meta 页面
            self.buffer_pool.fetch_page(current_meta_page_id)?
        };

        let meta = GraphMeta {
            meta_page_id: current_meta_page_id,
            next_vertex_id: self.next_vertex_id.load(Ordering::SeqCst),
            next_edge_id: self.next_edge_id.load(Ordering::SeqCst),
            vertex_pages: self.vertex_pages.read().clone(),
            edge_pages: self.edge_pages.read().clone(),
            schema: self.schema.read().clone(),
        };

        let data = bincode::serialize(&meta)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        {
            let mut guard = handle.write();
            if let Some(page) = guard.page_mut() {
                // 清空页面数据
                page.data.fill(0);
                page.free_offset = 0;
                page.item_count = 0;
                page.page_type = PageType::Meta;

                // 写入数据长度和数据
                let len = data.len() as u32;
                page.data[0..4].copy_from_slice(&len.to_le_bytes());
                page.data[4..4 + data.len()].copy_from_slice(&data);
                page.free_offset = (4 + data.len()) as u16;
                page.is_dirty = true;
            }
        }

        handle.mark_dirty();
        Ok(())
    }

    /// 将顶点写入磁盘页面
    fn write_vertex_to_disk(&self, vertex: &Vertex) -> Result<()> {
        let data = vertex.to_bytes();
        let entry_size = 4 + data.len(); // 4 字节长度 + 数据

        let mut current_space = self.current_vertex_page_space.write();
        let mut vertex_pages = self.vertex_pages.write();

        // 检查当前页面是否有足够空间
        if *current_space < entry_size || vertex_pages.is_empty() {
            // 需要新页面
            let handle = self.buffer_pool.new_page(PageType::Vertex)?;
            let page_id = handle.page_id();
            vertex_pages.push(page_id);
            *current_space = PAGE_DATA_SIZE;
            handle.mark_dirty();
        }

        // 写入到当前页面
        let page_id = *vertex_pages.last().unwrap();
        let handle = self.buffer_pool.fetch_page(page_id)?;

        {
            let mut guard = handle.write();
            if let Some(page) = guard.page_mut() {
                let offset = page.free_offset as usize;

                // 写入长度前缀
                let len = data.len() as u32;
                page.data[offset..offset + 4].copy_from_slice(&len.to_le_bytes());

                // 写入数据
                page.data[offset + 4..offset + 4 + data.len()].copy_from_slice(&data);

                page.free_offset += entry_size as u16;
                page.item_count += 1;
                page.is_dirty = true;
            }
        }

        *current_space -= entry_size;
        handle.mark_dirty();
        *self.dirty.write() = true;

        Ok(())
    }

    /// 将边写入磁盘页面
    fn write_edge_to_disk(&self, edge: &Edge) -> Result<()> {
        let data = edge.to_bytes();
        let entry_size = 4 + data.len();

        let mut current_space = self.current_edge_page_space.write();
        let mut edge_pages = self.edge_pages.write();

        // 检查当前页面是否有足够空间
        if *current_space < entry_size || edge_pages.is_empty() {
            // 需要新页面
            let handle = self.buffer_pool.new_page(PageType::Edge)?;
            let page_id = handle.page_id();
            edge_pages.push(page_id);
            *current_space = PAGE_DATA_SIZE;
            handle.mark_dirty();
        }

        // 写入到当前页面
        let page_id = *edge_pages.last().unwrap();
        let handle = self.buffer_pool.fetch_page(page_id)?;

        {
            let mut guard = handle.write();
            if let Some(page) = guard.page_mut() {
                let offset = page.free_offset as usize;

                // 写入长度前缀
                let len = data.len() as u32;
                page.data[offset..offset + 4].copy_from_slice(&len.to_le_bytes());

                // 写入数据
                page.data[offset + 4..offset + 4 + data.len()].copy_from_slice(&data);

                page.free_offset += entry_size as u16;
                page.item_count += 1;
                page.is_dirty = true;
            }
        }

        *current_space -= entry_size;
        handle.mark_dirty();
        *self.dirty.write() = true;

        Ok(())
    }

    /// 设置图 schema（来自 CREATE GRAPH 的内联 schema）
    pub fn set_schema(&self, s: StoredGraphSchema) {
        *self.schema.write() = Some(s);
        *self.dirty.write() = true;
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

        // 写入磁盘
        self.write_vertex_to_disk(&vertex)?;

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

        // 写入磁盘
        self.write_vertex_to_disk(&vertex)?;

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

        // 写入磁盘
        self.write_vertex_to_disk(&vertex)?;

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
        // 注意：当前实现不支持原地更新磁盘上的顶点
        // 更新只会影响内存缓存，需要重建持久化数据才能生效
        self.vertex_cache.write().insert(id, vertex);
        *self.dirty.write() = true;
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

        *self.dirty.write() = true;
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

        // 写入磁盘
        self.write_edge_to_disk(&edge)?;

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

        // 写入磁盘
        self.write_edge_to_disk(&edge)?;

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
        *self.dirty.write() = true;
        Ok(())
    }

    /// 删除边
    pub fn remove_edge(&self, id: EdgeId) -> Result<()> {
        let edge = self.edge_cache.write().remove(&id);
        if let Some(e) = edge {
            self.edge_index.remove(id, Some(e.label()));
        }
        *self.dirty.write() = true;
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
        // 保存元数据
        self.save_meta()?;
        // 刷新所有脏页到磁盘
        self.buffer_pool.flush_all()
    }

    /// 获取缓冲池引用
    pub fn buffer_pool(&self) -> &Arc<BufferPool> {
        &self.buffer_pool
    }

    /// 获取缓冲池水位信息
    pub fn buffer_pool_watermark(&self) -> crate::storage::BufferPoolWatermark {
        self.buffer_pool.watermark_info()
    }

    /// 获取边索引引用
    pub fn edge_index(&self) -> &EdgeIndex {
        &self.edge_index
    }

    /// 获取顶点索引引用
    pub fn vertex_index(&self) -> &VertexIndex {
        &self.vertex_index
    }

    /// 检查是否有未保存的更改
    pub fn is_dirty(&self) -> bool {
        *self.dirty.read()
    }
}

impl Drop for Graph {
    fn drop(&mut self) {
        // 自动保存元数据
        if *self.dirty.read() {
            if let Err(e) = self.save_meta() {
                eprintln!("警告: 保存图元数据失败: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenAmount;
    use tempfile::tempdir;

    #[test]
    fn test_graph_basic() {
        let graph = Graph::in_memory().unwrap();

        // 添加账户
        let v1 = graph
            .add_account("0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0".to_string())
            .unwrap();
        let v2 = graph
            .add_account("0x8ba1f109551bD432803012645Ac136ddd64DBA72".to_string())
            .unwrap();

        assert_eq!(graph.vertex_count(), 2);

        // 添加转账边
        let amount = TokenAmount::from_u64(1000);
        let e1 = graph.add_transfer(v1, v2, amount, 12345678).unwrap();

        assert_eq!(graph.edge_count(), 1);

        // 查询
        let v = graph
            .get_vertex_by_address("0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0")
            .unwrap();
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

    #[test]
    fn test_persistence_across_restarts() {
        let dir = tempdir().unwrap();
        let data_path = dir.path().to_path_buf();

        // 第一次：创建图并添加数据
        {
            let graph = Graph::open(&data_path, Some(512)).unwrap();

            let v1 = graph
                .add_account("0xAlice".to_string())
                .unwrap();
            let v2 = graph
                .add_account("0xBob".to_string())
                .unwrap();

            graph.add_edge(EdgeLabel::Transfer, v1, v2).unwrap();

            assert_eq!(graph.vertex_count(), 2);
            assert_eq!(graph.edge_count(), 1);

            // 显式刷新
            graph.flush().unwrap();
        }

        // 第二次：重新打开并验证数据
        {
            let graph = Graph::open(&data_path, Some(512)).unwrap();

            assert_eq!(graph.vertex_count(), 2, "顶点数量应该是 2");
            assert_eq!(graph.edge_count(), 1, "边数量应该是 1");

            // 验证可以通过地址找到顶点
            let alice = graph.get_vertex_by_address("0xAlice");
            assert!(alice.is_some(), "应该能找到 Alice");

            let bob = graph.get_vertex_by_address("0xBob");
            assert!(bob.is_some(), "应该能找到 Bob");
        }
    }

    #[test]
    fn test_schema_persistence() {
        let dir = tempdir().unwrap();
        let data_path = dir.path().to_path_buf();

        // 创建带 schema 的图
        {
            let graph = Graph::open(&data_path, Some(512)).unwrap();

            let mut schema = StoredGraphSchema::default();
            schema.node_types.insert(
                "Person".to_string(),
                vec![StoredPropertySpec {
                    name: "name".to_string(),
                    data_type: "String".to_string(),
                    is_primary_key: true,
                }],
            );

            graph.set_schema(schema);
            graph.flush().unwrap();
        }

        // 重新打开并验证 schema
        {
            let graph = Graph::open(&data_path, Some(512)).unwrap();
            let schema = graph.get_schema();
            assert!(schema.is_some(), "应该有 schema");

            let schema = schema.unwrap();
            assert!(
                schema.node_types.contains_key("Person"),
                "应该包含 Person 节点类型"
            );
        }
    }
}
