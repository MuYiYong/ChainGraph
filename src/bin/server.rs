//! ChainGraph 服务器入口
//!
//! 启动 HTTP API 服务器

use chaingraph::graph::Graph;
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

    // 打开图数据库
    let graph = Graph::open(&args.data_dir, Some(args.buffer_size))?;

    println!("图数据库已加载");
    println!("  顶点数: {}", graph.vertex_count());
    println!("  边数: {}", graph.edge_count());

    // 启动服务器
    let config = ServerConfig {
        host: args.host,
        port: args.port,
    };

    start_server(config, graph).await?;

    Ok(())
}
