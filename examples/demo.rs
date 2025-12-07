//! ChainGraph 演示脚本
//!
//! 导入样例数据并执行查询

use chaingraph::graph::Graph;
use chaingraph::import::BatchImporter;
use chaingraph::query::{GqlParser, QueryExecutor};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ChainGraph 演示");
    println!("================\n");

    // 创建内存图（避免持久化问题）
    let graph = Graph::in_memory()?;

    // 导入样例数据
    println!("1. 导入样例数据...");
    let importer = BatchImporter::new(graph.clone());
    let stats = importer.import_transfers_csv("examples/sample_transfers.csv")?;

    println!("   顶点导入: {}", stats.vertices_imported);
    println!("   边导入: {}", stats.edges_imported);
    println!(
        "   当前图大小: {} 顶点, {} 边\n",
        graph.vertex_count(),
        graph.edge_count()
    );

    // 创建查询执行器
    let executor = QueryExecutor::new(graph.clone());

    // 执行一些 GQL 查询
    println!("2. 执行 GQL 查询...\n");

    // 查询所有账户
    let query1 = "MATCH (n:Account) RETURN n LIMIT 10";
    println!("查询: {}", query1);
    match GqlParser::new(query1).parse() {
        Ok(stmt) => {
            let result = executor.execute(&stmt)?;
            println!("结果: {} 行\n", result.rows.len());
            for row in &result.rows {
                println!("  {:?}", row);
            }
        }
        Err(e) => println!("解析错误: {}", e),
    }
    println!();

    // 查询转账边
    let query2 = "MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, t, b LIMIT 5";
    println!("查询: {}", query2);
    match GqlParser::new(query2).parse() {
        Ok(stmt) => {
            let result = executor.execute(&stmt)?;
            println!("结果: {} 行\n", result.rows.len());
        }
        Err(e) => println!("解析错误: {}", e),
    }
    println!();

    // 路径查询
    let query3 = "MATCH SHORTEST (a:Account)-[*1..5]->(b:Account) WHERE a <> b RETURN a, b LIMIT 3";
    println!("查询: {}", query3);
    match GqlParser::new(query3).parse() {
        Ok(stmt) => {
            let result = executor.execute(&stmt)?;
            println!("结果: {} 行", result.rows.len());
        }
        Err(e) => println!("解析错误: {}", e),
    }

    println!("\n演示完成!");
    Ok(())
}
