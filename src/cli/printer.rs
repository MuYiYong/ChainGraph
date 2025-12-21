//! 结果打印器
//!
//! 提供表格和垂直格式的结果输出

use prettytable::{format, row, Cell, Row, Table};

/// 打印模式
#[derive(Clone, Copy, PartialEq)]
pub enum PrintMode {
    /// 表格模式
    Table,
    /// 垂直模式 (\G)
    Vertical,
}

/// 结果打印器
pub struct Printer {
    mode: PrintMode,
}

impl Default for Printer {
    fn default() -> Self {
        Self::new(PrintMode::Table)
    }
}

impl Printer {
    pub fn new(mode: PrintMode) -> Self {
        Self { mode }
    }

    /// 设置打印模式
    pub fn set_mode(&mut self, mode: PrintMode) {
        self.mode = mode;
    }

    /// 打印查询结果
    pub fn print_result(
        &self,
        columns: &[String],
        rows: &[Vec<String>],
        execution_time_ms: u64,
    ) -> String {
        if columns.is_empty() || rows.is_empty() {
            return format!("Empty set ({} ms)\n", execution_time_ms);
        }

        let output = match self.mode {
            PrintMode::Table => self.format_table(columns, rows),
            PrintMode::Vertical => self.format_vertical(columns, rows),
        };

        format!(
            "{}\n{} row(s) in set ({} ms)\n",
            output,
            rows.len(),
            execution_time_ms
        )
    }

    /// 表格格式
    fn format_table(&self, columns: &[String], rows: &[Vec<String>]) -> String {
        let mut table = Table::new();
        
        // 设置表格格式
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        // 添加表头
        let header: Vec<Cell> = columns.iter().map(|c| Cell::new(c)).collect();
        table.set_titles(Row::new(header));

        // 添加数据行
        for row_data in rows {
            let cells: Vec<Cell> = row_data.iter().map(|v| Cell::new(v)).collect();
            table.add_row(Row::new(cells));
        }

        table.to_string()
    }

    /// 垂直格式
    fn format_vertical(&self, columns: &[String], rows: &[Vec<String>]) -> String {
        let max_col_width = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        let mut output = String::new();

        for (i, row_data) in rows.iter().enumerate() {
            output.push_str(&format!(
                "*************************** {}. row ***************************\n",
                i + 1
            ));
            
            for (j, col) in columns.iter().enumerate() {
                let value = row_data.get(j).map(|s| s.as_str()).unwrap_or("");
                output.push_str(&format!("{:>width$}: {}\n", col, value, width = max_col_width));
            }
        }

        output
    }

    /// 打印统计信息
    pub fn print_stats(&self, vertex_count: usize, edge_count: usize) -> String {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);
        table.set_titles(row!["Property", "Value"]);
        table.add_row(row!["Vertex Count", vertex_count.to_string()]);
        table.add_row(row!["Edge Count", edge_count.to_string()]);
        table.to_string()
    }

    /// 打印帮助信息
    pub fn print_help() -> String {
        r#"
═══════════════════════════════════════════════════════════════
                   ChainGraph CLI 命令帮助
═══════════════════════════════════════════════════════════════

基础命令:
  help, h, ?           显示帮助
  quit, exit, q        退出程序
  stats, info          显示图统计信息
  
  query, gql <GQL>     执行 GQL 查询
                       示例: query MATCH (n:Account) RETURN n LIMIT 10
  
  vertex, v <ID|地址>  查看顶点详情
                       示例: vertex 1
                       示例: vertex 0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0
  
  neighbors, n <ID>    查看顶点邻居
                       示例: neighbors 1
  
  path <起点> <终点>   查找最短路径
                       示例: path 1 5
  
  trace <起点> [方向] [深度]
                       追踪路径
                       方向: forward(默认), backward, both
                       示例: trace 1 forward 5
  
  maxflow <源点> <汇点>
                       计算最大流
                       示例: maxflow 1 5

───────────────────────────────────────────────────────────────
GQL 语句 (ISO GQL 39075 标准)
───────────────────────────────────────────────────────────────

查询语句:
  MATCH (n:Account) RETURN n
  MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, b, t
  MATCH (n) WHERE n.balance > 1000 RETURN n LIMIT 10
  OPTIONAL MATCH (n:Account) RETURN n

DML 语句:
  INSERT (n:Account {address: '0x123...', balance: 1000})
  INSERT (a)-[:Transfer {amount: 100}]->(b)
  DELETE n WHERE n.id = 1
  SET n.balance = 2000
  REMOVE n.temp_flag

DDL 语句:
  CREATE GRAPH myGraph
  CREATE GRAPH myGraph { NODE Account, EDGE Transfer }
  DROP GRAPH IF EXISTS myGraph

元数据查询:
  SHOW GRAPHS                  -- 列出所有图
  SHOW LABELS                  -- 列出所有顶点标签
  SHOW EDGE TYPES              -- 列出所有边类型
  SHOW PROCEDURES              -- 列出所有过程
  SHOW FUNCTIONS               -- 列出所有函数
  SHOW INDEXES                 -- 列出所有索引

  DESCRIBE GRAPH myGraph       -- 查看图详情
  DESCRIBE LABEL Account       -- 查看顶点标签详情
  DESCRIBE EDGE TYPE Transfer  -- 查看边类型详情

过程调用:
  CALL shortest_path(1, 5)
  CALL all_paths(1, 5, 10)
  CALL trace(1, 'forward', 5)
  CALL max_flow(1, 5)
  CALL neighbors(1, 'both')
  CALL degree(1)

会话管理:
  SESSION SET SCHEMA mySchema
  SESSION SET GRAPH myGraph
  SESSION RESET ALL
  SESSION CLOSE

事务控制:
  START TRANSACTION READ WRITE
  START TRANSACTION READ ONLY
  COMMIT
  ROLLBACK

图切换:
  USE GRAPH ethereum_mainnet

═══════════════════════════════════════════════════════════════
控制台命令 (以 : 开头)
═══════════════════════════════════════════════════════════════

  :help, :h              显示控制台命令帮助
  :quit, :q              退出程序
  :sleep N               暂停 N 秒
  :tee [-o] <file>       输出到文件 (-o 覆盖)
  :notee                 停止输出到文件
  :pager <cmd> <limit>   设置分页器 (例: :pager less 100)
  :nopager               禁用分页器
  :timeout <seconds>     设置查询超时
  :clear                 清屏

提示: 在查询末尾加 \G 可垂直显示结果

═══════════════════════════════════════════════════════════════
"#.to_string()
    }
}

/// 检查查询是否以 \G 结尾（垂直显示）
pub fn check_vertical_display(query: &str) -> (String, bool) {
    let trimmed = query.trim();
    if trimmed.ends_with("\\G") || trimmed.ends_with("\\g") {
        let clean_query = trimmed[..trimmed.len() - 2].trim().to_string();
        (clean_query, true)
    } else {
        (trimmed.to_string(), false)
    }
}
