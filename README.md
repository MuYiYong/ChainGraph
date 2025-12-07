# ChainGraph

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
</p>

**ChainGraph** 是一款专为 Web3 场景设计的高性能图数据库，专注于区块链链路追踪和资金流分析。

## ✨ 特性

- 🚀 **SSD 优化存储** - 4KB 页面对齐，LRU 缓冲池，支持海量数据存储
- 🔗 **Web3 原生类型** - 内置 Address、TxHash、TokenAmount 等区块链类型
- 🔍 **链路追踪算法** - 支持最短路径、所有路径、N跳邻居等多种追踪方式
- 💧 **最大流分析** - Edmonds-Karp 算法，用于资金流动分析和反洗钱检测
- 📝 **ISO GQL 39075 标准** - 完整支持 ISO/IEC 39075 标准的图查询语言
  - ✅ MATCH 查询：模式匹配、量化路径、路径搜索前缀
  - ✅ DML 操作：INSERT、UPDATE、DELETE、DETACH DELETE
  - ✅ DDL 操作：CREATE/DROP GRAPH、CREATE/DROP GRAPH TYPE
  - ✅ 元数据查询：SHOW GRAPHS/LABELS/PROCEDURES、DESCRIBE GRAPH/LABEL
  - ✅ 过程调用：CALL/OPTIONAL CALL
  - ✅ 变量与控制流：LET、FOR、FILTER
  - ✅ SELECT 查询：DISTINCT、GROUP BY、HAVING、ORDER BY、LIMIT、OFFSET
  - ✅ 复合查询：UNION、EXCEPT、INTERSECT、OTHERWISE
  - ✅ 会话管理：SESSION SET/RESET/CLOSE
  - ✅ 事务控制：START TRANSACTION、COMMIT、ROLLBACK
- 📦 **批量数据导入** - 支持 CSV/JSON 格式，多线程并行导入
- 🌐 **REST API** - 完整的 HTTP API 服务

## 🚀 快速开始

### 使用 Docker (推荐)

```bash
# 克隆仓库
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# 构建并启动
docker compose up -d

# 查看日志
docker compose logs -f

# 使用 CLI
docker compose run --rm chaingraph-cli
```

### 使用预构建镜像

```bash
# 拉取镜像
docker pull ghcr.io/muyiyong/chaingraph:latest

# 启动服务
docker run -d \
  --name chaingraph \
  -p 8080:8080 \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest

# 使用 CLI
docker run -it --rm \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-cli -d /data
```

更多 Docker 使用说明请参阅 [DOCKER.md](DOCKER.md)

### 从源码构建 (可选)

```bash
# 克隆仓库
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# 编译
cargo build --release

# 运行测试
cargo test
```

### 启动服务器

```bash
# 使用默认配置启动
./target/release/chaingraph-server

# 指定参数启动
./target/release/chaingraph-server \
    --data-dir ./data \
    --host 0.0.0.0 \
    --port 8080 \
    --buffer-size 1024
```

### 使用 CLI

```bash
# 交互式命令行
./target/release/chaingraph-cli --data-dir ./data

# 执行单个查询
./target/release/chaingraph-cli -e "MATCH (n:Account) RETURN n LIMIT 10"
```

### 导入数据

```bash
# 从 CSV 导入
./target/release/chaingraph-import \
    --input transactions.csv \
    --format csv \
    --data-dir ./data

# 从 JSON 导入
./target/release/chaingraph-import \
    --input transactions.jsonl \
    --format jsonl \
    --parallel

# 使用 GQL INSERT 语句导入
./target/release/chaingraph-cli --data-dir ./data -e \
    'INSERT (a:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})'

# 运行示例数据导入脚本
./examples/import_sample_data.sh ./data
```

## 📖 GQL 查询示例

### 基本查询

```gql
-- 查找所有账户
MATCH (n:Account) RETURN n LIMIT 100

-- 查找指定地址的账户
MATCH (n:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"}) 
RETURN n

-- 查找转账关系
MATCH (a:Account)-[t:Transfer]->(b:Account) 
RETURN a, t, b LIMIT 50
```

### 链路追踪

```gql
-- 查找两个地址之间的转账路径
MATCH path = (a:Account)-[:Transfer*1..5]->(b:Account)
WHERE a.address = "0xAAA..." AND b.address = "0xBBB..."
RETURN path

-- 查找某地址的所有出向转账
MATCH (a:Account)-[t:Transfer]->(b:Account)
WHERE a.address = "0x742d35Cc..."
RETURN b.address, t.amount
```

### 数据写入 (INSERT)

```gql
-- 插入账户顶点
INSERT (alice:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})

-- 插入合约顶点
INSERT (uniswap:Contract {address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"})

-- 插入转账边
INSERT (a:Account {address: "0xAAA..."})-[t:Transfer {amount: 1000, block: 18500000}]->(b:Account {address: "0xBBB..."})
```

### 过程调用 (CALL) - ISO GQL 39075

ChainGraph 支持标准的 GQL CALL 语句来调用图算法和过程：

```gql
-- 最短路径
CALL shortest_path(1, 5)

-- 所有路径（指定最大深度）
CALL all_paths(1, 5, 10)

-- 链路追踪
CALL trace(1, 'forward', 5)       -- 前向追踪
CALL trace(1, 'backward', 5)      -- 后向追踪
CALL trace(1, 'both', 5)          -- 双向追踪

-- 最大流分析
CALL max_flow(1, 100)

-- 邻居查询
CALL neighbors(1, 'out')          -- 出边邻居
CALL neighbors(1, 'in')           -- 入边邻居
CALL neighbors(1, 'both')         -- 所有邻居

-- 度数查询
CALL degree(1)

-- 连通性检测
CALL connected(1, 100)

-- 可选调用（不存在时返回空而非报错）
OPTIONAL CALL shortest_path(1, 999)
```

#### 可用的过程列表

| 过程名 | 参数 | 描述 |
|--------|------|------|
| `shortest_path(source, target)` | source: 起点ID, target: 终点ID | 查找最短路径 |
| `all_paths(source, target, max_depth?)` | source, target, 可选深度限制 | 查找所有路径 |
| `trace(start, direction?, max_depth?)` | start: 起点, direction: forward/backward/both | 链路追踪 |
| `max_flow(source, sink)` | source: 源点, sink: 汇点 | 计算最大流 |
| `neighbors(vertex_id, direction?)` | vertex_id: 顶点ID, direction: in/out/both | 获取邻居 |
| `degree(vertex_id)` | vertex_id: 顶点ID | 获取顶点度数 |
| `connected(source, target)` | source, target: 顶点ID | 检查连通性 |

### SHOW 语句 - 查看数据库对象

查看数据库中的各类对象：

```gql
-- 查看所有图
SHOW GRAPHS

-- 查看所有图类型
SHOW GRAPH TYPES

-- 查看所有模式
SHOW SCHEMAS

-- 查看所有顶点标签
SHOW LABELS

-- 查看所有边类型
SHOW EDGE TYPES
SHOW RELATIONSHIP TYPES

-- 查看所有属性键
SHOW PROPERTY KEYS

-- 查看所有函数
SHOW FUNCTIONS

-- 查看所有过程
SHOW PROCEDURES

-- 查看所有索引
SHOW INDEXES

-- 查看所有约束
SHOW CONSTRAINTS
```

### DESCRIBE 语句 - 查看对象详情

查看数据库对象的详细信息：

```gql
-- 查看图详情
DESCRIBE GRAPH myGraph
DESC GRAPH myGraph

-- 查看图类型详情
DESCRIBE GRAPH TYPE myGraphType
DESC GRAPH TYPE myType

-- 查看模式详情
DESCRIBE SCHEMA public

-- 查看顶点标签详情
DESCRIBE LABEL Account

-- 查看边类型详情
DESCRIBE EDGE TYPE Transfer
```

### 变量绑定 (LET) - ISO GQL 39075

使用 LET 语句声明和绑定变量：

```gql
-- 单个变量绑定
LET x = 10

-- 多个变量绑定
LET x = 10, name = "Alice", active = true

-- 复杂表达式绑定
LET total = 100, tax_rate = 0.08
```

### 迭代语句 (FOR) - ISO GQL 39075

使用 FOR 语句进行列表迭代，支持序数变量：

```gql
-- 基本迭代
FOR x IN [1, 2, 3, 4, 5]

-- 使用 range() 函数
FOR i IN range(1, 10)

-- 带序数变量的迭代
FOR item IN list WITH ORDINALITY AS ord

-- 完整示例
FOR i IN range(1, 100) WITH ORDINALITY AS idx
```

### 过滤语句 (FILTER) - ISO GQL 39075

使用 FILTER 语句进行条件过滤：

```gql
-- 基本过滤
FILTER n.age > 18

-- 复合条件
FILTER n.status = "active" AND n.balance > 1000

-- 使用 OR
FILTER n.type = "Account" OR n.type = "Contract"

-- NOT 条件
FILTER NOT n.deleted
```

### 选择语句 (SELECT) - ISO GQL 39075

支持 SQL 风格的 SELECT 查询，包含分组、排序、聚合等功能：

```gql
-- 基本选择
SELECT n.name, n.age

-- 使用 DISTINCT
SELECT DISTINCT n.type

-- 分组查询
SELECT n.category, COUNT(*) 
GROUP BY n.category

-- 带 HAVING 的分组
SELECT n.type, SUM(n.amount) AS total
GROUP BY n.type
HAVING SUM(n.amount) > 10000

-- 排序和分页
SELECT n.name, n.created_at
ORDER BY n.created_at DESC
LIMIT 10 OFFSET 20
```

### 图上下文切换 (USE) - ISO GQL 39075

使用 USE 语句切换当前图上下文：

```gql
-- 切换到指定图
USE GRAPH ethereum_mainnet

-- 切换到另一个图
USE GRAPH polygon_network
```

### 复合查询 - ISO GQL 39075

支持 UNION、EXCEPT、INTERSECT、OTHERWISE 操作：

```gql
-- 联合查询
MATCH (a:Account) RETURN a
UNION ALL
MATCH (c:Contract) RETURN c

-- 差集
MATCH (a:Account) RETURN a
EXCEPT
MATCH (b:Account {status: "inactive"}) RETURN b

-- 交集
MATCH (a:Account {type: "whale"}) RETURN a
INTERSECT
MATCH (b:Account {active: true}) RETURN b

-- OTHERWISE (回退查询)
MATCH (n:Account {address: "0x..."}) RETURN n
OTHERWISE
MATCH (n:Account) RETURN n LIMIT 1
```

### 会话管理 (SESSION) - ISO GQL 39075

管理查询会话的模式、图和属性：

```gql
-- 设置当前模式
SESSION SET SCHEMA main_schema

-- 设置当前图
SESSION SET GRAPH ethereum

-- 设置属性图
SESSION SET PROPERTY GRAPH financial_graph

-- 设置多个属性
SESSION SET VALUE timeout = 30000

-- 重置会话
SESSION RESET SCHEMA
SESSION RESET GRAPH
SESSION RESET ALL

-- 关闭会话
SESSION CLOSE
```

### 事务控制 (TRANSACTION) - ISO GQL 39075

支持显式事务管理：

```gql
-- 开始读写事务
START TRANSACTION READ WRITE

-- 开始只读事务
START TRANSACTION READ ONLY

-- 提交事务
COMMIT

-- 回滚事务
ROLLBACK
```

### 图类型定义 (CREATE/DROP GRAPH TYPE) - ISO GQL 39075

定义和管理图类型模式：

```gql
-- 创建图类型
CREATE GRAPH TYPE financial_network AS (
  (account:Account {address STRING, balance DECIMAL}),
  (contract:Contract {address STRING, code_hash STRING}),
  (account)-[transfer:Transfer {amount DECIMAL}]->(account)
)

-- 删除图类型
DROP GRAPH TYPE financial_network
```

### 量化路径模式 - ISO GQL 39075

支持路径长度限制和搜索模式：

```gql
-- 可变长度路径
MATCH (a)-[*1..5]->(b) RETURN path

-- 精确长度路径
MATCH (a)-[*3]->(b) RETURN path

-- 最短路径前缀
MATCH SHORTEST (a)-[*]->(b) RETURN path

-- 所有最短路径
MATCH ALL SHORTEST (a)-[*]->(b) RETURN path

-- 任意路径
MATCH ANY (a)-[*1..10]->(b) RETURN path

-- 任意最短路径
MATCH ANY SHORTEST (a)-[*]->(b) RETURN path
```

## 🔌 REST API

### 查询端点

```bash
# 执行 GQL 查询
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10"}'

# 获取顶点
curl http://localhost:8080/vertices/1

# 通过地址获取顶点
curl http://localhost:8080/vertices/address/0x742d35Cc...
```

### 算法端点

```bash
# 最短路径
curl -X POST http://localhost:8080/algorithm/shortest-path \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "target": 100}'

# 所有路径
curl -X POST http://localhost:8080/algorithm/all-paths \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "target": 100, "max_depth": 5}'

# 最大流
curl -X POST http://localhost:8080/algorithm/max-flow \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "sink": 100}'

# 链路追踪
curl -X POST http://localhost:8080/algorithm/trace \
  -H "Content-Type: application/json" \
  -d '{"start": 1, "direction": "forward", "max_depth": 10}'
```

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────┐
│                      REST API Layer                      │
│                   (axum HTTP Server)                     │
├─────────────────────────────────────────────────────────┤
│                     Query Engine                         │
│              ┌─────────┬──────────────┐                  │
│              │ Parser  │   Executor   │                  │
│              │  (GQL)  │              │                  │
│              └─────────┴──────────────┘                  │
├─────────────────────────────────────────────────────────┤
│                   Graph Algorithms                       │
│     ┌──────────────┐        ┌──────────────────┐        │
│     │ Path Tracing │        │ Max Flow (E-K)   │        │
│     └──────────────┘        └──────────────────┘        │
├─────────────────────────────────────────────────────────┤
│                      Graph Core                          │
│   ┌──────────┐  ┌──────────┐  ┌───────────────────┐     │
│   │  Vertex  │  │   Edge   │  │      Index        │     │
│   └──────────┘  └──────────┘  └───────────────────┘     │
├─────────────────────────────────────────────────────────┤
│                    Storage Engine                        │
│   ┌──────────────┐  ┌──────────────┐  ┌────────────┐    │
│   │ Buffer Pool  │  │ Disk Storage │  │    Page    │    │
│   │    (LRU)     │  │   (mmap)     │  │   (4KB)    │    │
│   └──────────────┘  └──────────────┘  └────────────┘    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │     SSD      │
                    └──────────────┘
```

## 📊 数据模型

### 顶点类型 (VertexLabel)

| 类型 | 描述 | 典型属性 |
|------|------|----------|
| `Account` | EOA 账户 | address, balance, nonce |
| `Contract` | 智能合约 | address, code_hash, creator |
| `Token` | 代币 | address, symbol, decimals |
| `Transaction` | 交易 | hash, block_number, gas_used |
| `Block` | 区块 | number, hash, timestamp |

### 边类型 (EdgeLabel)

| 类型 | 描述 | 典型属性 |
|------|------|----------|
| `Transfer` | 代币转账 | amount, token, tx_hash |
| `Call` | 合约调用 | method, gas, tx_hash |
| `Create` | 合约创建 | tx_hash, block_number |
| `Approve` | 授权 | amount, spender |

## ⚙️ 配置

### 服务器配置

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `--data-dir` | `./data` | 数据存储目录 |
| `--host` | `127.0.0.1` | 监听地址 |
| `--port` | `8080` | 监听端口 |
| `--buffer-size` | `1024` | 缓冲池大小（页面数） |

### 性能调优

```bash
# 大规模数据场景
./target/release/chaingraph-server \
    --buffer-size 8192 \  # 32MB 缓冲池
    --data-dir /ssd/chaingraph
```

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test storage::
cargo test algorithm::
cargo test query::

# 运行基准测试
cargo bench
```

## 📄 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 🤝 贡献

欢迎贡献代码！请先阅读 [贡献指南](CONTRIBUTING.md)。

---

<p align="center">
  Made with ❤️ for Web3
</p>
