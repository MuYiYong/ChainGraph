
# ChainGraph（开发中，请勿直接用于生产）

<p align="center">
  <img src="https://github.com/MuYiYong/ChainGraph/actions/workflows/ci.yml/badge.svg" alt="CI">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/deployment-Docker-blue.svg" alt="Docker">
</p>

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

## 🚀 快速开始

### 方式一 — Docker Compose（推荐）

```bash
# 克隆仓库
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph

# 启动服务
docker compose up -d

# 查看日志
docker compose logs -f

# 停止服务
docker compose down
```

### 方式二 — 预构建镜像

```bash
# 拉取镜像
docker pull ghcr.io/muyiyong/chaingraph:latest

# 创建数据卷
docker volume create chaingraph-data

# 启动容器
docker run -d \
  --name chaingraph \
  -p 8080:8080 \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest
```

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

-- 查找转账关系
MATCH (a:Account)-[t:Transfer]->(b:Account)
RETURN a, t, b LIMIT 50
```

### 链路追踪

```gql
-- 查找两个地址之间的转账路径（ISO GQL 39075 量词语法）
MATCH path = (a:Account)-[:Transfer]->{1,5}(b:Account)
WHERE a.address = "0xAAA..." AND b.address = "0xBBB..."
RETURN path
```

### 写入数据

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
