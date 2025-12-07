//! ChainGraph 数据导入工具
//!
//! 从 CSV 或 JSON 文件批量导入区块链数据

use chaingraph::graph::Graph;
use chaingraph::import::BatchImporter;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "chaingraph-import")]
#[command(about = "ChainGraph 数据导入工具")]
struct Args {
    /// 输入文件路径
    #[arg(short, long)]
    input: PathBuf,

    /// 数据目录
    #[arg(short, long, default_value = "./data")]
    data_dir: String,

    /// 输入格式: csv, jsonl
    #[arg(short, long, default_value = "csv")]
    format: String,

    /// 批次大小
    #[arg(short, long, default_value = "10000")]
    batch_size: usize,

    /// 是否使用并行导入
    #[arg(short, long)]
    parallel: bool,

    /// 缓冲池大小（页面数）
    #[arg(long, default_value = "2048")]
    buffer_size: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("ChainGraph 数据导入工具");
    println!("========================");
    println!("输入文件: {:?}", args.input);
    println!("数据目录: {}", args.data_dir);
    println!("格式: {}", args.format);
    println!("批次大小: {}", args.batch_size);
    println!("并行模式: {}", args.parallel);

    // 打开图数据库
    let graph = Graph::open(&args.data_dir, Some(args.buffer_size))?;

    println!("\n开始导入...");

    let importer = BatchImporter::new(graph.clone()).with_batch_size(args.batch_size);

    let stats = match args.format.as_str() {
        "csv" => {
            if args.parallel {
                importer.import_transfers_csv_parallel(&args.input)?
            } else {
                importer.import_transfers_csv(&args.input)?
            }
        }
        "jsonl" | "json" => importer.import_jsonl(&args.input)?,
        _ => {
            eprintln!("不支持的格式: {}", args.format);
            std::process::exit(1);
        }
    };

    // 刷新到磁盘
    graph.flush()?;

    println!("\n导入完成!");
    println!("  顶点导入: {}", stats.vertices_imported);
    println!("  边导入: {}", stats.edges_imported);
    println!("  错误数: {}", stats.errors);
    println!("  耗时: {} ms", stats.duration_ms);
    println!("\n当前图大小:");
    println!("  顶点数: {}", graph.vertex_count());
    println!("  边数: {}", graph.edge_count());

    Ok(())
}
