# ChainGraph（开发中，请勿下载，如有兴趣，多多Star）

<p align="center">
  <img src="https://github.com/MuYiYong/ChainGraph/actions/workflows/ci.yml/badge.svg" alt="CI">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/deployment-Docker-blue.svg" alt="Docker">
</p>

ChainGraph is a high-performance graph database designed for Web3 scenarios, focused on on-chain link tracing and funds-flow analysis.

> ⚠️ ChainGraph is provided as a Docker containerized service only.

## Features

- 🐳 Container-first: runs as Docker containers for easy deployment
- 🚀 SSD-optimized storage: 4KB page alignment, LRU buffer pool, suitable for large datasets
- 🔗 Web3-native types: built-in `Address`, `TxHash`, `TokenAmount`, etc.
- 🔍 Link-tracing algorithms: shortest paths, all paths, N-hop neighbors
- 💧 Max flow analysis: Edmonds–Karp algorithm for funds analysis and AML
- 📝 ISO GQL 39075: core graph query language features supported

## Quick Start

### Option A — Docker Compose (recommended)

```bash
# clone repository
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# start services
docker compose up -d

# follow logs
docker compose logs -f

# stop services
docker compose down
```

### 方式二：预构建镜像

```bash
# 拉取镜像
docker pull ghcr.io/muyiyong/chaingraph:latest

# 创建数据卷
docker volume create chaingraph-data

# 启动服务
docker run -d \
  --name chaingraph \
  -p 8080:8080 \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest
```

## 🖥️ 使用 CLI

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

# 使用 Docker Compose
docker compose --profile import run --rm chaingraph-import

# 或直接使用 Docker
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/import:/import:ro \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-import -d /data -i /import/your_data.csv
```

## 🔌 REST API

服务启动后通过 http://localhost:8080 访问：

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
-- 查找所有账户
MATCH (n:Account) RETURN n LIMIT 100

-- 查找转账关系
MATCH (a:Account)-[t:Transfer]->(b:Account) 
RETURN a, t, b LIMIT 50
```

### 链路追踪

```gql
-- 查找两个地址之间的转账路径 (ISO GQL 39075 量词语法)
MATCH path = (a:Account)-[:Transfer]->{1,5}(b:Account)
WHERE a.address = "0xAAA..." AND b.address = "0xBBB..."
RETURN path
```

### 数据写入

```gql
-- 插入账户顶点
INSERT (alice:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})

-- 插入转账边
INSERT (a)-[:Transfer {amount: 1000}]->(b)
```

### 过程调用

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
-- 查看所有图
SHOW GRAPHS

-- 查看所有标签
SHOW LABELS

-- 查看图详情
DESCRIBE GRAPH myGraph
```

更多 GQL 语法详见 [用户手册](docs/manual.md)

## 💾 数据持久化

数据存储在 Docker Volume 中：

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
│                    Docker Container                      │
├─────────────────────────────────────────────────────────┤
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
│                    Storage Engine                        │
│   ┌──────────────┐  ┌──────────────┐  ┌────────────┐    │
│   │ Buffer Pool  │  │ Disk Storage │  │    Page    │    │
│   │    (LRU)     │  │   (mmap)     │  │   (4KB)    │    │
│   └──────────────┘  └──────────────┘  └────────────┘    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
                   ┌──────────────┐
                   │ Docker Volume │
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
