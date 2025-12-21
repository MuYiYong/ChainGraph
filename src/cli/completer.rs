//! GQL 关键字补全器
//!
//! 基于 rustyline 实现 Tab 补全功能

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

/// GQL 关键字列表
const GQL_KEYWORDS: &[&str] = &[
    // 查询关键字
    "MATCH", "OPTIONAL", "RETURN", "WHERE", "WITH", "UNWIND", "ORDER", "BY",
    "SKIP", "LIMIT", "DISTINCT", "AS", "AND", "OR", "NOT", "XOR", "IN", "IS",
    "NULL", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END",
    // DML
    "INSERT", "DELETE", "SET", "REMOVE", "MERGE", "CREATE", "DROP",
    // DDL
    "GRAPH", "NODE", "EDGE", "TYPE", "LABEL", "INDEX", "CONSTRAINT",
    // 元数据
    "SHOW", "DESCRIBE", "DESC", "GRAPHS", "LABELS", "TYPES", "SCHEMAS",
    "INDEXES", "CONSTRAINTS", "PROCEDURES", "FUNCTIONS",
    // 过程调用
    "CALL", "YIELD",
    // 会话和事务
    "USE", "SESSION", "START", "TRANSACTION", "COMMIT", "ROLLBACK",
    "READ", "WRITE", "ONLY",
    // 复合查询
    "UNION", "ALL", "EXCEPT", "INTERSECT", "OTHERWISE",
    // 控制流
    "LET", "FOR", "FILTER",
    // 聚合函数
    "COUNT", "SUM", "AVG", "MIN", "MAX", "COLLECT",
    // 路径
    "PATH", "SHORTEST", "ALL_PATHS",
    // 其他
    "IF", "EXISTS", "GROUP",
];

/// 子命令映射
fn get_sub_commands(keyword: &str) -> Option<&'static [&'static str]> {
    match keyword {
        "SHOW" => Some(&[
            "GRAPHS", "LABELS", "EDGE", "TYPES", "SCHEMAS", "INDEXES",
            "CONSTRAINTS", "PROCEDURES", "FUNCTIONS",
        ]),
        "DESCRIBE" | "DESC" => Some(&["GRAPH", "LABEL", "EDGE", "TYPE", "INDEX"]),
        "CREATE" => Some(&["GRAPH", "INDEX", "CONSTRAINT"]),
        "DROP" => Some(&["GRAPH", "INDEX", "CONSTRAINT", "IF"]),
        "IF" => Some(&["EXISTS", "NOT"]),
        "ORDER" => Some(&["BY"]),
        "GROUP" => Some(&["BY"]),
        "START" => Some(&["TRANSACTION"]),
        "SESSION" => Some(&["SET", "RESET", "CLOSE"]),
        "EDGE" => Some(&["TYPE", "TYPES"]),
        _ => None,
    }
}

/// 控制台命令列表
const CONSOLE_COMMANDS: &[&str] = &[
    ":help", ":h",
    ":quit", ":q",
    ":exit", ":e",
    ":sleep",
    ":tee",
    ":notee",
    ":pager",
    ":nopager",
    ":timeout",
    ":stats",
    ":clear",
];

/// ChainGraph CLI 补全器
#[derive(Default)]
pub struct GqlCompleter;

impl GqlCompleter {
    pub fn new() -> Self {
        Self
    }
}

impl Completer for GqlCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_cursor = &line[..pos];
        
        // 检查是否是控制台命令
        if line_to_cursor.starts_with(':') {
            let completions: Vec<Pair> = CONSOLE_COMMANDS
                .iter()
                .filter(|cmd| cmd.starts_with(line_to_cursor))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            return Ok((0, completions));
        }

        // 分割成单词
        let words: Vec<&str> = line_to_cursor.split_whitespace().collect();
        
        if words.is_empty() {
            return Ok((0, vec![]));
        }

        // 检查光标是否在单词末尾
        let at_word_end = !line_to_cursor.ends_with(' ');
        
        if at_word_end {
            // 补全当前正在输入的单词
            let current_word = words.last().unwrap().to_uppercase();
            let start_pos = pos - current_word.len();

            // 检查前一个单词是否有子命令
            if words.len() > 1 {
                let prev_word = words[words.len() - 2].to_uppercase();
                if let Some(sub_cmds) = get_sub_commands(&prev_word) {
                    let completions: Vec<Pair> = sub_cmds
                        .iter()
                        .filter(|kw| kw.starts_with(&current_word))
                        .map(|kw| Pair {
                            display: kw.to_string(),
                            replacement: kw.to_string(),
                        })
                        .collect();
                    if !completions.is_empty() {
                        return Ok((start_pos, completions));
                    }
                }
            }

            // 普通关键字补全
            let completions: Vec<Pair> = GQL_KEYWORDS
                .iter()
                .filter(|kw| kw.starts_with(&current_word))
                .map(|kw| Pair {
                    display: kw.to_string(),
                    replacement: kw.to_string(),
                })
                .collect();
            
            Ok((start_pos, completions))
        } else {
            // 在空格后，提供子命令建议
            let last_word = words.last().unwrap().to_uppercase();
            if let Some(sub_cmds) = get_sub_commands(&last_word) {
                let completions: Vec<Pair> = sub_cmds
                    .iter()
                    .map(|kw| Pair {
                        display: kw.to_string(),
                        replacement: kw.to_string(),
                    })
                    .collect();
                return Ok((pos, completions));
            }
            Ok((pos, vec![]))
        }
    }
}

impl Hinter for GqlCompleter {
    type Hint = String;
}

impl Highlighter for GqlCompleter {}

impl Validator for GqlCompleter {}

impl Helper for GqlCompleter {}
