//! ChainGraph GQL 语法测试
//! 测试 ISO GQL 39075 标准语法支持

use chaingraph::query::GqlParser;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║         ChainGraph GQL 语法测试 (ISO GQL 39075)               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // 基础 MATCH 查询
    test_category(
        "基础 MATCH 查询",
        vec![
            "MATCH (n:Account) RETURN n LIMIT 10",
            "MATCH (a)-[t:Transfer]->(b) RETURN a, b",
            "MATCH (a:Account)-[t:Transfer]->(b:Account) WHERE a.balance > 100 RETURN a, b",
            "OPTIONAL MATCH (n:Account) RETURN n",
        ],
    );

    // 量化路径模式 (ISO GQL Quantified Path)
    test_category(
        "量化路径模式",
        vec![
            "MATCH (v)-[e]->{1,2}(v2) RETURN v2",
            "MATCH (v)-[e]->{1,5}(v2) RETURN v2",
            "MATCH (v)-[e]->+(v2) RETURN v2",
            "MATCH (v)-[e]->*(v2) RETURN v2",
            "MATCH (v)-[e*1..2]->(v2) RETURN v2",
            "MATCH (v)-[e:Transfer*1..5]->(v2) RETURN v2",
        ],
    );

    // 路径模式前缀
    test_category(
        "路径搜索前缀",
        vec![
            "MATCH ANY SHORTEST (a)-[*]->(b) RETURN a, b",
            "MATCH ALL (a)-[*1..3]->(b) RETURN a, b",
        ],
    );

    // INSERT/DELETE/SET 语句
    test_category(
        "DML 语句",
        vec![
            "INSERT (a:Account {address: \"0x123\"})",
            "INSERT (a:Account)-[t:Transfer {amount: 100}]->(b:Account)",
            "SET n.balance = 1000",
            "REMOVE n.temp_flag",
            "DELETE n",
        ],
    );

    // CALL 过程调用
    test_category(
        "CALL 过程调用",
        vec![
            "CALL shortest_path(1, 5)",
            "CALL all_paths(1, 5, 10)",
            "CALL trace(1, 'forward', 5)",
            "CALL max_flow(1, 100)",
            "CALL neighbors(1, 'out')",
        ],
    );

    // CREATE/DROP GRAPH
    test_category(
        "Graph DDL",
        vec!["CREATE GRAPH myGraph", "DROP GRAPH IF EXISTS myGraph"],
    );

    // LET 语句
    test_category(
        "LET 语句 (变量绑定)",
        vec![
            "LET x = 10",
            "LET x = 10, y = 20",
            "LET name = \"Alice\", age = 30",
        ],
    );

    // FOR 语句
    test_category(
        "FOR 语句 (迭代)",
        vec!["FOR x IN [1, 2, 3]", "FOR item, idx IN [10, 20, 30]"],
    );

    // FILTER 语句
    test_category(
        "FILTER 语句",
        vec!["FILTER n.age > 18", "FILTER x > 0 AND x < 100"],
    );

    // SELECT 语句
    test_category(
        "SELECT 语句 (SQL风格)",
        vec![
            "SELECT *",
            "SELECT n.name, n.age",
            "SELECT n.name AS username, n.age AS user_age",
            "SELECT COUNT(n) GROUP BY n.type",
            "SELECT n.type, SUM(n.amount) GROUP BY n.type HAVING SUM(n.amount) > 1000",
            "SELECT * ORDER BY n.name LIMIT 10 OFFSET 5",
            "SELECT DISTINCT n.type",
        ],
    );

    // USE 语句
    test_category(
        "USE 语句",
        vec!["USE GRAPH myGraph", "USE myGraph", "USE CURRENT_GRAPH"],
    );

    // SESSION 语句
    test_category(
        "SESSION 语句",
        vec![
            "SESSION SET SCHEMA mySchema",
            "SESSION SET GRAPH myGraph",
            "SESSION SET TIME ZONE 'UTC'",
            "SESSION RESET ALL",
            "SESSION RESET SCHEMA",
            "SESSION CLOSE",
        ],
    );

    // TRANSACTION 语句
    test_category(
        "TRANSACTION 语句",
        vec![
            "START TRANSACTION",
            "START TRANSACTION READ ONLY",
            "START TRANSACTION READ WRITE",
            "COMMIT",
            "ROLLBACK",
        ],
    );

    // SHOW 语句
    test_category(
        "SHOW 语句",
        vec![
            "SHOW GRAPHS",
            "SHOW GRAPH TYPES",
            "SHOW SCHEMAS",
            "SHOW LABELS",
            "SHOW EDGE TYPES",
            "SHOW RELATIONSHIP TYPES",
            "SHOW PROPERTY KEYS",
            "SHOW FUNCTIONS",
            "SHOW PROCEDURES",
            "SHOW INDEXES",
            "SHOW CONSTRAINTS",
        ],
    );

    // DESCRIBE 语句
    test_category(
        "DESCRIBE 语句",
        vec![
            "DESCRIBE GRAPH myGraph",
            "DESC GRAPH myGraph",
            "DESCRIBE GRAPH TYPE myGraphType",
            "DESC GRAPH TYPE myType",
            "DESCRIBE SCHEMA public",
            "DESCRIBE LABEL Account",
            "DESCRIBE EDGE TYPE Transfer",
        ],
    );

    // CREATE/DROP GRAPH TYPE
    test_category(
        "GRAPH TYPE DDL",
        vec![
            "CREATE GRAPH TYPE myType",
            "CREATE GRAPH TYPE IF NOT EXISTS myType",
            "CREATE GRAPH TYPE myType AS (node:Node)",
            "DROP GRAPH TYPE myType",
            "DROP GRAPH TYPE IF EXISTS myType",
        ],
    );

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("测试完成!");
}

fn test_category(name: &str, queries: Vec<&str>) {
    println!("── {} ──", name);
    let mut passed = 0;
    let mut failed = 0;

    for query in &queries {
        match GqlParser::new(query).parse() {
            Ok(_stmt) => {
                println!("  ✓ {}", query);
                passed += 1;
            }
            Err(e) => {
                println!("  ✗ {} -- 错误: {}", query, e);
                failed += 1;
            }
        }
    }

    println!("  结果: {}/{} 通过\n", passed, queries.len());
}
