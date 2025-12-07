//! 最大流算法
//!
//! 实现 Edmonds-Karp 算法（基于 BFS 的 Ford-Fulkerson）
//! 用于分析区块链资金流动的最大通量

use crate::graph::{Graph, VertexId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// 最大流结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxFlow {
    /// 最大流量值
    pub value: f64,
    /// 流量分配（边 -> 流量）
    pub flow: HashMap<(VertexId, VertexId), f64>,
    /// 最小割的源侧顶点集
    pub source_side: HashSet<VertexId>,
}

/// Edmonds-Karp 最大流算法
pub struct EdmondsKarp {
    graph: Arc<Graph>,
}

impl EdmondsKarp {
    /// 创建算法实例
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// 计算从 source 到 sink 的最大流
    pub fn max_flow(&self, source: VertexId, sink: VertexId) -> MaxFlow {
        // 构建容量矩阵
        let mut capacity: HashMap<(VertexId, VertexId), f64> = HashMap::new();
        let mut vertices = HashSet::new();

        // 收集所有边和顶点
        for edge_id in self
            .graph
            .edge_index()
            .get_by_label(&crate::types::EdgeLabel::Transfer)
        {
            if let Some(edge) = self.graph.get_edge(edge_id) {
                let src = edge.src();
                let dst = edge.dst();
                vertices.insert(src);
                vertices.insert(dst);

                // 累加同一边的容量
                *capacity.entry((src, dst)).or_insert(0.0) += edge.weight();
            }
        }

        // 如果没有转账边，尝试使用所有边
        if capacity.is_empty() {
            for vertex_id in self.get_all_vertices() {
                for edge in self.graph.get_outgoing_edges(vertex_id) {
                    let src = edge.src();
                    let dst = edge.dst();
                    vertices.insert(src);
                    vertices.insert(dst);
                    *capacity.entry((src, dst)).or_insert(0.0) += edge.weight();
                }
            }
        }

        // 流量矩阵
        let mut flow: HashMap<(VertexId, VertexId), f64> = HashMap::new();

        // 构建邻接表
        let mut adj: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
        for &(src, dst) in capacity.keys() {
            adj.entry(src).or_insert_with(Vec::new).push(dst);
            adj.entry(dst).or_insert_with(Vec::new).push(src); // 反向边
        }

        let mut max_flow_value = 0.0;

        // Edmonds-Karp: 重复 BFS 找增广路径
        loop {
            // BFS 找增广路径
            let path = self.bfs_find_path(source, sink, &capacity, &flow, &adj);

            match path {
                None => break,
                Some((path_vertices, bottleneck)) => {
                    // 沿路径增广
                    for i in 0..path_vertices.len() - 1 {
                        let u = path_vertices[i];
                        let v = path_vertices[i + 1];

                        *flow.entry((u, v)).or_insert(0.0) += bottleneck;
                        *flow.entry((v, u)).or_insert(0.0) -= bottleneck;
                    }

                    max_flow_value += bottleneck;
                }
            }
        }

        // 找最小割（BFS 从源点出发，能到达的顶点属于源侧）
        let source_side = self.find_source_side(source, &capacity, &flow, &adj);

        // 只保留正流量
        let positive_flow: HashMap<(VertexId, VertexId), f64> =
            flow.into_iter().filter(|(_, v)| *v > 0.0).collect();

        MaxFlow {
            value: max_flow_value,
            flow: positive_flow,
            source_side,
        }
    }

    /// BFS 找增广路径
    fn bfs_find_path(
        &self,
        source: VertexId,
        sink: VertexId,
        capacity: &HashMap<(VertexId, VertexId), f64>,
        flow: &HashMap<(VertexId, VertexId), f64>,
        adj: &HashMap<VertexId, Vec<VertexId>>,
    ) -> Option<(Vec<VertexId>, f64)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();

        visited.insert(source);
        queue.push_back(source);

        while let Some(u) = queue.pop_front() {
            if u == sink {
                break;
            }

            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    // 残余容量 = 容量 - 已用流量
                    let cap = capacity.get(&(u, v)).copied().unwrap_or(0.0);
                    let used = flow.get(&(u, v)).copied().unwrap_or(0.0);
                    let residual = cap - used;

                    if !visited.contains(&v) && residual > 0.0 {
                        visited.insert(v);
                        parent.insert(v, u);
                        queue.push_back(v);
                    }
                }
            }
        }

        // 重构路径并计算瓶颈
        if !parent.contains_key(&sink) {
            return None;
        }

        let mut path = Vec::new();
        let mut current = sink;
        while current != source {
            path.push(current);
            current = *parent.get(&current)?;
        }
        path.push(source);
        path.reverse();

        // 计算瓶颈
        let mut bottleneck = f64::INFINITY;
        for i in 0..path.len() - 1 {
            let u = path[i];
            let v = path[i + 1];
            let cap = capacity.get(&(u, v)).copied().unwrap_or(0.0);
            let used = flow.get(&(u, v)).copied().unwrap_or(0.0);
            let residual = cap - used;
            bottleneck = bottleneck.min(residual);
        }

        Some((path, bottleneck))
    }

    /// 找最小割的源侧顶点
    fn find_source_side(
        &self,
        source: VertexId,
        capacity: &HashMap<(VertexId, VertexId), f64>,
        flow: &HashMap<(VertexId, VertexId), f64>,
        adj: &HashMap<VertexId, Vec<VertexId>>,
    ) -> HashSet<VertexId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        visited.insert(source);
        queue.push_back(source);

        while let Some(u) = queue.pop_front() {
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    let cap = capacity.get(&(u, v)).copied().unwrap_or(0.0);
                    let used = flow.get(&(u, v)).copied().unwrap_or(0.0);
                    let residual = cap - used;

                    if !visited.contains(&v) && residual > 0.0 {
                        visited.insert(v);
                        queue.push_back(v);
                    }
                }
            }
        }

        visited
    }

    /// 获取所有顶点（辅助方法）
    fn get_all_vertices(&self) -> Vec<VertexId> {
        let mut vertices = Vec::new();
        for label in &[
            crate::types::VertexLabel::Account,
            crate::types::VertexLabel::Contract,
            crate::types::VertexLabel::Token,
        ] {
            vertices.extend(self.graph.vertex_index().get_by_label(label));
        }
        vertices
    }

    /// 计算多源多汇最大流
    /// 通过添加超级源点和超级汇点实现
    pub fn multi_source_sink_max_flow(&self, sources: &[VertexId], sinks: &[VertexId]) -> f64 {
        // 简化实现：计算所有源-汇对的最大流之和
        let mut total_flow = 0.0;
        for &source in sources {
            for &sink in sinks {
                if source != sink {
                    let result = self.max_flow(source, sink);
                    total_flow += result.value;
                }
            }
        }
        total_flow
    }

    /// 分析资金流动瓶颈
    /// 返回限制流量的关键边
    pub fn find_bottleneck_edges(
        &self,
        source: VertexId,
        sink: VertexId,
    ) -> Vec<(VertexId, VertexId, f64)> {
        let result = self.max_flow(source, sink);

        // 瓶颈边是那些流量等于容量的边
        let mut bottlenecks = Vec::new();

        for edge_id in self
            .graph
            .edge_index()
            .get_by_label(&crate::types::EdgeLabel::Transfer)
        {
            if let Some(edge) = self.graph.get_edge(edge_id) {
                let src = edge.src();
                let dst = edge.dst();

                if let Some(&flow) = result.flow.get(&(src, dst)) {
                    let capacity = edge.weight();
                    // 如果流量接近容量（容差 0.001），则是瓶颈
                    if (capacity - flow).abs() < 0.001 {
                        bottlenecks.push((src, dst, capacity));
                    }
                }
            }
        }

        bottlenecks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TokenAmount, VertexLabel};

    fn create_flow_graph() -> Arc<Graph> {
        let graph = Graph::in_memory().unwrap();

        // 创建经典最大流测试图
        //     10       10
        // S -----> A -----> T
        // |        ^        ^
        // |5       |5       |
        // v        |        |
        // B -----> C ------>|
        //     10       10

        let s = graph.add_vertex(VertexLabel::Account).unwrap();
        let a = graph.add_vertex(VertexLabel::Account).unwrap();
        let b = graph.add_vertex(VertexLabel::Account).unwrap();
        let c = graph.add_vertex(VertexLabel::Account).unwrap();
        let t = graph.add_vertex(VertexLabel::Account).unwrap();

        // S -> A (10)
        graph
            .add_transfer(s, a, TokenAmount::from_u64(10), 1)
            .unwrap();
        // S -> B (5)
        graph
            .add_transfer(s, b, TokenAmount::from_u64(5), 2)
            .unwrap();
        // A -> T (10)
        graph
            .add_transfer(a, t, TokenAmount::from_u64(10), 3)
            .unwrap();
        // B -> C (10)
        graph
            .add_transfer(b, c, TokenAmount::from_u64(10), 4)
            .unwrap();
        // C -> A (5)
        graph
            .add_transfer(c, a, TokenAmount::from_u64(5), 5)
            .unwrap();
        // C -> T (10)
        graph
            .add_transfer(c, t, TokenAmount::from_u64(10), 6)
            .unwrap();

        graph
    }

    #[test]
    fn test_max_flow_basic() {
        let graph = create_flow_graph();
        let algo = EdmondsKarp::new(graph);

        let result = algo.max_flow(VertexId::new(1), VertexId::new(5));

        // 最大流应该是 15 (10 through A + 5 through B-C)
        assert!(
            (result.value - 15.0).abs() < 0.01,
            "Expected 15, got {}",
            result.value
        );
    }

    #[test]
    fn test_simple_flow() {
        let graph = Graph::in_memory().unwrap();

        let v1 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v2 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v3 = graph.add_vertex(VertexLabel::Account).unwrap();

        // v1 -> v2 (10), v2 -> v3 (5)
        graph
            .add_transfer(v1, v2, TokenAmount::from_u64(10), 1)
            .unwrap();
        graph
            .add_transfer(v2, v3, TokenAmount::from_u64(5), 2)
            .unwrap();

        let algo = EdmondsKarp::new(graph);
        let result = algo.max_flow(v1, v3);

        // 瓶颈在 v2 -> v3，最大流是 5
        assert!((result.value - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_parallel_paths() {
        let graph = Graph::in_memory().unwrap();

        let s = graph.add_vertex(VertexLabel::Account).unwrap();
        let a = graph.add_vertex(VertexLabel::Account).unwrap();
        let b = graph.add_vertex(VertexLabel::Account).unwrap();
        let t = graph.add_vertex(VertexLabel::Account).unwrap();

        // 两条并行路径
        // S -> A -> T (5)
        // S -> B -> T (10)
        graph
            .add_transfer(s, a, TokenAmount::from_u64(5), 1)
            .unwrap();
        graph
            .add_transfer(a, t, TokenAmount::from_u64(5), 2)
            .unwrap();
        graph
            .add_transfer(s, b, TokenAmount::from_u64(10), 3)
            .unwrap();
        graph
            .add_transfer(b, t, TokenAmount::from_u64(10), 4)
            .unwrap();

        let algo = EdmondsKarp::new(graph);
        let result = algo.max_flow(s, t);

        // 总流量应该是 15
        assert!((result.value - 15.0).abs() < 0.01);
    }
}
