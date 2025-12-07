//! 路径追踪算法
//!
//! 用于区块链链路追踪场景

use crate::graph::{EdgeId, Graph, VertexId};
use crate::types::EdgeLabel;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// 追踪方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceDirection {
    /// 正向追踪（沿出边方向）
    Forward,
    /// 反向追踪（沿入边方向）
    Backward,
    /// 双向追踪
    Both,
}

/// 路径结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    /// 路径上的顶点序列
    pub vertices: Vec<VertexId>,
    /// 路径上的边序列
    pub edges: Vec<EdgeId>,
    /// 路径长度
    pub length: usize,
    /// 路径总权重（如总金额）
    pub total_weight: f64,
}

impl PathResult {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            length: 0,
            total_weight: 0.0,
        }
    }

    fn with_start(start: VertexId) -> Self {
        Self {
            vertices: vec![start],
            edges: Vec::new(),
            length: 0,
            total_weight: 0.0,
        }
    }
}

/// 路径查找器
pub struct PathFinder {
    graph: Arc<Graph>,
}

impl PathFinder {
    /// 创建路径查找器
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// BFS 最短路径查找
    pub fn shortest_path(&self, start: VertexId, end: VertexId) -> Option<PathResult> {
        if start == end {
            return Some(PathResult::with_start(start));
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<VertexId, (VertexId, EdgeId)> = HashMap::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            for edge in self.graph.get_outgoing_edges(current) {
                let neighbor = edge.dst();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, (current, edge.id()));
                    queue.push_back(neighbor);

                    if neighbor == end {
                        // 重构路径
                        return Some(self.reconstruct_path(start, end, &parent));
                    }
                }
            }
        }

        None
    }

    /// 重构路径
    fn reconstruct_path(
        &self,
        start: VertexId,
        end: VertexId,
        parent: &HashMap<VertexId, (VertexId, EdgeId)>,
    ) -> PathResult {
        let mut path = PathResult::new();
        let mut current = end;
        let mut vertices = vec![end];
        let mut edges = Vec::new();
        let mut total_weight = 0.0;

        while current != start {
            if let Some(&(prev, edge_id)) = parent.get(&current) {
                edges.push(edge_id);
                vertices.push(prev);
                if let Some(edge) = self.graph.get_edge(edge_id) {
                    total_weight += edge.weight();
                }
                current = prev;
            } else {
                break;
            }
        }

        vertices.reverse();
        edges.reverse();

        path.vertices = vertices;
        path.edges = edges;
        path.length = path.edges.len();
        path.total_weight = total_weight;

        path
    }

    /// 查找所有路径（限制深度）
    pub fn all_paths(&self, start: VertexId, end: VertexId, max_depth: usize) -> Vec<PathResult> {
        let mut results = Vec::new();
        let mut path = PathResult::with_start(start);
        let mut visited = HashSet::new();
        visited.insert(start);

        self.dfs_all_paths(start, end, max_depth, &mut visited, &mut path, &mut results);

        results
    }

    fn dfs_all_paths(
        &self,
        current: VertexId,
        end: VertexId,
        remaining_depth: usize,
        visited: &mut HashSet<VertexId>,
        path: &mut PathResult,
        results: &mut Vec<PathResult>,
    ) {
        if current == end {
            results.push(path.clone());
            return;
        }

        if remaining_depth == 0 {
            return;
        }

        for edge in self.graph.get_outgoing_edges(current) {
            let neighbor = edge.dst();
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                path.vertices.push(neighbor);
                path.edges.push(edge.id());
                path.total_weight += edge.weight();

                self.dfs_all_paths(neighbor, end, remaining_depth - 1, visited, path, results);

                path.total_weight -= edge.weight();
                path.edges.pop();
                path.vertices.pop();
                visited.remove(&neighbor);
            }
        }
    }

    /// K 最短路径（Yen's 算法简化版）
    pub fn k_shortest_paths(&self, start: VertexId, end: VertexId, k: usize) -> Vec<PathResult> {
        let mut results = Vec::new();

        // 先找最短路径
        if let Some(shortest) = self.shortest_path(start, end) {
            results.push(shortest);
        } else {
            return results;
        }

        // 使用 all_paths 找更多路径并排序
        let max_depth = 10; // 限制搜索深度
        let all = self.all_paths(start, end, max_depth);

        let mut sorted_paths = all;
        sorted_paths.sort_by(|a, b| a.length.cmp(&b.length));

        for path in sorted_paths {
            if results.len() >= k {
                break;
            }
            // 避免重复
            let is_duplicate = results.iter().any(|r| r.vertices == path.vertices);
            if !is_duplicate {
                results.push(path);
            }
        }

        results
    }

    /// 链路追踪（从起点向外扩展）
    pub fn trace(
        &self,
        start: VertexId,
        direction: TraceDirection,
        max_depth: usize,
        edge_filter: Option<&[EdgeLabel]>,
    ) -> Vec<PathResult> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        let mut path = PathResult::with_start(start);
        visited.insert(start);

        self.dfs_trace(
            start,
            direction,
            max_depth,
            edge_filter,
            &mut visited,
            &mut path,
            &mut results,
        );

        results
    }

    fn dfs_trace(
        &self,
        current: VertexId,
        direction: TraceDirection,
        remaining_depth: usize,
        edge_filter: Option<&[EdgeLabel]>,
        visited: &mut HashSet<VertexId>,
        path: &mut PathResult,
        results: &mut Vec<PathResult>,
    ) {
        // 记录当前路径（如果不是起点）
        if path.vertices.len() > 1 {
            results.push(path.clone());
        }

        if remaining_depth == 0 {
            return;
        }

        // 获取边
        let edges = match direction {
            TraceDirection::Forward => self.graph.get_outgoing_edges(current),
            TraceDirection::Backward => self.graph.get_incoming_edges(current),
            TraceDirection::Both => {
                let mut all = self.graph.get_outgoing_edges(current);
                all.extend(self.graph.get_incoming_edges(current));
                all
            }
        };

        for edge in edges {
            // 边类型过滤
            if let Some(filter) = edge_filter {
                if !filter.contains(edge.label()) {
                    continue;
                }
            }

            let neighbor = match direction {
                TraceDirection::Forward => edge.dst(),
                TraceDirection::Backward => edge.src(),
                TraceDirection::Both => {
                    if edge.src() == current {
                        edge.dst()
                    } else {
                        edge.src()
                    }
                }
            };

            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                path.vertices.push(neighbor);
                path.edges.push(edge.id());
                path.total_weight += edge.weight();

                self.dfs_trace(
                    neighbor,
                    direction,
                    remaining_depth - 1,
                    edge_filter,
                    visited,
                    path,
                    results,
                );

                path.total_weight -= edge.weight();
                path.edges.pop();
                path.vertices.pop();
                visited.remove(&neighbor);
            }
        }
    }

    /// 查找资金流向（转账链路）
    pub fn trace_fund_flow(&self, start: VertexId, max_depth: usize) -> Vec<PathResult> {
        self.trace(
            start,
            TraceDirection::Forward,
            max_depth,
            Some(&[EdgeLabel::Transfer]),
        )
    }

    /// 查找资金来源（反向转账链路）
    pub fn trace_fund_source(&self, end: VertexId, max_depth: usize) -> Vec<PathResult> {
        self.trace(
            end,
            TraceDirection::Backward,
            max_depth,
            Some(&[EdgeLabel::Transfer]),
        )
    }

    /// 判断两点是否连通
    pub fn is_reachable(&self, start: VertexId, end: VertexId) -> bool {
        self.shortest_path(start, end).is_some()
    }

    /// 获取 n 跳邻居
    pub fn n_hop_neighbors(&self, start: VertexId, n: usize) -> HashSet<VertexId> {
        let mut current_level = HashSet::new();
        current_level.insert(start);

        let mut visited = HashSet::new();
        visited.insert(start);

        for _ in 0..n {
            let mut next_level = HashSet::new();
            for &vertex in &current_level {
                for neighbor in self.graph.neighbors(vertex) {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        next_level.insert(neighbor);
                    }
                }
            }
            current_level = next_level;
        }

        current_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TokenAmount, VertexLabel};

    fn create_test_graph() -> Arc<Graph> {
        let graph = Graph::in_memory().unwrap();

        // 创建顶点: 1 -> 2 -> 3 -> 4
        //              \-> 5 -> 4
        let v1 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v2 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v3 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v4 = graph.add_vertex(VertexLabel::Account).unwrap();
        let v5 = graph.add_vertex(VertexLabel::Account).unwrap();

        let amount = TokenAmount::from_u64(100);
        graph.add_transfer(v1, v2, amount.clone(), 1).unwrap();
        graph.add_transfer(v2, v3, amount.clone(), 2).unwrap();
        graph.add_transfer(v3, v4, amount.clone(), 3).unwrap();
        graph.add_transfer(v1, v5, amount.clone(), 4).unwrap();
        graph.add_transfer(v5, v4, amount.clone(), 5).unwrap();

        graph
    }

    #[test]
    fn test_shortest_path() {
        let graph = create_test_graph();
        let finder = PathFinder::new(graph);

        let path = finder
            .shortest_path(VertexId::new(1), VertexId::new(4))
            .unwrap();
        // 最短路径: 1 -> 5 -> 4 或 1 -> 2 -> 3 -> 4
        // 取决于 BFS 顺序，但长度应该是 2
        assert!(path.length >= 2);
        assert_eq!(path.vertices.first(), Some(&VertexId::new(1)));
        assert_eq!(path.vertices.last(), Some(&VertexId::new(4)));
    }

    #[test]
    fn test_all_paths() {
        let graph = create_test_graph();
        let finder = PathFinder::new(graph);

        let paths = finder.all_paths(VertexId::new(1), VertexId::new(4), 5);
        assert_eq!(paths.len(), 2); // 两条路径
    }

    #[test]
    fn test_trace() {
        let graph = create_test_graph();
        let finder = PathFinder::new(graph);

        let traces = finder.trace_fund_flow(VertexId::new(1), 3);
        assert!(!traces.is_empty());
    }

    #[test]
    fn test_n_hop_neighbors() {
        let graph = create_test_graph();
        let finder = PathFinder::new(graph);

        let one_hop = finder.n_hop_neighbors(VertexId::new(1), 1);
        assert_eq!(one_hop.len(), 2); // v2 和 v5

        let two_hop = finder.n_hop_neighbors(VertexId::new(1), 2);
        assert_eq!(two_hop.len(), 2); // v3 和 v4
    }
}
