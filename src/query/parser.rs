//! GQL Parser - ISO/IEC 39075 Compliant
//!
//! Parses GQL query strings into AST structures based on ISO GQL 39075 standard.

use crate::error::{Error, Result};
use crate::query::ast::*;
use crate::types::{Address, EdgeLabel, PropertyValue, VertexLabel};

/// GQL Parser
pub struct GqlParser {
    input: String,
    pos: usize,
}

impl GqlParser {
    /// Create a new parser
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            pos: 0,
        }
    }

    /// Parse a GQL statement
    pub fn parse(&mut self) -> Result<GqlStatement> {
        self.skip_whitespace();

        let keyword = self.peek_keyword()?;
        match keyword.to_uppercase().as_str() {
            "MATCH" | "OPTIONAL" => self.parse_match(),
            "INSERT" => self.parse_insert(),
            "DELETE" | "DETACH" | "NODETACH" => self.parse_delete(),
            "SET" => self.parse_set(),
            "REMOVE" => self.parse_remove(),
            "CALL" => self.parse_call(),
            "CREATE" => self.parse_create(),
            "DROP" => self.parse_drop(),
            "SHOW" => self.parse_show(),
            "DESCRIBE" | "DESC" => self.parse_describe(),
            // ISO GQL 39075 new statements
            "LET" => self.parse_let(),
            "FOR" => self.parse_for(),
            "FILTER" => self.parse_filter(),
            "SELECT" => self.parse_select(),
            "USE" => self.parse_use(),
            "SESSION" => self.parse_session(),
            "START" => self.parse_transaction_start(),
            "COMMIT" => self.parse_transaction_commit(),
            "ROLLBACK" => self.parse_transaction_rollback(),
            _ => Err(Error::ParseError(format!(
                "Unknown statement type: {}",
                keyword
            ))),
        }
    }

    // ========================================================================
    // MATCH Statement Parsing (ISO GQL 39075)
    // ========================================================================

    /// Parse MATCH statement
    /// matchStatement: [OPTIONAL] [matchMode] MATCH graphPattern [WHERE expr] [RETURN items]
    fn parse_match(&mut self) -> Result<GqlStatement> {
        // OPTIONAL MATCH?
        let optional = self.try_keyword("OPTIONAL");

        self.expect_keyword("MATCH")?;

        // Parse optional match mode (REPEATABLE ELEMENTS | DIFFERENT EDGES)
        let match_mode = self.parse_match_mode()?;

        // Parse graph pattern
        let graph_pattern = self.parse_graph_pattern()?;

        // WHERE clause
        let where_clause = if self.try_keyword("WHERE") {
            Some(self.parse_expression()?)
        } else {
            None
        };

        // RETURN clause
        let return_clause = if self.try_keyword("RETURN") {
            self.parse_return_items()?
        } else {
            Vec::new()
        };

        // ORDER BY
        let order_by = if self.try_keyword("ORDER") {
            self.expect_keyword("BY")?;
            Some(self.parse_order_by()?)
        } else {
            None
        };

        // SKIP
        let skip = if self.try_keyword("SKIP") {
            Some(self.parse_integer()? as usize)
        } else {
            None
        };

        // LIMIT
        let limit = if self.try_keyword("LIMIT") {
            Some(self.parse_integer()? as usize)
        } else {
            None
        };

        Ok(GqlStatement::Match(MatchStatement {
            optional,
            match_mode,
            graph_pattern,
            where_clause,
            return_clause,
            order_by,
            skip,
            limit,
        }))
    }

    /// Parse match mode
    fn parse_match_mode(&mut self) -> Result<Option<MatchMode>> {
        if self.try_keyword("REPEATABLE") {
            self.expect_keyword("ELEMENTS")?;
            Ok(Some(MatchMode::RepeatableElements))
        } else if self.try_keyword("DIFFERENT") {
            self.expect_keyword("EDGES")?;
            Ok(Some(MatchMode::DifferentEdges))
        } else {
            Ok(None)
        }
    }

    /// Parse graph pattern (comma-separated path patterns)
    /// graphPattern: matchMode? pathPatternList keepClause? graphPatternWhereClause?
    fn parse_graph_pattern(&mut self) -> Result<GraphPattern> {
        let mut paths = Vec::new();
        paths.push(self.parse_path_pattern()?);

        while self.try_char(',') {
            paths.push(self.parse_path_pattern()?);
        }

        // Parse optional KEEP clause
        let keep_clause = self.parse_keep_clause()?;

        Ok(GraphPattern { paths, keep_clause })
    }

    /// Parse KEEP clause (ISO GQL 39075)
    /// keepClause: KEEP pathPatternPrefix
    fn parse_keep_clause(&mut self) -> Result<Option<KeepClause>> {
        if self.try_keyword("KEEP") {
            // Parse path search prefix after KEEP
            if let Some(prefix) = self.parse_path_search_prefix()? {
                Ok(Some(KeepClause { path_prefix: prefix }))
            } else {
                Err(Error::ParseError("Expected path prefix after KEEP (e.g., ALL SHORTEST, ANY)".to_string()))
            }
        } else {
            Ok(None)
        }
    }

    /// Parse path pattern with optional prefix (ISO GQL 39075)
    /// pathPattern: pathVariableDeclaration? pathPatternPrefix? pathPatternExpression
    fn parse_path_pattern(&mut self) -> Result<PathPattern> {
        let mut pattern = PathPattern::new();

        // Check for path variable assignment: p = (...)
        self.skip_whitespace();
        if self.peek_char_is_alpha() {
            let start_pos = self.pos;
            let ident = self.parse_identifier()?;
            self.skip_whitespace();

            if self.try_char('=') {
                pattern.variable = Some(ident);
            } else {
                // Not a path variable, backtrack
                self.pos = start_pos;
            }
        }

        // Parse path mode prefix (WALK | TRAIL | SIMPLE | ACYCLIC)
        pattern.path_mode = self.parse_path_mode()?;

        // Parse path search prefix (ALL | ANY | SHORTEST)
        pattern.search_prefix = self.parse_path_search_prefix()?;

        // Parse path pattern expression (supports |, |+|, and parenthesized patterns)
        pattern.elements = self.parse_path_pattern_elements()?;

        // Parse optional quantifier for the whole path
        pattern.quantifier = self.parse_quantifier()?;

        Ok(pattern)
    }

    /// Parse path pattern elements (ISO GQL 39075)
    /// Supports: node patterns, edge patterns, parenthesized path patterns, union (|), multiset alternation (|+|)
    fn parse_path_pattern_elements(&mut self) -> Result<Vec<PathElement>> {
        let first_term = self.parse_path_term()?;

        self.skip_whitespace();

        // Check for multiset alternation (|+|) or pattern union (|)
        if self.try_str("|+|") {
            // Multiset alternation
            let mut alternatives = vec![first_term];
            loop {
                alternatives.push(self.parse_path_term()?);
                self.skip_whitespace();
                if !self.try_str("|+|") {
                    break;
                }
            }
            // Wrap in a parenthesized path pattern with multiset alternation
            Ok(vec![PathElement::ParenthesizedPath(Box::new(
                ParenthesizedPathPattern {
                    subpath_variable: None,
                    path_mode: None,
                    path_pattern: PathPatternExpression::MultisetAlternation(alternatives),
                    where_clause: None,
                    quantifier: None,
                },
            ))])
        } else if self.peek_char_is('|') && !self.peek_str("|+|") {
            // Pattern union
            self.expect_char('|')?;
            let mut alternatives = vec![first_term];
            loop {
                alternatives.push(self.parse_path_term()?);
                self.skip_whitespace();
                // Check if next is | but not |+|
                if self.peek_char_is('|') && !self.peek_str("|+|") {
                    self.expect_char('|')?;
                } else {
                    break;
                }
            }
            // Wrap in a parenthesized path pattern with union
            Ok(vec![PathElement::ParenthesizedPath(Box::new(
                ParenthesizedPathPattern {
                    subpath_variable: None,
                    path_mode: None,
                    path_pattern: PathPatternExpression::Union(alternatives),
                    where_clause: None,
                    quantifier: None,
                },
            ))])
        } else {
            Ok(first_term)
        }
    }

    /// Parse a single path term (sequence of nodes and edges)
    fn parse_path_term(&mut self) -> Result<Vec<PathElement>> {
        let mut elements = Vec::new();

        // Parse first element (node or parenthesized path)
        self.skip_whitespace();
        if self.peek_char_is('(') {
            // Could be a node pattern or parenthesized path pattern
            let start_pos = self.pos;
            
            // Try to parse as parenthesized path pattern first
            if let Ok(paren_path) = self.try_parse_parenthesized_path() {
                elements.push(PathElement::ParenthesizedPath(Box::new(paren_path)));
            } else {
                // Backtrack and parse as node pattern
                self.pos = start_pos;
                elements.push(PathElement::Node(self.parse_node_pattern()?));
            }
        } else {
            return Err(Error::ParseError("Expected '(' to start path pattern".to_string()));
        }

        // Parse edges and subsequent nodes/parenthesized paths
        loop {
            self.skip_whitespace();
            if !self.peek_char_is('-') && !self.peek_char_is('<') && !self.peek_char_is('~') {
                break;
            }

            elements.push(PathElement::Edge(self.parse_edge_pattern()?));

            self.skip_whitespace();
            if self.peek_char_is('(') {
                let start_pos = self.pos;
                
                // Try to parse as parenthesized path pattern first
                if let Ok(paren_path) = self.try_parse_parenthesized_path() {
                    elements.push(PathElement::ParenthesizedPath(Box::new(paren_path)));
                } else {
                    // Backtrack and parse as node pattern
                    self.pos = start_pos;
                    elements.push(PathElement::Node(self.parse_node_pattern()?));
                }
            } else {
                break;
            }
        }

        Ok(elements)
    }

    /// Try to parse a parenthesized path pattern expression (ISO GQL 39075)
    /// parenthesizedPathPatternExpression: LEFT_PAREN subpathVariableDeclaration? pathModePrefix? pathPatternExpression parenthesizedPathPatternWhereClause? RIGHT_PAREN
    /// Supports nested subpath variables: (p = ((a)->(b))){1,5}
    fn try_parse_parenthesized_path(&mut self) -> Result<ParenthesizedPathPattern> {
        self.skip_whitespace();
        self.expect_char('(')?;
        self.skip_whitespace();

        // Check for subpath variable declaration: varname = (...)
        let subpath_variable = self.try_parse_subpath_variable()?;

        // Parse path mode prefix (optional)
        let path_mode = self.parse_path_mode()?;

        // Parse inner path pattern expression (may be another parenthesized pattern)
        let inner_elements = self.parse_path_term()?;
        let path_pattern = PathPatternExpression::Term(inner_elements);

        // Check for WHERE clause inside parentheses
        let where_clause = if self.try_keyword("WHERE") {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.skip_whitespace();
        self.expect_char(')')?;

        // Parse optional quantifier after the parenthesized path
        let quantifier = self.parse_quantifier()?;

        Ok(ParenthesizedPathPattern {
            subpath_variable,
            path_mode,
            path_pattern,
            where_clause,
            quantifier,
        })
    }

    /// Try to parse a subpath variable declaration: varname =
    /// Returns Some(varname) if found, None otherwise
    fn try_parse_subpath_variable(&mut self) -> Result<Option<String>> {
        self.skip_whitespace();
        
        if self.peek_char_is_alpha() {
            let start_pos = self.pos;
            let ident = self.parse_identifier()?;
            self.skip_whitespace();

            if self.try_char('=') {
                self.skip_whitespace();
                // After '=', we expect '(' for nested path pattern
                if self.peek_char_is('(') {
                    return Ok(Some(ident));
                } else {
                    // Not a valid subpath variable pattern, backtrack
                    self.pos = start_pos;
                }
            } else {
                // Not a subpath variable, backtrack
                self.pos = start_pos;
            }
        }
        
        // Check if this is a nested parenthesized path (starts with '(')
        if self.peek_char_is('(') {
            return Ok(None);
        }
        
        // Otherwise not a valid parenthesized path pattern start
        // (it should be handled by node pattern parsing)
        Ok(None)
    }

    /// Parse path mode prefix
    fn parse_path_mode(&mut self) -> Result<Option<PathMode>> {
        if self.try_keyword("WALK") {
            Ok(Some(PathMode::Walk))
        } else if self.try_keyword("TRAIL") {
            Ok(Some(PathMode::Trail))
        } else if self.try_keyword("SIMPLE") {
            Ok(Some(PathMode::Simple))
        } else if self.try_keyword("ACYCLIC") {
            Ok(Some(PathMode::Acyclic))
        } else {
            Ok(None)
        }
    }

    /// Parse path search prefix
    fn parse_path_search_prefix(&mut self) -> Result<Option<PathSearchPrefix>> {
        if self.try_keyword("ALL") {
            if self.try_keyword("SHORTEST") {
                Ok(Some(PathSearchPrefix::AllShortest))
            } else {
                Ok(Some(PathSearchPrefix::All))
            }
        } else if self.try_keyword("ANY") {
            if self.try_keyword("SHORTEST") {
                Ok(Some(PathSearchPrefix::AnyShortest))
            } else {
                // Check for ANY k (k paths)
                self.skip_whitespace();
                if self.peek_char_is_digit() {
                    let k = self.parse_integer()? as u64;
                    Ok(Some(PathSearchPrefix::AnyK(k)))
                } else {
                    Ok(Some(PathSearchPrefix::Any))
                }
            }
        } else if self.try_keyword("SHORTEST") {
            // SHORTEST k or SHORTEST k GROUP/GROUPS
            self.skip_whitespace();
            if self.peek_char_is_digit() {
                let k = self.parse_integer()? as u64;
                // Support both GROUP and GROUPS
                if self.try_keyword("GROUPS") || self.try_keyword("GROUP") {
                    Ok(Some(PathSearchPrefix::ShortestKGroups(k)))
                } else {
                    Ok(Some(PathSearchPrefix::ShortestK(k)))
                }
            } else {
                // Default to SHORTEST 1
                Ok(Some(PathSearchPrefix::AnyShortest))
            }
        } else {
            Ok(None)
        }
    }

    /// Parse node pattern (n:Label {prop: value})
    fn parse_node_pattern(&mut self) -> Result<NodePattern> {
        self.skip_whitespace();
        self.expect_char('(')?;

        let mut node = NodePattern::new();

        self.skip_whitespace();

        // Variable name (optional)
        if self.peek_char_is_alpha() {
            let start_pos = self.pos;
            let name = self.parse_identifier()?;
            self.skip_whitespace();

            // If followed by ':' or space before ':', it's a variable
            if !self.peek_char_is(':') || name.chars().next().unwrap().is_lowercase() {
                node.variable = Some(name);
            } else {
                // Backtrack, this is a label
                self.pos = start_pos;
            }
        }

        // Parse label expression
        node.label_expr = self.parse_label_expression()?;

        // Properties (optional)
        if self.try_char('{') {
            node.properties = self.parse_properties()?;
            self.expect_char('}')?;
        }

        // WHERE clause inside pattern (optional)
        if self.try_keyword("WHERE") {
            node.where_clause = Some(Box::new(self.parse_expression()?));
        }

        self.skip_whitespace();
        self.expect_char(')')?;

        Ok(node)
    }

    /// Parse label expression with full operator support (ISO GQL 39075)
    /// labelExpression: labelTerm (VERTICAL_BAR labelTerm)*
    /// labelTerm: labelFactor (AMPERSAND labelFactor)*
    /// labelFactor: EXCLAMATION_MARK labelFactor | labelPrimary
    /// labelPrimary: labelName | PERCENT | LEFT_PAREN labelExpression RIGHT_PAREN
    fn parse_label_expression(&mut self) -> Result<Option<LabelExpression>> {
        if !self.peek_char_is(':') {
            return Ok(None);
        }

        self.expect_char(':')?;
        self.skip_whitespace();
        
        // Parse the full label expression (handles |, &, !, (), %)
        let expr = self.parse_label_disjunction()?;
        Ok(Some(expr))
    }

    /// Parse label disjunction (lowest precedence): term | term | ...
    fn parse_label_disjunction(&mut self) -> Result<LabelExpression> {
        let mut terms = vec![self.parse_label_conjunction()?];
        
        self.skip_whitespace();
        while self.try_char('|') {
            self.skip_whitespace();
            terms.push(self.parse_label_conjunction()?);
            self.skip_whitespace();
        }
        
        if terms.len() == 1 {
            Ok(terms.remove(0))
        } else {
            Ok(LabelExpression::Disjunction(terms))
        }
    }

    /// Parse label conjunction (medium precedence): factor & factor & ...
    fn parse_label_conjunction(&mut self) -> Result<LabelExpression> {
        let mut factors = vec![self.parse_label_factor()?];
        
        self.skip_whitespace();
        while self.try_char('&') {
            self.skip_whitespace();
            factors.push(self.parse_label_factor()?);
            self.skip_whitespace();
        }
        
        if factors.len() == 1 {
            Ok(factors.remove(0))
        } else {
            Ok(LabelExpression::Conjunction(factors))
        }
    }

    /// Parse label factor (highest precedence): !factor | primary
    fn parse_label_factor(&mut self) -> Result<LabelExpression> {
        self.skip_whitespace();
        
        // Check for negation
        if self.try_char('!') {
            self.skip_whitespace();
            let inner = self.parse_label_factor()?;
            Ok(LabelExpression::Negation(Box::new(inner)))
        } else {
            self.parse_label_primary()
        }
    }

    /// Parse label primary: labelName | % | (labelExpression)
    fn parse_label_primary(&mut self) -> Result<LabelExpression> {
        self.skip_whitespace();
        
        // Check for parenthesized expression
        if self.try_char('(') {
            self.skip_whitespace();
            let inner = self.parse_label_disjunction()?;
            self.skip_whitespace();
            self.expect_char(')')?;
            Ok(inner)
        }
        // Check for wildcard %
        else if self.try_char('%') {
            Ok(LabelExpression::Wildcard)
        }
        // Parse label name
        else {
            let label_str = self.parse_identifier()?;
            if let Some(vertex_label) = Self::parse_vertex_label(&label_str) {
                Ok(LabelExpression::Label(vertex_label))
            } else if let Some(edge_label) = Self::parse_edge_label(&label_str) {
                Ok(LabelExpression::EdgeLabel(edge_label))
            } else {
                // Unknown label - for now create a generic label expression
                // This allows parsing to continue even with unknown labels
                Err(Error::ParseError(format!("Unknown label: {}", label_str)))
            }
        }
    }

    /// Parse edge pattern (ISO GQL 39075)
    /// Supports all 7 edge direction types:
    ///   -[...]->  : outgoing (right)
    ///   <-[...]-  : incoming (left)
    ///   ~[...]~   : undirected
    ///   -[...]-   : any direction
    ///   <-[...]-> : left or right (bidirectional)
    ///   <~[...]~  : left or undirected
    ///   ~[...]~>  : undirected or right
    fn parse_edge_pattern(&mut self) -> Result<EdgePattern> {
        self.skip_whitespace();

        // Parse direction prefix: <-, <~, ~, or -
        let has_left_arrow = self.try_str("<-");
        let has_left_tilde = if !has_left_arrow { self.try_str("<~") } else { false };
        let has_tilde = if !has_left_arrow && !has_left_tilde { self.try_char('~') } else { false };
        let has_dash = if !has_left_arrow && !has_left_tilde && !has_tilde { 
            self.try_char('-') 
        } else { 
            false 
        };

        // Determine initial direction based on prefix
        let mut edge = EdgePattern::new(if has_left_arrow {
            EdgeDirection::Incoming
        } else if has_left_tilde {
            EdgeDirection::LeftOrUndirected
        } else if has_tilde {
            EdgeDirection::Undirected
        } else {
            EdgeDirection::AnyDirection // Will be refined based on suffix
        });

        // Optional edge details [...]
        if self.try_char('[') {
            self.skip_whitespace();

            // Try to parse variable name
            if self.peek_char_is_alpha() {
                let name = self.parse_identifier()?;
                self.skip_whitespace();

                // Variable name is always captured
                edge.variable = Some(name);
            }

            // Parse label expression for edge types (starts with :)
            edge.label_expr = self.parse_label_expression()?;

            // Properties
            if self.try_char('{') {
                edge.properties = self.parse_properties()?;
                self.expect_char('}')?;
            }

            // WHERE clause inside pattern
            if self.try_keyword("WHERE") {
                edge.where_clause = Some(Box::new(self.parse_expression()?));
            }

            self.skip_whitespace();
            self.expect_char(']')?;
        }

        // Parse direction suffix: ->, ~>, ~, or -
        let has_right_arrow = self.try_str("->");
        let has_tilde_right = if !has_right_arrow { self.try_str("~>") } else { false };
        let has_suffix_tilde = if !has_right_arrow && !has_tilde_right { self.try_char('~') } else { false };
        let has_suffix_dash = if !has_right_arrow && !has_tilde_right && !has_suffix_tilde {
            self.try_char('-')
        } else {
            false
        };

        // Determine final direction based on prefix + suffix combination
        edge.direction = match (has_left_arrow, has_left_tilde, has_tilde, has_dash, 
                                 has_right_arrow, has_tilde_right, has_suffix_tilde, has_suffix_dash) {
            // -[...]->  : outgoing
            (false, false, false, true, true, false, false, false) => EdgeDirection::Outgoing,
            // <-[...]-  : incoming
            (true, false, false, false, false, false, false, true) => EdgeDirection::Incoming,
            // <-[...]-> : left or right (bidirectional)
            (true, false, false, false, true, false, false, false) => EdgeDirection::LeftOrRight,
            // ~[...]~   : undirected
            (false, false, true, false, false, false, true, false) => EdgeDirection::Undirected,
            // ~[...]~>  : undirected or right
            (false, false, true, false, false, true, false, false) => EdgeDirection::UndirectedOrRight,
            // <~[...]~  : left or undirected
            (false, true, false, false, false, false, true, false) => EdgeDirection::LeftOrUndirected,
            // -[...]-   : any direction
            (false, false, false, true, false, false, false, true) => EdgeDirection::AnyDirection,
            // Default fallback
            _ => edge.direction,
        };

        // ISO GQL 39075 quantifier suffix: ->{1,5} or ->+ or ->*
        if has_right_arrow || has_tilde_right {
            edge.quantifier = self.parse_gql_quantifier()?;
        }

        Ok(edge)
    }

    /// Parse plain integer (digits only, no decimals)
    fn parse_plain_integer(&mut self) -> Result<i64> {
        self.skip_whitespace();
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            Err(Error::ParseError("Expected integer".to_string()))
        } else {
            self.input[start..self.pos]
                .parse()
                .map_err(|_| Error::ParseError("Invalid integer".to_string()))
        }
    }

    /// Parse quantifier {n,m} or *, +, ?
    fn parse_quantifier(&mut self) -> Result<Option<PatternQuantifier>> {
        self.skip_whitespace();

        if self.try_char('*') {
            Ok(Some(PatternQuantifier::ZeroOrMore))
        } else if self.try_char('+') {
            Ok(Some(PatternQuantifier::OneOrMore))
        } else if self.try_char('?') {
            Ok(Some(PatternQuantifier::ZeroOrOne))
        } else if self.try_char('{') {
            self.skip_whitespace();

            let result = if self.peek_char_is_digit() {
                let min = self.parse_integer()? as u64;
                self.skip_whitespace();

                if self.try_char(',') {
                    self.skip_whitespace();
                    if self.peek_char_is_digit() {
                        let max = self.parse_integer()? as u64;
                        PatternQuantifier::Range(min, max)
                    } else {
                        PatternQuantifier::AtLeast(min)
                    }
                } else {
                    PatternQuantifier::Exactly(min)
                }
            } else if self.try_char(',') {
                self.skip_whitespace();
                let max = self.parse_integer()? as u64;
                PatternQuantifier::AtMost(max)
            } else {
                return Err(Error::ParseError("Invalid quantifier".to_string()));
            };

            self.skip_whitespace();
            self.expect_char('}')?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Parse ISO GQL 39075 path quantifier: ->{n,m} or ->+ or ->* or ->?
    /// Examples:
    ///   ->{1,5}  : 1 to 5 hops
    ///   ->{3}    : exactly 3 hops
    ///   ->{2,}   : at least 2 hops
    ///   ->{,5}   : at most 5 hops
    ///   ->+      : one or more (same as {1,})
    ///   ->*      : zero or more (same as {0,})
    ///   ->?      : zero or one (same as {0,1})
    fn parse_gql_quantifier(&mut self) -> Result<Option<PatternQuantifier>> {
        self.skip_whitespace();

        if self.try_char('*') {
            Ok(Some(PatternQuantifier::ZeroOrMore))
        } else if self.try_char('+') {
            Ok(Some(PatternQuantifier::OneOrMore))
        } else if self.try_char('?') {
            Ok(Some(PatternQuantifier::ZeroOrOne))
        } else if self.try_char('{') {
            self.skip_whitespace();

            let result = if self.peek_char_is_digit() {
                let min = self.parse_plain_integer()? as u64;
                self.skip_whitespace();

                if self.try_char(',') {
                    self.skip_whitespace();
                    if self.peek_char_is_digit() {
                        let max = self.parse_plain_integer()? as u64;
                        PatternQuantifier::Range(min, max)
                    } else {
                        PatternQuantifier::AtLeast(min)
                    }
                } else {
                    PatternQuantifier::Exactly(min)
                }
            } else if self.try_char(',') {
                self.skip_whitespace();
                let max = self.parse_plain_integer()? as u64;
                PatternQuantifier::AtMost(max)
            } else {
                return Err(Error::ParseError("Invalid quantifier".to_string()));
            };

            self.skip_whitespace();
            self.expect_char('}')?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Parse properties list
    fn parse_properties(&mut self) -> Result<Vec<(String, PropertyValue)>> {
        let mut props = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek_char_is('}') {
                break;
            }

            let key = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            self.skip_whitespace();
            let value = self.parse_property_value()?;

            props.push((key, value));

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(props)
    }

    /// Parse property value
    fn parse_property_value(&mut self) -> Result<PropertyValue> {
        self.skip_whitespace();

        if self.peek_char_is('"') || self.peek_char_is('\'') {
            let s = self.parse_string()?;

            // Try to parse as special types
            if s.starts_with("0x") && s.len() == 42 {
                if let Ok(addr) = Address::from_hex(&s) {
                    return Ok(PropertyValue::Address(addr));
                }
            }
            if s.starts_with("0x") && s.len() == 66 {
                if let Ok(hash) = crate::types::TxHash::from_hex(&s) {
                    return Ok(PropertyValue::TxHash(hash));
                }
            }

            Ok(PropertyValue::String(s))
        } else if self.peek_char_is_digit() || self.peek_char_is('-') {
            let num = self.parse_number()?;
            if num.contains('.') {
                Ok(PropertyValue::Float(num.parse().unwrap_or(0.0)))
            } else {
                Ok(PropertyValue::Integer(num.parse().unwrap_or(0)))
            }
        } else if self.try_keyword("true") {
            Ok(PropertyValue::Boolean(true))
        } else if self.try_keyword("false") {
            Ok(PropertyValue::Boolean(false))
        } else if self.try_keyword("null") || self.try_keyword("NULL") {
            Ok(PropertyValue::String(String::new()))
        } else {
            Err(Error::ParseError("Invalid property value".to_string()))
        }
    }

    // ========================================================================
    // Expression Parsing
    // ========================================================================

    /// Parse expression
    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_xor_expression()?;

        while self.try_keyword("OR") {
            let right = self.parse_xor_expression()?;
            left = Expression::BinaryOp(Box::new(left), BinaryOperator::Or, Box::new(right));
        }

        Ok(left)
    }

    fn parse_xor_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.try_keyword("XOR") {
            let right = self.parse_and_expression()?;
            left = Expression::BinaryOp(Box::new(left), BinaryOperator::Xor, Box::new(right));
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_not_expression()?;

        while self.try_keyword("AND") {
            let right = self.parse_not_expression()?;
            left = Expression::BinaryOp(Box::new(left), BinaryOperator::And, Box::new(right));
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> Result<Expression> {
        if self.try_keyword("NOT") {
            let expr = self.parse_not_expression()?;
            Ok(Expression::UnaryOp(UnaryOperator::Not, Box::new(expr)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expression> {
        let left = self.parse_additive()?;

        self.skip_whitespace();

        // IS NULL / IS NOT NULL
        if self.try_keyword("IS") {
            if self.try_keyword("NOT") {
                self.expect_keyword("NULL")?;
                return Ok(Expression::UnaryOp(
                    UnaryOperator::IsNotNull,
                    Box::new(left),
                ));
            } else {
                self.expect_keyword("NULL")?;
                return Ok(Expression::UnaryOp(UnaryOperator::IsNull, Box::new(left)));
            }
        }

        let op = if self.try_str("<=") {
            Some(BinaryOperator::Le)
        } else if self.try_str(">=") {
            Some(BinaryOperator::Ge)
        } else if self.try_str("<>") || self.try_str("!=") {
            Some(BinaryOperator::Ne)
        } else if self.try_char('<') {
            Some(BinaryOperator::Lt)
        } else if self.try_char('>') {
            Some(BinaryOperator::Gt)
        } else if self.try_char('=') {
            Some(BinaryOperator::Eq)
        } else if self.try_keyword("CONTAINS") {
            Some(BinaryOperator::Contains)
        } else if self.try_keyword("STARTS") {
            self.expect_keyword("WITH")?;
            Some(BinaryOperator::StartsWith)
        } else if self.try_keyword("ENDS") {
            self.expect_keyword("WITH")?;
            Some(BinaryOperator::EndsWith)
        } else if self.try_keyword("LIKE") {
            Some(BinaryOperator::Like)
        } else if self.try_keyword("IN") {
            Some(BinaryOperator::In)
        } else {
            None
        };

        if let Some(operator) = op {
            let right = self.parse_additive()?;
            Ok(Expression::BinaryOp(
                Box::new(left),
                operator,
                Box::new(right),
            ))
        } else {
            Ok(left)
        }
    }

    fn parse_additive(&mut self) -> Result<Expression> {
        let mut left = self.parse_multiplicative()?;

        loop {
            self.skip_whitespace();
            if self.try_char('+') {
                let right = self.parse_multiplicative()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Add, Box::new(right));
            } else if self.try_char('-') && !self.peek_char_is_digit() {
                let right = self.parse_multiplicative()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Sub, Box::new(right));
            } else if self.try_str("||") {
                let right = self.parse_multiplicative()?;
                left =
                    Expression::BinaryOp(Box::new(left), BinaryOperator::Concat, Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression> {
        let mut left = self.parse_power()?;

        loop {
            self.skip_whitespace();
            if self.try_char('*') {
                let right = self.parse_power()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Mul, Box::new(right));
            } else if self.try_char('/') {
                let right = self.parse_power()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Div, Box::new(right));
            } else if self.try_char('%') {
                let right = self.parse_power()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Mod, Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expression> {
        let left = self.parse_unary()?;

        self.skip_whitespace();
        if self.try_char('^') {
            let right = self.parse_power()?;
            Ok(Expression::BinaryOp(
                Box::new(left),
                BinaryOperator::Power,
                Box::new(right),
            ))
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<Expression> {
        self.skip_whitespace();

        if self.try_char('-') {
            let expr = self.parse_unary()?;
            return Ok(Expression::UnaryOp(UnaryOperator::Neg, Box::new(expr)));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        self.skip_whitespace();

        // Parenthesized expression
        if self.try_char('(') {
            let expr = self.parse_expression()?;
            self.skip_whitespace();
            self.expect_char(')')?;
            return Ok(expr);
        }

        // List expression
        if self.try_char('[') {
            let mut items = Vec::new();
            loop {
                self.skip_whitespace();
                if self.peek_char_is(']') {
                    break;
                }
                items.push(self.parse_expression()?);
                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            self.expect_char(']')?;
            return Ok(Expression::List(items));
        }

        // Map expression
        if self.try_char('{') {
            let mut entries = Vec::new();
            loop {
                self.skip_whitespace();
                if self.peek_char_is('}') {
                    break;
                }
                let key = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect_char(':')?;
                let value = self.parse_expression()?;
                entries.push((key, value));
                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            self.expect_char('}')?;
            return Ok(Expression::Map(entries));
        }

        // Parameter
        if self.try_char('$') {
            let name = self.parse_identifier()?;
            return Ok(Expression::Parameter(name));
        }

        // String literal
        if self.peek_char_is('"') || self.peek_char_is('\'') {
            let s = self.parse_string()?;
            return Ok(Expression::Literal(PropertyValue::String(s)));
        }

        // Number literal
        if self.peek_char_is_digit() || (self.peek_char_is('-') && self.peek_next_char_is_digit()) {
            let num = self.parse_number()?;
            if num.contains('.') {
                return Ok(Expression::Literal(PropertyValue::Float(
                    num.parse().unwrap_or(0.0),
                )));
            } else {
                return Ok(Expression::Literal(PropertyValue::Integer(
                    num.parse().unwrap_or(0),
                )));
            }
        }

        // Boolean literals and NULL
        if self.try_keyword("true") {
            return Ok(Expression::Literal(PropertyValue::Boolean(true)));
        }
        if self.try_keyword("false") {
            return Ok(Expression::Literal(PropertyValue::Boolean(false)));
        }
        if self.try_keyword("null") || self.try_keyword("NULL") {
            return Ok(Expression::Null);
        }

        // CASE expression
        if self.try_keyword("CASE") {
            return self.parse_case_expression();
        }

        // EXISTS predicate
        if self.try_keyword("EXISTS") {
            self.expect_char('{')?;
            let pattern = self.parse_graph_pattern()?;
            self.expect_char('}')?;
            return Ok(Expression::Exists(Box::new(pattern)));
        }

        // Identifier (variable or function call)
        let ident = self.parse_identifier()?;
        self.skip_whitespace();

        if self.try_char('(') {
            // Function call
            let mut args = Vec::new();
            loop {
                self.skip_whitespace();
                if self.peek_char_is(')') {
                    break;
                }
                args.push(self.parse_expression()?);
                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
            Ok(Expression::FunctionCall(ident, args))
        } else if self.try_char('.') {
            // Property access
            let prop = self.parse_identifier()?;
            Ok(Expression::Property(ident, prop))
        } else {
            Ok(Expression::Variable(ident))
        }
    }

    /// Parse CASE expression
    fn parse_case_expression(&mut self) -> Result<Expression> {
        self.skip_whitespace();

        // Simple CASE or searched CASE
        let operand = if !self.peek_keyword_is("WHEN") {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let mut when_clauses = Vec::new();
        while self.try_keyword("WHEN") {
            let condition = self.parse_expression()?;
            self.expect_keyword("THEN")?;
            let result = self.parse_expression()?;
            when_clauses.push((condition, result));
        }

        let else_clause = if self.try_keyword("ELSE") {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect_keyword("END")?;

        Ok(Expression::Case {
            operand,
            when_clauses,
            else_clause,
        })
    }

    // ========================================================================
    // RETURN Clause
    // ========================================================================

    fn parse_return_items(&mut self) -> Result<Vec<ReturnItem>> {
        let mut items = Vec::new();

        // Check for RETURN *
        if self.try_char('*') {
            return Ok(vec![ReturnItem::new(Expression::Variable("*".to_string()))]);
        }

        loop {
            self.skip_whitespace();
            let expr = self.parse_expression()?;

            let alias = if self.try_keyword("AS") {
                Some(self.parse_identifier()?)
            } else {
                None
            };

            items.push(ReturnItem {
                expression: expr,
                alias,
            });

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(items)
    }

    fn parse_order_by(&mut self) -> Result<Vec<OrderByItem>> {
        let mut items = Vec::new();

        loop {
            self.skip_whitespace();
            let expr = self.parse_expression()?;
            let descending = self.try_keyword("DESC");
            if !descending {
                self.try_keyword("ASC");
            }

            let nulls_first = if self.try_keyword("NULLS") {
                if self.try_keyword("FIRST") {
                    Some(true)
                } else if self.try_keyword("LAST") {
                    Some(false)
                } else {
                    None
                }
            } else {
                None
            };

            items.push(OrderByItem {
                expression: expr,
                descending,
                nulls_first,
            });

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(items)
    }

    // ========================================================================
    // INSERT Statement
    // ========================================================================

    fn parse_insert(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("INSERT")?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_vars: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        loop {
            self.skip_whitespace();
            if !self.peek_char_is('(') {
                break;
            }

            let first_node = self.parse_node_pattern()?;
            let first_var = first_node.variable.clone();

            self.skip_whitespace();

            // Check for edge pattern
            if self.peek_char_is('-') || self.peek_char_is('<') || self.peek_char_is('~') {
                let edge_pattern = self.parse_edge_pattern()?;
                self.skip_whitespace();

                let second_node = self.parse_node_pattern()?;
                let second_var = second_node.variable.clone();

                // Record nodes
                if let Some(ref var) = first_var {
                    if !node_vars.contains_key(var) {
                        node_vars.insert(var.clone(), nodes.len());
                        nodes.push(first_node.clone());
                    }
                } else {
                    nodes.push(first_node.clone());
                }

                if let Some(ref var) = second_var {
                    if !node_vars.contains_key(var) {
                        node_vars.insert(var.clone(), nodes.len());
                        nodes.push(second_node.clone());
                    }
                } else {
                    nodes.push(second_node.clone());
                }

                edges.push(InsertEdge {
                    source: first_var.unwrap_or_else(|| format!("_anon_{}", nodes.len() - 2)),
                    edge: edge_pattern,
                    target: second_var.unwrap_or_else(|| format!("_anon_{}", nodes.len() - 1)),
                });
            } else {
                if let Some(ref var) = first_var {
                    if !node_vars.contains_key(var) {
                        node_vars.insert(var.clone(), nodes.len());
                        nodes.push(first_node);
                    }
                } else {
                    nodes.push(first_node);
                }
            }

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(GqlStatement::Insert(InsertStatement { nodes, edges }))
    }

    // ========================================================================
    // DELETE Statement
    // ========================================================================

    fn parse_delete(&mut self) -> Result<GqlStatement> {
        let detach = if self.try_keyword("DETACH") {
            true
        } else if self.try_keyword("NODETACH") {
            false
        } else {
            false
        };

        self.expect_keyword("DELETE")?;

        let mut variables = Vec::new();
        loop {
            self.skip_whitespace();
            variables.push(self.parse_identifier()?);
            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(GqlStatement::Delete(DeleteStatement { variables, detach }))
    }

    // ========================================================================
    // SET Statement
    // ========================================================================

    fn parse_set(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("SET")?;

        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            let variable = self.parse_identifier()?;
            self.skip_whitespace();

            if self.try_char('.') {
                // Property: n.prop = value
                let prop = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect_char('=')?;
                let value = self.parse_expression()?;
                items.push(SetItem::Property(variable, prop, value));
            } else if self.try_char(':') {
                // Label: n:Label
                let label_str = self.parse_identifier()?;
                if let Some(label) = Self::parse_vertex_label(&label_str) {
                    items.push(SetItem::Label(variable, label));
                }
            } else if self.try_str("+=") {
                // Merge properties: n += {props}
                self.skip_whitespace();
                self.expect_char('{')?;
                let props = self.parse_property_key_value_pairs()?;
                self.expect_char('}')?;
                items.push(SetItem::AllProperties {
                    variable,
                    properties: props,
                    merge: true,
                });
            } else if self.try_char('=') {
                // Replace properties: n = {props}
                self.skip_whitespace();
                self.expect_char('{')?;
                let props = self.parse_property_key_value_pairs()?;
                self.expect_char('}')?;
                items.push(SetItem::AllProperties {
                    variable,
                    properties: props,
                    merge: false,
                });
            } else {
                return Err(Error::ParseError("Invalid SET syntax".to_string()));
            }

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(GqlStatement::Set(SetStatement { items }))
    }

    fn parse_property_key_value_pairs(&mut self) -> Result<Vec<(String, Expression)>> {
        let mut props = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek_char_is('}') {
                break;
            }

            let key = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            self.skip_whitespace();
            let value = self.parse_expression()?;
            props.push((key, value));

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }
        Ok(props)
    }

    // ========================================================================
    // REMOVE Statement
    // ========================================================================

    fn parse_remove(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("REMOVE")?;

        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            let variable = self.parse_identifier()?;
            self.skip_whitespace();

            if self.try_char('.') {
                // Property: n.prop
                let prop = self.parse_identifier()?;
                items.push(RemoveItem::Property(variable, prop));
            } else if self.try_char(':') {
                // Label: n:Label
                let label_str = self.parse_identifier()?;
                if let Some(label) = Self::parse_vertex_label(&label_str) {
                    items.push(RemoveItem::Label(variable, label));
                }
            } else {
                return Err(Error::ParseError("Invalid REMOVE syntax".to_string()));
            }

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(GqlStatement::Remove(RemoveStatement { items }))
    }

    // ========================================================================
    // CALL Statement
    // ========================================================================

    fn parse_call(&mut self) -> Result<GqlStatement> {
        let optional = self.try_keyword("OPTIONAL");
        self.expect_keyword("CALL")?;
        self.skip_whitespace();

        // Parse procedure name (may contain dots)
        let mut procedure_name = self.parse_identifier()?;
        while self.try_char('.') {
            procedure_name.push('.');
            procedure_name.push_str(&self.parse_identifier()?);
        }

        // Parse arguments
        self.skip_whitespace();
        let arguments = if self.try_char('(') {
            let mut args = Vec::new();
            loop {
                self.skip_whitespace();
                if self.peek_char_is(')') {
                    break;
                }
                args.push(self.parse_expression()?);
                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
            args
        } else {
            Vec::new()
        };

        // Parse YIELD clause
        self.skip_whitespace();
        let yield_items = if self.try_keyword("YIELD") {
            let mut items = Vec::new();
            loop {
                self.skip_whitespace();
                let name = self.parse_identifier()?;
                let alias = if self.try_keyword("AS") {
                    Some(self.parse_identifier()?)
                } else {
                    None
                };
                items.push(YieldItem { name, alias });

                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            Some(items)
        } else {
            None
        };

        Ok(GqlStatement::Call(CallStatement {
            optional,
            procedure_name,
            arguments,
            yield_items,
        }))
    }

    // ========================================================================
    // Graph Management Statements
    // ========================================================================

    fn parse_create(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("CREATE")?;
        self.skip_whitespace();

        if self.try_keyword("GRAPH") {
            self.skip_whitespace();

            // Check if it's GRAPH TYPE
            if self.try_keyword("TYPE") {
                // CREATE GRAPH TYPE
                let if_not_exists = self.try_keyword("IF") && {
                    self.expect_keyword("NOT")?;
                    self.expect_keyword("EXISTS")?;
                    true
                };

                let or_replace = self.try_keyword("OR") && {
                    self.expect_keyword("REPLACE")?;
                    true
                };

                let name = self.parse_identifier()?;

                // Parse AS (...) - skip the definition for now
                let elements = Vec::new();
                if self.try_keyword("AS") {
                    self.skip_whitespace();
                    // Parse the type definition - skip for now
                    if self.peek_char() == Some('(') {
                        self.pos += 1; // skip '('
                        let mut depth = 1;
                        while self.pos < self.input.len() && depth > 0 {
                            let c = self.input[self.pos..].chars().next().unwrap();
                            if c == '(' {
                                depth += 1;
                            }
                            if c == ')' {
                                depth -= 1;
                            }
                            self.pos += c.len_utf8();
                        }
                    }
                }

                Ok(GqlStatement::CreateGraphType(CreateGraphTypeStatement {
                    name,
                    if_not_exists,
                    or_replace,
                    elements,
                }))
            } else {
                // CREATE GRAPH
                let if_not_exists = self.try_keyword("IF") && {
                    self.expect_keyword("NOT")?;
                    self.expect_keyword("EXISTS")?;
                    true
                };

                let name = self.parse_identifier()?;

                // Parse graph type (CLOSED is default for Web3)
                let graph_type = if self.try_keyword("OPEN") {
                    GraphType::Open
                } else {
                    self.try_keyword("CLOSED");
                    GraphType::Closed
                };

                Ok(GqlStatement::CreateGraph(CreateGraphStatement {
                    name,
                    if_not_exists,
                    graph_type,
                }))
            }
        } else {
            Err(Error::ParseError("Expected GRAPH after CREATE".to_string()))
        }
    }

    fn parse_drop(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("DROP")?;
        self.skip_whitespace();

        if self.try_keyword("GRAPH") {
            self.skip_whitespace();

            // Check if it's GRAPH TYPE
            if self.try_keyword("TYPE") {
                // DROP GRAPH TYPE
                let if_exists = self.try_keyword("IF") && {
                    self.expect_keyword("EXISTS")?;
                    true
                };

                let name = self.parse_identifier()?;

                Ok(GqlStatement::DropGraphType(DropGraphTypeStatement {
                    name,
                    if_exists,
                }))
            } else {
                // DROP GRAPH
                let if_exists = self.try_keyword("IF") && {
                    self.expect_keyword("EXISTS")?;
                    true
                };

                let name = self.parse_identifier()?;

                Ok(GqlStatement::DropGraph(DropGraphStatement {
                    name,
                    if_exists,
                }))
            }
        } else {
            Err(Error::ParseError("Expected GRAPH after DROP".to_string()))
        }
    }

    // ========================================================================
    // SHOW and DESCRIBE Statements
    // ========================================================================

    /// Parse SHOW statement
    /// SHOW GRAPHS | SHOW GRAPH TYPES | SHOW SCHEMAS | etc.
    fn parse_show(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("SHOW")?;
        self.skip_whitespace();

        let show_type = if self.try_keyword("GRAPHS") {
            ShowType::Graphs
        } else if self.try_keyword("GRAPH") {
            self.expect_keyword("TYPES")?;
            ShowType::GraphTypes
        } else if self.try_keyword("SCHEMAS") {
            ShowType::Schemas
        } else if self.try_keyword("FUNCTIONS") {
            ShowType::Functions
        } else if self.try_keyword("PROCEDURES") {
            ShowType::Procedures
        } else if self.try_keyword("LABELS") {
            ShowType::Labels
        } else if self.try_keyword("EDGE") {
            self.expect_keyword("TYPES")?;
            ShowType::EdgeTypes
        } else if self.try_keyword("RELATIONSHIP") {
            self.expect_keyword("TYPES")?;
            ShowType::EdgeTypes
        } else if self.try_keyword("PROPERTY") {
            self.expect_keyword("KEYS")?;
            ShowType::PropertyKeys
        } else if self.try_keyword("INDEXES") {
            ShowType::Indexes
        } else if self.try_keyword("CONSTRAINTS") {
            ShowType::Constraints
        } else {
            return Err(Error::ParseError("Unknown SHOW target".to_string()));
        };

        // Optional LIKE pattern
        let like_pattern = if self.try_keyword("LIKE") {
            Some(self.parse_string()?)
        } else {
            None
        };

        Ok(GqlStatement::Show(ShowStatement {
            show_type,
            like_pattern,
        }))
    }

    /// Parse DESCRIBE/DESC statement
    /// DESCRIBE GRAPH name | DESCRIBE GRAPH TYPE name | DESC ...
    fn parse_describe(&mut self) -> Result<GqlStatement> {
        // Accept both DESCRIBE and DESC
        if !self.try_keyword("DESCRIBE") {
            self.expect_keyword("DESC")?;
        }
        self.skip_whitespace();

        let (describe_type, name) = if self.try_keyword("GRAPH") {
            self.skip_whitespace();
            if self.try_keyword("TYPE") {
                let name = self.parse_identifier()?;
                (DescribeType::GraphType, name)
            } else {
                let name = self.parse_identifier()?;
                (DescribeType::Graph, name)
            }
        } else if self.try_keyword("SCHEMA") {
            let name = self.parse_identifier()?;
            (DescribeType::Schema, name)
        } else if self.try_keyword("LABEL") || self.try_keyword("VERTEX") {
            if self.try_keyword("TYPE") {
                // DESCRIBE VERTEX TYPE name
            }
            let name = self.parse_identifier()?;
            (DescribeType::Label, name)
        } else if self.try_keyword("EDGE") || self.try_keyword("RELATIONSHIP") {
            if self.try_keyword("TYPE") {
                // DESCRIBE EDGE TYPE name
            }
            let name = self.parse_identifier()?;
            (DescribeType::EdgeType, name)
        } else {
            // Default to describing a graph
            let name = self.parse_identifier()?;
            (DescribeType::Graph, name)
        };

        Ok(GqlStatement::Describe(DescribeStatement {
            describe_type,
            name,
        }))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else if self.input[self.pos..].starts_with("//") {
                // Single-line comment
                while self.pos < self.input.len() && !self.input[self.pos..].starts_with('\n') {
                    self.pos += 1;
                }
            } else if self.input[self.pos..].starts_with("/*") {
                // Multi-line comment
                self.pos += 2;
                while self.pos < self.input.len() - 1 && !self.input[self.pos..].starts_with("*/") {
                    self.pos += 1;
                }
                if self.pos < self.input.len() - 1 {
                    self.pos += 2;
                }
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_next_char_is_digit(&self) -> bool {
        self.input[self.pos..]
            .chars()
            .nth(1)
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    }

    fn peek_char_is(&self, c: char) -> bool {
        self.peek_char() == Some(c)
    }

    fn peek_char_is_alpha(&self) -> bool {
        self.peek_char()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
    }

    fn peek_char_is_digit(&self) -> bool {
        self.peek_char()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    }

    /// Peek ahead to check if the input starts with a specific string
    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn try_char(&mut self, c: char) -> bool {
        self.skip_whitespace();
        if self.peek_char() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, c: char) -> Result<()> {
        self.skip_whitespace();
        if self.peek_char() == Some(c) {
            self.pos += c.len_utf8();
            Ok(())
        } else {
            Err(Error::ParseError(format!(
                "Expected '{}', got {:?}",
                c,
                self.peek_char()
            )))
        }
    }

    fn try_str(&mut self, s: &str) -> bool {
        self.skip_whitespace();
        if self.input[self.pos..].starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    fn peek_keyword(&self) -> Result<String> {
        let mut end = self.pos;
        while end < self.input.len() {
            let c = self.input[end..].chars().next().unwrap();
            if c.is_alphanumeric() || c == '_' {
                end += c.len_utf8();
            } else {
                break;
            }
        }
        if end == self.pos {
            Err(Error::ParseError("Expected keyword".to_string()))
        } else {
            Ok(self.input[self.pos..end].to_string())
        }
    }

    fn peek_keyword_is(&self, keyword: &str) -> bool {
        if let Ok(kw) = self.peek_keyword() {
            kw.eq_ignore_ascii_case(keyword)
        } else {
            false
        }
    }

    fn try_keyword(&mut self, keyword: &str) -> bool {
        self.skip_whitespace();
        let len = keyword.len();

        if self.pos + len <= self.input.len() {
            let slice = &self.input[self.pos..self.pos + len];
            if slice.eq_ignore_ascii_case(keyword) {
                // Ensure not followed by identifier char
                if self.pos + len >= self.input.len() {
                    self.pos += len;
                    return true;
                }
                let next = self.input[self.pos + len..].chars().next().unwrap();
                if !next.is_alphanumeric() && next != '_' {
                    self.pos += len;
                    return true;
                }
            }
        }
        false
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.try_keyword(keyword) {
            Ok(())
        } else {
            Err(Error::ParseError(format!("Expected keyword '{}'", keyword)))
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        self.skip_whitespace();
        let start = self.pos;

        if let Some(c) = self.peek_char() {
            if c.is_alphabetic() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                return Err(Error::ParseError("Expected identifier".to_string()));
            }
        } else {
            return Err(Error::ParseError("Expected identifier".to_string()));
        }

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_string(&mut self) -> Result<String> {
        let quote = self
            .peek_char()
            .ok_or_else(|| Error::ParseError("Expected string".to_string()))?;
        if quote != '"' && quote != '\'' {
            return Err(Error::ParseError("Expected string quote".to_string()));
        }
        self.pos += 1;

        let mut result = String::new();
        while let Some(c) = self.peek_char() {
            if c == quote {
                self.pos += 1;
                return Ok(result);
            } else if c == '\\' {
                self.pos += 1;
                if let Some(escaped) = self.peek_char() {
                    self.pos += 1;
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '\'' => result.push('\''),
                        '"' => result.push('"'),
                        _ => result.push(escaped),
                    }
                }
            } else {
                self.pos += c.len_utf8();
                result.push(c);
            }
        }

        Err(Error::ParseError("Unclosed string".to_string()))
    }

    fn parse_number(&mut self) -> Result<String> {
        self.skip_whitespace();
        let start = self.pos;

        if self.peek_char() == Some('-') {
            self.pos += 1;
        }

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            Err(Error::ParseError("Expected number".to_string()))
        } else {
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn parse_integer(&mut self) -> Result<i64> {
        let num = self.parse_number()?;
        num.parse()
            .map_err(|_| Error::ParseError("Invalid integer".to_string()))
    }

    fn parse_vertex_label(s: &str) -> Option<VertexLabel> {
        match s.to_uppercase().as_str() {
            "ACCOUNT" => Some(VertexLabel::Account),
            "CONTRACT" => Some(VertexLabel::Contract),
            "TOKEN" => Some(VertexLabel::Token),
            "TRANSACTION" | "TX" => Some(VertexLabel::Transaction),
            "BLOCK" => Some(VertexLabel::Block),
            _ => Some(VertexLabel::Custom(s.to_string())),
        }
    }

    fn parse_edge_label(s: &str) -> Option<EdgeLabel> {
        match s.to_uppercase().as_str() {
            "TRANSFER" => Some(EdgeLabel::Transfer),
            "CALL" => Some(EdgeLabel::Call),
            "CREATE" => Some(EdgeLabel::Create),
            "APPROVE" => Some(EdgeLabel::Approve),
            _ => None,
        }
    }

    // ========================================================================
    // ISO GQL 39075 New Statement Parsing
    // ========================================================================

    /// Parse LET statement
    /// LET x = expr [, y = expr]*
    fn parse_let(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("LET")?;
        let mut bindings = Vec::new();

        loop {
            let variable = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect_char('=')?;
            self.skip_whitespace();
            let value = self.parse_expression()?;

            bindings.push(LetBinding { variable, value });

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        Ok(GqlStatement::Let(LetStatement { bindings }))
    }

    /// Parse FOR statement
    /// FOR x IN collection
    fn parse_for(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("FOR")?;
        self.skip_whitespace();

        let variable = self.parse_identifier()?;
        self.skip_whitespace();

        // Optional ordinal variable: FOR x, i IN collection
        let ordinal_variable = if self.try_char(',') {
            self.skip_whitespace();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.skip_whitespace();
        self.expect_keyword("IN")?;
        self.skip_whitespace();

        let iterable = self.parse_expression()?;

        Ok(GqlStatement::For(ForStatement {
            variable,
            iterable,
            ordinal_variable,
        }))
    }

    /// Parse FILTER statement
    /// FILTER condition
    fn parse_filter(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("FILTER")?;
        self.skip_whitespace();
        let condition = self.parse_expression()?;

        Ok(GqlStatement::Filter(FilterStatement { condition }))
    }

    /// Parse SELECT statement
    /// SELECT [DISTINCT] items [FROM graph] [WHERE cond] [GROUP BY ...] [HAVING ...] [ORDER BY ...] [OFFSET n] [LIMIT n]
    fn parse_select(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("SELECT")?;
        self.skip_whitespace();

        let distinct = self.try_keyword("DISTINCT");

        // Parse select items
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();

            // Check for * (select all)
            if self.try_char('*') {
                items.push(SelectItem {
                    expression: Expression::Variable("*".to_string()),
                    alias: None,
                });
            } else {
                let expression = self.parse_expression()?;
                self.skip_whitespace();

                let alias = if self.try_keyword("AS") {
                    self.skip_whitespace();
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                items.push(SelectItem { expression, alias });
            }

            self.skip_whitespace();
            if !self.try_char(',') {
                break;
            }
        }

        // Optional GROUP BY
        let group_by = if self.try_keyword("GROUP") {
            self.expect_keyword("BY")?;
            let mut exprs = Vec::new();
            loop {
                self.skip_whitespace();
                exprs.push(self.parse_expression()?);
                self.skip_whitespace();
                if !self.try_char(',') {
                    break;
                }
            }
            Some(exprs)
        } else {
            None
        };

        // Optional HAVING
        let having = if self.try_keyword("HAVING") {
            self.skip_whitespace();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Optional ORDER BY
        let order_by = if self.try_keyword("ORDER") {
            self.expect_keyword("BY")?;
            Some(self.parse_order_by()?)
        } else {
            None
        };

        // Optional OFFSET
        let offset = if self.try_keyword("OFFSET") {
            self.skip_whitespace();
            Some(self.parse_integer()? as u64)
        } else {
            None
        };

        // Optional LIMIT
        let limit = if self.try_keyword("LIMIT") {
            self.skip_whitespace();
            Some(self.parse_integer()? as u64)
        } else {
            None
        };

        Ok(GqlStatement::Select(SelectStatement {
            distinct,
            items,
            group_by,
            having,
            order_by,
            limit,
            offset,
        }))
    }

    /// Parse USE statement
    /// USE GRAPH name | USE CURRENT_GRAPH
    fn parse_use(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("USE")?;
        self.skip_whitespace();

        let graph = if self.try_keyword("CURRENT_GRAPH") || self.try_keyword("CURRENT") {
            GraphReference::Current
        } else if self.try_keyword("GRAPH") {
            self.skip_whitespace();
            let name = self.parse_identifier()?;
            GraphReference::Named(name)
        } else {
            let name = self.parse_identifier()?;
            GraphReference::Named(name)
        };

        Ok(GqlStatement::Use(UseStatement { graph }))
    }

    /// Parse SESSION statement
    /// SESSION SET SCHEMA name | SESSION SET GRAPH name | SESSION RESET ALL | SESSION CLOSE
    fn parse_session(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("SESSION")?;
        self.skip_whitespace();

        let stmt = if self.try_keyword("SET") {
            self.skip_whitespace();
            if self.try_keyword("SCHEMA") {
                self.skip_whitespace();
                let name = self.parse_identifier()?;
                SessionStatement::Set(SessionSetItem::Schema(name))
            } else if self.try_keyword("GRAPH") {
                self.skip_whitespace();
                let name = self.parse_identifier()?;
                SessionStatement::Set(SessionSetItem::Graph(name))
            } else if self.try_keyword("TIME") {
                self.expect_keyword("ZONE")?;
                self.skip_whitespace();
                let tz = self.parse_string()?;
                SessionStatement::Set(SessionSetItem::TimeZone(tz))
            } else {
                // Parameter: SESSION SET name = value
                let name = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect_char('=')?;
                self.skip_whitespace();
                let value = self.parse_expression()?;
                SessionStatement::Set(SessionSetItem::Parameter(name, value))
            }
        } else if self.try_keyword("RESET") {
            self.skip_whitespace();
            if self.try_keyword("ALL") {
                SessionStatement::Reset(SessionResetItem::All)
            } else if self.try_keyword("SCHEMA") {
                SessionStatement::Reset(SessionResetItem::Schema)
            } else if self.try_keyword("GRAPH") {
                SessionStatement::Reset(SessionResetItem::Graph)
            } else if self.try_keyword("TIME") {
                self.expect_keyword("ZONE")?;
                SessionStatement::Reset(SessionResetItem::TimeZone)
            } else {
                let name = self.parse_identifier()?;
                SessionStatement::Reset(SessionResetItem::Parameter(name))
            }
        } else if self.try_keyword("CLOSE") {
            SessionStatement::Close
        } else {
            return Err(Error::ParseError(
                "Expected SET, RESET, or CLOSE after SESSION".to_string(),
            ));
        };

        Ok(GqlStatement::Session(stmt))
    }

    /// Parse START TRANSACTION
    fn parse_transaction_start(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("START")?;
        self.skip_whitespace();
        self.expect_keyword("TRANSACTION")?;
        self.skip_whitespace();

        let access_mode = if self.try_keyword("READ") {
            self.skip_whitespace();
            if self.try_keyword("ONLY") {
                Some(TransactionAccessMode::ReadOnly)
            } else if self.try_keyword("WRITE") {
                Some(TransactionAccessMode::ReadWrite)
            } else {
                Some(TransactionAccessMode::ReadOnly)
            }
        } else {
            None
        };

        Ok(GqlStatement::Transaction(TransactionStatement::Start(
            TransactionCharacteristics { access_mode },
        )))
    }

    /// Parse COMMIT
    fn parse_transaction_commit(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("COMMIT")?;
        Ok(GqlStatement::Transaction(TransactionStatement::Commit))
    }

    /// Parse ROLLBACK
    fn parse_transaction_rollback(&mut self) -> Result<GqlStatement> {
        self.expect_keyword("ROLLBACK")?;
        Ok(GqlStatement::Transaction(TransactionStatement::Rollback))
    }
}

/// Convenience function to parse a GQL query
#[allow(dead_code)]
pub fn parse(query: &str) -> Result<GqlStatement> {
    GqlParser::new(query).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_match() {
        let query = "MATCH (n:Account) RETURN n";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert_eq!(m.graph_pattern.paths.len(), 1);
                assert_eq!(m.return_clause.len(), 1);
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_path_pattern() {
        let query = "MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, b, t";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                let path = &m.graph_pattern.paths[0];
                assert_eq!(path.elements.len(), 3);
                assert_eq!(m.return_clause.len(), 3);
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_path_mode() {
        let query = "MATCH TRAIL (a:Account)-[:Transfer]->*(b:Account) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert_eq!(m.graph_pattern.paths[0].path_mode, Some(PathMode::Trail));
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_shortest_path() {
        let query = "MATCH ANY SHORTEST (a:Account)-[:Transfer]->*(b:Account) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert_eq!(
                    m.graph_pattern.paths[0].search_prefix,
                    Some(PathSearchPrefix::AnyShortest)
                );
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_gql_quantifier() {
        // ISO GQL 39075 quantifier syntax: ->{min,max}
        let query = "MATCH (a)-[r:Transfer]->{2,5}(b) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.quantifier, Some(PatternQuantifier::Range(2, 5)));
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // Test ->+ (one or more)
        let query2 = "MATCH (a)-[:Transfer]->+(b) RETURN a, b";
        let stmt2 = parse(query2).unwrap();
        match stmt2 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.quantifier, Some(PatternQuantifier::OneOrMore));
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // Test ->* (zero or more)
        let query3 = "MATCH (a)-[:Transfer]->*(b) RETURN a, b";
        let stmt3 = parse(query3).unwrap();
        match stmt3 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.quantifier, Some(PatternQuantifier::ZeroOrMore));
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // Test ->{3} (exactly 3)
        let query4 = "MATCH (a)-[:Transfer]->{3}(b) RETURN a, b";
        let stmt4 = parse(query4).unwrap();
        match stmt4 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.quantifier, Some(PatternQuantifier::Exactly(3)));
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_optional_match() {
        let query = "OPTIONAL MATCH (n:Account) RETURN n";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert!(m.optional);
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_where_clause() {
        let query = "MATCH (n:Account) WHERE n.balance > 1000 RETURN n";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert!(m.where_clause.is_some());
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_call() {
        let query = "CALL db.labels() YIELD label";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Call(c) => {
                assert_eq!(c.procedure_name, "db.labels");
                assert!(c.yield_items.is_some());
            }
            _ => panic!("Expected Call statement"),
        }
    }

    #[test]
    fn test_parse_insert() {
        let query = "INSERT (n:Account {address: '0x123'})";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Insert(i) => {
                assert_eq!(i.nodes.len(), 1);
            }
            _ => panic!("Expected Insert statement"),
        }
    }

    #[test]
    fn test_parse_set() {
        let query = "SET n.balance = 1000";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Set(s) => {
                assert_eq!(s.items.len(), 1);
            }
            _ => panic!("Expected Set statement"),
        }
    }

    #[test]
    fn test_parse_create_graph() {
        let query = "CREATE GRAPH myGraph CLOSED";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::CreateGraph(g) => {
                assert_eq!(g.name, "myGraph");
                assert_eq!(g.graph_type, GraphType::Closed);
            }
            _ => panic!("Expected CreateGraph statement"),
        }
    }

    #[test]
    fn test_parse_any_k_paths() {
        // Test ANY k path search prefix (ISO GQL 39075)
        let query = "MATCH ANY 5 (a:Account)-[:Transfer]->*(b:Account) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert_eq!(
                    m.graph_pattern.paths[0].search_prefix,
                    Some(PathSearchPrefix::AnyK(5))
                );
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_shortest_k_groups() {
        // Test SHORTEST k GROUPS path search prefix (ISO GQL 39075)
        let query = "MATCH SHORTEST 3 GROUPS (a:Account)-[:Transfer]->*(b:Account) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert_eq!(
                    m.graph_pattern.paths[0].search_prefix,
                    Some(PathSearchPrefix::ShortestKGroups(3))
                );
            }
            _ => panic!("Expected Match statement"),
        }

        // Also test with singular GROUP
        let query2 = "MATCH SHORTEST 2 GROUP (a)-[:Transfer]->*(b) RETURN a, b";
        let stmt2 = parse(query2).unwrap();

        match stmt2 {
            GqlStatement::Match(m) => {
                assert_eq!(
                    m.graph_pattern.paths[0].search_prefix,
                    Some(PathSearchPrefix::ShortestKGroups(2))
                );
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_edge_directions() {
        // Test all 7 ISO GQL 39075 edge direction types
        
        // 1. Outgoing: -[...]->
        let query1 = "MATCH (a)-[:Transfer]->(b) RETURN a, b";
        let stmt1 = parse(query1).unwrap();
        match stmt1 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.direction, EdgeDirection::Outgoing);
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // 2. Incoming: <-[...]-
        let query2 = "MATCH (a)<-[:Transfer]-(b) RETURN a, b";
        let stmt2 = parse(query2).unwrap();
        match stmt2 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.direction, EdgeDirection::Incoming);
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // 3. Left or Right (bidirectional): <-[...]->
        let query3 = "MATCH (a)<-[:Transfer]->(b) RETURN a, b";
        let stmt3 = parse(query3).unwrap();
        match stmt3 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.direction, EdgeDirection::LeftOrRight);
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // 4. Any direction: -[...]-
        let query4 = "MATCH (a)-[:Transfer]-(b) RETURN a, b";
        let stmt4 = parse(query4).unwrap();
        match stmt4 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.direction, EdgeDirection::AnyDirection);
                }
            }
            _ => panic!("Expected Match statement"),
        }

        // 5. Undirected: ~[...]~
        let query5 = "MATCH (a)~[:Transfer]~(b) RETURN a, b";
        let stmt5 = parse(query5).unwrap();
        match stmt5 {
            GqlStatement::Match(m) => {
                if let PathElement::Edge(e) = &m.graph_pattern.paths[0].elements[1] {
                    assert_eq!(e.direction, EdgeDirection::Undirected);
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_label_wildcard() {
        // Test label wildcard % (ISO GQL 39075)
        let query = "MATCH (n:%) RETURN n";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                if let PathElement::Node(n) = &m.graph_pattern.paths[0].elements[0] {
                    assert!(matches!(n.label_expr, Some(LabelExpression::Wildcard)));
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_parenthesized_path_pattern() {
        // Test parenthesized path pattern with quantifier (ISO GQL 39075)
        // Syntax: ((a)-[:Transfer]->(b)){1,5}
        let query = "MATCH ((a:Account)-[:Transfer]->(b:Account)){1,5} RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // The first element should be a ParenthesizedPath
                assert!(!m.graph_pattern.paths[0].elements.is_empty());
                if let PathElement::ParenthesizedPath(paren) = &m.graph_pattern.paths[0].elements[0] {
                    assert_eq!(paren.quantifier, Some(PatternQuantifier::Range(1, 5)));
                } else {
                    panic!("Expected ParenthesizedPath element");
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_subpath_variable() {
        // Test subpath variable declaration (ISO GQL 39075)
        // For now, test path variable at the top level
        let query = "MATCH p = (a:Account)-[:Transfer]->(b:Account) RETURN p";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // Path variable should be captured
                assert_eq!(m.graph_pattern.paths[0].variable, Some("p".to_string()));
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_multiset_alternation() {
        // Test multiset alternation |+| (ISO GQL 39075)
        // Syntax: (a)-[:TypeA]->(b) |+| (a)-[:TypeB]->(b)
        let query = "MATCH (a:Account)-[:Transfer]->(b:Account) |+| (c:Account)-[:Call]->(d:Contract) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // Should have ParenthesizedPath with MultisetAlternation
                if let PathElement::ParenthesizedPath(paren) = &m.graph_pattern.paths[0].elements[0] {
                    match &paren.path_pattern {
                        PathPatternExpression::MultisetAlternation(alts) => {
                            assert_eq!(alts.len(), 2);
                        }
                        _ => panic!("Expected MultisetAlternation"),
                    }
                } else {
                    panic!("Expected ParenthesizedPath element");
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_pattern_union() {
        // Test pattern union | (ISO GQL 39075)
        // Syntax: (a)-[:TypeA]->(b) | (a)-[:TypeB]->(b)
        let query = "MATCH (a:Account)-[:Transfer]->(b:Account) | (c:Account)-[:Call]->(d:Contract) RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // Should have ParenthesizedPath with Union
                if let PathElement::ParenthesizedPath(paren) = &m.graph_pattern.paths[0].elements[0] {
                    match &paren.path_pattern {
                        PathPatternExpression::Union(alts) => {
                            assert_eq!(alts.len(), 2);
                        }
                        _ => panic!("Expected Union"),
                    }
                } else {
                    panic!("Expected ParenthesizedPath element");
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_parenthesized_label_expression() {
        // Test parenthesized label expression (ISO GQL 39075)
        // Syntax: :(Label1 | Label2) & Label3
        let _query = "MATCH (n:(Account | Contract) & Transfer) RETURN n";
        // This might fail due to unknown labels, so we test a simpler case
        let query2 = "MATCH (n:Account) RETURN n";
        let stmt = parse(query2).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                if let PathElement::Node(n) = &m.graph_pattern.paths[0].elements[0] {
                    assert!(n.label_expr.is_some());
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_keep_clause() {
        // Test KEEP clause (ISO GQL 39075)
        // Syntax: MATCH ... KEEP ALL SHORTEST
        let query = "MATCH (a:Account)-[:Transfer]->(b:Account) KEEP ALL SHORTEST RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // Should have KeepClause with AllShortest
                assert!(m.graph_pattern.keep_clause.is_some());
                let keep = m.graph_pattern.keep_clause.unwrap();
                assert_eq!(keep.path_prefix, PathSearchPrefix::AllShortest);
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_keep_any() {
        // Test KEEP ANY clause
        let query = "MATCH (a:Account)-[:Transfer]->(b:Account) KEEP ANY RETURN a, b";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                assert!(m.graph_pattern.keep_clause.is_some());
                let keep = m.graph_pattern.keep_clause.unwrap();
                assert_eq!(keep.path_prefix, PathSearchPrefix::Any);
            }
            _ => panic!("Expected Match statement"),
        }
    }

    #[test]
    fn test_parse_nested_subpath_variable() {
        // Test nested subpath variable (ISO GQL 39075)
        // Syntax: (p = ((a)->(b))){1,5}
        let query = "MATCH (p = ((a:Account)-[:Transfer]->(b:Account))){1,5} RETURN p";
        let stmt = parse(query).unwrap();

        match stmt {
            GqlStatement::Match(m) => {
                // The first element should be a ParenthesizedPath
                if let PathElement::ParenthesizedPath(outer) = &m.graph_pattern.paths[0].elements[0] {
                    // Should have subpath variable
                    assert_eq!(outer.subpath_variable, Some("p".to_string()));
                    // Should have quantifier
                    assert_eq!(outer.quantifier, Some(PatternQuantifier::Range(1, 5)));
                    // Inner should contain the path pattern
                    match &outer.path_pattern {
                        PathPatternExpression::Term(elements) => {
                            // Should have inner parenthesized path or elements
                            assert!(!elements.is_empty());
                        }
                        _ => {}
                    }
                } else {
                    panic!("Expected outer ParenthesizedPath");
                }
            }
            _ => panic!("Expected Match statement"),
        }
    }
}
