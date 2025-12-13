//! ChainGraph CLI 工具
//!
//! 交互式命令行界面

use chaingraph::algorithm::{EdmondsKarp, PathFinder, TraceDirection};
use chaingraph::graph::{Graph, VertexId};
use chaingraph::query::{GqlParser, QueryExecutor};
use chaingraph::types::Address;
use clap::Parser;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "chaingraph-cli")]
#[command(about = "ChainGraph 命令行工具")]
struct Args {
    /// 数据目录
    #[arg(short, long, default_value = "./data")]
    data_dir: String,

    /// 缓冲池大小（页面数）
    #[arg(short, long, default_value = "512")]
    buffer_size: usize,

    /// 执行单个查询后退出
    #[arg(short = 'e', long)]
    execute: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("ChainGraph CLI - Web3 区块链链路追踪图数据库");
    println!("=============================================");

    let graph = Graph::open(&args.data_dir, Some(args.buffer_size))?;

    println!("数据库已连接: {}", args.data_dir);
    println!("  顶点数: {}", graph.vertex_count());
    println!("  边数: {}", graph.edge_count());

    // 单个查询模式
    if let Some(query) = args.execute {
        execute_query(&graph, &query)?;
        return Ok(());
    }

    // 交互模式
    println!("\n输入 'help' 查看命令列表，'quit' 退出\n");

    let stdin = io::stdin();
    loop {
        print!("chaingraph> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match handle_command(&graph, line) {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => println!("错误: {}", e),
        }
    }

    println!("再见！");
    Ok(())
}

fn handle_command(graph: &Arc<Graph>, input: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).copied().unwrap_or("");

    match cmd.as_str() {
        "quit" | "exit" | "q" => return Ok(true),

        "help" | "h" | "?" => {
            print_help();
        }

        "stats" | "info" => {
            println!("图统计信息:");
            println!("  顶点数: {}", graph.vertex_count());
            println!("  边数: {}", graph.edge_count());
            println!("  缓冲池大小: {} 页", graph.buffer_pool().pool_size());
            println!("  缓存页面数: {}", graph.buffer_pool().cached_pages());
        }

        "query" | "gql" => {
            if args.is_empty() {
                println!("用法: query <GQL 语句>");
            } else {
                execute_query(graph, args)?;
            }
        }

        "vertex" | "v" => {
            if args.is_empty() {
                println!("用法: vertex <ID 或 地址>");
            } else if let Ok(id) = args.parse::<u64>() {
                show_vertex(graph, VertexId::new(id));
            } else if let Ok(addr) = Address::from_hex(args) {
                if let Some(v) = graph.get_vertex_by_address(&addr) {
                    show_vertex(graph, v.id());
                } else {
                    println!("未找到该地址");
                }
            } else {
                println!("无效的 ID 或地址");
            }
        }

        "neighbors" | "n" => {
            if args.is_empty() {
                println!("用法: neighbors <顶点 ID>");
            } else if let Ok(id) = args.parse::<u64>() {
                let vid = VertexId::new(id);
                println!("出边邻居: {:?}", graph.neighbors(vid));
                println!("入边邻居: {:?}", graph.predecessors(vid));
            }
        }

        "path" | "shortest" => {
            let ids: Vec<&str> = args.split_whitespace().collect();
            if ids.len() < 2 {
                println!("用法: path <起点 ID> <终点 ID>");
            } else if let (Ok(src), Ok(dst)) = (ids[0].parse::<u64>(), ids[1].parse::<u64>()) {
                let finder = PathFinder::new(graph.clone());
                if let Some(path) = finder.shortest_path(VertexId::new(src), VertexId::new(dst)) {
                    println!("最短路径长度: {}", path.length);
                    println!("路径: {:?}", path.vertices);
                    println!("总权重: {}", path.total_weight);
                } else {
                    println!("未找到路径");
                }
            }
        }

        "trace" => {
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.is_empty() {
                println!("用法: trace <起点 ID> [forward|backward|both] [深度]");
            } else if let Ok(start) = parts[0].parse::<u64>() {
                let direction = parts
                    .get(1)
                    .map(|&d| match d {
                        "backward" | "back" | "b" => TraceDirection::Backward,
                        "both" => TraceDirection::Both,
                        _ => TraceDirection::Forward,
                    })
                    .unwrap_or(TraceDirection::Forward);

                let depth = parts.get(2).and_then(|d| d.parse().ok()).unwrap_or(5);

                let finder = PathFinder::new(graph.clone());
                let traces = finder.trace(VertexId::new(start), direction, depth, None);

                println!("找到 {} 条路径:", traces.len());
                for (i, trace) in traces.iter().take(10).enumerate() {
                    println!(
                        "  {}: {:?} (权重: {})",
                        i + 1,
                        trace.vertices,
                        trace.total_weight
                    );
                }
                if traces.len() > 10 {
                    println!("  ... 还有 {} 条路径", traces.len() - 10);
                }
            }
        }

        "maxflow" | "flow" => {
            let ids: Vec<&str> = args.split_whitespace().collect();
            if ids.len() < 2 {
                println!("用法: maxflow <源点 ID> <汇点 ID>");
            } else if let (Ok(src), Ok(sink)) = (ids[0].parse::<u64>(), ids[1].parse::<u64>()) {
                let algo = EdmondsKarp::new(graph.clone());
                let result = algo.max_flow(VertexId::new(src), VertexId::new(sink));
                println!("最大流: {}", result.value);
                println!("流量分配:");
                for ((u, v), flow) in result.flow.iter().take(10) {
                    println!("  {:?} -> {:?}: {}", u, v, flow);
                }
            }
        }

        "call" => {
            // 通过 GQL 解析器处理 CALL 语句
            let gql = format!("CALL {}", args);
            execute_query(graph, &gql)?;
        }

        _ => {
            // 尝试作为 GQL 查询执行
            let upper = input.to_uppercase();
            if upper.starts_with("MATCH")
                || upper.starts_with("INSERT")
                || upper.starts_with("DELETE")
                || upper.starts_with("CALL")
                || upper.starts_with("OPTIONAL")
                || upper.starts_with("SET")
                || upper.starts_with("REMOVE")
                || upper.starts_with("CREATE")
                || upper.starts_with("DROP")
                || upper.starts_with("SHOW")
                || upper.starts_with("DESCRIBE")
                || upper.starts_with("DESC")
                || upper.starts_with("LET")
                || upper.starts_with("FOR")
                || upper.starts_with("FILTER")
                || upper.starts_with("SELECT")
                || upper.starts_with("USE")
                || upper.starts_with("SESSION")
                || upper.starts_with("START")
                || upper.starts_with("COMMIT")
                || upper.starts_with("ROLLBACK")
            {
                execute_query(graph, input)?;
            } else {
                println!("未知命令: {}。输入 'help' 查看帮助。", cmd);
            }
        }
    }

    Ok(false)
}

fn execute_query(graph: &Arc<Graph>, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stmt = GqlParser::new(query).parse()?;
    let executor = QueryExecutor::new(graph.clone());
    let result = executor.execute(&stmt)?;

    // 打印结果
    if !result.columns.is_empty() {
        println!("\n{}", result.columns.join(" | "));
        println!("{}", "-".repeat(result.columns.len() * 15));

        for row in &result.rows {
            let values: Vec<String> = row.iter().map(|v| format!("{:?}", v)).collect();
            println!("{}", values.join(" | "));
        }

        println!(
            "\n{} 行结果 (耗时 {} ms)",
            result.rows.len(),
            result.stats.execution_time_ms
        );
    }

    Ok(())
}

fn show_vertex(graph: &Arc<Graph>, id: VertexId) {
    if let Some(v) = graph.get_vertex(id) {
        println!("顶点 {:?}:", id);
        println!("  标签: {:?}", v.label());
        println!("  属性:");
        for (k, v) in v.properties() {
            println!("    {}: {:?}", k, v);
        }

        println!("  出边: {} 条", graph.out_degree(id));
        println!("  入边: {} 条", graph.in_degree(id));
    } else {
        println!("顶点不存在");
    }
}

fn print_help() {
    println!(
        "
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
  INSERT (n:Account {{address: '0x123...', balance: 1000}})
  INSERT (a)-[:Transfer {{amount: 100}}]->(b)
  DELETE n WHERE n.id = 1
  SET n.balance = 2000
  REMOVE n.temp_flag

DDL 语句:
  CREATE GRAPH myGraph
  CREATE GRAPH myGraph {{ NODE Account, EDGE Transfer }}
  DROP GRAPH IF EXISTS myGraph

元数据查询:
  SHOW GRAPHS                  -- 列出所有图
  SHOW SCHEMAS                 -- 列出所有模式
  SHOW LABELS                  -- 列出所有顶点标签
  SHOW EDGE TYPES              -- 列出所有边类型
  SHOW PROCEDURES              -- 列出所有过程
  SHOW FUNCTIONS               -- 列出所有函数
  SHOW INDEXES                 -- 列出所有索引
  SHOW CONSTRAINTS             -- 列出所有约束

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
  CALL connected(1, 5)
  OPTIONAL CALL shortest_path(1, 999)

变量与控制流:
  LET x = 10, name = 'Alice'
  FOR i IN range(1, 10)
  FILTER n.age > 18

SELECT 查询:
  SELECT n.name, COUNT(*) GROUP BY n.type
  SELECT DISTINCT n.category
  SELECT n.name ORDER BY n.created_at DESC LIMIT 10

复合查询:
  ... UNION ALL ...
  ... EXCEPT ...
  ... INTERSECT ...
  ... OTHERWISE ...

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
"
    );
}
