//! ChainGraph CLI 工具
//!
//! 基于 rustyline 的交互式命令行界面，支持：
//! - 命令历史记录和持久化
//! - Tab 补全 GQL 关键字
//! - 控制台命令 (:help, :tee, :pager 等)
//! - 表格和垂直格式输出
//! - 脚本文件执行

use chaingraph::algorithm::{EdmondsKarp, PathFinder, TraceDirection};
use chaingraph::cli::commands::{execute_console_command, is_console_command, CommandResult, ConsoleState};
use chaingraph::cli::completer::GqlCompleter;
use chaingraph::cli::printer::{check_vertical_display, PrintMode, Printer};
use chaingraph::graph::{GraphCatalog, VertexId};
use chaingraph::query::{GqlParser, QueryExecutor};
use clap::Parser;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "chaingraph-cli")]
#[command(about = "ChainGraph 命令行工具 - Web3 区块链链路追踪图数据库")]
#[command(version)]
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

    /// 执行脚本文件
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,

    /// 查询超时（秒）
    #[arg(long)]
    timeout: Option<u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 打印欢迎信息
    println!("{}", "ChainGraph CLI - Web3 区块链链路追踪图数据库".green().bold());
    println!("{}", "=".repeat(50).dimmed());

    // 打开图目录（多图）
    let catalog = GraphCatalog::open(&args.data_dir, Some(args.buffer_size))?;
    let catalog = Arc::new(catalog);
    let graph = catalog.current_graph();

    println!("数据库已连接: {}", args.data_dir.cyan());
    println!("  当前图: {}", catalog.current_graph_name().yellow());
    println!("  顶点数: {}", graph.vertex_count().to_string().yellow());
    println!("  边数: {}", graph.edge_count().to_string().yellow());

    // 初始化控制台状态
    let mut console_state = ConsoleState::new();
    if let Some(timeout) = args.timeout {
        console_state.timeout = Some(timeout);
    }

    // 单个查询模式
    if let Some(query) = args.execute {
        let printer = Printer::default();
        execute_query(&catalog, &query, &printer, &mut console_state)?;
        return Ok(());
    }

    // 脚本文件模式
    if let Some(file_path) = args.file {
        return execute_script(&catalog, &file_path, &mut console_state);
    }

    // 交互模式
    run_interactive(&catalog, &mut console_state)
}

/// 运行交互模式
fn run_interactive(
    catalog: &Arc<GraphCatalog>,
    console_state: &mut ConsoleState,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "输入 'help' 或 ':help' 查看命令列表，':quit' 退出".dimmed());
    println!("{}", "使用 Tab 键自动补全 GQL 关键字".dimmed());
    println!();

    // 获取历史文件路径
    let history_path = dirs::home_dir()
        .map(|p| p.join(".chaingraph_history"))
        .unwrap_or_else(|| PathBuf::from(".chaingraph_history"));

    // 创建 rustyline 编辑器
    let config = Config::builder()
        .auto_add_history(true)
        .build();

    let mut rl: Editor<GqlCompleter, _> = Editor::with_config(config)?;
    rl.set_helper(Some(GqlCompleter::new()));

    // 加载历史记录
    let _ = rl.load_history(&history_path);

    let mut printer = Printer::default();

    loop {
        let graph = catalog.current_graph();
        let prompt = format!("{} ", "chaingraph>".green().bold());
        
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // 处理控制台命令
                if is_console_command(line) {
                    match execute_console_command(line, console_state, &graph) {
                        CommandResult::Continue => {}
                        CommandResult::Exit => {
                            // 退出前保存数据
                            if let Err(e) = graph.flush() {
                                println!("{}: {}", "警告: 保存数据失败".yellow(), e);
                            }
                            println!("{}", "数据已保存，再见！".cyan());
                            break;
                        }
                        CommandResult::Message(msg) => {
                            println!("{}", msg);
                        }
                        CommandResult::Error(err) => {
                            println!("{}: {}", "错误".red().bold(), err);
                        }
                    }
                    continue;
                }

                // 处理普通命令
                match handle_command(catalog, line, &mut printer, console_state) {
                    Ok(true) => {
                        // 退出前保存数据
                        if let Err(e) = graph.flush() {
                            println!("{}: {}", "警告: 保存数据失败".yellow(), e);
                        }
                        println!("{}", "数据已保存，再见！".cyan());
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        println!("{}: {}", "错误".red().bold(), e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".dimmed());
                continue;
            }
            Err(ReadlineError::Eof) => {
                // 退出前保存数据
                if let Err(e) = graph.flush() {
                    println!("{}: {}", "警告: 保存数据失败".yellow(), e);
                }
                println!("{}", "数据已保存，再见！".cyan());
                break;
            }
            Err(err) => {
                println!("{}: {:?}", "错误".red().bold(), err);
                break;
            }
        }
    }

    // 保存历史记录
    let _ = rl.save_history(&history_path);

    Ok(())
}

/// 执行脚本文件
fn execute_script(
    catalog: &Arc<GraphCatalog>,
    file_path: &PathBuf,
    console_state: &mut ConsoleState,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let printer = Printer::default();
    let mut line_num = 0;

    for line in reader.lines() {
        line_num += 1;
        let line = line?;
        let line = line.trim();
        
        // 跳过空行和注释
        if line.is_empty() || line.starts_with("--") || line.starts_with("//") {
            continue;
        }

        println!("{} {}", format!("[{}]", line_num).dimmed(), line.cyan());
        
        if let Err(e) = execute_query(catalog, line, &printer, console_state) {
            println!("{}: {}", "错误".red().bold(), e);
            return Err(e);
        }
    }

    println!("{}", format!("脚本执行完成，共 {} 行", line_num).green());
    Ok(())
}

/// 处理命令
fn handle_command(
    catalog: &Arc<GraphCatalog>,
    input: &str,
    printer: &mut Printer,
    console_state: &mut ConsoleState,
) -> Result<bool, Box<dyn std::error::Error>> {
    let graph = catalog.current_graph();
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).copied().unwrap_or("");

    match cmd.as_str() {
        "quit" | "exit" | "q" => return Ok(true),

        "help" | "h" | "?" => {
            println!("{}", Printer::print_help());
        }

        "stats" | "info" => {
            let output = printer.print_stats(graph.vertex_count(), graph.edge_count());
            println!("{}", output);
            println!("  缓冲池大小: {} 页", graph.buffer_pool().pool_size());
            println!("  缓存页面数: {}", graph.buffer_pool().cached_pages());
        }

        "query" | "gql" => {
            if args.is_empty() {
                println!("用法: query <GQL 语句>");
            } else {
                execute_query(catalog, args, printer, console_state)?;
            }
        }

        "vertex" | "v" => {
            if args.is_empty() {
                println!("用法: vertex <ID 或 地址>");
            } else if let Ok(id) = args.parse::<u64>() {
                show_vertex(catalog, VertexId::new(id));
            } else {
                let addr = args.to_string();
                if let Some(v) = graph.get_vertex_by_address(&addr) {
                    show_vertex(catalog, v.id());
                } else {
                    println!("{}", "未找到该地址".yellow());
                }
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
                    println!("最短路径长度: {}", path.length.to_string().green());
                    println!("路径: {:?}", path.vertices);
                    println!("总权重: {}", path.total_weight);
                } else {
                    println!("{}", "未找到路径".yellow());
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

                println!("找到 {} 条路径:", traces.len().to_string().green());
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
                println!("最大流: {}", result.value.to_string().green());
                println!("流量分配:");
                for ((u, v), flow) in result.flow.iter().take(10) {
                    println!("  {:?} -> {:?}: {}", u, v, flow);
                }
            }
        }

        "call" => {
            let gql = format!("CALL {}", args);
            execute_query(catalog, &gql, printer, console_state)?;
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
                execute_query(catalog, input, printer, console_state)?;
            } else {
                println!(
                    "{}: {}。输入 'help' 查看帮助。",
                    "未知命令".yellow(),
                    cmd
                );
            }
        }
    }

    Ok(false)
}

/// 执行 GQL 查询
fn execute_query(
    catalog: &Arc<GraphCatalog>,
    query: &str,
    _printer: &Printer,
    console_state: &mut ConsoleState,
) -> Result<(), Box<dyn std::error::Error>> {
    // 检查是否需要垂直显示
    let (clean_query, vertical) = check_vertical_display(query);
    
    let local_printer = Printer::new(if vertical {
        PrintMode::Vertical
    } else {
        PrintMode::Table
    });

    let stmt = GqlParser::new(&clean_query).parse()?;
    let executor = QueryExecutor::new(catalog.clone());
    let result = executor.execute(&stmt)?;

    // 格式化输出
    if !result.columns.is_empty() {
        let string_rows: Vec<Vec<String>> = result
            .rows
            .iter()
            .map(|row| row.iter().map(|v| format!("{:?}", v)).collect())
            .collect();

        let output = local_printer.print_result(
            &result.columns,
            &string_rows,
            result.stats.execution_time_ms,
        );

        // 使用分页器或直接输出
        if !console_state.paginate(&output, result.rows.len()) {
            console_state.write_output(&output);
        }
    }

    Ok(())
}

/// 显示顶点详情
fn show_vertex(catalog: &Arc<GraphCatalog>, id: VertexId) {
    let graph = catalog.current_graph();
    if let Some(v) = graph.get_vertex(id) {
        println!("顶点 {}:", format!("{:?}", id).cyan());
        println!("  标签: {}", format!("{:?}", v.label()).yellow());
        println!("  属性:");
        for (k, val) in v.properties() {
            println!("    {}: {:?}", k.green(), val);
        }

        println!("  出边: {} 条", graph.out_degree(id).to_string().yellow());
        println!("  入边: {} 条", graph.in_degree(id).to_string().yellow());
    } else {
        println!("{}", "顶点不存在".yellow());
    }
}
