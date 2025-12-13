# ChainGraph

<p align="center">
  <img src="https://github.com/MuYiYong/ChainGraph/actions/workflows/ci.yml/badge.svg" alt="CI">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/deployment-Docker-blue.svg" alt="Docker">
</p>

**ChainGraph** 是一款专为 Web3 场景设计的高性能图数据库，专注于链上链路追踪和资金流分析。

> ⚠️ **仅支持容器化部署**：ChainGraph 仅通过 Docker 容器方式提供服务。

## ✨ 特性

- 🐳 **容器化部署** - 仅支持 Docker，简单易用，开箱即用
- 🚀 **SSD 优化存储** - 4KB 页面对齐，LRU 缓冲池，支持海量数据存储
- 🔗 **Web3 原生类型** - 内置 `Address`、`TxHash`、`TokenAmount` 等区块链类型
- 🔍 **链路追踪算法** - 支持最短路径、所有路径、N跳邻居等多种追踪方式
- 💧 **最大流分析** - Edmonds–Karp 算法，用于资金流动分析和反洗钱检测
- 📝 **ISO GQL 39075 标准** - 完整支持图查询语言的主要特性

## 🚀 快速开始

### 方式一：Docker Compose (推荐)

```bash
# 克隆仓库
git clone https://github.com/MuYiYong/ChainGraph.git
cd ChainGraph
```

...（其余使用说明与英文版一致）

## 详细文档
请参阅 `docs/manual.md` 以获取完整使用指南与语法说明。

---

要切换到英文版本，请访问主仓库 `README.md`。
