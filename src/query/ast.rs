//! GQL Abstract Syntax Tree (AST) - ISO/IEC 39075 Compliant
//!
//! This module defines the Abstract Syntax Tree for GQL (Graph Query Language)
//! based on the ISO/IEC 39075 standard (April 2024).
//!
//! Reference: https://github.com/opengql/grammar
//!
//! Key GQL features:
//! - Path modes: WALK, TRAIL, SIMPLE, ACYCLIC
//! - Path search prefixes: ALL, ANY, SHORTEST
//! - Match modes: REPEATABLE ELEMENTS, DIFFERENT EDGES
//! - Inline Graph Types: NODE and EDGE definitions within CREATE GRAPH
//! - Full label expressions with negation, conjunction, disjunction
//! - Quantified path patterns

use crate::types::{EdgeLabel, PropertyValue, VertexLabel};
use std::fmt;

// ============================================================================
// GQL Statement (Top-Level)
// ============================================================================

/// Top-level GQL statement
#[derive(Debug, Clone)]
pub enum GqlStatement {
    /// MATCH query statement
    Match(MatchStatement),
    /// INSERT statement
    Insert(InsertStatement),
    /// DELETE statement
    Delete(DeleteStatement),
    /// SET statement
    Set(SetStatement),
    /// REMOVE statement
    Remove(RemoveStatement),
    /// CALL procedure statement
    Call(CallStatement),
    /// CREATE GRAPH statement
    CreateGraph(CreateGraphStatement),
    /// DROP GRAPH statement
    DropGraph(DropGraphStatement),
    /// SHOW statement (SHOW GRAPHS, SHOW LABELS, etc.)
    Show(ShowStatement),
    /// DESCRIBE statement (DESCRIBE GRAPH, etc.)
    Describe(DescribeStatement),
    /// LET statement (variable definition)
    Let(LetStatement),
    /// FOR statement (iteration)
    For(ForStatement),
    /// FILTER statement
    Filter(FilterStatement),
    /// Composite query (UNION/EXCEPT/INTERSECT)
    Composite(CompositeQueryStatement),
    /// USE graph clause
    Use(UseStatement),
    /// SELECT statement (SQL-like)
    Select(SelectStatement),
    /// Session management statement
    Session(SessionStatement),
    /// Transaction statement
    Transaction(TransactionStatement),
}

// ============================================================================
// MATCH Statement (ISO GQL 39075)
// ============================================================================

/// MATCH statement with full GQL support
/// matchStatement: simpleMatchStatement | optionalMatchStatement
#[derive(Debug, Clone)]
pub struct MatchStatement {
    /// Is this an OPTIONAL MATCH?
    pub optional: bool,
    /// Match mode (REPEATABLE ELEMENTS or DIFFERENT EDGES)
    pub match_mode: Option<MatchMode>,
    /// Graph pattern to match
    pub graph_pattern: GraphPattern,
    /// WHERE clause
    pub where_clause: Option<Expression>,
    /// RETURN clause
    pub return_clause: Vec<ReturnItem>,
    /// ORDER BY clause
    pub order_by: Option<Vec<OrderByItem>>,
    /// SKIP clause
    pub skip: Option<usize>,
    /// LIMIT clause
    pub limit: Option<usize>,
}

impl MatchStatement {
    pub fn new(graph_pattern: GraphPattern) -> Self {
        MatchStatement {
            optional: false,
            match_mode: None,
            graph_pattern,
            where_clause: None,
            return_clause: Vec::new(),
            order_by: None,
            skip: None,
            limit: None,
        }
    }

    pub fn with_where(mut self, expr: Expression) -> Self {
        self.where_clause = Some(expr);
        self
    }

    pub fn with_return(mut self, items: Vec<ReturnItem>) -> Self {
        self.return_clause = items;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Match mode (ISO GQL 39075)
/// matchMode: repeatableElementsMatchMode | differentEdgesMatchMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    /// REPEATABLE ELEMENTS - allows repeated vertices and edges
    RepeatableElements,
    /// DIFFERENT EDGES - edges must be different (default in most cases)
    DifferentEdges,
}

impl fmt::Display for MatchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchMode::RepeatableElements => write!(f, "REPEATABLE ELEMENTS"),
            MatchMode::DifferentEdges => write!(f, "DIFFERENT EDGES"),
        }
    }
}

// ============================================================================
// Graph Pattern (ISO GQL 39075)
// ============================================================================

/// Graph pattern containing path patterns
/// graphPattern: matchMode? pathPatternList keepClause? graphPatternWhereClause?
#[derive(Debug, Clone)]
pub struct GraphPattern {
    /// List of path patterns (comma-separated in GQL)
    pub paths: Vec<PathPattern>,
    /// KEEP clause for path filtering (ISO GQL 39075)
    pub keep_clause: Option<KeepClause>,
}

impl GraphPattern {
    pub fn new() -> Self {
        GraphPattern { 
            paths: Vec::new(),
            keep_clause: None,
        }
    }

    pub fn with_path(mut self, path: PathPattern) -> Self {
        self.paths.push(path);
        self
    }

    pub fn with_keep(mut self, keep: KeepClause) -> Self {
        self.keep_clause = Some(keep);
        self
    }
}

/// KEEP clause for path filtering (ISO GQL 39075)
/// keepClause: KEEP pathPatternPrefix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepClause {
    /// Path search prefix to keep (e.g., ALL SHORTEST, ANY, etc.)
    pub path_prefix: PathSearchPrefix,
}

impl Default for GraphPattern {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Path Pattern (ISO GQL 39075)
// ============================================================================

/// Path pattern with optional prefix and quantifier
/// pathPattern: pathVariableDeclaration? pathPatternPrefix? pathPatternExpression
#[derive(Debug, Clone)]
pub struct PathPattern {
    /// Path variable name (e.g., p = ...)
    pub variable: Option<String>,
    /// Path mode prefix (WALK, TRAIL, SIMPLE, ACYCLIC)
    pub path_mode: Option<PathMode>,
    /// Path search prefix (ALL, ANY, SHORTEST)
    pub search_prefix: Option<PathSearchPrefix>,
    /// Path elements (alternating nodes and edges)
    pub elements: Vec<PathElement>,
    /// Quantifier for the entire path pattern
    pub quantifier: Option<PatternQuantifier>,
}

impl PathPattern {
    pub fn new() -> Self {
        PathPattern {
            variable: None,
            path_mode: None,
            search_prefix: None,
            elements: Vec::new(),
            quantifier: None,
        }
    }

    pub fn add_node(mut self, node: NodePattern) -> Self {
        self.elements.push(PathElement::Node(node));
        self
    }

    pub fn add_edge(mut self, edge: EdgePattern) -> Self {
        self.elements.push(PathElement::Edge(edge));
        self
    }

    pub fn with_mode(mut self, mode: PathMode) -> Self {
        self.path_mode = Some(mode);
        self
    }

    pub fn with_search(mut self, search: PathSearchPrefix) -> Self {
        self.search_prefix = Some(search);
        self
    }
}

impl Default for PathPattern {
    fn default() -> Self {
        Self::new()
    }
}

/// Path element: node, edge, or parenthesized path pattern
#[derive(Debug, Clone)]
pub enum PathElement {
    Node(NodePattern),
    Edge(EdgePattern),
    /// Parenthesized path pattern expression (ISO GQL 39075)
    /// Allows grouping of path patterns with optional subpath variable, path mode, and WHERE clause
    ParenthesizedPath(Box<ParenthesizedPathPattern>),
}

/// Parenthesized path pattern expression (ISO GQL 39075)
/// parenthesizedPathPatternExpression: LEFT_PAREN subpathVariableDeclaration? pathModePrefix? pathPatternExpression parenthesizedPathPatternWhereClause? RIGHT_PAREN
#[derive(Debug, Clone)]
pub struct ParenthesizedPathPattern {
    /// Subpath variable name (e.g., p = ...)
    pub subpath_variable: Option<String>,
    /// Path mode prefix (WALK, TRAIL, SIMPLE, ACYCLIC)
    pub path_mode: Option<PathMode>,
    /// Inner path pattern expression
    pub path_pattern: PathPatternExpression,
    /// WHERE clause inside the parenthesized path pattern
    pub where_clause: Option<Box<Expression>>,
    /// Quantifier for the parenthesized path pattern
    pub quantifier: Option<PatternQuantifier>,
}

impl ParenthesizedPathPattern {
    pub fn new(path_pattern: PathPatternExpression) -> Self {
        ParenthesizedPathPattern {
            subpath_variable: None,
            path_mode: None,
            path_pattern,
            where_clause: None,
            quantifier: None,
        }
    }

    pub fn with_subpath_variable(mut self, var: String) -> Self {
        self.subpath_variable = Some(var);
        self
    }

    pub fn with_quantifier(mut self, quantifier: PatternQuantifier) -> Self {
        self.quantifier = Some(quantifier);
        self
    }
}

/// Path pattern expression (ISO GQL 39075)
/// pathPatternExpression: pathTerm | pathTerm (MULTISET_ALTERNATION_OPERATOR pathTerm)+ | pathTerm (VERTICAL_BAR pathTerm)+
#[derive(Debug, Clone)]
pub enum PathPatternExpression {
    /// Single path term
    Term(Vec<PathElement>),
    /// Pattern union (|): matches any of the path terms
    Union(Vec<Vec<PathElement>>),
    /// Multiset alternation (|+|): returns all matches from all alternatives
    MultisetAlternation(Vec<Vec<PathElement>>),
}

impl Default for PathPatternExpression {
    fn default() -> Self {
        PathPatternExpression::Term(Vec::new())
    }
}

/// Path mode prefix (ISO GQL 39075)
/// pathModePrefix: WALK | TRAIL | SIMPLE | ACYCLIC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMode {
    /// WALK - any path, vertices and edges can repeat (default)
    Walk,
    /// TRAIL - edges must be distinct, vertices may repeat
    Trail,
    /// SIMPLE - vertices must be distinct (except start/end for cycles)
    Simple,
    /// ACYCLIC - vertices must be distinct, no cycles allowed
    Acyclic,
}

impl Default for PathMode {
    fn default() -> Self {
        PathMode::Walk
    }
}

impl fmt::Display for PathMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathMode::Walk => write!(f, "WALK"),
            PathMode::Trail => write!(f, "TRAIL"),
            PathMode::Simple => write!(f, "SIMPLE"),
            PathMode::Acyclic => write!(f, "ACYCLIC"),
        }
    }
}

/// Path search prefix (ISO GQL 39075)
/// pathSearchPrefix: ALL | ANY | allShortestPathSearch | anyShortestPathSearch | countedShortestPathSearch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSearchPrefix {
    /// ALL PATH - return all matching paths
    All,
    /// ANY - return any single matching path
    Any,
    /// ANY k - return k matching paths
    AnyK(u64),
    /// ALL SHORTEST - return all shortest paths
    AllShortest,
    /// ANY SHORTEST - return any single shortest path
    AnyShortest,
    /// SHORTEST k - return k shortest paths
    ShortestK(u64),
    /// SHORTEST k GROUPS - return shortest paths grouped by length
    ShortestKGroups(u64),
}

impl fmt::Display for PathSearchPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSearchPrefix::All => write!(f, "ALL"),
            PathSearchPrefix::Any => write!(f, "ANY"),
            PathSearchPrefix::AnyK(k) => write!(f, "ANY {}", k),
            PathSearchPrefix::AllShortest => write!(f, "ALL SHORTEST"),
            PathSearchPrefix::AnyShortest => write!(f, "ANY SHORTEST"),
            PathSearchPrefix::ShortestK(k) => write!(f, "SHORTEST {}", k),
            PathSearchPrefix::ShortestKGroups(k) => write!(f, "SHORTEST {} GROUPS", k),
        }
    }
}

/// Pattern quantifier (ISO GQL 39075)
/// graphPatternQuantifier: ASTERISK | PLUS_SIGN | fixedQuantifier | generalQuantifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternQuantifier {
    /// * - zero or more
    ZeroOrMore,
    /// + - one or more
    OneOrMore,
    /// ? - zero or one
    ZeroOrOne,
    /// {n} - exactly n times
    Exactly(u64),
    /// {n,} - at least n times
    AtLeast(u64),
    /// {,m} - at most m times
    AtMost(u64),
    /// {n,m} - between n and m times
    Range(u64, u64),
}

impl fmt::Display for PatternQuantifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternQuantifier::ZeroOrMore => write!(f, "*"),
            PatternQuantifier::OneOrMore => write!(f, "+"),
            PatternQuantifier::ZeroOrOne => write!(f, "?"),
            PatternQuantifier::Exactly(n) => write!(f, "{{{}}}", n),
            PatternQuantifier::AtLeast(n) => write!(f, "{{{},}}", n),
            PatternQuantifier::AtMost(m) => write!(f, "{{,{}}}", m),
            PatternQuantifier::Range(n, m) => write!(f, "{{{},{}}}", n, m),
        }
    }
}

// ============================================================================
// Node Pattern (ISO GQL 39075)
// ============================================================================

/// Node pattern
/// nodePattern: LEFT_PAREN elementPatternFiller RIGHT_PAREN
#[derive(Debug, Clone)]
pub struct NodePattern {
    /// Variable name
    pub variable: Option<String>,
    /// Label expression
    pub label_expr: Option<LabelExpression>,
    /// Property filter
    pub properties: Vec<(String, PropertyValue)>,
    /// WHERE predicate within the pattern
    pub where_clause: Option<Box<Expression>>,
}

impl NodePattern {
    pub fn new() -> Self {
        NodePattern {
            variable: None,
            label_expr: None,
            properties: Vec::new(),
            where_clause: None,
        }
    }

    pub fn with_variable(mut self, var: String) -> Self {
        self.variable = Some(var);
        self
    }

    pub fn with_label(mut self, label: VertexLabel) -> Self {
        let new_label = LabelExpression::Label(label);
        self.label_expr = Some(match self.label_expr {
            Some(expr) => LabelExpression::Conjunction(vec![expr, new_label]),
            None => new_label,
        });
        self
    }

    pub fn with_labels(mut self, labels: Vec<VertexLabel>) -> Self {
        if !labels.is_empty() {
            let exprs: Vec<LabelExpression> =
                labels.into_iter().map(LabelExpression::Label).collect();
            self.label_expr = Some(if exprs.len() == 1 {
                exprs.into_iter().next().unwrap()
            } else {
                LabelExpression::Conjunction(exprs)
            });
        }
        self
    }

    pub fn with_property(mut self, key: String, value: PropertyValue) -> Self {
        self.properties.push((key, value));
        self
    }

    /// Get labels as a vector (for compatibility)
    pub fn labels(&self) -> Vec<VertexLabel> {
        self.label_expr
            .as_ref()
            .map(|e| e.to_vertex_labels())
            .unwrap_or_default()
    }
}

impl Default for NodePattern {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Edge Pattern (ISO GQL 39075)
// ============================================================================

/// Edge pattern
/// edgePattern: fullEdgePattern | abbreviatedEdgePattern
#[derive(Debug, Clone)]
pub struct EdgePattern {
    /// Variable name
    pub variable: Option<String>,
    /// Label expression
    pub label_expr: Option<LabelExpression>,
    /// Edge direction
    pub direction: EdgeDirection,
    /// Property filter
    pub properties: Vec<(String, PropertyValue)>,
    /// Quantifier for variable-length paths
    pub quantifier: Option<PatternQuantifier>,
    /// WHERE predicate within the pattern
    pub where_clause: Option<Box<Expression>>,
}

impl EdgePattern {
    pub fn new(direction: EdgeDirection) -> Self {
        EdgePattern {
            variable: None,
            label_expr: None,
            direction,
            properties: Vec::new(),
            quantifier: None,
            where_clause: None,
        }
    }

    pub fn with_variable(mut self, var: String) -> Self {
        self.variable = Some(var);
        self
    }

    pub fn with_label(mut self, label: EdgeLabel) -> Self {
        let new_label = LabelExpression::EdgeLabel(label);
        self.label_expr = Some(match self.label_expr {
            Some(expr) => LabelExpression::Disjunction(vec![expr, new_label]),
            None => new_label,
        });
        self
    }

    pub fn with_labels(mut self, labels: Vec<EdgeLabel>) -> Self {
        if !labels.is_empty() {
            let exprs: Vec<LabelExpression> =
                labels.into_iter().map(LabelExpression::EdgeLabel).collect();
            self.label_expr = Some(if exprs.len() == 1 {
                exprs.into_iter().next().unwrap()
            } else {
                // Edge types use disjunction (OR) by default: :TYPE1|TYPE2
                LabelExpression::Disjunction(exprs)
            });
        }
        self
    }

    pub fn with_quantifier(mut self, quantifier: PatternQuantifier) -> Self {
        self.quantifier = Some(quantifier);
        self
    }

    /// Get labels as a vector (for compatibility)
    pub fn labels(&self) -> Vec<EdgeLabel> {
        self.label_expr
            .as_ref()
            .map(|e| e.to_edge_labels())
            .unwrap_or_default()
    }
}

/// Edge direction (ISO GQL 39075)
/// Supports all 7 direction types from the standard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    /// Outgoing edge: -[...]->
    Outgoing,
    /// Incoming edge: <-[...]-
    Incoming,
    /// Undirected edge: ~[...]~
    Undirected,
    /// Any direction (bidirectional): -[...]-
    AnyDirection,
    /// Left or undirected: <~[...]~
    LeftOrUndirected,
    /// Undirected or right: ~[...]~>
    UndirectedOrRight,
    /// Left or right (but not undirected): <-[...]->
    LeftOrRight,
}

impl fmt::Display for EdgeDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeDirection::Outgoing => write!(f, "->"),
            EdgeDirection::Incoming => write!(f, "<-"),
            EdgeDirection::Undirected => write!(f, "~"),
            EdgeDirection::AnyDirection => write!(f, "-"),
            EdgeDirection::LeftOrUndirected => write!(f, "<~"),
            EdgeDirection::UndirectedOrRight => write!(f, "~>"),
            EdgeDirection::LeftOrRight => write!(f, "<->"),
        }
    }
}

// ============================================================================
// Label Expression (ISO GQL 39075)
// ============================================================================

/// Label expression supporting negation, conjunction, and disjunction
/// labelExpression: labelTerm (VERTICAL_BAR labelTerm)*
/// labelTerm: labelFactor (AMPERSAND labelFactor)*
/// labelFactor: labelNegation | labelPrimary
#[derive(Debug, Clone)]
pub enum LabelExpression {
    /// Single vertex label
    Label(VertexLabel),
    /// Single edge label
    EdgeLabel(EdgeLabel),
    /// Wildcard (%) - matches any label
    Wildcard,
    /// Negation (!label)
    Negation(Box<LabelExpression>),
    /// Conjunction (label1 & label2) - must have all labels
    Conjunction(Vec<LabelExpression>),
    /// Disjunction (label1 | label2) - must have any of the labels
    Disjunction(Vec<LabelExpression>),
}

impl LabelExpression {
    /// Convert to vertex labels (for compatibility)
    pub fn to_vertex_labels(&self) -> Vec<VertexLabel> {
        match self {
            LabelExpression::Label(label) => vec![label.clone()],
            LabelExpression::Conjunction(exprs) | LabelExpression::Disjunction(exprs) => {
                exprs.iter().flat_map(|e| e.to_vertex_labels()).collect()
            }
            _ => vec![],
        }
    }

    /// Convert to edge labels (for compatibility)
    pub fn to_edge_labels(&self) -> Vec<EdgeLabel> {
        match self {
            LabelExpression::EdgeLabel(label) => vec![label.clone()],
            LabelExpression::Conjunction(exprs) | LabelExpression::Disjunction(exprs) => {
                exprs.iter().flat_map(|e| e.to_edge_labels()).collect()
            }
            _ => vec![],
        }
    }
}

impl fmt::Display for LabelExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabelExpression::Label(label) => write!(f, ":{:?}", label),
            LabelExpression::EdgeLabel(label) => write!(f, ":{:?}", label),
            LabelExpression::Wildcard => write!(f, ":%"),
            LabelExpression::Negation(expr) => write!(f, "!{}", expr),
            LabelExpression::Conjunction(exprs) => {
                let labels: Vec<String> = exprs.iter().map(|e| format!("{}", e)).collect();
                write!(f, "{}", labels.join(" & "))
            }
            LabelExpression::Disjunction(exprs) => {
                let labels: Vec<String> = exprs.iter().map(|e| format!("{}", e)).collect();
                write!(f, "{}", labels.join(" | "))
            }
        }
    }
}

// ============================================================================
// Expression (ISO GQL 39075)
// ============================================================================

/// GQL expression
#[derive(Debug, Clone)]
pub enum Expression {
    /// Literal value
    Literal(PropertyValue),
    /// Variable reference
    Variable(String),
    /// Property access (variable, property_name)
    Property(String, String),
    /// Function call
    FunctionCall(String, Vec<Expression>),
    /// Binary operation
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    /// Unary operation
    UnaryOp(UnaryOperator, Box<Expression>),
    /// List expression
    List(Vec<Expression>),
    /// Map/Record expression
    Map(Vec<(String, Expression)>),
    /// Path expression (for path pattern matching)
    PathExpr(Box<PathPattern>),
    /// CASE expression
    Case {
        operand: Option<Box<Expression>>,
        when_clauses: Vec<(Expression, Expression)>,
        else_clause: Option<Box<Expression>>,
    },
    /// EXISTS predicate
    Exists(Box<GraphPattern>),
    /// Quantified expression (ALL, ANY, NONE, SINGLE)
    Quantified {
        quantifier: Quantifier,
        variable: String,
        list: Box<Expression>,
        predicate: Box<Expression>,
    },
    /// Parameter reference ($param)
    Parameter(String),
    /// NULL literal
    Null,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    Xor,
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
    // String
    Concat,
    Contains,
    StartsWith,
    EndsWith,
    Like,
    // List
    In,
    // Null
    IsNull,
    IsNotNull,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOperator::Eq => write!(f, "="),
            BinaryOperator::Ne => write!(f, "<>"),
            BinaryOperator::Lt => write!(f, "<"),
            BinaryOperator::Le => write!(f, "<="),
            BinaryOperator::Gt => write!(f, ">"),
            BinaryOperator::Ge => write!(f, ">="),
            BinaryOperator::And => write!(f, "AND"),
            BinaryOperator::Or => write!(f, "OR"),
            BinaryOperator::Xor => write!(f, "XOR"),
            BinaryOperator::Add => write!(f, "+"),
            BinaryOperator::Sub => write!(f, "-"),
            BinaryOperator::Mul => write!(f, "*"),
            BinaryOperator::Div => write!(f, "/"),
            BinaryOperator::Mod => write!(f, "%"),
            BinaryOperator::Power => write!(f, "^"),
            BinaryOperator::Concat => write!(f, "||"),
            BinaryOperator::Contains => write!(f, "CONTAINS"),
            BinaryOperator::StartsWith => write!(f, "STARTS WITH"),
            BinaryOperator::EndsWith => write!(f, "ENDS WITH"),
            BinaryOperator::Like => write!(f, "LIKE"),
            BinaryOperator::In => write!(f, "IN"),
            BinaryOperator::IsNull => write!(f, "IS NULL"),
            BinaryOperator::IsNotNull => write!(f, "IS NOT NULL"),
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Neg,
    IsNull,
    IsNotNull,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOperator::Not => write!(f, "NOT"),
            UnaryOperator::Neg => write!(f, "-"),
            UnaryOperator::IsNull => write!(f, "IS NULL"),
            UnaryOperator::IsNotNull => write!(f, "IS NOT NULL"),
        }
    }
}

/// Quantifier for quantified expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantifier {
    All,
    Any,
    None,
    Single,
}

// ============================================================================
// RETURN Clause (ISO GQL 39075)
// ============================================================================

/// RETURN item
#[derive(Debug, Clone)]
pub struct ReturnItem {
    /// Expression to return
    pub expression: Expression,
    /// Optional alias
    pub alias: Option<String>,
}

impl ReturnItem {
    pub fn new(expression: Expression) -> Self {
        ReturnItem {
            expression,
            alias: None,
        }
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }
}

/// ORDER BY item
#[derive(Debug, Clone)]
pub struct OrderByItem {
    /// Expression to sort by
    pub expression: Expression,
    /// Descending order?
    pub descending: bool,
    /// NULLS FIRST or NULLS LAST
    pub nulls_first: Option<bool>,
}

// ============================================================================
// INSERT Statement (ISO GQL 39075)
// ============================================================================

/// INSERT statement
#[derive(Debug, Clone)]
pub struct InsertStatement {
    /// Nodes to insert
    pub nodes: Vec<NodePattern>,
    /// Edges to insert
    pub edges: Vec<InsertEdge>,
}

/// Edge to insert with source and target references
#[derive(Debug, Clone)]
pub struct InsertEdge {
    /// Source node variable
    pub source: String,
    /// Edge pattern
    pub edge: EdgePattern,
    /// Target node variable
    pub target: String,
}

// ============================================================================
// DELETE Statement (ISO GQL 39075)
// ============================================================================

/// DELETE statement
/// deleteStatement: (DETACH | NODETACH)? DELETE deleteItemList
#[derive(Debug, Clone)]
pub struct DeleteStatement {
    /// Variables to delete
    pub variables: Vec<String>,
    /// DETACH DELETE mode (also delete connected edges)
    pub detach: bool,
}

// ============================================================================
// SET Statement (ISO GQL 39075)
// ============================================================================

/// SET statement
#[derive(Debug, Clone)]
pub struct SetStatement {
    /// Set items
    pub items: Vec<SetItem>,
}

/// SET item
#[derive(Debug, Clone)]
pub enum SetItem {
    /// SET n.property = value
    Property(String, String, Expression),
    /// SET n = {properties} or SET n += {properties}
    AllProperties {
        variable: String,
        properties: Vec<(String, Expression)>,
        merge: bool, // += vs =
    },
    /// SET n:Label
    Label(String, VertexLabel),
}

// ============================================================================
// REMOVE Statement (ISO GQL 39075)
// ============================================================================

/// REMOVE statement
#[derive(Debug, Clone)]
pub struct RemoveStatement {
    /// Remove items
    pub items: Vec<RemoveItem>,
}

/// REMOVE item
#[derive(Debug, Clone)]
pub enum RemoveItem {
    /// REMOVE n.property
    Property(String, String),
    /// REMOVE n:Label
    Label(String, VertexLabel),
}

// ============================================================================
// CALL Statement (ISO GQL 39075)
// ============================================================================

/// CALL procedure statement
/// callProcedureStatement: OPTIONAL? CALL procedureCall
#[derive(Debug, Clone)]
pub struct CallStatement {
    /// OPTIONAL CALL?
    pub optional: bool,
    /// Procedure reference (catalog.schema.name)
    pub procedure_name: String,
    /// Arguments
    pub arguments: Vec<Expression>,
    /// YIELD clause
    pub yield_items: Option<Vec<YieldItem>>,
}

impl CallStatement {
    pub fn new(procedure_name: String) -> Self {
        CallStatement {
            optional: false,
            procedure_name,
            arguments: Vec::new(),
            yield_items: None,
        }
    }
}

/// YIELD item
#[derive(Debug, Clone)]
pub struct YieldItem {
    /// Field name
    pub name: String,
    /// Alias
    pub alias: Option<String>,
}

// ============================================================================
// Graph Management Statements (ISO GQL 39075)
// ============================================================================

/// CREATE GRAPH statement with optional inline Graph Type definition
/// Example: CREATE GRAPH myGraph { NODE Account, EDGE Transfer }
#[derive(Debug, Clone)]
pub struct CreateGraphStatement {
    /// Graph name
    pub name: String,
    /// IF NOT EXISTS clause
    pub if_not_exists: bool,
    /// Optional inline Graph Type definition
    pub schema: Option<GraphSchema>,
}

/// Inline Graph Type definition
#[derive(Debug, Clone)]
pub struct GraphSchema {
    /// Node type specifications
    pub node_types: Vec<NodeTypeSpec>,
    /// Edge type specifications
    pub edge_types: Vec<EdgeTypeSpec>,
}

/// Property specification for Graph Type
#[derive(Debug, Clone)]
pub struct PropertySpec {
    /// Property name
    pub name: String,
    /// Property data type (e.g., "String", "int", "SET<enum>")
    pub data_type: String,
    /// Is this property a Primary Key?
    pub is_primary_key: bool,
}

/// Node type specification in inline Graph Type
#[derive(Debug, Clone)]
pub struct NodeTypeSpec {
    /// Node label
    pub label: String,
    /// Properties (empty if referencing builtin type)
    pub properties: Vec<PropertySpec>,
    /// Is this a builtin type reference (e.g., __Account)?
    pub is_builtin_ref: bool,
}

/// Edge type specification in inline Graph Type
#[derive(Debug, Clone)]
pub struct EdgeTypeSpec {
    /// Edge label
    pub label: String,
    /// Source node label (empty if referencing builtin type)
    pub source_label: String,
    /// Target node label (empty if referencing builtin type)
    pub target_label: String,
    /// Properties (empty if referencing builtin type)
    pub properties: Vec<PropertySpec>,
    /// Is this a builtin type reference (e.g., __Transfer)?
    pub is_builtin_ref: bool,
}

/// DROP GRAPH statement
#[derive(Debug, Clone)]
pub struct DropGraphStatement {
    /// Graph name
    pub name: String,
    /// IF EXISTS clause
    pub if_exists: bool,
}

// ============================================================================
// GQL Additional Statements (ISO GQL 39075)
// ============================================================================

/// LET statement - local variable binding (ISO GQL 39075)
/// Example: LET x = 10, y = "hello"
#[derive(Debug, Clone)]
pub struct LetStatement {
    /// Variable bindings
    pub bindings: Vec<LetBinding>,
}

/// A single LET binding
#[derive(Debug, Clone)]
pub struct LetBinding {
    /// Variable name
    pub variable: String,
    /// Value expression
    pub value: Expression,
}

/// FOR statement - iteration (ISO GQL 39075)
/// Example: FOR x IN [1, 2, 3]
#[derive(Debug, Clone)]
pub struct ForStatement {
    /// Loop variable
    pub variable: String,
    /// Collection to iterate over
    pub iterable: Expression,
    /// Optional ordinal variable
    pub ordinal_variable: Option<String>,
}

/// FILTER statement - filtering results (ISO GQL 39075)
/// Example: FILTER n.age > 18
#[derive(Debug, Clone)]
pub struct FilterStatement {
    /// Filter condition
    pub condition: Expression,
}

/// SELECT statement - SQL-like projection (ISO GQL 39075)
/// Example: SELECT n.name, SUM(t.amount) GROUP BY n.name
#[derive(Debug, Clone)]
pub struct SelectStatement {
    /// DISTINCT modifier
    pub distinct: bool,
    /// Select items (projections)
    pub items: Vec<SelectItem>,
    /// GROUP BY clause
    pub group_by: Option<Vec<Expression>>,
    /// HAVING clause
    pub having: Option<Expression>,
    /// ORDER BY clause
    pub order_by: Option<Vec<OrderByItem>>,
    /// OFFSET clause
    pub offset: Option<u64>,
    /// LIMIT clause
    pub limit: Option<u64>,
}

/// SELECT item
#[derive(Debug, Clone)]
pub struct SelectItem {
    /// Expression to select
    pub expression: Expression,
    /// Optional alias
    pub alias: Option<String>,
}

/// USE statement - specify graph context (ISO GQL 39075)
/// Example: USE GRAPH myGraph
#[derive(Debug, Clone)]
pub struct UseStatement {
    /// Graph reference
    pub graph: GraphReference,
}

/// Graph reference
#[derive(Debug, Clone)]
pub enum GraphReference {
    /// Named graph
    Named(String),
    /// CURRENT_GRAPH
    Current,
    /// Subquery that returns a graph
    Subquery(Box<GqlStatement>),
}

/// Composite query statement - set operations (ISO GQL 39075)
/// Example: MATCH ... UNION MATCH ... EXCEPT MATCH ...
#[derive(Debug, Clone)]
pub struct CompositeQueryStatement {
    /// Primary query
    pub primary: Box<GqlStatement>,
    /// Set operation
    pub operation: SetOperation,
    /// Secondary query
    pub secondary: Box<GqlStatement>,
    /// ALL modifier (keep duplicates)
    pub all: bool,
}

/// Set operation type
#[derive(Debug, Clone)]
pub enum SetOperation {
    /// UNION
    Union,
    /// EXCEPT
    Except,
    /// INTERSECT
    Intersect,
    /// OTHERWISE
    Otherwise,
}

/// Session statement - session management (ISO GQL 39075)
/// Example: SESSION SET GRAPH TYPE myGraphType
#[derive(Debug, Clone)]
pub enum SessionStatement {
    /// SET operation
    Set(SessionSetItem),
    /// RESET operation  
    Reset(SessionResetItem),
    /// CLOSE
    Close,
}

/// Session SET item
#[derive(Debug, Clone)]
pub enum SessionSetItem {
    /// SET GRAPH TYPE
    GraphType(String),
    /// SET GRAPH
    Graph(String),
    /// SET TIME ZONE
    TimeZone(String),
    /// SET parameter
    Parameter(String, Expression),
}

/// Session RESET item
#[derive(Debug, Clone)]
pub enum SessionResetItem {
    /// RESET GRAPH TYPE
    GraphType,
    /// RESET GRAPH
    Graph,
    /// RESET TIME ZONE
    TimeZone,
    /// RESET ALL
    All,
    /// RESET parameter
    Parameter(String),
}

/// Transaction statement (ISO GQL 39075)
/// Example: START TRANSACTION, COMMIT, ROLLBACK
#[derive(Debug, Clone)]
pub enum TransactionStatement {
    /// START TRANSACTION
    Start(TransactionCharacteristics),
    /// COMMIT
    Commit,
    /// ROLLBACK
    Rollback,
}

/// Transaction characteristics
#[derive(Debug, Clone, Default)]
pub struct TransactionCharacteristics {
    /// Transaction access mode
    pub access_mode: Option<TransactionAccessMode>,
}

/// Transaction access mode
#[derive(Debug, Clone)]
pub enum TransactionAccessMode {
    ReadOnly,
    ReadWrite,
}

// ============================================================================
// SHOW and DESCRIBE Statements
// ============================================================================

/// SHOW statement - list database objects
/// Examples:
///   SHOW GRAPHS
///   SHOW GRAPH TYPES
///   SHOW FUNCTIONS
///   SHOW PROCEDURES
#[derive(Debug, Clone)]
pub struct ShowStatement {
    /// What to show
    pub show_type: ShowType,
    /// Optional filter (LIKE pattern)
    pub like_pattern: Option<String>,
}

/// What to show in SHOW statement
#[derive(Debug, Clone, PartialEq)]
pub enum ShowType {
    /// SHOW GRAPHS - list all graphs
    Graphs,
    /// SHOW GRAPH TYPES - list all graph types
    GraphTypes,
    /// SHOW FUNCTIONS - list all functions
    Functions,
    /// SHOW PROCEDURES - list all procedures
    Procedures,
    /// SHOW LABELS - list all vertex labels
    Labels,
    /// SHOW EDGE TYPES / RELATIONSHIP TYPES - list all edge types
    EdgeTypes,
    /// SHOW PROPERTY KEYS - list all property keys
    PropertyKeys,
    /// SHOW INDEXES - list all indexes
    Indexes,
    /// SHOW CONSTRAINTS - list all constraints
    Constraints,
}

/// DESCRIBE statement - show details of a database object
/// Examples:
///   DESCRIBE GRAPH myGraph
///   DESC GRAPH myGraph
#[derive(Debug, Clone)]
pub struct DescribeStatement {
    /// What to describe
    pub describe_type: DescribeType,
    /// Name of the object
    pub name: String,
}

/// What to describe in DESCRIBE statement
#[derive(Debug, Clone, PartialEq)]
pub enum DescribeType {
    /// DESCRIBE GRAPH - show graph details
    Graph,
    /// DESCRIBE GRAPH TYPE - show Graph Type details
    GraphType,
    /// DESCRIBE LABEL / VERTEX TYPE - show vertex type details
    Label,
    /// DESCRIBE EDGE TYPE / RELATIONSHIP TYPE - show edge type details
    EdgeType,
}

/// CASE expression (ISO GQL 39075)
/// Example: CASE WHEN x > 10 THEN 'high' ELSE 'low' END
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CaseExpression {
    /// Simple CASE
    Simple {
        operand: Box<Expression>,
        when_clauses: Vec<WhenClause>,
        else_result: Option<Box<Expression>>,
    },
    /// Searched CASE
    Searched {
        when_clauses: Vec<WhenClause>,
        else_result: Option<Box<Expression>>,
    },
}

/// WHEN clause in CASE expression
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WhenClause {
    /// Condition (or value for simple CASE)
    pub condition: Expression,
    /// Result expression
    pub result: Expression,
}

/// EXISTS predicate (ISO GQL 39075)
/// Example: EXISTS { MATCH (n)-[e]->(m) }
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExistsPredicate {
    /// Pattern to check existence
    pub pattern: GraphPattern,
    /// Optional WHERE clause
    pub where_clause: Option<Expression>,
}

/// Aggregate function (ISO GQL 39075)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AggregateFunction {
    Count(Option<Box<Expression>>, bool), // expr, distinct
    Sum(Box<Expression>, bool),
    Avg(Box<Expression>, bool),
    Min(Box<Expression>),
    Max(Box<Expression>),
    CollectList(Box<Expression>, bool),
    StdDev(Box<Expression>),
    Variance(Box<Expression>),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_mode_display() {
        assert_eq!(format!("{}", PathMode::Walk), "WALK");
        assert_eq!(format!("{}", PathMode::Trail), "TRAIL");
        assert_eq!(format!("{}", PathMode::Simple), "SIMPLE");
        assert_eq!(format!("{}", PathMode::Acyclic), "ACYCLIC");
    }

    #[test]
    fn test_path_search_prefix_display() {
        assert_eq!(format!("{}", PathSearchPrefix::All), "ALL");
        assert_eq!(format!("{}", PathSearchPrefix::AnyShortest), "ANY SHORTEST");
        assert_eq!(format!("{}", PathSearchPrefix::ShortestK(3)), "SHORTEST 3");
    }

    #[test]
    fn test_pattern_quantifier_display() {
        assert_eq!(format!("{}", PatternQuantifier::ZeroOrMore), "*");
        assert_eq!(format!("{}", PatternQuantifier::OneOrMore), "+");
        assert_eq!(format!("{}", PatternQuantifier::Exactly(3)), "{3}");
        assert_eq!(format!("{}", PatternQuantifier::Range(2, 5)), "{2,5}");
    }

    #[test]
    fn test_node_pattern_builder() {
        let node = NodePattern::new()
            .with_variable("n".to_string())
            .with_label(VertexLabel::Account);

        assert_eq!(node.variable, Some("n".to_string()));
        assert!(node.label_expr.is_some());
        assert_eq!(node.labels().len(), 1);
    }

    #[test]
    fn test_edge_pattern_builder() {
        let edge = EdgePattern::new(EdgeDirection::Outgoing)
            .with_variable("r".to_string())
            .with_label(EdgeLabel::Transfer)
            .with_quantifier(PatternQuantifier::OneOrMore);

        assert_eq!(edge.variable, Some("r".to_string()));
        assert_eq!(edge.direction, EdgeDirection::Outgoing);
        assert!(edge.quantifier.is_some());
    }

    #[test]
    fn test_match_statement_builder() {
        let pattern = GraphPattern::new().with_path(
            PathPattern::new().add_node(NodePattern::new().with_variable("n".to_string())),
        );

        let stmt = MatchStatement::new(pattern).with_limit(10);

        assert_eq!(stmt.limit, Some(10));
        assert!(!stmt.optional);
    }

    #[test]
    fn test_label_expression() {
        let expr = LabelExpression::Conjunction(vec![
            LabelExpression::Label(VertexLabel::Account),
            LabelExpression::Label(VertexLabel::Contract),
        ]);

        let labels = expr.to_vertex_labels();
        assert_eq!(labels.len(), 2);
    }
}
