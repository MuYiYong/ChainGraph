
# ChainGraph（开发中，请勿直接用于生产）

<p align="center">
  <img src="https://github.com/MuYiYong/ChainGraph/actions/workflows/ci.yml/badge.svg" alt="CI">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/deployment-Docker-blue.svg" alt="Docker">
</p>

<!-- Prominent language switch -->

<div align="center" style="margin-bottom: 8px;">
  <a href="./README.md" title="English" style="text-decoration:none;">
    <span style="display:inline-block;padding:8px 28px;margin:4px 8px;border:2px solid #1e90ff;border-radius:6px;font-size:1.08em;font-weight:600;background:#f8faff;color:#1e90ff;">English</span>
  </a>
  <a href="./README.zh-CN.md" title="中文" style="text-decoration:none;">
    <span style="display:inline-block;padding:8px 28px;margin:4px 8px;border:2px solid #ff4d4f;border-radius:6px;font-size:1.08em;font-weight:600;background:#fff8f8;color:#ff4d4f;">中文</span>
  </a>
</div>

**ChainGraph** 是一款为 Web3 场景设计的高性能图数据库，专注于链上链路追踪与资金流分析。

> ⚠️ ChainGraph 仅以 Docker 容器方式提供服务。

---

**语言 / Language**:  
- 中文（当前文档）：`README.zh-CN.md`  
- 英文（默认）：`README.md`

## ✨ 特性

- 🐳 容器优先：以 Docker 容器方式运行，便于部署
- 🚀 SSD 优化存储：4KB 页面对齐，LRU 缓冲池，适用于大规模数据
- 🔗 Web3 原生类型：内置 `Address`、`TxHash`、`TokenAmount` 等
- 🔍 链路追踪算法：支持最短路径、所有路径、N 跳邻居等
- 💧 最大流分析：使用 Edmonds–Karp 算法，适用于资金流与 AML 分析
- 📝 支持 ISO GQL 39075：实现核心图查询语言特性

## 🚀 快速上手 (Quick Start)

本指南将带你完成从部署到数据查询的完整流程（约 5 分钟）。

### 一行快速试用（极简）

在仓库根目录使用以下一行命令快速体验：

### 第一步：启动服务

使用 Docker Compose 快速启动 ChainGraph 服务和 CLI 工具。

```bash
# 1. 下载示例配置
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# 2. 启动服务 (后台运行)
docker compose up -d

# 启动服务
docker compose up -d

# 3. 检查服务状态
docker compose ps
```

### 第二步：连接数据库

使用内置的 CLI 工具连接到 ChainGraph 服务。

```bash
# 启动 CLI 并连接到本地服务
docker compose run --rm chaingraph-cli
```

成功连接后，你将看到 `GQL >` 提示符。

### 第三步：创建图

在 CLI 中输入以下命令，创建一个简单的金融交易图。我们使用**内联 Schema** 直接定义节点和边类型。

```gql
-- 创建名为 financial_graph 的图
CREATE GRAPH financial_graph {
  -- 定义 Account 节点，address 为主键
  NODE Account {
    address String PRIMARY KEY,
    type String
  },
  -- 定义 Transfer 边，连接两个 Account
  EDGE Transfer (Account)-[{
    amount int,
    timestamp int
  }]->(Account)
};

-- 切换到刚创建的图
USE GRAPH financial_graph;
```

### 第四步：导入数据 (写入)

使用 `INSERT` 语句写入一些测试数据。

```gql
-- 1. 创建两个账户 (Alice 和 Bob)
INSERT (alice:Account { address: "0xAlice", type: "EOA" });
INSERT (bob:Account { address: "0xBob", type: "EOA" });

-- 2. 创建一笔转账 (Alice -> Bob, 金额 100)
INSERT (a:Account {address: "0xAlice"})-[t:Transfer {amount: 100, timestamp: 1625000000}]->(b:Account {address: "0xBob"});

-- 3. 再创建一笔转账 (Bob -> Alice, 金额 50)
INSERT (b:Account {address: "0xBob"})-[t2:Transfer {amount: 50, timestamp: 1625000100}]->(a:Account {address: "0xAlice"});
```

### 第五步：执行查询

现在可以查询刚才写入的数据了。

```gql
-- 查询所有账户
MATCH (n:Account) RETURN n;

-- 查询 Alice 的转账记录 (包含方向)
MATCH (a:Account {address: "0xAlice"})-[t:Transfer]-(partner)
RETURN a.address, t.amount, partner.address;

-- 查询资金流向路径 (1到3跳)
MATCH path = (start:Account {address: "0xAlice"})-[:Transfer]->{1,3}(end)
RETURN path;
```

🎉 恭喜！你已经完成了 ChainGraph 的基本操作流程。输入 `exit` 退出 CLI。

## 🖥️ CLI 使用

```bash
# Docker Compose 方式
docker compose run --rm chaingraph-cli

# 直接 Docker 方式
docker run -it --rm \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-cli -d /data
```

## 📥 导入数据

```bash
# 将数据文件放入 import 目录
mkdir -p import
cp your_data.csv import/

# 使用 Docker Compose 导入
docker compose --profile import run --rm chaingraph-import

# 或直接使用 Docker 导入
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/import:/import:ro \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-import -d /data -i /import/your_data.csv
```

## 🔌 REST API

服务启动后，通过 `http://localhost:8080` 访问 API：

```bash
# 健康检查
curl http://localhost:8080/health

# 执行 GQL 查询
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10"}'

# 获取统计信息
curl http://localhost:8080/stats

# 最短路径
curl -X POST http://localhost:8080/algorithm/shortest-path \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "target": 100}'

# 最大流分析
curl -X POST http://localhost:8080/algorithm/max-flow \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "sink": 100}'
```

## 📖 GQL 查询示例

### 基本查询

```gql
-- 查找账户
MATCH (n:Account) RETURN n LIMIT 100

CREATE GRAPH financial_graph {
  -- 定义 Account 节点，address 为主键
  NODE Account {
    address String PRIMARY KEY,
    name String,
    balance Integer
  },

  -- 定义 Contract 节点
  NODE Contract {
    address String PRIMARY KEY,
    name String,
    protocol String
  },

  -- 定义连接两个 Account 的 Transfer 边
  EDGE Transfer (Account)-[{
    amount Integer,
    token String,
    blockNumber Integer,
    timestamp Integer
  }]->(Account)
};

-- 切换到新图
USE GRAPH financial_graph;

```gql
-- 插入账户顶点
INSERT (alice:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})

-- 插入转账边
INSERT (a)-[:Transfer {amount: 1000}]->(b)
```

### 调用过程

```gql
-- 最短路径
CALL shortest_path(1, 5)

-- 链路追踪
CALL trace(1, 'forward', 5)

-- 最大流分析
CALL max_flow(1, 100)
```

### 元数据查询

```gql
-- 列出图
SHOW GRAPHS

-- 列出标签
SHOW LABELS

-- 查看图详情
DESCRIBE GRAPH myGraph
```

更多 GQL 语法请参阅用户手册：`docs/manual.md`

## 💾 数据持久化

数据保存在 Docker 卷中：

```bash
# 查看数据卷
docker volume inspect chaingraph-data

# 备份数据
docker run --rm \
  -v chaingraph-data:/data:ro \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/chaingraph-backup.tar.gz -C /data .

# 恢复数据
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/backup:/backup:ro \
  alpine tar xzf /backup/chaingraph-backup.tar.gz -C /data
```

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────┐
│                    Docker 容器                          │
├─────────────────────────────────────────────────────────┤
│                      REST API 层                        │
│                   (axum HTTP 服务器)                    │
├─────────────────────────────────────────────────────────┤
│                     查询引擎                             │
│              ┌─────────┬──────────────┐                  │
│              │ 解析器  │   执行器     │                  │
│              │  (GQL)  │              │                  │
│              └─────────┴──────────────┘                  │
├─────────────────────────────────────────────────────────┤
│                   图算法模块                             │
│     ┌──────────────┐        ┌──────────────────┐        │
│     │ 路径追踪      │        │ 最大流 (E-K)     │        │
│     └──────────────┘        └──────────────────┘        │
├─────────────────────────────────────────────────────────┤
│                    存储引擎                              │
│   ┌──────────────┐  ┌──────────────┐  ┌────────────┐    │
│   │ 缓冲池 (LRU) │  │ 磁盘存储 (mmap)│  │ 页面 (4KB) │    │
│   └──────────────┘  └──────────────┘  └────────────┘    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
                   ┌──────────────┐
                   │ Docker 卷     │
                   └──────────────┘
```

## 📊 数据模型

### 顶点类型

| 类型 | 描述 | 典型属性 |
|------|------|----------|
| `Account` | EOA 账户 | address, balance |
| `Contract` | 智能合约 | address, code_hash |
| `Token` | 代币 | address, symbol |

### 边类型

| 类型 | 描述 | 典型属性 |
|------|------|----------|
| `Transfer` | 代币转账 | amount, token |
| `Call` | 合约调用 | method, gas |

## ⚙️ 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别 (debug, info, warn, error) |

## 📚 文档

- [Docker 使用指南](DOCKER.md)
- [用户手册](docs/manual.md)

## 📄 许可证

本项目采用 Apache-2.0 许可证。详见 [LICENSE](LICENSE) 文件。

## 🤝 贡献

欢迎贡献代码！请先阅读 [贡献指南](CONTRIBUTING.md)。

---

<p align="center">
  Made with ❤️ for Web3 | 🐳 Container Only
</p>
