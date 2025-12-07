#!/bin/bash
set -e

# ChainGraph Linux 构建脚本
# 使用 Docker 在 macOS 上构建 Linux 二进制文件

VERSION="0.1.0"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/release"

echo "═══════════════════════════════════════════════════════════════"
echo "  ChainGraph Linux Build v${VERSION}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# 检查 Docker 是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ 错误: Docker 未安装"
    echo "请先安装 Docker Desktop: https://www.docker.com/products/docker-desktop"
    exit 1
fi

# 检查 Docker 是否运行
if ! docker info &> /dev/null; then
    echo "❌ 错误: Docker 未运行"
    echo "请启动 Docker Desktop"
    exit 1
fi

cd "$PROJECT_ROOT"

echo "🐳 使用 Docker 构建 Linux 二进制文件..."
echo ""

# 创建临时 Dockerfile
cat > Dockerfile.linux << 'DOCKERFILE'
FROM rust:1.75-bookworm

# 安装必要的依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 先复制 Cargo 文件以利用缓存
COPY Cargo.toml Cargo.lock* ./

# 创建虚拟 src 以构建依赖缓存
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# 复制实际源码
COPY . .

# 重新构建
RUN cargo build --release

# 输出构建信息
RUN ls -la target/release/chaingraph-*
DOCKERFILE

echo "📦 构建 Docker 镜像..."
docker build -t chaingraph-linux-builder -f Dockerfile.linux . 

echo ""
echo "📋 提取二进制文件..."

# 创建输出目录
LINUX_RELEASE_DIR="$OUTPUT_DIR/chaingraph-${VERSION}-linux"
rm -rf "$LINUX_RELEASE_DIR"
mkdir -p "$LINUX_RELEASE_DIR/bin"
mkdir -p "$LINUX_RELEASE_DIR/docs/html"
mkdir -p "$LINUX_RELEASE_DIR/examples"
mkdir -p "$LINUX_RELEASE_DIR/data"

# 创建临时容器并提取文件
CONTAINER_ID=$(docker create chaingraph-linux-builder)
docker cp "$CONTAINER_ID:/app/target/release/chaingraph-cli" "$LINUX_RELEASE_DIR/bin/"
docker cp "$CONTAINER_ID:/app/target/release/chaingraph-server" "$LINUX_RELEASE_DIR/bin/"
docker cp "$CONTAINER_ID:/app/target/release/chaingraph-import" "$LINUX_RELEASE_DIR/bin/"
docker rm "$CONTAINER_ID"

# 复制文档和示例
cp "$PROJECT_ROOT/README.md" "$LINUX_RELEASE_DIR/"
cp "$PROJECT_ROOT/docs/manual.md" "$LINUX_RELEASE_DIR/docs/"
cp "$PROJECT_ROOT/docs/html/"*.html "$LINUX_RELEASE_DIR/docs/html/" 2>/dev/null || true
cp "$PROJECT_ROOT/examples/sample_data.gql" "$LINUX_RELEASE_DIR/examples/"
cp "$PROJECT_ROOT/examples/sample_dml.gql" "$LINUX_RELEASE_DIR/examples/"
cp "$PROJECT_ROOT/examples/sample_transfers.csv" "$LINUX_RELEASE_DIR/examples/"
cp "$PROJECT_ROOT/examples/import_sample_data.sh" "$LINUX_RELEASE_DIR/examples/"

# 创建版本文件
echo "$VERSION" > "$LINUX_RELEASE_DIR/VERSION"

# 创建启动脚本
cat > "$LINUX_RELEASE_DIR/start-server.sh" << 'SCRIPT'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
echo "启动 ChainGraph 服务器..."
./bin/chaingraph-server -d ./data -p 8080
SCRIPT
chmod +x "$LINUX_RELEASE_DIR/start-server.sh"

cat > "$LINUX_RELEASE_DIR/cli.sh" << 'SCRIPT'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
./bin/chaingraph-cli -d ./data
SCRIPT
chmod +x "$LINUX_RELEASE_DIR/cli.sh"

# 创建快速入门指南
cat > "$LINUX_RELEASE_DIR/QUICKSTART.md" << 'GUIDE'
# ChainGraph 快速入门 (Linux)

## 1. 解压

```bash
tar -xzf chaingraph-0.1.0-linux.tar.gz
cd chaingraph-0.1.0
```

## 2. 启动服务器

```bash
./start-server.sh
```

服务器将在 http://localhost:8080 启动

## 3. 使用 CLI

```bash
./cli.sh
```

## 4. 导入示例数据

```bash
cd examples
./import_sample_data.sh
```

## 5. 执行查询

在 CLI 中:
```
query MATCH (n:Account) RETURN n LIMIT 10
```

更多信息请参阅 docs/manual.md
GUIDE

echo ""
echo "📦 创建压缩包..."

cd "$OUTPUT_DIR"
tar -czvf "chaingraph-${VERSION}-linux.tar.gz" "chaingraph-${VERSION}-linux"
zip -r "chaingraph-${VERSION}-linux.zip" "chaingraph-${VERSION}-linux"

echo ""
echo "🔐 计算校验和..."
shasum -a 256 "chaingraph-${VERSION}-linux.tar.gz" > "chaingraph-${VERSION}-linux.tar.gz.sha256"
shasum -a 256 "chaingraph-${VERSION}-linux.zip" > "chaingraph-${VERSION}-linux.zip.sha256"

# 清理
rm -f "$PROJECT_ROOT/Dockerfile.linux"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ✅ Linux Release 打包完成！"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "📁 输出目录: $OUTPUT_DIR"
echo ""
echo "📦 生成的文件:"
ls -lh "$OUTPUT_DIR/chaingraph-${VERSION}-linux.tar.gz"
ls -lh "$OUTPUT_DIR/chaingraph-${VERSION}-linux.zip"
echo ""
echo "🚀 在 Ubuntu 22.04 上使用:"
echo "   tar -xzf chaingraph-${VERSION}-linux.tar.gz"
echo "   cd chaingraph-${VERSION}"
echo "   ./start-server.sh"
