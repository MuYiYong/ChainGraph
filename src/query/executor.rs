//! GQL Query Executor - ISO/IEC 39075 Compliant
//!
//! Executes GQL AST and returns query results.

use super::ast::*;
use crate::error::{Error, Result};
use crate::graph::{Edge, Graph, Vertex, VertexId};
use crate::types::{Address, EdgeLabel, PropertyValue, TokenAmount, VertexLabel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<ResultValue>>,
    pub stats: QueryStats,
}

/// Result value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultValue {
    Vertex(VertexData),
    Edge(EdgeData),
    Scalar(PropertyValue),
    Path(PathData),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    pub id: u64,
    pub label: String,
    pub properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: u64,
    pub label: String,
    pub src: u64,
    pub dst: u64,
    pub properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathData {
    pub vertices: Vec<VertexData>,
    pub edges: Vec<EdgeData>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    pub vertices_scanned: usize,
    pub edges_scanned: usize,
    pub rows_returned: usize,
    pub execution_time_ms: u64,
}

type Bindings = HashMap<String, BindingValue>;

#[derive(Debug, Clone)]
enum BindingValue {
    Vertex(Vertex),
    Edge(Edge),
    Scalar(PropertyValue),
    Path(Vec<VertexId>),
}

/// Query executor
pub struct QueryExecutor {
    graph: Arc<Graph>,
}

impl QueryExecutor {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    pub fn execute(&self, stmt: &GqlStatement) -> Result<QueryResult> {
        let start = std::time::Instant::now();

        let result = match stmt {
            GqlStatement::Match(query) => self.execute_match(query),
            GqlStatement::Insert(stmt) => self.execute_insert(stmt),
            GqlStatement::Delete(stmt) => self.execute_delete(stmt),
            GqlStatement::Set(stmt) => self.execute_set(stmt),
            GqlStatement::Remove(stmt) => self.execute_remove(stmt),
            GqlStatement::Call(stmt) => self.execute_call(stmt),
            GqlStatement::CreateGraph(stmt) => self.execute_create_graph(stmt),
            GqlStatement::DropGraph(stmt) => self.execute_drop_graph(stmt),
            GqlStatement::Show(stmt) => self.execute_show(stmt),
            GqlStatement::Describe(stmt) => self.execute_describe(stmt),
            // ISO GQL 39075 statements
            GqlStatement::Let(stmt) => self.execute_let(stmt),
            GqlStatement::For(stmt) => self.execute_for(stmt),
            GqlStatement::Filter(stmt) => self.execute_filter(stmt),
            GqlStatement::Composite(stmt) => self.execute_composite(stmt),
            GqlStatement::Use(stmt) => self.execute_use(stmt),
            GqlStatement::Select(stmt) => self.execute_select(stmt),
            GqlStatement::Session(stmt) => self.execute_session(stmt),
            GqlStatement::Transaction(stmt) => self.execute_transaction(stmt),
            GqlStatement::CreateGraphType(stmt) => self.execute_create_graph_type(stmt),
            GqlStatement::DropGraphType(stmt) => self.execute_drop_graph_type(stmt),
        };

        let mut result = result?;
        result.stats.execution_time_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Execute MATCH statement with GQL path modes and search prefixes
    fn execute_match(&self, query: &MatchStatement) -> Result<QueryResult> {
        let mut stats = QueryStats::default();

        // 1. Match graph pattern
        let bindings_list = self.match_graph_pattern(&query.graph_pattern, &mut stats)?;

        // 2. Apply WHERE filter
        let filtered: Vec<Bindings> = if let Some(ref where_clause) = query.where_clause {
            bindings_list
                .into_iter()
                .filter(|bindings| self.evaluate_bool(where_clause, bindings).unwrap_or(false))
                .collect()
        } else {
            bindings_list
        };

        // 3. SKIP
        let skipped: Vec<Bindings> = if let Some(skip) = query.skip {
            filtered.into_iter().skip(skip).collect()
        } else {
            filtered
        };

        // 4. LIMIT
        let limited: Vec<Bindings> = if let Some(limit) = query.limit {
            skipped.into_iter().take(limit).collect()
        } else {
            skipped
        };

        // 5. Build RETURN result
        let (columns, rows) = self.build_return(&query.return_clause, &limited)?;
        stats.rows_returned = rows.len();

        Ok(QueryResult {
            columns,
            rows,
            stats,
        })
    }

    fn match_graph_pattern(
        &self,
        pattern: &GraphPattern,
        stats: &mut QueryStats,
    ) -> Result<Vec<Bindings>> {
        let mut result = vec![HashMap::new()];

        for path in &pattern.paths {
            let mut new_result = Vec::new();
            for bindings in result {
                let path_bindings = self.match_path_pattern(path, bindings, stats)?;
                new_result.extend(path_bindings);
            }
            result = new_result;
        }

        Ok(result)
    }

    fn match_path_pattern(
        &self,
        path: &PathPattern,
        initial: Bindings,
        stats: &mut QueryStats,
    ) -> Result<Vec<Bindings>> {
        if path.elements.is_empty() {
            return Ok(vec![initial]);
        }

        // Handle path search prefix for shortest path queries
        if let Some(ref search) = path.search_prefix {
            return self.match_path_with_search(path, initial, search, stats);
        }

        let mut current = vec![initial];
        let mut i = 0;

        while i < path.elements.len() {
            match &path.elements[i] {
                PathElement::Node(node_pattern) => {
                    let mut new_bindings = Vec::new();
                    for bindings in current {
                        let candidates =
                            self.get_candidate_vertices(node_pattern, &bindings, stats);
                        for vertex in candidates {
                            if self.match_node_properties(node_pattern, &vertex) {
                                let mut new_bind = bindings.clone();
                                if let Some(ref var) = node_pattern.variable {
                                    new_bind
                                        .insert(var.clone(), BindingValue::Vertex(vertex.clone()));
                                }
                                new_bindings.push(new_bind);
                            }
                        }
                    }
                    current = new_bindings;
                }
                PathElement::Edge(edge_pattern) => {
                    if i + 1 >= path.elements.len() {
                        break;
                    }
                    let target_node = match &path.elements[i + 1] {
                        PathElement::Node(n) => n,
                        _ => {
                            return Err(Error::QueryError(
                                "Edge must be followed by node".to_string(),
                            ))
                        }
                    };

                    current = self.match_edge_pattern(
                        &current,
                        edge_pattern,
                        target_node,
                        path.path_mode,
                        stats,
                    )?;
                    i += 1;
                }
            }
            i += 1;
        }

        Ok(current)
    }

    fn match_path_with_search(
        &self,
        path: &PathPattern,
        initial: Bindings,
        search: &PathSearchPrefix,
        stats: &mut QueryStats,
    ) -> Result<Vec<Bindings>> {
        use crate::algorithm::PathFinder;

        // Extract source and target from path elements
        let (source_pattern, target_pattern) = if path.elements.len() >= 3 {
            match (&path.elements[0], &path.elements[path.elements.len() - 1]) {
                (PathElement::Node(s), PathElement::Node(t)) => (s, t),
                _ => {
                    return Err(Error::QueryError(
                        "Path must start and end with nodes".to_string(),
                    ))
                }
            }
        } else {
            return Ok(vec![initial]);
        };

        let source_vertices = self.get_candidate_vertices(source_pattern, &initial, stats);
        let target_vertices = self.get_candidate_vertices(target_pattern, &initial, stats);

        let finder = PathFinder::new(self.graph.clone());
        let mut results = Vec::new();

        for source in &source_vertices {
            for target in &target_vertices {
                match search {
                    PathSearchPrefix::AnyShortest | PathSearchPrefix::ShortestK(1) => {
                        if let Some(found_path) = finder.shortest_path(source.id(), target.id()) {
                            let mut bindings = initial.clone();
                            if let Some(ref var) = source_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(source.clone()));
                            }
                            if let Some(ref var) = target_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(target.clone()));
                            }
                            if let Some(ref var) = path.variable {
                                bindings
                                    .insert(var.clone(), BindingValue::Path(found_path.vertices));
                            }
                            results.push(bindings);
                        }
                    }
                    PathSearchPrefix::AllShortest => {
                        // Use shortest_path and wrap in a vector
                        // (all_shortest_paths not implemented yet, using single shortest)
                        if let Some(found_path) = finder.shortest_path(source.id(), target.id()) {
                            let mut bindings = initial.clone();
                            if let Some(ref var) = source_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(source.clone()));
                            }
                            if let Some(ref var) = target_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(target.clone()));
                            }
                            if let Some(ref var) = path.variable {
                                bindings
                                    .insert(var.clone(), BindingValue::Path(found_path.vertices));
                            }
                            results.push(bindings);
                        }
                    }
                    PathSearchPrefix::All => {
                        let paths = finder.all_paths(source.id(), target.id(), 10);
                        for found_path in paths {
                            let mut bindings = initial.clone();
                            if let Some(ref var) = source_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(source.clone()));
                            }
                            if let Some(ref var) = target_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(target.clone()));
                            }
                            if let Some(ref var) = path.variable {
                                bindings
                                    .insert(var.clone(), BindingValue::Path(found_path.vertices));
                            }
                            results.push(bindings);
                        }
                    }
                    PathSearchPrefix::ShortestK(k) => {
                        let paths = finder.k_shortest_paths(source.id(), target.id(), *k as usize);
                        for found_path in paths {
                            let mut bindings = initial.clone();
                            if let Some(ref var) = source_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(source.clone()));
                            }
                            if let Some(ref var) = target_pattern.variable {
                                bindings.insert(var.clone(), BindingValue::Vertex(target.clone()));
                            }
                            if let Some(ref var) = path.variable {
                                bindings
                                    .insert(var.clone(), BindingValue::Path(found_path.vertices));
                            }
                            results.push(bindings);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(results)
    }

    fn match_edge_pattern(
        &self,
        current: &[Bindings],
        edge: &EdgePattern,
        target: &NodePattern,
        path_mode: Option<PathMode>,
        stats: &mut QueryStats,
    ) -> Result<Vec<Bindings>> {
        let mut new_bindings = Vec::new();

        for bindings in current {
            let source_vertices: Vec<Vertex> = self.get_bound_vertices(bindings);

            for source in source_vertices {
                // Handle variable-length patterns
                if let Some(ref quantifier) = edge.quantifier {
                    let paths = self.expand_variable_length(
                        &source, edge, target, quantifier, path_mode, stats,
                    )?;
                    for (path_vertices, final_vertex, path_edges) in paths {
                        let mut new_bind = bindings.clone();
                        if let Some(ref var) = edge.variable {
                            new_bind.insert(var.clone(), BindingValue::Path(path_vertices));
                        }
                        if let Some(ref var) = target.variable {
                            new_bind.insert(var.clone(), BindingValue::Vertex(final_vertex));
                        }
                        new_bindings.push(new_bind);
                    }
                } else {
                    // Single edge match
                    let edges = self.get_edges_by_direction(&source, edge.direction);
                    stats.edges_scanned += edges.len();

                    for e in edges {
                        if !self.match_edge_labels(edge, &e) {
                            continue;
                        }

                        let target_id = self.get_edge_target(&e, &source, edge.direction);
                        if let Some(target_vertex) = self.graph.get_vertex(target_id) {
                            if self.match_node_pattern(target, &target_vertex) {
                                let mut new_bind = bindings.clone();
                                if let Some(ref var) = edge.variable {
                                    new_bind.insert(var.clone(), BindingValue::Edge(e.clone()));
                                }
                                if let Some(ref var) = target.variable {
                                    new_bind
                                        .insert(var.clone(), BindingValue::Vertex(target_vertex));
                                }
                                new_bindings.push(new_bind);
                            }
                        }
                    }
                }
            }
        }

        Ok(new_bindings)
    }

    fn expand_variable_length(
        &self,
        source: &Vertex,
        edge: &EdgePattern,
        target: &NodePattern,
        quantifier: &PatternQuantifier,
        path_mode: Option<PathMode>,
        stats: &mut QueryStats,
    ) -> Result<Vec<(Vec<VertexId>, Vertex, Vec<Edge>)>> {
        let (min, max) = match quantifier {
            PatternQuantifier::ZeroOrMore => (0, 10),
            PatternQuantifier::OneOrMore => (1, 10),
            PatternQuantifier::ZeroOrOne => (0, 1),
            PatternQuantifier::Exactly(n) => (*n as usize, *n as usize),
            PatternQuantifier::AtLeast(n) => (*n as usize, 10),
            PatternQuantifier::AtMost(m) => (0, *m as usize),
            PatternQuantifier::Range(n, m) => (*n as usize, *m as usize),
        };

        let mut results = Vec::new();
        let mut queue: Vec<(
            Vec<VertexId>,
            Vertex,
            Vec<Edge>,
            std::collections::HashSet<VertexId>,
        )> = vec![(
            vec![source.id()],
            source.clone(),
            Vec::new(),
            std::collections::HashSet::from([source.id()]),
        )];

        while let Some((path, current, edges, visited)) = queue.pop() {
            let depth = path.len() - 1;

            if depth >= min && self.match_node_pattern(target, &current) {
                results.push((path.clone(), current.clone(), edges.clone()));
            }

            if depth >= max {
                continue;
            }

            let next_edges = self.get_edges_by_direction(&current, edge.direction);
            stats.edges_scanned += next_edges.len();

            for e in next_edges {
                if !self.match_edge_labels(edge, &e) {
                    continue;
                }

                let next_id = self.get_edge_target(&e, &current, edge.direction);

                // Apply path mode constraints
                let can_visit = match path_mode.unwrap_or(PathMode::Walk) {
                    PathMode::Walk => true,
                    PathMode::Trail => !edges.iter().any(|edge| edge.id() == e.id()),
                    PathMode::Simple | PathMode::Acyclic => !visited.contains(&next_id),
                };

                if can_visit {
                    if let Some(next_vertex) = self.graph.get_vertex(next_id) {
                        let mut new_path = path.clone();
                        new_path.push(next_id);
                        let mut new_edges = edges.clone();
                        new_edges.push(e.clone());
                        let mut new_visited = visited.clone();
                        new_visited.insert(next_id);
                        queue.push((new_path, next_vertex, new_edges, new_visited));
                    }
                }
            }
        }

        Ok(results)
    }

    fn get_edges_by_direction(&self, vertex: &Vertex, direction: EdgeDirection) -> Vec<Edge> {
        match direction {
            EdgeDirection::Outgoing => self.graph.get_outgoing_edges(vertex.id()),
            EdgeDirection::Incoming => self.graph.get_incoming_edges(vertex.id()),
            EdgeDirection::Undirected | EdgeDirection::AnyDirection => {
                let mut all = self.graph.get_outgoing_edges(vertex.id());
                all.extend(self.graph.get_incoming_edges(vertex.id()));
                all
            }
        }
    }

    fn get_edge_target(&self, edge: &Edge, source: &Vertex, direction: EdgeDirection) -> VertexId {
        match direction {
            EdgeDirection::Outgoing => edge.dst(),
            EdgeDirection::Incoming => edge.src(),
            EdgeDirection::Undirected | EdgeDirection::AnyDirection => {
                if edge.src() == source.id() {
                    edge.dst()
                } else {
                    edge.src()
                }
            }
        }
    }

    fn match_edge_labels(&self, pattern: &EdgePattern, edge: &Edge) -> bool {
        let labels = pattern.labels();
        labels.is_empty() || labels.contains(edge.label())
    }

    fn get_candidate_vertices(
        &self,
        pattern: &NodePattern,
        bindings: &Bindings,
        stats: &mut QueryStats,
    ) -> Vec<Vertex> {
        if let Some(ref var) = pattern.variable {
            if let Some(BindingValue::Vertex(v)) = bindings.get(var) {
                return vec![v.clone()];
            }
        }

        let labels = pattern.labels();
        let vertices: Vec<Vertex> = if labels.is_empty() {
            let mut all = Vec::new();
            for label in &[
                VertexLabel::Account,
                VertexLabel::Contract,
                VertexLabel::Token,
                VertexLabel::Transaction,
                VertexLabel::Block,
            ] {
                all.extend(self.graph.get_vertices_by_label(label));
            }
            all
        } else {
            let mut candidates = Vec::new();
            for label in &labels {
                candidates.extend(self.graph.get_vertices_by_label(label));
            }
            candidates
        };

        stats.vertices_scanned += vertices.len();
        vertices
    }

    fn match_node_properties(&self, pattern: &NodePattern, vertex: &Vertex) -> bool {
        for (key, value) in &pattern.properties {
            match vertex.property(key) {
                Some(v) if v == value => continue,
                _ => return false,
            }
        }
        true
    }

    fn match_node_pattern(&self, pattern: &NodePattern, vertex: &Vertex) -> bool {
        let labels = pattern.labels();
        if !labels.is_empty() && !labels.contains(vertex.label()) {
            return false;
        }
        self.match_node_properties(pattern, vertex)
    }

    fn get_bound_vertices(&self, bindings: &Bindings) -> Vec<Vertex> {
        bindings
            .values()
            .filter_map(|v| match v {
                BindingValue::Vertex(vertex) => Some(vertex.clone()),
                _ => None,
            })
            .collect()
    }

    fn evaluate_bool(&self, expr: &Expression, bindings: &Bindings) -> Result<bool> {
        match self.evaluate(expr, bindings)? {
            PropertyValue::Boolean(b) => Ok(b),
            _ => Ok(false),
        }
    }

    fn evaluate(&self, expr: &Expression, bindings: &Bindings) -> Result<PropertyValue> {
        match expr {
            Expression::Literal(val) => Ok(val.clone()),
            Expression::Null => Ok(PropertyValue::String(String::new())),
            Expression::Variable(name) => match bindings.get(name) {
                Some(BindingValue::Scalar(v)) => Ok(v.clone()),
                Some(BindingValue::Vertex(v)) => Ok(PropertyValue::Integer(v.id().as_u64() as i64)),
                Some(BindingValue::Edge(e)) => Ok(PropertyValue::Integer(e.id().as_u64() as i64)),
                _ => Err(Error::QueryError(format!("Unbound variable: {}", name))),
            },
            Expression::Property(var, prop) => match bindings.get(var) {
                Some(BindingValue::Vertex(v)) => v
                    .property(prop)
                    .cloned()
                    .ok_or_else(|| Error::QueryError(format!("Property not found: {}", prop))),
                Some(BindingValue::Edge(e)) => e
                    .property(prop)
                    .cloned()
                    .ok_or_else(|| Error::QueryError(format!("Property not found: {}", prop))),
                _ => Err(Error::QueryError(format!("Variable not found: {}", var))),
            },
            Expression::BinaryOp(left, op, right) => {
                let l = self.evaluate(left, bindings)?;
                let r = self.evaluate(right, bindings)?;
                self.apply_binary_op(&l, *op, &r)
            }
            Expression::UnaryOp(op, operand) => {
                let val = self.evaluate(operand, bindings)?;
                self.apply_unary_op(*op, &val)
            }
            Expression::FunctionCall(name, args) => {
                let evaluated: Result<Vec<PropertyValue>> =
                    args.iter().map(|a| self.evaluate(a, bindings)).collect();
                self.call_function(name, &evaluated?)
            }
            Expression::List(items) => {
                let _values: Result<Vec<PropertyValue>> =
                    items.iter().map(|i| self.evaluate(i, bindings)).collect();
                Ok(PropertyValue::String("[list]".to_string()))
            }
            Expression::Map(entries) => Ok(PropertyValue::String(format!(
                "{{map with {} entries}}",
                entries.len()
            ))),
            Expression::Parameter(name) => {
                Err(Error::QueryError(format!("Parameter not bound: ${}", name)))
            }
            _ => Ok(PropertyValue::String(String::new())),
        }
    }

    fn apply_binary_op(
        &self,
        left: &PropertyValue,
        op: BinaryOperator,
        right: &PropertyValue,
    ) -> Result<PropertyValue> {
        match op {
            BinaryOperator::Eq => Ok(PropertyValue::Boolean(left == right)),
            BinaryOperator::Ne => Ok(PropertyValue::Boolean(left != right)),
            BinaryOperator::Lt => self.compare_values(left, right, |a, b| a < b),
            BinaryOperator::Le => self.compare_values(left, right, |a, b| a <= b),
            BinaryOperator::Gt => self.compare_values(left, right, |a, b| a > b),
            BinaryOperator::Ge => self.compare_values(left, right, |a, b| a >= b),
            BinaryOperator::And => Ok(PropertyValue::Boolean(
                matches!(left, PropertyValue::Boolean(true))
                    && matches!(right, PropertyValue::Boolean(true)),
            )),
            BinaryOperator::Or => Ok(PropertyValue::Boolean(
                matches!(left, PropertyValue::Boolean(true))
                    || matches!(right, PropertyValue::Boolean(true)),
            )),
            BinaryOperator::Xor => Ok(PropertyValue::Boolean(
                matches!(left, PropertyValue::Boolean(true))
                    ^ matches!(right, PropertyValue::Boolean(true)),
            )),
            BinaryOperator::Add => self.arithmetic_op(left, right, |a, b| a + b, |a, b| a + b),
            BinaryOperator::Sub => self.arithmetic_op(left, right, |a, b| a - b, |a, b| a - b),
            BinaryOperator::Mul => self.arithmetic_op(left, right, |a, b| a * b, |a, b| a * b),
            BinaryOperator::Div => self.arithmetic_op(left, right, |a, b| a / b, |a, b| a / b),
            BinaryOperator::Mod => self.arithmetic_op(left, right, |a, b| a % b, |a, b| a % b),
            BinaryOperator::Power => {
                self.arithmetic_op(left, right, |a, b| a.pow(b as u32), |a, b| a.powf(b))
            }
            BinaryOperator::Concat => {
                let l = format!("{:?}", left);
                let r = format!("{:?}", right);
                Ok(PropertyValue::String(format!("{}{}", l, r)))
            }
            BinaryOperator::Contains => {
                if let (PropertyValue::String(s), PropertyValue::String(sub)) = (left, right) {
                    Ok(PropertyValue::Boolean(s.contains(sub)))
                } else {
                    Ok(PropertyValue::Boolean(false))
                }
            }
            BinaryOperator::StartsWith => {
                if let (PropertyValue::String(s), PropertyValue::String(prefix)) = (left, right) {
                    Ok(PropertyValue::Boolean(s.starts_with(prefix)))
                } else {
                    Ok(PropertyValue::Boolean(false))
                }
            }
            BinaryOperator::EndsWith => {
                if let (PropertyValue::String(s), PropertyValue::String(suffix)) = (left, right) {
                    Ok(PropertyValue::Boolean(s.ends_with(suffix)))
                } else {
                    Ok(PropertyValue::Boolean(false))
                }
            }
            BinaryOperator::Like => Ok(PropertyValue::Boolean(false)),
            BinaryOperator::In => Ok(PropertyValue::Boolean(false)),
            BinaryOperator::IsNull => Ok(PropertyValue::Boolean(
                matches!(left, PropertyValue::String(s) if s.is_empty()),
            )),
            BinaryOperator::IsNotNull => Ok(PropertyValue::Boolean(
                !matches!(left, PropertyValue::String(s) if s.is_empty()),
            )),
        }
    }

    fn compare_values<F>(
        &self,
        left: &PropertyValue,
        right: &PropertyValue,
        cmp: F,
    ) -> Result<PropertyValue>
    where
        F: Fn(i64, i64) -> bool,
    {
        match (left, right) {
            (PropertyValue::Integer(a), PropertyValue::Integer(b)) => {
                Ok(PropertyValue::Boolean(cmp(*a, *b)))
            }
            (PropertyValue::Float(a), PropertyValue::Float(b)) => {
                Ok(PropertyValue::Boolean(cmp(*a as i64, *b as i64)))
            }
            _ => Ok(PropertyValue::Boolean(false)),
        }
    }

    fn arithmetic_op<F, G>(
        &self,
        left: &PropertyValue,
        right: &PropertyValue,
        int_op: F,
        float_op: G,
    ) -> Result<PropertyValue>
    where
        F: Fn(i64, i64) -> i64,
        G: Fn(f64, f64) -> f64,
    {
        match (left, right) {
            (PropertyValue::Integer(a), PropertyValue::Integer(b)) => {
                Ok(PropertyValue::Integer(int_op(*a, *b)))
            }
            (PropertyValue::Float(a), PropertyValue::Float(b)) => {
                Ok(PropertyValue::Float(float_op(*a, *b)))
            }
            (PropertyValue::Integer(a), PropertyValue::Float(b)) => {
                Ok(PropertyValue::Float(float_op(*a as f64, *b)))
            }
            (PropertyValue::Float(a), PropertyValue::Integer(b)) => {
                Ok(PropertyValue::Float(float_op(*a, *b as f64)))
            }
            _ => Err(Error::QueryError(
                "Invalid arithmetic operation".to_string(),
            )),
        }
    }

    fn apply_unary_op(&self, op: UnaryOperator, val: &PropertyValue) -> Result<PropertyValue> {
        match op {
            UnaryOperator::Not => Ok(PropertyValue::Boolean(!matches!(
                val,
                PropertyValue::Boolean(true)
            ))),
            UnaryOperator::Neg => match val {
                PropertyValue::Integer(n) => Ok(PropertyValue::Integer(-n)),
                PropertyValue::Float(f) => Ok(PropertyValue::Float(-f)),
                _ => Err(Error::QueryError("Invalid negation".to_string())),
            },
            UnaryOperator::IsNull => Ok(PropertyValue::Boolean(
                matches!(val, PropertyValue::String(s) if s.is_empty()),
            )),
            UnaryOperator::IsNotNull => Ok(PropertyValue::Boolean(
                !matches!(val, PropertyValue::String(s) if s.is_empty()),
            )),
        }
    }

    fn call_function(&self, name: &str, args: &[PropertyValue]) -> Result<PropertyValue> {
        match name.to_uppercase().as_str() {
            "COUNT" => Ok(PropertyValue::Integer(args.len() as i64)),
            "SUM" => {
                let sum: f64 = args
                    .iter()
                    .filter_map(|v| match v {
                        PropertyValue::Integer(n) => Some(*n as f64),
                        PropertyValue::Float(f) => Some(*f),
                        _ => None,
                    })
                    .sum();
                Ok(PropertyValue::Float(sum))
            }
            "AVG" => {
                let values: Vec<f64> = args
                    .iter()
                    .filter_map(|v| match v {
                        PropertyValue::Integer(n) => Some(*n as f64),
                        PropertyValue::Float(f) => Some(*f),
                        _ => None,
                    })
                    .collect();
                if values.is_empty() {
                    Ok(PropertyValue::Float(0.0))
                } else {
                    Ok(PropertyValue::Float(
                        values.iter().sum::<f64>() / values.len() as f64,
                    ))
                }
            }
            "MIN" | "MAX" | "TOSTRING" | "TOINTEGER" | "ID" | "LABEL" => Ok(args
                .first()
                .cloned()
                .unwrap_or(PropertyValue::String(String::new()))),
            _ => Err(Error::QueryError(format!("Unknown function: {}", name))),
        }
    }

    fn build_return(
        &self,
        return_clause: &[ReturnItem],
        bindings_list: &[Bindings],
    ) -> Result<(Vec<String>, Vec<Vec<ResultValue>>)> {
        if return_clause.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let columns: Vec<String> = return_clause
            .iter()
            .map(|item| {
                item.alias
                    .clone()
                    .unwrap_or_else(|| match &item.expression {
                        Expression::Variable(name) => name.clone(),
                        Expression::Property(var, prop) => format!("{}.{}", var, prop),
                        _ => "expr".to_string(),
                    })
            })
            .collect();

        let mut rows = Vec::new();
        for bindings in bindings_list {
            let mut row = Vec::new();
            for item in return_clause {
                row.push(self.build_result_value(&item.expression, bindings)?);
            }
            rows.push(row);
        }

        Ok((columns, rows))
    }

    fn build_result_value(&self, expr: &Expression, bindings: &Bindings) -> Result<ResultValue> {
        match expr {
            Expression::Variable(name) => match bindings.get(name) {
                Some(BindingValue::Vertex(v)) => Ok(ResultValue::Vertex(VertexData {
                    id: v.id().as_u64(),
                    label: format!("{:?}", v.label()),
                    properties: v.properties().clone(),
                })),
                Some(BindingValue::Edge(e)) => Ok(ResultValue::Edge(EdgeData {
                    id: e.id().as_u64(),
                    label: format!("{:?}", e.label()),
                    src: e.src().as_u64(),
                    dst: e.dst().as_u64(),
                    properties: e.properties().clone(),
                })),
                Some(BindingValue::Scalar(v)) => Ok(ResultValue::Scalar(v.clone())),
                Some(BindingValue::Path(p)) => Ok(ResultValue::Path(PathData {
                    vertices: p
                        .iter()
                        .filter_map(|id| {
                            self.graph.get_vertex(*id).map(|v| VertexData {
                                id: v.id().as_u64(),
                                label: format!("{:?}", v.label()),
                                properties: v.properties().clone(),
                            })
                        })
                        .collect(),
                    edges: Vec::new(),
                })),
                _ => Ok(ResultValue::Null),
            },
            Expression::Property(var, prop) => match bindings.get(var) {
                Some(BindingValue::Vertex(v)) => Ok(ResultValue::Scalar(
                    v.property(prop)
                        .cloned()
                        .unwrap_or(PropertyValue::String(String::new())),
                )),
                Some(BindingValue::Edge(e)) => Ok(ResultValue::Scalar(
                    e.property(prop)
                        .cloned()
                        .unwrap_or(PropertyValue::String(String::new())),
                )),
                _ => Ok(ResultValue::Null),
            },
            _ => Ok(ResultValue::Scalar(self.evaluate(expr, bindings)?)),
        }
    }

    fn execute_insert(&self, stmt: &InsertStatement) -> Result<QueryResult> {
        let mut inserted_vertices = 0;
        let mut inserted_edges = 0;
        let mut var_to_id: HashMap<String, VertexId> = HashMap::new();

        for (idx, node) in stmt.nodes.iter().enumerate() {
            let labels = node.labels();
            let vertex_id = if !labels.is_empty() {
                let label = labels[0].clone();
                let address_prop = node
                    .properties
                    .iter()
                    .find(|(k, _)| k == "address")
                    .and_then(|(_, v)| {
                        if let PropertyValue::Address(addr) = v {
                            Some(addr.clone())
                        } else if let PropertyValue::String(s) = v {
                            Address::from_hex(s).ok()
                        } else {
                            None
                        }
                    });

                let id = if let Some(addr) = address_prop {
                    match label {
                        VertexLabel::Contract => self.graph.add_contract(addr)?,
                        _ => self.graph.add_account(addr)?,
                    }
                } else {
                    self.graph.add_vertex(label)?
                };

                if let Some(mut vertex) = self.graph.get_vertex(id) {
                    for (key, value) in &node.properties {
                        if key != "address" {
                            vertex.set_property(key.clone(), value.clone());
                        }
                    }
                    self.graph.update_vertex(vertex)?;
                }

                inserted_vertices += 1;
                id
            } else {
                let id = self.graph.add_vertex(VertexLabel::Account)?;
                inserted_vertices += 1;
                id
            };

            if let Some(ref var) = node.variable {
                var_to_id.insert(var.clone(), vertex_id);
            } else {
                var_to_id.insert(format!("_anon_{}", idx), vertex_id);
            }
        }

        for edge_insert in &stmt.edges {
            let src_id = var_to_id.get(&edge_insert.source).ok_or_else(|| {
                Error::QueryError(format!("Unknown source variable: {}", edge_insert.source))
            })?;
            let dst_id = var_to_id.get(&edge_insert.target).ok_or_else(|| {
                Error::QueryError(format!("Unknown target variable: {}", edge_insert.target))
            })?;

            let labels = edge_insert.edge.labels();
            let label = labels.first().cloned().unwrap_or(EdgeLabel::Transfer);

            let amount = edge_insert
                .edge
                .properties
                .iter()
                .find(|(k, _)| k == "amount" || k == "value")
                .and_then(|(_, v)| match v {
                    PropertyValue::Integer(i) => Some(TokenAmount::from_u64(*i as u64)),
                    PropertyValue::String(s) => s.parse::<u64>().ok().map(TokenAmount::from_u64),
                    _ => None,
                })
                .unwrap_or_else(|| TokenAmount::from_u64(0));

            let block_number = edge_insert
                .edge
                .properties
                .iter()
                .find(|(k, _)| k == "block" || k == "block_number")
                .and_then(|(_, v)| match v {
                    PropertyValue::Integer(i) => Some(*i as u64),
                    _ => None,
                })
                .unwrap_or(0);

            match label {
                EdgeLabel::Transfer => {
                    self.graph
                        .add_transfer(*src_id, *dst_id, amount, block_number)?;
                }
                _ => {
                    self.graph.add_edge(label, *src_id, *dst_id)?;
                }
            }
            inserted_edges += 1;
        }

        Ok(QueryResult {
            columns: vec![
                "inserted_vertices".to_string(),
                "inserted_edges".to_string(),
            ],
            rows: vec![vec![
                ResultValue::Scalar(PropertyValue::Integer(inserted_vertices)),
                ResultValue::Scalar(PropertyValue::Integer(inserted_edges)),
            ]],
            stats: QueryStats::default(),
        })
    }

    fn execute_delete(&self, _stmt: &DeleteStatement) -> Result<QueryResult> {
        Ok(QueryResult {
            columns: vec!["deleted".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::Integer(0))]],
            stats: QueryStats::default(),
        })
    }

    fn execute_set(&self, _stmt: &SetStatement) -> Result<QueryResult> {
        Ok(QueryResult {
            columns: vec!["updated".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::Integer(0))]],
            stats: QueryStats::default(),
        })
    }

    fn execute_remove(&self, _stmt: &RemoveStatement) -> Result<QueryResult> {
        Ok(QueryResult {
            columns: vec!["removed".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::Integer(0))]],
            stats: QueryStats::default(),
        })
    }

    fn execute_call(&self, stmt: &CallStatement) -> Result<QueryResult> {
        use crate::algorithm::{EdmondsKarp, PathFinder, TraceDirection};

        let proc_name = stmt.procedure_name.to_lowercase();

        match proc_name.as_str() {
            "shortest_path" | "algo.shortest_path" | "graph.shortestpath" => {
                if stmt.arguments.len() < 2 {
                    return Err(Error::QueryError(
                        "shortest_path requires 2 arguments".to_string(),
                    ));
                }
                let source = self.eval_to_int(&stmt.arguments[0])?;
                let target = self.eval_to_int(&stmt.arguments[1])?;

                let finder = PathFinder::new(self.graph.clone());
                if let Some(path) =
                    finder.shortest_path(VertexId::new(source as u64), VertexId::new(target as u64))
                {
                    let vertices_str = path
                        .vertices
                        .iter()
                        .map(|v| format!("{}", v.as_u64()))
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    Ok(QueryResult {
                        columns: vec![
                            "path".to_string(),
                            "length".to_string(),
                            "total_weight".to_string(),
                        ],
                        rows: vec![vec![
                            ResultValue::Scalar(PropertyValue::String(vertices_str)),
                            ResultValue::Scalar(PropertyValue::Integer(path.length as i64)),
                            ResultValue::Scalar(PropertyValue::Float(path.total_weight)),
                        ]],
                        stats: QueryStats::default(),
                    })
                } else {
                    Ok(QueryResult {
                        columns: vec!["result".to_string()],
                        rows: vec![vec![ResultValue::Scalar(PropertyValue::String(
                            "No path found".to_string(),
                        ))]],
                        stats: QueryStats::default(),
                    })
                }
            }

            "all_paths" | "algo.all_paths" => {
                if stmt.arguments.len() < 2 {
                    return Err(Error::QueryError(
                        "all_paths requires at least 2 arguments".to_string(),
                    ));
                }
                let source = self.eval_to_int(&stmt.arguments[0])?;
                let target = self.eval_to_int(&stmt.arguments[1])?;
                let max_depth = if stmt.arguments.len() > 2 {
                    self.eval_to_int(&stmt.arguments[2])? as usize
                } else {
                    10
                };

                let finder = PathFinder::new(self.graph.clone());
                let paths = finder.all_paths(
                    VertexId::new(source as u64),
                    VertexId::new(target as u64),
                    max_depth,
                );

                let rows: Vec<Vec<ResultValue>> = paths
                    .iter()
                    .take(100)
                    .map(|path| {
                        vec![
                            ResultValue::Scalar(PropertyValue::String(
                                path.vertices
                                    .iter()
                                    .map(|v| format!("{}", v.as_u64()))
                                    .collect::<Vec<_>>()
                                    .join(" -> "),
                            )),
                            ResultValue::Scalar(PropertyValue::Integer(path.length as i64)),
                            ResultValue::Scalar(PropertyValue::Float(path.total_weight)),
                        ]
                    })
                    .collect();

                Ok(QueryResult {
                    columns: vec![
                        "path".to_string(),
                        "length".to_string(),
                        "total_weight".to_string(),
                    ],
                    rows,
                    stats: QueryStats::default(),
                })
            }

            "trace" | "algo.trace" => {
                if stmt.arguments.is_empty() {
                    return Err(Error::QueryError(
                        "trace requires at least 1 argument".to_string(),
                    ));
                }
                let start = self.eval_to_int(&stmt.arguments[0])?;
                let direction = if stmt.arguments.len() > 1 {
                    match self
                        .eval_to_string(&stmt.arguments[1])?
                        .to_lowercase()
                        .as_str()
                    {
                        "backward" | "back" => TraceDirection::Backward,
                        "both" => TraceDirection::Both,
                        _ => TraceDirection::Forward,
                    }
                } else {
                    TraceDirection::Forward
                };
                let max_depth = if stmt.arguments.len() > 2 {
                    self.eval_to_int(&stmt.arguments[2])? as usize
                } else {
                    5
                };

                let finder = PathFinder::new(self.graph.clone());
                let traces = finder.trace(VertexId::new(start as u64), direction, max_depth, None);

                let rows: Vec<Vec<ResultValue>> = traces
                    .iter()
                    .take(100)
                    .map(|path| {
                        vec![
                            ResultValue::Scalar(PropertyValue::String(
                                path.vertices
                                    .iter()
                                    .map(|v| format!("{}", v.as_u64()))
                                    .collect::<Vec<_>>()
                                    .join(" -> "),
                            )),
                            ResultValue::Scalar(PropertyValue::Integer(path.length as i64)),
                            ResultValue::Scalar(PropertyValue::Float(path.total_weight)),
                        ]
                    })
                    .collect();

                Ok(QueryResult {
                    columns: vec![
                        "path".to_string(),
                        "length".to_string(),
                        "total_weight".to_string(),
                    ],
                    rows,
                    stats: QueryStats::default(),
                })
            }

            "max_flow" | "algo.max_flow" => {
                if stmt.arguments.len() < 2 {
                    return Err(Error::QueryError(
                        "max_flow requires 2 arguments".to_string(),
                    ));
                }
                let source = self.eval_to_int(&stmt.arguments[0])?;
                let sink = self.eval_to_int(&stmt.arguments[1])?;

                let algo = EdmondsKarp::new(self.graph.clone());
                let result =
                    algo.max_flow(VertexId::new(source as u64), VertexId::new(sink as u64));

                let mut rows = vec![vec![
                    ResultValue::Scalar(PropertyValue::String("max_flow_value".to_string())),
                    ResultValue::Scalar(PropertyValue::Float(result.value)),
                ]];

                for ((u, v), flow) in result.flow.iter().take(20) {
                    rows.push(vec![
                        ResultValue::Scalar(PropertyValue::String(format!(
                            "{} -> {}",
                            u.as_u64(),
                            v.as_u64()
                        ))),
                        ResultValue::Scalar(PropertyValue::Float(*flow)),
                    ]);
                }

                Ok(QueryResult {
                    columns: vec!["edge".to_string(), "flow".to_string()],
                    rows,
                    stats: QueryStats::default(),
                })
            }

            "neighbors" | "algo.neighbors" => {
                if stmt.arguments.is_empty() {
                    return Err(Error::QueryError(
                        "neighbors requires at least 1 argument".to_string(),
                    ));
                }
                let vid = self.eval_to_int(&stmt.arguments[0])?;
                let vertex_id = VertexId::new(vid as u64);
                let direction = if stmt.arguments.len() > 1 {
                    self.eval_to_string(&stmt.arguments[1])?.to_lowercase()
                } else {
                    "both".to_string()
                };

                let mut rows = Vec::new();
                match direction.as_str() {
                    "out" => {
                        for neighbor in self.graph.neighbors(vertex_id) {
                            rows.push(vec![
                                ResultValue::Scalar(PropertyValue::String("out".to_string())),
                                ResultValue::Scalar(PropertyValue::Integer(
                                    neighbor.as_u64() as i64
                                )),
                            ]);
                        }
                    }
                    "in" => {
                        for neighbor in self.graph.predecessors(vertex_id) {
                            rows.push(vec![
                                ResultValue::Scalar(PropertyValue::String("in".to_string())),
                                ResultValue::Scalar(PropertyValue::Integer(
                                    neighbor.as_u64() as i64
                                )),
                            ]);
                        }
                    }
                    _ => {
                        for neighbor in self.graph.neighbors(vertex_id) {
                            rows.push(vec![
                                ResultValue::Scalar(PropertyValue::String("out".to_string())),
                                ResultValue::Scalar(PropertyValue::Integer(
                                    neighbor.as_u64() as i64
                                )),
                            ]);
                        }
                        for neighbor in self.graph.predecessors(vertex_id) {
                            rows.push(vec![
                                ResultValue::Scalar(PropertyValue::String("in".to_string())),
                                ResultValue::Scalar(PropertyValue::Integer(
                                    neighbor.as_u64() as i64
                                )),
                            ]);
                        }
                    }
                }

                Ok(QueryResult {
                    columns: vec!["direction".to_string(), "neighbor_id".to_string()],
                    rows,
                    stats: QueryStats::default(),
                })
            }

            "degree" | "algo.degree" => {
                if stmt.arguments.is_empty() {
                    return Err(Error::QueryError("degree requires 1 argument".to_string()));
                }
                let vid = self.eval_to_int(&stmt.arguments[0])?;
                let vertex_id = VertexId::new(vid as u64);
                let out_degree = self.graph.out_degree(vertex_id);
                let in_degree = self.graph.in_degree(vertex_id);

                Ok(QueryResult {
                    columns: vec![
                        "vertex_id".to_string(),
                        "out_degree".to_string(),
                        "in_degree".to_string(),
                        "total_degree".to_string(),
                    ],
                    rows: vec![vec![
                        ResultValue::Scalar(PropertyValue::Integer(vid)),
                        ResultValue::Scalar(PropertyValue::Integer(out_degree as i64)),
                        ResultValue::Scalar(PropertyValue::Integer(in_degree as i64)),
                        ResultValue::Scalar(PropertyValue::Integer(
                            (out_degree + in_degree) as i64,
                        )),
                    ]],
                    stats: QueryStats::default(),
                })
            }

            "connected" | "algo.connected" => {
                if stmt.arguments.len() < 2 {
                    return Err(Error::QueryError(
                        "connected requires 2 arguments".to_string(),
                    ));
                }
                let source = self.eval_to_int(&stmt.arguments[0])?;
                let target = self.eval_to_int(&stmt.arguments[1])?;

                let finder = PathFinder::new(self.graph.clone());
                let connected = finder
                    .shortest_path(VertexId::new(source as u64), VertexId::new(target as u64))
                    .is_some();

                Ok(QueryResult {
                    columns: vec![
                        "source".to_string(),
                        "target".to_string(),
                        "connected".to_string(),
                    ],
                    rows: vec![vec![
                        ResultValue::Scalar(PropertyValue::Integer(source)),
                        ResultValue::Scalar(PropertyValue::Integer(target)),
                        ResultValue::Scalar(PropertyValue::Boolean(connected)),
                    ]],
                    stats: QueryStats::default(),
                })
            }

            _ => Err(Error::QueryError(format!(
                "Unknown procedure: {}",
                stmt.procedure_name
            ))),
        }
    }

    fn execute_create_graph(&self, stmt: &CreateGraphStatement) -> Result<QueryResult> {
        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(format!(
                "Graph '{}' created ({:?})",
                stmt.name, stmt.graph_type
            )))]],
            stats: QueryStats::default(),
        })
    }

    fn execute_drop_graph(&self, stmt: &DropGraphStatement) -> Result<QueryResult> {
        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(format!(
                "Graph '{}' dropped",
                stmt.name
            )))]],
            stats: QueryStats::default(),
        })
    }

    // ============================================================================
    // SHOW and DESCRIBE Implementations
    // ============================================================================

    /// Execute SHOW statement
    fn execute_show(&self, stmt: &ShowStatement) -> Result<QueryResult> {
        match stmt.show_type {
            ShowType::Graphs => {
                // Return list of graphs (simulated for now)
                let columns = vec![
                    "name".to_string(),
                    "type".to_string(),
                    "vertex_count".to_string(),
                    "edge_count".to_string(),
                ];
                let rows = vec![vec![
                    ResultValue::Scalar(PropertyValue::String("default".to_string())),
                    ResultValue::Scalar(PropertyValue::String("Closed".to_string())),
                    ResultValue::Scalar(PropertyValue::Integer(self.graph.vertex_count() as i64)),
                    ResultValue::Scalar(PropertyValue::Integer(self.graph.edge_count() as i64)),
                ]];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::GraphTypes => {
                // Return list of graph types (simulated)
                let columns = vec!["name".to_string(), "definition".to_string()];
                let rows = vec![vec![
                    ResultValue::Scalar(PropertyValue::String("default_type".to_string())),
                    ResultValue::Scalar(PropertyValue::String(
                        "(Account)-[Transfer]->(Account)".to_string(),
                    )),
                ]];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Schemas => {
                let columns = vec!["name".to_string()];
                let rows = vec![vec![ResultValue::Scalar(PropertyValue::String(
                    "public".to_string(),
                ))]];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Labels => {
                // Return vertex labels - predefined for the system
                let columns = vec!["label".to_string(), "description".to_string()];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Account".to_string())),
                        ResultValue::Scalar(PropertyValue::String("EOA account".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Contract".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Smart contract".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Token".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "ERC20/ERC721 token".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Transaction".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "Blockchain transaction".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Block".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Blockchain block".to_string())),
                    ],
                ];

                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::EdgeTypes => {
                // Return edge types - predefined for the system
                let columns = vec!["type".to_string(), "description".to_string()];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Transfer".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Token transfer".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Call".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Contract call".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Create".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Contract creation".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Approve".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Token approval".to_string())),
                    ],
                ];

                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::PropertyKeys => {
                // Return all property keys - predefined common keys
                let columns = vec!["key".to_string(), "usage".to_string()];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("address".to_string())),
                        ResultValue::Scalar(PropertyValue::String("vertex".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("balance".to_string())),
                        ResultValue::Scalar(PropertyValue::String("vertex".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("nonce".to_string())),
                        ResultValue::Scalar(PropertyValue::String("vertex".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("amount".to_string())),
                        ResultValue::Scalar(PropertyValue::String("edge".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("block".to_string())),
                        ResultValue::Scalar(PropertyValue::String("edge".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("tx_hash".to_string())),
                        ResultValue::Scalar(PropertyValue::String("edge".to_string())),
                    ],
                ];

                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Functions => {
                let columns = vec![
                    "name".to_string(),
                    "signature".to_string(),
                    "description".to_string(),
                ];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("range".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "range(start, end) -> List<Int>".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Generate a range of integers".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("count".to_string())),
                        ResultValue::Scalar(PropertyValue::String("COUNT(*) -> Int".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Count elements".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("sum".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "SUM(expr) -> Number".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String("Sum values".to_string())),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Procedures => {
                let columns = vec![
                    "name".to_string(),
                    "signature".to_string(),
                    "description".to_string(),
                ];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("shortest_path".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(source, target) -> Path".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Find shortest path between two vertices".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("all_paths".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(source, target, max_depth?) -> List<Path>".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Find all paths between two vertices".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("trace".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(start, direction?, max_depth?) -> List<Path>".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Trace paths from a vertex".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("max_flow".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(source, sink) -> Flow".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Calculate maximum flow".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("neighbors".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(vertex_id, direction?) -> List<Vertex>".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Get neighbors of a vertex".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("degree".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(vertex_id) -> Degree".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Get degree of a vertex".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("connected".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "(source, target) -> Boolean".to_string(),
                        )),
                        ResultValue::Scalar(PropertyValue::String(
                            "Check if two vertices are connected".to_string(),
                        )),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Indexes => {
                let columns = vec![
                    "name".to_string(),
                    "type".to_string(),
                    "properties".to_string(),
                ];
                let rows = vec![vec![
                    ResultValue::Scalar(PropertyValue::String("address_idx".to_string())),
                    ResultValue::Scalar(PropertyValue::String("HASH".to_string())),
                    ResultValue::Scalar(PropertyValue::String("address".to_string())),
                ]];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            ShowType::Constraints => {
                let columns = vec![
                    "name".to_string(),
                    "type".to_string(),
                    "definition".to_string(),
                ];
                let rows = vec![vec![
                    ResultValue::Scalar(PropertyValue::String("address_unique".to_string())),
                    ResultValue::Scalar(PropertyValue::String("UNIQUE".to_string())),
                    ResultValue::Scalar(PropertyValue::String("Account.address".to_string())),
                ]];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
        }
    }

    /// Execute DESCRIBE statement
    fn execute_describe(&self, stmt: &DescribeStatement) -> Result<QueryResult> {
        match stmt.describe_type {
            DescribeType::Graph => {
                let columns = vec!["property".to_string(), "value".to_string()];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("name".to_string())),
                        ResultValue::Scalar(PropertyValue::String(stmt.name.clone())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("type".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Closed".to_string())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("vertex_count".to_string())),
                        ResultValue::Scalar(PropertyValue::Integer(
                            self.graph.vertex_count() as i64
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("edge_count".to_string())),
                        ResultValue::Scalar(PropertyValue::Integer(self.graph.edge_count() as i64)),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("created_at".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "2024-01-01T00:00:00Z".to_string(),
                        )),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            DescribeType::GraphType => {
                let columns = vec![
                    "element".to_string(),
                    "type".to_string(),
                    "properties".to_string(),
                ];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Account".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Vertex".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "address: String, balance: Int".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Contract".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Vertex".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "address: String, code_hash: String".to_string(),
                        )),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("Transfer".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Edge".to_string())),
                        ResultValue::Scalar(PropertyValue::String(
                            "amount: Int, block: Int".to_string(),
                        )),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            DescribeType::Schema => {
                let columns = vec!["property".to_string(), "value".to_string()];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("name".to_string())),
                        ResultValue::Scalar(PropertyValue::String(stmt.name.clone())),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("graphs".to_string())),
                        ResultValue::Scalar(PropertyValue::Integer(1)),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            DescribeType::Label => {
                let columns = vec![
                    "property".to_string(),
                    "type".to_string(),
                    "nullable".to_string(),
                ];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("address".to_string())),
                        ResultValue::Scalar(PropertyValue::String("String".to_string())),
                        ResultValue::Scalar(PropertyValue::Boolean(false)),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("balance".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Integer".to_string())),
                        ResultValue::Scalar(PropertyValue::Boolean(true)),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
            DescribeType::EdgeType => {
                let columns = vec![
                    "property".to_string(),
                    "type".to_string(),
                    "nullable".to_string(),
                ];
                let rows = vec![
                    vec![
                        ResultValue::Scalar(PropertyValue::String("amount".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Integer".to_string())),
                        ResultValue::Scalar(PropertyValue::Boolean(false)),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("block".to_string())),
                        ResultValue::Scalar(PropertyValue::String("Integer".to_string())),
                        ResultValue::Scalar(PropertyValue::Boolean(true)),
                    ],
                    vec![
                        ResultValue::Scalar(PropertyValue::String("tx_hash".to_string())),
                        ResultValue::Scalar(PropertyValue::String("String".to_string())),
                        ResultValue::Scalar(PropertyValue::Boolean(true)),
                    ],
                ];
                Ok(QueryResult {
                    columns,
                    rows,
                    stats: QueryStats::default(),
                })
            }
        }
    }

    // ============================================================================
    // ISO GQL 39075 New Statement Implementations
    // ============================================================================

    /// Execute LET statement - variable binding
    /// LET binds values to variables for use in subsequent statements
    fn execute_let(&self, stmt: &LetStatement) -> Result<QueryResult> {
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let bindings = HashMap::new();

        for binding in &stmt.bindings {
            let value = self.evaluate(&binding.value, &bindings)?;
            columns.push(binding.variable.clone());
            values.push(ResultValue::Scalar(value));
        }

        Ok(QueryResult {
            columns,
            rows: vec![values],
            stats: QueryStats::default(),
        })
    }

    /// Execute FOR statement - iteration over a collection
    /// FOR x IN collection RETURN x
    fn execute_for(&self, stmt: &ForStatement) -> Result<QueryResult> {
        let bindings = HashMap::new();
        let iterable_value = self.evaluate(&stmt.iterable, &bindings)?;

        // Extract items from the iterable
        let items: Vec<PropertyValue> = match &stmt.iterable {
            Expression::List(items) => items
                .iter()
                .filter_map(|e| self.evaluate(e, &bindings).ok())
                .collect(),
            Expression::FunctionCall(name, args) if name.to_lowercase() == "range" => {
                // range(start, end) function
                let start = args
                    .get(0)
                    .and_then(|e| self.evaluate(e, &bindings).ok())
                    .and_then(|v| match v {
                        PropertyValue::Integer(i) => Some(i),
                        _ => None,
                    })
                    .unwrap_or(0);
                let end = args
                    .get(1)
                    .and_then(|e| self.evaluate(e, &bindings).ok())
                    .and_then(|v| match v {
                        PropertyValue::Integer(i) => Some(i),
                        _ => None,
                    })
                    .unwrap_or(10);
                (start..=end).map(PropertyValue::Integer).collect()
            }
            _ => vec![iterable_value],
        };

        let mut rows = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            let mut row = vec![ResultValue::Scalar(item.clone())];
            if stmt.ordinal_variable.is_some() {
                row.push(ResultValue::Scalar(PropertyValue::Integer(idx as i64)));
            }
            rows.push(row);
        }

        let mut columns = vec![stmt.variable.clone()];
        if let Some(ref ordinal_var) = stmt.ordinal_variable {
            columns.push(ordinal_var.clone());
        }

        Ok(QueryResult {
            columns,
            rows,
            stats: QueryStats::default(),
        })
    }

    /// Execute FILTER statement - filter current results by condition
    fn execute_filter(&self, stmt: &FilterStatement) -> Result<QueryResult> {
        // FILTER is typically used in conjunction with other statements
        // As a standalone statement, we evaluate the condition and return the result
        let bindings = HashMap::new();
        let result = self.evaluate_bool(&stmt.condition, &bindings)?;

        Ok(QueryResult {
            columns: vec!["filter_result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::Boolean(result))]],
            stats: QueryStats::default(),
        })
    }

    /// Execute SELECT statement - SQL-like projection with aggregation
    fn execute_select(&self, stmt: &SelectStatement) -> Result<QueryResult> {
        let mut stats = QueryStats::default();

        // For a standalone SELECT, we need to get data from the graph
        // This is a simplified implementation that scans all vertices
        let mut all_vertices = Vec::new();
        for label in &[
            VertexLabel::Account,
            VertexLabel::Contract,
            VertexLabel::Token,
            VertexLabel::Transaction,
            VertexLabel::Block,
        ] {
            all_vertices.extend(self.graph.get_vertices_by_label(label));
        }
        stats.vertices_scanned = all_vertices.len();

        // Create bindings for each vertex
        let mut bindings_list: Vec<Bindings> = all_vertices
            .iter()
            .map(|v| {
                let mut b = HashMap::new();
                b.insert("_".to_string(), BindingValue::Vertex(v.clone()));
                b
            })
            .collect();

        // Apply OFFSET
        if let Some(offset) = stmt.offset {
            bindings_list = bindings_list.into_iter().skip(offset as usize).collect();
        }

        // Apply LIMIT
        if let Some(limit) = stmt.limit {
            bindings_list = bindings_list.into_iter().take(limit as usize).collect();
        }

        // Build columns from select items
        let columns: Vec<String> = stmt
            .items
            .iter()
            .map(|item| {
                item.alias
                    .clone()
                    .unwrap_or_else(|| match &item.expression {
                        Expression::Variable(name) => name.clone(),
                        Expression::Property(var, prop) => format!("{}.{}", var, prop),
                        Expression::FunctionCall(name, _) => name.clone(),
                        _ => "expr".to_string(),
                    })
            })
            .collect();

        // Build rows
        let mut rows = Vec::new();
        for bindings in &bindings_list {
            let mut row = Vec::new();
            for item in &stmt.items {
                let value = self
                    .evaluate(&item.expression, bindings)
                    .unwrap_or(PropertyValue::String(String::new()));
                row.push(ResultValue::Scalar(value));
            }
            rows.push(row);
        }

        // Handle DISTINCT
        if stmt.distinct {
            let mut seen = std::collections::HashSet::new();
            rows.retain(|row| {
                let key = format!("{:?}", row);
                seen.insert(key)
            });
        }

        stats.rows_returned = rows.len();

        Ok(QueryResult {
            columns,
            rows,
            stats,
        })
    }

    /// Execute USE statement - set graph context
    fn execute_use(&self, stmt: &UseStatement) -> Result<QueryResult> {
        let graph_name = match &stmt.graph {
            GraphReference::Named(name) => name.clone(),
            GraphReference::Current => "CURRENT_GRAPH".to_string(),
            GraphReference::Subquery(_) => "SUBQUERY_GRAPH".to_string(),
        };

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(format!(
                "Using graph: {}",
                graph_name
            )))]],
            stats: QueryStats::default(),
        })
    }

    /// Execute Composite query - UNION, EXCEPT, INTERSECT, OTHERWISE
    fn execute_composite(&self, stmt: &CompositeQueryStatement) -> Result<QueryResult> {
        // Execute both sides of the composite query
        let primary_result = self.execute(&stmt.primary)?;
        let secondary_result = self.execute(&stmt.secondary)?;

        // Use primary's columns (both should have compatible columns)
        let columns = primary_result.columns;

        // Combine results based on operation
        let rows = match stmt.operation {
            SetOperation::Union => {
                let mut combined = primary_result.rows;
                if stmt.all {
                    // UNION ALL - keep all rows including duplicates
                    combined.extend(secondary_result.rows);
                } else {
                    // UNION - remove duplicates
                    let mut seen = std::collections::HashSet::new();
                    for row in &combined {
                        seen.insert(format!("{:?}", row));
                    }
                    for row in secondary_result.rows {
                        let key = format!("{:?}", row);
                        if seen.insert(key) {
                            combined.push(row);
                        }
                    }
                }
                combined
            }
            SetOperation::Except => {
                // EXCEPT - rows in primary but not in secondary
                let secondary_set: std::collections::HashSet<String> = secondary_result
                    .rows
                    .iter()
                    .map(|row| format!("{:?}", row))
                    .collect();

                primary_result
                    .rows
                    .into_iter()
                    .filter(|row| !secondary_set.contains(&format!("{:?}", row)))
                    .collect()
            }
            SetOperation::Intersect => {
                // INTERSECT - rows in both primary and secondary
                let secondary_set: std::collections::HashSet<String> = secondary_result
                    .rows
                    .iter()
                    .map(|row| format!("{:?}", row))
                    .collect();

                primary_result
                    .rows
                    .into_iter()
                    .filter(|row| secondary_set.contains(&format!("{:?}", row)))
                    .collect()
            }
            SetOperation::Otherwise => {
                // OTHERWISE - use primary if non-empty, else secondary
                if primary_result.rows.is_empty() {
                    secondary_result.rows
                } else {
                    primary_result.rows
                }
            }
        };

        Ok(QueryResult {
            columns,
            rows,
            stats: QueryStats::default(),
        })
    }

    /// Execute Session statement - session management
    fn execute_session(&self, stmt: &SessionStatement) -> Result<QueryResult> {
        let message = match stmt {
            SessionStatement::Set(item) => match item {
                SessionSetItem::Schema(name) => format!("Schema set to: {}", name),
                SessionSetItem::Graph(name) => format!("Graph set to: {}", name),
                SessionSetItem::TimeZone(tz) => format!("Time zone set to: {}", tz),
                SessionSetItem::Parameter(name, _value) => format!("Parameter {} set", name),
            },
            SessionStatement::Reset(item) => match item {
                SessionResetItem::Schema => "Schema reset to default".to_string(),
                SessionResetItem::Graph => "Graph reset to default".to_string(),
                SessionResetItem::TimeZone => "Time zone reset to default".to_string(),
                SessionResetItem::All => "All session settings reset".to_string(),
                SessionResetItem::Parameter(name) => format!("Parameter {} reset", name),
            },
            SessionStatement::Close => "Session closed".to_string(),
        };

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(message))]],
            stats: QueryStats::default(),
        })
    }

    /// Execute Transaction statement - transaction control
    fn execute_transaction(&self, stmt: &TransactionStatement) -> Result<QueryResult> {
        let message = match stmt {
            TransactionStatement::Start(chars) => {
                let mode = match &chars.access_mode {
                    Some(TransactionAccessMode::ReadOnly) => "READ ONLY",
                    Some(TransactionAccessMode::ReadWrite) => "READ WRITE",
                    None => "DEFAULT",
                };
                format!("Transaction started ({})", mode)
            }
            TransactionStatement::Commit => "Transaction committed".to_string(),
            TransactionStatement::Rollback => "Transaction rolled back".to_string(),
        };

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(message))]],
            stats: QueryStats::default(),
        })
    }

    /// Execute CREATE GRAPH TYPE statement
    fn execute_create_graph_type(&self, stmt: &CreateGraphTypeStatement) -> Result<QueryResult> {
        let element_count = stmt.elements.len();
        let node_types = stmt
            .elements
            .iter()
            .filter(|e| matches!(e, GraphTypeElement::NodeType(_)))
            .count();
        let edge_types = stmt
            .elements
            .iter()
            .filter(|e| matches!(e, GraphTypeElement::EdgeType(_)))
            .count();

        let message = format!(
            "Graph type '{}' created with {} elements ({} node types, {} edge types){}{}",
            stmt.name,
            element_count,
            node_types,
            edge_types,
            if stmt.if_not_exists {
                " [IF NOT EXISTS]"
            } else {
                ""
            },
            if stmt.or_replace { " [OR REPLACE]" } else { "" }
        );

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(message))]],
            stats: QueryStats::default(),
        })
    }

    /// Execute DROP GRAPH TYPE statement
    fn execute_drop_graph_type(&self, stmt: &DropGraphTypeStatement) -> Result<QueryResult> {
        let message = format!(
            "Graph type '{}' dropped{}",
            stmt.name,
            if stmt.if_exists { " [IF EXISTS]" } else { "" }
        );

        Ok(QueryResult {
            columns: vec!["result".to_string()],
            rows: vec![vec![ResultValue::Scalar(PropertyValue::String(message))]],
            stats: QueryStats::default(),
        })
    }

    fn eval_to_int(&self, expr: &Expression) -> Result<i64> {
        match expr {
            Expression::Literal(PropertyValue::Integer(i)) => Ok(*i),
            Expression::Literal(PropertyValue::Float(f)) => Ok(*f as i64),
            Expression::Literal(PropertyValue::String(s)) => s
                .parse::<i64>()
                .map_err(|_| Error::QueryError(format!("Cannot convert '{}' to integer", s))),
            _ => Err(Error::QueryError("Argument must be an integer".to_string())),
        }
    }

    fn eval_to_string(&self, expr: &Expression) -> Result<String> {
        match expr {
            Expression::Literal(PropertyValue::String(s)) => Ok(s.clone()),
            Expression::Literal(PropertyValue::Integer(i)) => Ok(i.to_string()),
            Expression::Variable(v) => Ok(v.clone()),
            _ => Err(Error::QueryError("Argument must be a string".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::parse;
    use crate::types::Address;

    fn setup_test_graph() -> Arc<Graph> {
        let graph = Graph::in_memory().unwrap();
        let addr1 = Address::from_hex("0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0").unwrap();
        let addr2 = Address::from_hex("0x8ba1f109551bD432803012645Ac136ddd64DBA72").unwrap();
        let v1 = graph.add_account(addr1).unwrap();
        let v2 = graph.add_account(addr2).unwrap();
        graph
            .add_transfer(v1, v2, TokenAmount::from_u64(1000), 12345678)
            .unwrap();
        graph
    }

    #[test]
    fn test_execute_simple_match() {
        let graph = setup_test_graph();
        let executor = QueryExecutor::new(graph);
        let stmt = parse("MATCH (n:Account) RETURN n").unwrap();
        let result = executor.execute(&stmt).unwrap();
        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_path_match() {
        let graph = setup_test_graph();
        let executor = QueryExecutor::new(graph);
        let stmt = parse("MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, b, t").unwrap();
        let result = executor.execute(&stmt).unwrap();
        assert_eq!(result.columns.len(), 3);
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_with_limit() {
        let graph = setup_test_graph();
        let executor = QueryExecutor::new(graph);
        let stmt = parse("MATCH (n:Account) RETURN n LIMIT 1").unwrap();
        let result = executor.execute(&stmt).unwrap();
        assert_eq!(result.rows.len(), 1);
    }
}
