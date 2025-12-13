# ChainGraph


<div align="center" style="margin-bottom: 8px;">
  <a href="./README.md" title="English" style="text-decoration:none;">
    <span style="display:inline-block;padding:8px 28px;margin:4px 8px;border:2px solid #1e90ff;border-radius:6px;font-size:1.08em;font-weight:600;background:#f8faff;color:#1e90ff;">English</span>
  </a>
  <a href="./README.zh-CN.md" title="中文" style="text-decoration:none;">
    <span style="display:inline-block;padding:8px 28px;margin:4px 8px;border:2px solid #ff4d4f;border-radius:6px;font-size:1.08em;font-weight:600;background:#fff8f8;color:#ff4d4f;">中文</span>
  </a>
</div>

<p align="center">
  <img src="https://github.com/MuYiYong/ChainGraph/actions/workflows/ci.yml/badge.svg" alt="CI">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/deployment-Docker-blue.svg" alt="Docker">
</p>

ChainGraph is a high-performance graph database designed for Web3 scenarios, focused on on-chain link tracing and funds-flow analysis.

> ⚠️ ChainGraph is provided as a Docker containerized service only.

---

**Language / 语言**:  
- English (default): this `README.md` — click to view English content.  
- 中文: see `README.zh-CN.md` — click to view Chinese content.

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

### Option B — Prebuilt image

```bash
# pull image
docker pull ghcr.io/muyiyong/chaingraph:latest

# create volume
docker volume create chaingraph-data

# start container
docker run -d \
  --name chaingraph \
  -p 8080:8080 \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest
```

## 🖥️ CLI Usage

```bash
# Docker Compose
docker compose run --rm chaingraph-cli

# Direct Docker
docker run -it --rm \
  -v chaingraph-data:/data \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-cli -d /data
```

## 📥 Import Data

```bash
# place your data file into the import directory
mkdir -p import
cp your_data.csv import/

# using Docker Compose
docker compose --profile import run --rm chaingraph-import

# or using Docker directly
docker run --rm \
  -v chaingraph-data:/data \
  -v $(pwd)/import:/import:ro \
  ghcr.io/muyiyong/chaingraph:latest \
  chaingraph-import -d /data -i /import/your_data.csv
```

## 🔌 REST API

After the service starts, access the API at `http://localhost:8080`:

```bash
# health check
curl http://localhost:8080/health

# execute a GQL query
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10"}'

# get statistics
curl http://localhost:8080/stats

# shortest path
curl -X POST http://localhost:8080/algorithm/shortest-path \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "target": 100}'

# max flow
curl -X POST http://localhost:8080/algorithm/max-flow \
  -H "Content-Type: application/json" \
  -d '{"source": 1, "sink": 100}'
```

## 📖 GQL Query Examples

### Basic queries

```gql
-- find accounts
MATCH (n:Account) RETURN n LIMIT 100

-- find transfers
MATCH (a:Account)-[t:Transfer]->(b:Account)
RETURN a, t, b LIMIT 50
```

### Link tracing

```gql
-- find transfer paths between two addresses (ISO GQL 39075 quantified path syntax)
MATCH path = (a:Account)-[:Transfer]->{1,5}(b:Account)
WHERE a.address = "0xAAA..." AND b.address = "0xBBB..."
RETURN path
```

### Writing data

```gql
-- insert an account vertex
INSERT (alice:Account {address: "0x742d35Cc6634C0532925a3b844Bc9e7595f3fBb0"})

-- insert a transfer edge
INSERT (a)-[:Transfer {amount: 1000}]->(b)
```

### Procedures / Calls

```gql
-- shortest path
CALL shortest_path(1, 5)

-- trace
CALL trace(1, 'forward', 5)

-- max flow
CALL max_flow(1, 100)
```

### Metadata queries

```gql
-- show graphs
SHOW GRAPHS

-- show labels
SHOW LABELS

-- describe graph
DESCRIBE GRAPH myGraph
```

See the user manual for full GQL syntax: [docs/manual.md](docs/manual.md)

## 💾 Data persistence

Data is stored in a Docker volume:

```bash
# inspect volume
docker volume inspect chaingraph-data

# backup data
docker run --rm \
  -v chaingraph-data:/data:ro \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/chaingraph-backup.tar.gz -C /data .

# restore data
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

## 📊 Data model

### Vertex labels

| Label | Description | Typical properties |
|------|------|----------|
| `Account` | EOA account | address, balance |
| `Contract` | Smart contract | address, code_hash |
| `Token` | Token contract | address, symbol |

### Edge labels

| Label | Description | Typical properties |
|------|------|----------|
| `Transfer` | Token transfer | amount, token |
| `Call` | Contract call | method, gas |

## ⚙️ Environment variables

| Variable | Default | Description |
|------|--------|------|
| `RUST_LOG` | `info` | logging level (debug, info, warn, error) |

## 📚 Documentation

- [Docker guide](DOCKER.md)
- [User manual](docs/manual.md)

## 📄 License

This project is licensed under Apache-2.0. See [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions are welcome — please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

---

<p align="center">
  Made with ❤️ for Web3 | 🐳 Container Only
</p>
