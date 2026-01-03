//! ChainGraph 服务器入口
//!
//! 启动 HTTP API 服务器

use chaingraph::graph::GraphCatalog;
use chaingraph::server::{start_server, ServerConfig};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "chaingraph-server")]
#[command(about = "ChainGraph HTTP API 服务器")]
struct Args {
    /// 数据目录
    #[arg(short, long, default_value = "./data")]
    data_dir: String,

    /// 监听地址
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// 监听端口
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// 缓冲池大小（页面数）
    #[arg(short, long, default_value = "1024")]
    buffer_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("ChainGraph - Web3 区块链链路追踪图数据库");
    println!("=========================================");
    println!("数据目录: {}", args.data_dir);
    println!("缓冲池大小: {} 页", args.buffer_size);

    // 打开图目录（多图）
    let catalog = GraphCatalog::open(&args.data_dir, Some(args.buffer_size))?;
    let current = catalog.current_graph();

    println!("图数据库已加载");
    println!("  当前图: default");
    println!("  顶点数: {}", current.vertex_count());
    println!("  边数: {}", current.edge_count());

    // 启动服务器
    let config = ServerConfig {
        host: args.host,
        port: args.port,
    };

    start_server(config, catalog).await?;

    Ok(())
}
