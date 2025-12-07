# ChainGraph 产品手册

## 目录

1. [产品概述](#1-产品概述)
2. [系统要求](#2-系统要求)
3. [安装部署](#3-安装部署)
4. [核心概念](#4-核心概念)
5. [数据模型](#5-数据模型)
6. [GQL 查询语言](#6-gql-查询语言)
   - [6.1 语法概述](#61-语法概述)
   - [6.2 MATCH 语句](#62-match-语句)
   - [6.3 WHERE 子句](#63-where-子句)
   - [6.4 RETURN 子句](#64-return-子句)
   - [6.5 ORDER BY 和 LIMIT](#65-order-by-和-limit)
   - [6.6 INSERT 语句](#66-insert-语句)
   - [6.7 DELETE 语句](#67-delete-语句)
   - [6.8 UPDATE 语句](#68-update-语句)
   - [6.9 LET 变量绑定](#69-let-变量绑定-iso-gql-39075)
   - [6.10 FOR 迭代语句](#610-for-迭代语句-iso-gql-39075)
   - [6.11 FILTER 过滤语句](#611-filter-过滤语句-iso-gql-39075)
   - [6.12 SELECT 查询语句](#612-select-查询语句-iso-gql-39075)
   - [6.13 USE 图切换语句](#613-use-图切换语句-iso-gql-39075)
   - [6.14 复合查询](#614-复合查询-iso-gql-39075)
   - [6.15 SESSION 会话管理](#615-session-会话管理-iso-gql-39075)
   - [6.16 TRANSACTION 事务控制](#616-transaction-事务控制-iso-gql-39075)
   - [6.17 CREATE/DROP GRAPH TYPE](#617-createdrop-graph-type-iso-gql-39075)
   - [6.18 量化路径模式](#618-量化路径模式-iso-gql-39075)
   - [6.19 SHOW 语句](#619-show-语句---查看数据库对象)
   - [6.20 DESCRIBE 语句](#620-describe-语句---查看对象详情)
7. [图算法](#7-图算法)
8. [REST API 参考](#8-rest-api-参考)
9. [数据导入](#9-数据导入)
10. [命令行工具](#10-命令行工具)
11. [性能调优](#11-性能调优)
12. [最佳实践](#12-最佳实践)
13. [故障排除](#13-故障排除)
14. [附录](#附录)

---

## 1. 产品概述

### 1.1 什么是 ChainGraph

ChainGraph 是一款专为 Web3 场景设计的高性能图数据库。它专注于区块链数据的存储、查询和分析，特别适用于：

- **链路追踪**：追踪资金流向，发现地址之间的关联关系
- **反洗钱分析**：检测可疑交易模式，识别混币行为
- **智能合约分析**：分析合约调用关系和依赖图
- **DeFi 监控**：监控 DeFi 协议的资金流动

### 1.2 核心优势

| 特性 | 描述 |
|------|------|
| **SSD 优化** | 4KB 页面对齐，最大化 SSD 性能 |
| **海量数据** | 支持存储数十亿顶点和边 |
| **实时分析** | 毫秒级路径查询和图分析 |
| **Web3 原生** | 内置区块链专用数据类型 |
| **标准查询** | 基于 ISO GQL 39075 标准 |

### 1.3 典型应用场景

```
┌─────────────────────────────────────────────────────────────┐
│                    ChainGraph 应用场景                       │
├─────────────────┬─────────────────┬─────────────────────────┤
│   安全合规      │    业务分析     │      风险控制           │
├─────────────────┼─────────────────┼─────────────────────────┤
│ • 反洗钱检测    │ • 用户画像      │ • 欺诈检测              │
│ • 制裁筛查      │ • 交易分析      │ • 异常行为识别          │
│ • 合规报告      │ • 市场监控      │ • 风险评估              │
└─────────────────┴─────────────────┴─────────────────────────┘
```

---

## 2. 系统要求

### 2.1 硬件要求

| 组件 | 最低配置 | 推荐配置 |
|------|----------|----------|
| CPU | 4 核 | 8 核以上 |
| 内存 | 8 GB | 32 GB 以上 |
| 存储 | 100 GB SSD | 1 TB NVMe SSD |
| 网络 | 100 Mbps | 1 Gbps |

### 2.2 软件要求

| 软件 | 版本要求 |
|------|----------|
| 操作系统 | Linux (推荐) / macOS / Windows |
| Rust | 1.70+ |
| 文件系统 | ext4 / xfs / APFS |

### 2.3 端口要求

| 端口 | 用途 |
|------|------|
| 8080 | HTTP API 服务 |

---

## 3. 安装部署

### 3.1 从源码编译

```bash
# 1. 克隆仓库
git clone https://github.com/your-org/chaingraph.git
cd chaingraph

# 2. 编译 Release 版本
cargo build --release

# 3. 可执行文件位于
ls target/release/chaingraph-*
# chaingraph-server  - HTTP 服务器
# chaingraph-cli     - 命令行工具
# chaingraph-import  - 数据导入工具
```

### 3.2 目录结构

```
chaingraph/
├── target/release/
│   ├── chaingraph-server    # 服务器
│   ├── chaingraph-cli       # CLI 工具
│   └── chaingraph-import    # 导入工具
└── data/                    # 数据目录（自动创建）
    ├── chaingraph.data      # 数据文件
    └── chaingraph.meta      # 元数据文件
```

### 3.3 启动服务

```bash
# 基本启动
./chaingraph-server

# 完整参数
./chaingraph-server \
    --data-dir /data/chaingraph \
    --host 0.0.0.0 \
    --port 8080 \
    --buffer-size 2048
```

### 3.4 服务验证

```bash
# 健康检查
curl http://localhost:8080/health

# 预期响应
{"status": "ok", "version": "0.1.0"}
```

---

## 4. 核心概念

### 4.1 图模型

ChainGraph 使用属性图模型（Property Graph Model）：

```
     ┌─────────────────┐          ┌─────────────────┐
     │     Vertex      │          │      Edge       │
     ├─────────────────┤          ├─────────────────┤
     │ • ID            │          │ • ID            │
     │ • Label         │─────────▶│ • Label         │
     │ • Properties    │          │ • Source        │
     └─────────────────┘          │ • Target        │
                                  │ • Properties    │
                                  └─────────────────┘
```

### 4.2 顶点（Vertex）

顶点代表区块链中的实体：

```rust
Vertex {
    id: VertexId,           // 唯一标识
    label: VertexLabel,     // 类型标签
    address: Address,       // 区块链地址
    properties: Properties, // 属性集合
}
```

### 4.3 边（Edge）

边代表实体之间的关系：

```rust
Edge {
    id: EdgeId,             // 唯一标识
    label: EdgeLabel,       // 类型标签
    source: VertexId,       // 起点
    target: VertexId,       // 终点
    properties: Properties, // 属性集合
}
```

### 4.4 属性类型

| 类型 | Rust 类型 | 描述 |
|------|-----------|------|
| `Null` | `()` | 空值 |
| `Bool` | `bool` | 布尔值 |
| `Int` | `i64` | 64位整数 |
| `UInt` | `u64` | 64位无符号整数 |
| `Float` | `f64` | 64位浮点数 |
| `String` | `String` | 字符串 |
| `Address` | `H160` | 以太坊地址 |
| `TxHash` | `H256` | 交易哈希 |
| `Amount` | `U256` | 代币金额 |
| `Timestamp` | `i64` | Unix 时间戳 |
| `Bytes` | `Vec<u8>` | 字节数组 |
| `List` | `Vec<PropertyValue>` | 列表 |
| `Map` | `HashMap<String, PropertyValue>` | 映射 |

---

## 5. 数据模型

### 5.1 顶点类型

#### Account（账户）

表示外部拥有账户（EOA）。

```json
{
  "label": "Account",
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0",
  "properties": {
    "balance": "1000000000000000000",
    "nonce": 42,
    "first_seen_block": 12000000,
    "last_active_block": 15000000,
    "tx_count": 156
  }
}
```

#### Contract（合约）

表示智能合约。

```json
{
  "label": "Contract",
  "address": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
  "properties": {
    "name": "TetherToken",
    "symbol": "USDT",
    "decimals": 6,
    "creator": "0x...",
    "creation_block": 4634748,
    "is_verified": true
  }
}
```

#### Token（代币）

表示 ERC20/ERC721 等代币。

```json
{
  "label": "Token",
  "address": "0xA0b86a33E6dC663CDA4e0A....",
  "properties": {
    "name": "USD Coin",
    "symbol": "USDC",
    "decimals": 6,
    "total_supply": "1000000000000000"
  }
}
```

#### Transaction（交易）

表示链上交易。

```json
{
  "label": "Transaction",
  "properties": {
    "hash": "0x...",
    "block_number": 15000000,
    "from": "0x...",
    "to": "0x...",
    "value": "1000000000000000000",
    "gas_used": 21000,
    "gas_price": "50000000000",
    "status": 1
  }
}
```

#### Block（区块）

表示区块。

```json
{
  "label": "Block",
  "properties": {
    "number": 15000000,
    "hash": "0x...",
    "parent_hash": "0x...",
    "timestamp": 1660000000,
    "miner": "0x...",
    "gas_limit": 30000000,
    "gas_used": 15000000,
    "tx_count": 200
  }
}
```

### 5.2 边类型

#### Transfer（转账）

表示代币转账。

```json
{
  "label": "Transfer",
  "source": 1,
  "target": 2,
  "properties": {
    "amount": "1000000000000000000",
    "token": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
    "tx_hash": "0x...",
    "block_number": 15000000,
    "log_index": 0
  }
}
```

#### Call（调用）

表示合约调用。

```json
{
  "label": "Call",
  "source": 1,
  "target": 2,
  "properties": {
    "method": "transfer(address,uint256)",
    "tx_hash": "0x...",
    "gas_used": 50000,
    "success": true
  }
}
```

#### Create（创建）

表示合约创建。

```json
{
  "label": "Create",
  "source": 1,
  "target": 2,
  "properties": {
    "tx_hash": "0x...",
    "block_number": 15000000
  }
}
```

#### Approve（授权）

表示 ERC20 授权。

```json
{
  "label": "Approve",
  "source": 1,
  "target": 2,
  "properties": {
    "amount": "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    "token": "0x...",
    "tx_hash": "0x..."
  }
}
```

---

## 6. GQL 查询语言

### 6.1 语法概述

ChainGraph 支持基于 ISO GQL 39075 标准的查询语言。

```
GQL 语句 ::= MATCH 模式 [WHERE 条件] RETURN 返回项 [ORDER BY 排序] [LIMIT 限制]
           | INSERT 模式
           | DELETE 模式
           | UPDATE 模式 SET 属性
```

### 6.2 MATCH 语句

#### 基本节点匹配

```gql
-- 匹配所有账户
MATCH (n:Account) RETURN n

-- 匹配指定地址的账户
MATCH (n:Account {address: "0x742d35Cc..."}) RETURN n

-- 匹配多种类型
MATCH (n:Account|Contract) RETURN n
```

#### 关系匹配

```gql
-- 匹配转账关系
MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, t, b

-- 匹配入向关系
MATCH (a:Account)<-[t:Transfer]-(b:Account) RETURN a, t, b

-- 匹配任意方向
MATCH (a:Account)-[t:Transfer]-(b:Account) RETURN a, t, b
```

#### 路径匹配

```gql
-- 可变长度路径（1到5跳）
MATCH (a)-[:Transfer*1..5]->(b) RETURN a, b

-- 指定长度路径
MATCH (a)-[:Transfer*3]->(b) RETURN a, b
```

### 6.3 WHERE 子句

```gql
-- 数值比较
MATCH (n:Account) WHERE n.balance > 1000000000000000000 RETURN n

-- 字符串匹配
MATCH (n:Contract) WHERE n.name = "TetherToken" RETURN n

-- 逻辑运算
MATCH (n:Account) 
WHERE n.balance > 1000 AND n.tx_count > 10 
RETURN n

-- 地址过滤
MATCH (a)-[t:Transfer]->(b) 
WHERE a.address = "0x..." 
RETURN b, t.amount
```

### 6.4 RETURN 子句

```gql
-- 返回节点
MATCH (n:Account) RETURN n

-- 返回属性
MATCH (n:Account) RETURN n.address, n.balance

-- 返回别名
MATCH (n:Account) RETURN n.address AS addr, n.balance AS bal

-- 返回路径
MATCH path = (a)-[:Transfer*]->(b) RETURN path
```

### 6.5 ORDER BY 和 LIMIT

```gql
-- 排序
MATCH (n:Account) 
RETURN n 
ORDER BY n.balance DESC

-- 限制结果
MATCH (n:Account) 
RETURN n 
LIMIT 100

-- 分页
MATCH (n:Account) 
RETURN n 
ORDER BY n.balance DESC 
SKIP 100 LIMIT 50
```

### 6.6 INSERT 语句

INSERT 语句用于插入顶点和边，支持多种语法格式。

#### 插入顶点

```gql
-- 插入带标签的顶点
INSERT (n:Account)

-- 插入带属性的顶点
INSERT (n:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})

-- 插入带多个属性的顶点
INSERT (alice:Account {
    address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0",
    name: "Alice",
    balance: 10000
})

-- 插入合约顶点
INSERT (uniswap:Contract {
    address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
    name: "Uniswap V2 Router"
})
```

#### 插入边（转账关系）

```gql
-- 插入带属性的边
INSERT (a:Account {address: "0xAAA..."})-[t:Transfer {amount: 1000}]->(b:Account {address: "0xBBB..."})

-- 插入带完整属性的转账边
INSERT (a:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})
       -[t:Transfer {amount: 1000, block: 18500000}]->
       (b:Account {address: "0x8ba1f109551bD432803012645Ac136ddd64DBA72"})
```

#### 边属性说明

| 属性 | 类型 | 说明 |
|------|------|------|
| `amount` 或 `value` | 整数 | 转账金额 |
| `block` 或 `block_number` | 整数 | 区块高度 |

**注意**：插入边时，如果顶点已存在（通过 address 匹配），会自动复用现有顶点。

### 6.7 DELETE 语句

```gql
-- 删除顶点
MATCH (n:Account {address: "0x..."}) DELETE n

-- 删除边
MATCH (a)-[t:Transfer]->(b) WHERE t.amount < 100 DELETE t
```

### 6.8 UPDATE 语句

```gql
-- 更新属性
MATCH (n:Account {address: "0x..."}) 
SET n.balance = 2000000000000000000
```

### 6.9 LET 变量绑定 (ISO GQL 39075)

LET 语句用于声明和绑定变量，支持单个或多个变量同时绑定。

```gql
-- 单个变量绑定
LET x = 10

-- 多个变量绑定
LET x = 10, name = "Alice", active = true

-- 使用表达式
LET total = 100, rate = 0.08, result = 1 + 2

-- 布尔变量
LET is_active = true, is_verified = false
```

#### 支持的数据类型

| 类型 | 示例 |
|------|------|
| 整数 | `LET x = 42` |
| 浮点数 | `LET pi = 3.14` |
| 字符串 | `LET name = "Alice"` |
| 布尔值 | `LET active = true` |

### 6.10 FOR 迭代语句 (ISO GQL 39075)

FOR 语句用于在列表或范围上进行迭代，支持序数变量。

```gql
-- 基本迭代
FOR x IN [1, 2, 3, 4, 5]

-- 使用 range() 函数
FOR i IN range(1, 10)

-- 带序数变量的迭代
FOR item IN [10, 20, 30] WITH ORDINALITY AS ord

-- 综合示例
FOR i IN range(0, 100) WITH ORDINALITY AS idx
```

#### range() 函数

`range(start, end)` 函数生成从 `start` 到 `end-1` 的整数列表。

```gql
-- range(1, 5) 生成 [1, 2, 3, 4]
FOR i IN range(1, 5)

-- range(0, 3) 生成 [0, 1, 2]
FOR i IN range(0, 3)
```

### 6.11 FILTER 过滤语句 (ISO GQL 39075)

FILTER 语句用于基于条件过滤结果。

```gql
-- 基本过滤
FILTER n.age > 18

-- 相等比较
FILTER n.status = "active"

-- 复合条件 (AND)
FILTER n.balance > 1000 AND n.tx_count > 10

-- 复合条件 (OR)
FILTER n.type = "Account" OR n.type = "Contract"

-- NOT 条件
FILTER NOT n.deleted

-- 复杂表达式
FILTER n.amount > 1000 AND (n.status = "pending" OR n.priority = "high")
```

#### 支持的比较运算符

| 运算符 | 描述 | 示例 |
|--------|------|------|
| `=` | 相等 | `FILTER n.status = "active"` |
| `<>` 或 `!=` | 不等 | `FILTER n.type <> "test"` |
| `>` | 大于 | `FILTER n.balance > 1000` |
| `<` | 小于 | `FILTER n.age < 30` |
| `>=` | 大于等于 | `FILTER n.score >= 60` |
| `<=` | 小于等于 | `FILTER n.count <= 100` |

### 6.12 SELECT 查询语句 (ISO GQL 39075)

SELECT 语句提供 SQL 风格的查询功能，支持 DISTINCT、GROUP BY、HAVING、ORDER BY、LIMIT 和 OFFSET。

#### 基本 SELECT

```gql
-- 选择属性
SELECT n.name, n.age

-- 使用别名
SELECT n.name AS name, n.balance AS amount

-- 所有属性
SELECT *
```

#### DISTINCT 去重

```gql
-- 去除重复值
SELECT DISTINCT n.type

-- 多列去重
SELECT DISTINCT n.category, n.status
```

#### GROUP BY 分组

```gql
-- 基本分组
SELECT n.type, COUNT(*) 
GROUP BY n.type

-- 多列分组
SELECT n.category, n.status, COUNT(*)
GROUP BY n.category, n.status

-- 带聚合函数
SELECT n.type, SUM(n.amount), AVG(n.balance)
GROUP BY n.type
```

#### HAVING 分组过滤

```gql
-- 过滤分组结果
SELECT n.type, COUNT(*) AS cnt
GROUP BY n.type
HAVING COUNT(*) > 5

-- 复杂 HAVING
SELECT n.category, SUM(n.amount) AS total
GROUP BY n.category
HAVING SUM(n.amount) > 10000 AND COUNT(*) >= 3
```

#### ORDER BY 排序

```gql
-- 升序排序（默认）
SELECT n.name, n.created_at
ORDER BY n.created_at

-- 降序排序
SELECT n.name, n.balance
ORDER BY n.balance DESC

-- 多列排序
SELECT n.category, n.name, n.priority
ORDER BY n.category ASC, n.priority DESC
```

#### LIMIT 和 OFFSET 分页

```gql
-- 限制结果数量
SELECT n.name
LIMIT 10

-- 分页查询
SELECT n.name, n.created_at
ORDER BY n.created_at DESC
LIMIT 20 OFFSET 40

-- 结合所有功能
SELECT n.type, COUNT(*) AS cnt
GROUP BY n.type
HAVING COUNT(*) > 5
ORDER BY cnt DESC
LIMIT 10 OFFSET 0
```

#### 支持的聚合函数

| 函数 | 描述 | 示例 |
|------|------|------|
| `COUNT(*)` | 计数 | `SELECT COUNT(*)` |
| `SUM(expr)` | 求和 | `SELECT SUM(n.amount)` |
| `AVG(expr)` | 平均值 | `SELECT AVG(n.balance)` |
| `MIN(expr)` | 最小值 | `SELECT MIN(n.price)` |
| `MAX(expr)` | 最大值 | `SELECT MAX(n.score)` |

### 6.13 USE 图切换语句 (ISO GQL 39075)

USE 语句用于切换当前查询的图上下文。

```gql
-- 切换到指定图
USE GRAPH ethereum_mainnet

-- 切换到另一个图
USE GRAPH polygon_network

-- 切换后执行查询
USE GRAPH bsc_mainnet
MATCH (n:Account) RETURN n LIMIT 10
```

### 6.14 复合查询 (ISO GQL 39075)

支持 UNION、EXCEPT、INTERSECT、OTHERWISE 操作来组合多个查询结果。

#### UNION 联合

```gql
-- 联合查询（保留重复）
MATCH (a:Account) RETURN a
UNION ALL
MATCH (c:Contract) RETURN c

-- 联合查询（去除重复）
MATCH (a:Account) RETURN a
UNION
MATCH (b:Account) RETURN b
```

#### EXCEPT 差集

```gql
-- 差集查询
MATCH (a:Account) RETURN a
EXCEPT
MATCH (b:Account {status: "inactive"}) RETURN b
```

#### INTERSECT 交集

```gql
-- 交集查询
MATCH (a:Account {type: "whale"}) RETURN a
INTERSECT
MATCH (b:Account {verified: true}) RETURN b
```

#### OTHERWISE 回退

当第一个查询返回空结果时，执行第二个查询：

```gql
-- 回退查询
MATCH (n:Account {address: "0x..."}) RETURN n
OTHERWISE
MATCH (n:Account) RETURN n LIMIT 1
```

### 6.15 SESSION 会话管理 (ISO GQL 39075)

SESSION 语句用于管理查询会话的属性。

#### SET 设置

```gql
-- 设置当前模式
SESSION SET SCHEMA main_schema

-- 设置当前图
SESSION SET GRAPH ethereum

-- 设置属性图
SESSION SET PROPERTY GRAPH financial_graph

-- 设置属性值
SESSION SET VALUE timeout = 30000
```

#### RESET 重置

```gql
-- 重置模式
SESSION RESET SCHEMA

-- 重置图
SESSION RESET GRAPH

-- 重置所有
SESSION RESET ALL
```

#### CLOSE 关闭

```gql
-- 关闭当前会话
SESSION CLOSE
```

### 6.16 TRANSACTION 事务控制 (ISO GQL 39075)

事务控制语句用于管理数据库事务。

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

#### 事务使用示例

```gql
-- 典型事务流程
START TRANSACTION READ WRITE
INSERT (a:Account {address: "0x..."})
INSERT (a:Account {address: "0xAAA"})-[t:Transfer {amount: 1000}]->(b:Account {address: "0xBBB"})
COMMIT

-- 发生错误时回滚
START TRANSACTION READ WRITE
-- ... 执行操作 ...
ROLLBACK
```

### 6.17 CREATE/DROP GRAPH TYPE (ISO GQL 39075)

图类型定义语句用于创建和删除图类型模式。

#### CREATE GRAPH TYPE

```gql
-- 创建图类型
CREATE GRAPH TYPE financial_network AS (
  (account:Account {address STRING, balance DECIMAL}),
  (contract:Contract {address STRING, code_hash STRING}),
  (account)-[transfer:Transfer {amount DECIMAL}]->(account)
)

-- 简单图类型
CREATE GRAPH TYPE simple_graph AS (
  (node:Node {id INT, name STRING})
)
```

#### DROP GRAPH TYPE

```gql
-- 删除图类型
DROP GRAPH TYPE financial_network

-- 强制删除（如果存在）
DROP GRAPH TYPE IF EXISTS old_schema
```

### 6.18 量化路径模式 (ISO GQL 39075)

支持可变长度路径和路径搜索前缀。

#### 可变长度路径

```gql
-- 1 到 5 跳
MATCH (a)-[*1..5]->(b) RETURN a, b

-- 精确 3 跳
MATCH (a)-[*3]->(b) RETURN a, b

-- 0 跳或更多
MATCH (a)-[*0..]->(b) RETURN a, b

-- 任意跳数（有限制）
MATCH (a)-[*..10]->(b) RETURN a, b
```

#### 路径搜索前缀

```gql
-- 最短路径
MATCH SHORTEST (a)-[*]->(b) RETURN path

-- 所有最短路径
MATCH ALL SHORTEST (a)-[*]->(b) RETURN path

-- 任意路径
MATCH ANY (a)-[*1..10]->(b) RETURN path

-- 任意最短路径
MATCH ANY SHORTEST (a)-[*]->(b) RETURN path
```

### 6.19 SHOW 语句 - 查看数据库对象

SHOW 语句用于列出数据库中的各类对象。

#### SHOW GRAPHS

```gql
-- 查看所有图
SHOW GRAPHS

-- 返回列: name, type, vertex_count, edge_count
```

#### SHOW GRAPH TYPES

```gql
-- 查看所有图类型
SHOW GRAPH TYPES

-- 返回列: name, definition
```

#### SHOW SCHEMAS

```gql
-- 查看所有模式
SHOW SCHEMAS

-- 返回列: name
```

#### SHOW LABELS

```gql
-- 查看所有顶点标签
SHOW LABELS

-- 返回列: label, description
```

#### SHOW EDGE TYPES

```gql
-- 查看所有边类型
SHOW EDGE TYPES
SHOW RELATIONSHIP TYPES

-- 返回列: type, description
```

#### SHOW PROPERTY KEYS

```gql
-- 查看所有属性键
SHOW PROPERTY KEYS

-- 返回列: key, usage
```

#### SHOW FUNCTIONS

```gql
-- 查看所有内置函数
SHOW FUNCTIONS

-- 返回列: name, signature, description
```

#### SHOW PROCEDURES

```gql
-- 查看所有存储过程
SHOW PROCEDURES

-- 返回列: name, signature, description
```

#### SHOW INDEXES

```gql
-- 查看所有索引
SHOW INDEXES

-- 返回列: name, type, properties
```

#### SHOW CONSTRAINTS

```gql
-- 查看所有约束
SHOW CONSTRAINTS

-- 返回列: name, type, definition
```

### 6.20 DESCRIBE 语句 - 查看对象详情

DESCRIBE 语句（可缩写为 DESC）用于查看数据库对象的详细信息。

#### DESCRIBE GRAPH

```gql
-- 查看图的详细信息
DESCRIBE GRAPH myGraph
DESC GRAPH myGraph

-- 返回属性: name, type, vertex_count, edge_count, created_at
```

#### DESCRIBE GRAPH TYPE

```gql
-- 查看图类型的详细信息
DESCRIBE GRAPH TYPE myGraphType
DESC GRAPH TYPE myType

-- 返回列: element, type, properties
```

#### DESCRIBE SCHEMA

```gql
-- 查看模式的详细信息
DESCRIBE SCHEMA public

-- 返回属性: name, graphs
```

#### DESCRIBE LABEL

```gql
-- 查看顶点标签的详细信息
DESCRIBE LABEL Account
DESCRIBE VERTEX TYPE Account

-- 返回列: property, type, nullable
```

#### DESCRIBE EDGE TYPE

```gql
-- 查看边类型的详细信息
DESCRIBE EDGE TYPE Transfer
DESCRIBE RELATIONSHIP TYPE Transfer

-- 返回列: property, type, nullable
```

---

## 7. 图算法

ChainGraph 提供多种图算法，可通过 GQL CALL 语句或 REST API 调用。

### 7.0 GQL CALL 语法 (ISO GQL 39075)

ChainGraph 完全支持 ISO GQL 39075 标准的 CALL 过程调用语法：

```gql
-- 基本语法
CALL procedure_name(arg1, arg2, ...)

-- 可选调用（不存在时返回空结果）
OPTIONAL CALL procedure_name(arg1, arg2, ...)
```

#### 可用过程列表

| 过程名 | 参数 | 返回字段 | 描述 |
|--------|------|----------|------|
| `shortest_path(source, target)` | 起点ID, 终点ID | path, length, total_weight | 最短路径 |
| `all_paths(source, target, max_depth?)` | 起点, 终点, 可选深度 | path, length, total_weight | 所有路径 |
| `trace(start, direction?, max_depth?)` | 起点, 方向, 深度 | path, length, total_weight | 链路追踪 |
| `max_flow(source, sink)` | 源点, 汇点 | edge, flow | 最大流 |
| `neighbors(vertex_id, direction?)` | 顶点ID, 方向 | direction, neighbor_id | 邻居查询 |
| `degree(vertex_id)` | 顶点ID | in_degree, out_degree | 度数查询 |
| `connected(source, target)` | 起点, 终点 | connected | 连通性检测 |

#### CALL 示例

```gql
-- 最短路径
CALL shortest_path(1, 5)

-- 所有路径（最大深度 10）
CALL all_paths(1, 5, 10)

-- 正向链路追踪
CALL trace(1, 'forward', 5)

-- 反向链路追踪
CALL trace(1, 'backward', 5)

-- 双向链路追踪
CALL trace(1, 'both', 5)

-- 最大流
CALL max_flow(1, 100)

-- 出边邻居
CALL neighbors(1, 'out')

-- 入边邻居
CALL neighbors(1, 'in')

-- 所有邻居
CALL neighbors(1, 'both')

-- 度数
CALL degree(1)

-- 连通性检测
CALL connected(1, 100)

-- 可选调用（顶点不存在时返回空）
OPTIONAL CALL shortest_path(1, 999999)
```

### 7.1 路径追踪

#### 最短路径

找到两个顶点之间的最短路径。

**GQL 调用：**
```gql
CALL shortest_path(1, 100)
```

**REST API 调用：**
```bash
POST /algorithm/shortest-path
{
  "source": 1,
  "target": 100
}
```

响应：
```json
{
  "success": true,
  "data": {
    "vertices": [1, 42, 78, 100],
    "edges": [101, 203, 305],
    "length": 3,
    "total_weight": 1500000.0
  }
}
```

#### 所有路径

找到两个顶点之间的所有路径（限制深度）。

**GQL 调用：**
```gql
CALL all_paths(1, 100, 5)
```

**REST API 调用：**
```bash
POST /algorithm/all-paths
{
  "source": 1,
  "target": 100,
  "max_depth": 5,
  "k": 10  // 最多返回 10 条路径
}
```

#### N跳邻居

查找指定顶点的 N 跳邻居。

**GQL 调用：**
```gql
CALL trace(1, 'forward', 3)
```

**REST API 调用：**
```bash
POST /algorithm/trace
{
  "start": 1,
  "direction": "forward",  // forward, backward, both
  "max_depth": 3
}
```

### 7.2 最大流算法

使用 Edmonds-Karp 算法计算最大流，用于分析资金流动的最大通量。

**GQL 调用：**
```gql
CALL max_flow(1, 100)
```

**REST API 调用：**
```bash
POST /algorithm/max-flow
{
  "source": 1,
  "sink": 100
}
```

响应：
```json
{
  "success": true,
  "data": {
    "value": 1500000.0,
    "flow": {
      "(1, 42)": 500000.0,
      "(1, 56)": 1000000.0,
      "(42, 100)": 500000.0,
      "(56, 100)": 1000000.0
    },
    "source_side": [1, 42, 56]
  }
}
```

#### 最大流应用场景

1. **资金瓶颈分析**：找出资金流动的瓶颈路径
2. **洗钱检测**：识别资金分散和汇聚模式
3. **风险评估**：评估地址之间的资金关联强度

### 7.3 链路追踪

#### 正向追踪

从源地址出发，追踪资金流向。

**GQL 调用：**
```gql
CALL trace(1, 'forward', 10)
```

**REST API 调用：**
```bash
POST /algorithm/trace
{
  "start": 1,
  "direction": "forward",
  "max_depth": 10
}
```

#### 反向追踪

从目标地址出发，追溯资金来源。

**GQL 调用：**
```gql
CALL trace(100, 'backward', 10)
```

**REST API 调用：**
```bash
POST /algorithm/trace
{
  "start": 100,
  "direction": "backward",
  "max_depth": 10
}
```

#### 双向追踪

同时进行正向和反向追踪。

**GQL 调用：**
```gql
CALL trace(50, 'both', 5)
```

**REST API 调用：**
```bash
POST /algorithm/trace
{
  "start": 50,
  "direction": "both",
  "max_depth": 5
}
```

---

## 8. REST API 参考

### 8.1 通用说明

#### 基础 URL

```
http://localhost:8080
```

#### 响应格式

所有 API 返回 JSON 格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

错误响应：

```json
{
  "success": false,
  "data": null,
  "error": "错误信息"
}
```

### 8.2 健康检查

```
GET /health
```

**响应：**

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### 8.3 查询接口

#### 执行 GQL 查询

```
POST /query
Content-Type: application/json
```

**请求体：**

```json
{
  "query": "MATCH (n:Account) RETURN n LIMIT 10"
}
```

**响应：**

```json
{
  "success": true,
  "data": {
    "columns": ["n"],
    "rows": [
      [{"id": 1, "label": "Account", "address": "0x..."}]
    ],
    "stats": {
      "execution_time_ms": 5,
      "vertices_scanned": 1000,
      "edges_scanned": 0
    }
  }
}
```

### 8.4 顶点接口

#### 获取顶点

```
GET /vertices/{id}
```

**响应：**

```json
{
  "success": true,
  "data": {
    "id": 1,
    "label": "Account",
    "address": "0x742d35Cc...",
    "properties": {
      "balance": "1000000000000000000",
      "nonce": 42
    }
  }
}
```

#### 通过地址获取顶点

```
GET /vertices/address/{address}
```

### 8.5 边接口

#### 获取边

```
GET /edges/{id}
```

#### 获取顶点的出边

```
GET /vertices/{id}/outgoing
```

#### 获取顶点的入边

```
GET /vertices/{id}/incoming
```

### 8.6 算法接口

#### 最短路径

```
POST /algorithm/shortest-path
```

**请求体：**

```json
{
  "source": 1,
  "target": 100
}
```

#### 所有路径

```
POST /algorithm/all-paths
```

**请求体：**

```json
{
  "source": 1,
  "target": 100,
  "max_depth": 5,
  "k": 10
}
```

#### 最大流

```
POST /algorithm/max-flow
```

**请求体：**

```json
{
  "source": 1,
  "sink": 100
}
```

#### 链路追踪

```
POST /algorithm/trace
```

**请求体：**

```json
{
  "start": 1,
  "direction": "forward",
  "max_depth": 10
}
```

### 8.7 统计接口

```
GET /stats
```

**响应：**

```json
{
  "success": true,
  "data": {
    "vertex_count": 1000000,
    "edge_count": 5000000,
    "buffer_pool_size": 1024,
    "cached_pages": 512
  }
}
```

---

## 9. 数据导入

### 9.1 CSV 格式

#### 顶点 CSV

```csv
address,label,balance,nonce
0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0,Account,1000000000000000000,42
0xdAC17F958D2ee523a2206206994597C13D831ec7,Contract,0,0
```

#### 边 CSV

```csv
from_address,to_address,label,amount,tx_hash,block_number
0x742d35Cc...,0xdAC17F958D2...,Transfer,1000000000000000000,0x...,15000000
```

### 9.2 JSON 格式

使用 JSONL（每行一个 JSON 对象）格式。

#### 顶点 JSONL

```json
{"type":"vertex","label":"Account","address":"0x742d35Cc...","properties":{"balance":"1000","nonce":42}}
{"type":"vertex","label":"Contract","address":"0xdAC17F958D2...","properties":{"name":"USDT"}}
```

#### 边 JSONL

```json
{"type":"edge","label":"Transfer","from":"0x742d35Cc...","to":"0xdAC17F958D2...","properties":{"amount":"1000","tx_hash":"0x..."}}
```

### 9.3 导入命令

```bash
# CSV 导入
./chaingraph-import \
    --input vertices.csv \
    --format csv \
    --data-dir ./data \
    --batch-size 10000

# JSONL 导入（并行）
./chaingraph-import \
    --input data.jsonl \
    --format jsonl \
    --data-dir ./data \
    --parallel \
    --batch-size 50000
```

### 9.4 导入参数

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `--input` | 必需 | 输入文件路径 |
| `--format` | `csv` | 文件格式：csv, jsonl |
| `--data-dir` | `./data` | 数据目录 |
| `--batch-size` | `10000` | 批次大小 |
| `--parallel` | `false` | 启用并行导入 |

### 9.5 导入统计

导入完成后会显示统计信息：

```
导入完成！
├─ 顶点: 1,000,000
├─ 边: 5,000,000
├─ 错误: 0
└─ 耗时: 120.5 秒
```

### 9.6 GQL DML 导入

除了使用文件导入外，还可以使用 GQL INSERT 语句直接导入数据，适用于小批量数据或交互式操作。

#### 交互式导入

在 CLI 中直接执行 INSERT 语句：

```bash
# 启动 CLI
./chaingraph-cli --data-dir ./data

# 在交互模式中执行 INSERT
chaingraph> INSERT (alice:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})
inserted_vertices | inserted_edges
------------------------------
1 | 0

chaingraph> INSERT (a:Account {address: "0x742d35Cc..."})-[t:Transfer {amount: 1000}]->(b:Account {address: "0x8ba1f1..."})
inserted_vertices | inserted_edges
------------------------------
2 | 1
```

#### 批量导入脚本

可以通过管道将多条语句传入 CLI：

```bash
# 从文件导入
cat examples/sample_dml.gql | grep -v "^--" | ./chaingraph-cli --data-dir ./data

# 使用 echo 批量导入
echo 'INSERT (a:Account {address: "0xAAA..."})
INSERT (b:Account {address: "0xBBB..."})
INSERT (a:Account {address: "0xAAA..."})-[t:Transfer {amount: 500}]->(b:Account {address: "0xBBB..."})
stats
quit' | ./chaingraph-cli --data-dir ./data
```

#### 单条命令执行

使用 `-e` 参数执行单条 GQL 语句：

```bash
./chaingraph-cli --data-dir ./data -e 'INSERT (n:Account {address: "0x742d35Cc..."})'
```

#### 示例数据文件

项目提供了示例数据文件 `examples/sample_dml.gql`，包含：

- 5 个账户顶点 (Alice, Bob, Charlie, Dave, Eve)
- 2 个合约顶点 (Uniswap, AAVE)
- 资金流转链路 (形成环路)
- DeFi 交互交易
- 可疑资金拆分链路

```bash
# 执行示例导入脚本
./examples/import_sample_data.sh ./data
```

---

## 10. 命令行工具

### 10.1 启动 CLI

```bash
./chaingraph-cli --data-dir ./data
```

### 10.2 交互式命令

```
ChainGraph CLI v0.1.0
输入 GQL 查询或命令。输入 'help' 查看帮助。

chaingraph> MATCH (n:Account) RETURN n LIMIT 5
┌────┬─────────────────────────────────────────────┬─────────────┐
│ ID │ Address                                     │ Balance     │
├────┼─────────────────────────────────────────────┼─────────────┤
│ 1  │ 0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0 │ 1.0 ETH     │
│ 2  │ 0xdAC17F958D2ee523a2206206994597C13D831ec7 │ 0.0 ETH     │
└────┴─────────────────────────────────────────────┴─────────────┘

5 行结果，耗时 2ms

chaingraph> 
```

### 10.3 内置命令

| 命令 | 描述 |
|------|------|
| `help` | 显示帮助信息 |
| `stats` | 显示数据库统计 |
| `vertex <ID>` | 查看顶点详情 |
| `path <source> <target>` | 快速查找最短路径 |
| `trace <start> [direction] [depth]` | 快速链路追踪 |
| `maxflow <source> <sink>` | 快速计算最大流 |
| `clear` | 清屏 |
| `exit` 或 `quit` | 退出 |

### 10.4 GQL CALL 过程调用

ChainGraph CLI 完全支持 ISO GQL 39075 标准的 CALL 语句：

```bash
# 查找最短路径
chaingraph> CALL shortest_path(1, 100)

path                   | length | total_weight
-----------------------------------------------
1 -> 42 -> 78 -> 100   | 3      | 1500000.0

1 行结果 (耗时 1 ms)

# 查找所有路径
chaingraph> CALL all_paths(1, 100, 5)

path                   | length | total_weight
-----------------------------------------------
1 -> 42 -> 100         | 2      | 800000.0
1 -> 42 -> 78 -> 100   | 3      | 1500000.0
1 -> 56 -> 89 -> 100   | 3      | 1200000.0

3 行结果 (耗时 2 ms)

# 链路追踪（正向）
chaingraph> CALL trace(1, 'forward', 3)

path                          | length | total_weight
------------------------------------------------------
1 -> 42                       | 1      | 100000.0
1 -> 42 -> 78                 | 2      | 250000.0
1 -> 42 -> 78 -> 100          | 3      | 400000.0
1 -> 56                       | 1      | 150000.0
...

# 链路追踪（反向）
chaingraph> CALL trace(100, 'backward', 3)

# 最大流计算
chaingraph> CALL max_flow(1, 100)

edge            | flow
--------------------------
max_flow_value  | 1500000.0
1 -> 42         | 500000.0
1 -> 56         | 1000000.0
42 -> 100       | 500000.0
56 -> 100       | 1000000.0

5 行结果 (耗时 5 ms)

# 邻居查询
chaingraph> CALL neighbors(1, 'out')

direction | neighbor_id
------------------------
out       | 42
out       | 56
out       | 78

3 行结果 (耗时 0 ms)

# 度数查询
chaingraph> CALL degree(1)

in_degree | out_degree
----------------------
2         | 5

1 行结果 (耗时 0 ms)

# 连通性检测
chaingraph> CALL connected(1, 100)

connected
---------
true

1 行结果 (耗时 1 ms)

# 可选调用（顶点不存在时不报错）
chaingraph> OPTIONAL CALL shortest_path(1, 999999)

result
------
未找到路径

1 行结果 (耗时 0 ms)
```

### 10.5 单次执行

```bash
# 执行单个查询
./chaingraph-cli -e "MATCH (n:Account) RETURN n LIMIT 10"

# 执行 CALL 语句
./chaingraph-cli -e "CALL shortest_path(1, 100)"

# 执行链路追踪
./chaingraph-cli -e "CALL trace(1, 'forward', 5)"
```

---

## 11. 性能调优

### 11.1 缓冲池大小

缓冲池是影响性能的关键因素。

```bash
# 小规模数据（< 10 GB）
--buffer-size 512    # 2 MB

# 中等规模（10-100 GB）
--buffer-size 4096   # 16 MB

# 大规模数据（> 100 GB）
--buffer-size 16384  # 64 MB
```

**经验法则**：缓冲池大小应为可用内存的 10-25%。

### 11.2 存储优化

1. **使用 NVMe SSD**：比 SATA SSD 快 3-5 倍
2. **文件系统**：推荐 ext4 或 xfs
3. **挂载选项**：使用 `noatime` 减少写入

```bash
# 推荐的挂载选项
mount -o noatime,nodiratime /dev/nvme0n1 /data
```

### 11.3 查询优化

1. **使用 LIMIT**：避免返回过多结果
2. **精确匹配**：使用地址索引
3. **限制路径深度**：避免过深的路径搜索

```gql
-- 好的查询
MATCH (n:Account {address: "0x..."}) RETURN n

-- 避免的查询
MATCH (n) RETURN n  -- 全表扫描
```

### 11.4 导入优化

1. **增大批次**：`--batch-size 50000`
2. **启用并行**：`--parallel`
3. **禁用日志**：生产环境可关闭详细日志

---

## 12. 最佳实践

### 12.1 数据建模

1. **合理使用标签**：为不同类型的实体使用不同标签
2. **属性规范化**：统一属性命名和类型
3. **避免过度嵌套**：保持图结构扁平

### 12.2 查询建议

```gql
-- ✅ 推荐：使用索引字段
MATCH (n:Account {address: "0x..."}) RETURN n

-- ❌ 避免：全图扫描
MATCH (n) WHERE n.balance > 1000 RETURN n

-- ✅ 推荐：限制路径深度
MATCH path = (a)-[:Transfer*1..3]->(b) RETURN path

-- ❌ 避免：无限深度
MATCH path = (a)-[:Transfer*]->(b) RETURN path
```

### 12.3 运维建议

1. **定期备份**：备份数据目录
2. **监控指标**：关注缓冲池命中率
3. **日志分析**：定期检查错误日志

---

## 13. 故障排除

### 13.1 常见问题

#### 服务无法启动

```
Error: Failed to bind to address
```

**解决方案**：检查端口是否被占用

```bash
lsof -i :8080
```

#### 内存不足

```
Error: Out of memory
```

**解决方案**：减小缓冲池大小或增加系统内存

#### 磁盘空间不足

```
Error: No space left on device
```

**解决方案**：清理磁盘空间或迁移数据目录

### 13.2 性能问题

#### 查询缓慢

1. 检查缓冲池命中率：`GET /stats`
2. 优化查询语句
3. 增加缓冲池大小

#### 导入缓慢

1. 检查磁盘 I/O
2. 增大批次大小
3. 启用并行导入

### 13.3 数据问题

#### 数据校验失败

```
Error: Checksum mismatch
```

**解决方案**：数据可能已损坏，从备份恢复

---

## 附录

### A. GQL 语法参考

```bnf
<statement>     ::= <match-stmt> | <insert-stmt> | <delete-stmt> | <update-stmt>

<match-stmt>    ::= MATCH <pattern> [WHERE <expression>] RETURN <return-items>
                    [ORDER BY <order-items>] [SKIP <integer>] [LIMIT <integer>]

<pattern>       ::= <path-pattern> (',' <path-pattern>)*

<path-pattern>  ::= <node-pattern> (<relationship-pattern> <node-pattern>)*

<node-pattern>  ::= '(' [<variable>] [':' <label>] [<properties>] ')'

<relationship>  ::= '-[' [<variable>] [':' <label>] ['*' <length>] [<properties>] ']->'
                  | '<-[' [<variable>] [':' <label>] ['*' <length>] [<properties>] ']-'

<properties>    ::= '{' <property> (',' <property>)* '}'

<property>      ::= <identifier> ':' <value>
```

### B. 错误代码

| 代码 | 描述 |
|------|------|
| `E001` | 顶点不存在 |
| `E002` | 边不存在 |
| `E003` | 存储错误 |
| `E004` | 解析错误 |
| `E005` | 查询错误 |
| `E006` | 校验和不匹配 |
| `E007` | 服务器错误 |

### C. 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.1.0 | 2024-12 | 初始版本 |

---

© 2024 ChainGraph. All rights reserved.
