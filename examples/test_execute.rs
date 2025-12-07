//! ChainGraph GQL 执行测试
//! 测试 ISO GQL 39075 标准语句的执行

use chaingraph::graph::Graph;
use chaingraph::query::{GqlParser, QueryExecutor};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║         ChainGraph GQL 执行测试 (ISO GQL 39075)               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // 创建内存图 (in_memory() 返回 Arc<Graph>)
    let graph = Graph::in_memory().expect("Failed to create graph");
    let executor = QueryExecutor::new(graph);

    // 首先插入一些测试数据
    println!("── 准备测试数据 ──");
    execute_and_show(
        &executor,
        "INSERT (a:Account {address: \"0xAAA\", balance: 1000})",
    );
    execute_and_show(
        &executor,
        "INSERT (b:Account {address: \"0xBBB\", balance: 2000})",
    );
    execute_and_show(
        &executor,
        "INSERT (c:Account {address: \"0xCCC\", balance: 500})",
    );
    println!();

    // LET 语句测试
    println!("── LET 语句测试 ──");
    execute_and_show(&executor, "LET x = 10");
    execute_and_show(&executor, "LET x = 10, y = 20, name = \"test\"");
    println!();

    // FOR 语句测试
    println!("── FOR 语句测试 ──");
    execute_and_show(&executor, "FOR i IN [1, 2, 3, 4, 5]");
    execute_and_show(&executor, "FOR item, idx IN [10, 20, 30]");
    println!();

    // FILTER 语句测试
    println!("── FILTER 语句测试 ──");
    execute_and_show(&executor, "FILTER 10 > 5");
    execute_and_show(&executor, "FILTER 3 < 1");
    println!();

    // SELECT 语句测试
    println!("── SELECT 语句测试 ──");
    execute_and_show(&executor, "SELECT * LIMIT 5");
    execute_and_show(&executor, "SELECT DISTINCT * LIMIT 3");
    println!();

    // USE 语句测试
    println!("── USE 语句测试 ──");
    execute_and_show(&executor, "USE GRAPH myGraph");
    execute_and_show(&executor, "USE CURRENT_GRAPH");
    println!();

    // SESSION 语句测试
    println!("── SESSION 语句测试 ──");
    execute_and_show(&executor, "SESSION SET SCHEMA mySchema");
    execute_and_show(&executor, "SESSION SET GRAPH testGraph");
    execute_and_show(&executor, "SESSION RESET ALL");
    execute_and_show(&executor, "SESSION CLOSE");
    println!();

    // TRANSACTION 语句测试
    println!("── TRANSACTION 语句测试 ──");
    execute_and_show(&executor, "START TRANSACTION");
    execute_and_show(&executor, "START TRANSACTION READ ONLY");
    execute_and_show(&executor, "COMMIT");
    execute_and_show(&executor, "ROLLBACK");
    println!();

    // CREATE/DROP GRAPH 测试
    println!("── Graph DDL 测试 ──");
    execute_and_show(&executor, "CREATE GRAPH testGraph");
    execute_and_show(&executor, "DROP GRAPH IF EXISTS testGraph");
    println!();

    // SHOW 语句测试
    println!("── SHOW 语句测试 ──");
    execute_and_show(&executor, "SHOW GRAPHS");
    execute_and_show(&executor, "SHOW GRAPH TYPES");
    execute_and_show(&executor, "SHOW LABELS");
    execute_and_show(&executor, "SHOW EDGE TYPES");
    execute_and_show(&executor, "SHOW PROCEDURES");
    println!();

    // DESCRIBE 语句测试
    println!("── DESCRIBE 语句测试 ──");
    execute_and_show(&executor, "DESCRIBE GRAPH default");
    execute_and_show(&executor, "DESC GRAPH TYPE default_type");
    execute_and_show(&executor, "DESCRIBE LABEL Account");
    execute_and_show(&executor, "DESCRIBE EDGE TYPE Transfer");
    println!();

    // CREATE/DROP GRAPH TYPE 测试
    println!("── GRAPH TYPE DDL 测试 ──");
    execute_and_show(&executor, "CREATE GRAPH TYPE testType");
    execute_and_show(
        &executor,
        "CREATE GRAPH TYPE IF NOT EXISTS testType AS (n:Node)",
    );
    execute_and_show(&executor, "DROP GRAPH TYPE IF EXISTS testType");
    println!();

    // MATCH 查询测试
    println!("── MATCH 查询测试 ──");
    execute_and_show(&executor, "MATCH (n:Account) RETURN n LIMIT 5");
    println!();

    // CALL 过程调用测试
    println!("── CALL 过程调用测试 ──");
    execute_and_show(&executor, "CALL degree(1)");
    execute_and_show(&executor, "CALL neighbors(1, 'both')");
    println!();

    println!("═══════════════════════════════════════════════════════════════");
    println!("执行测试完成!");
}

fn execute_and_show(executor: &QueryExecutor, query: &str) {
    print!("GQL> {} ... ", query);

    match GqlParser::new(query).parse() {
        Ok(stmt) => match executor.execute(&stmt) {
            Ok(result) => {
                println!("✓");
                if !result.rows.is_empty() {
                    println!("  列: {:?}", result.columns);
                    for (i, row) in result.rows.iter().enumerate().take(3) {
                        println!("  行{}: {:?}", i + 1, row);
                    }
                    if result.rows.len() > 3 {
                        println!("  ... 共 {} 行", result.rows.len());
                    }
                }
            }
            Err(e) => println!("✗ 执行错误: {}", e),
        },
        Err(e) => println!("✗ 解析错误: {}", e),
    }
}
