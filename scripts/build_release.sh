#!/bin/bash
#
# ChainGraph Release 打包脚本
# 用法: ./scripts/build_release.sh [version]
#

set -e

# 默认版本
VERSION="${1:-0.1.0}"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="${PROJECT_ROOT}/release"
PACKAGE_NAME="chaingraph-${VERSION}"
PACKAGE_DIR="${RELEASE_DIR}/${PACKAGE_NAME}"

echo "═══════════════════════════════════════════════════════════════"
echo "  ChainGraph Release Build v${VERSION}"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# 清理旧的 release 目录
echo "📦 准备打包目录..."
rm -rf "${RELEASE_DIR}"
mkdir -p "${PACKAGE_DIR}"
mkdir -p "${PACKAGE_DIR}/bin"
mkdir -p "${PACKAGE_DIR}/docs"
mkdir -p "${PACKAGE_DIR}/examples"
mkdir -p "${PACKAGE_DIR}/data"

# 编译 Release 版本
echo ""
echo "🔨 编译 Release 版本..."
cd "${PROJECT_ROOT}"
cargo build --release

# 复制可执行文件
echo ""
echo "📋 复制可执行文件..."
cp target/release/chaingraph-server "${PACKAGE_DIR}/bin/"
cp target/release/chaingraph-cli "${PACKAGE_DIR}/bin/"
cp target/release/chaingraph-import "${PACKAGE_DIR}/bin/"

# 复制文档
echo "📄 复制文档..."
cp README.md "${PACKAGE_DIR}/"
cp -r docs/html "${PACKAGE_DIR}/docs/" 2>/dev/null || true
cp docs/manual.md "${PACKAGE_DIR}/docs/"

# 复制示例
echo "📝 复制示例..."
cp examples/*.sh "${PACKAGE_DIR}/examples/" 2>/dev/null || true
cp examples/*.gql "${PACKAGE_DIR}/examples/" 2>/dev/null || true
cp examples/*.csv "${PACKAGE_DIR}/examples/" 2>/dev/null || true

# 创建启动脚本
echo "🚀 创建启动脚本..."
cat > "${PACKAGE_DIR}/start-server.sh" << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"${SCRIPT_DIR}/bin/chaingraph-server" --data-dir "${SCRIPT_DIR}/data" "$@"
EOF
chmod +x "${PACKAGE_DIR}/start-server.sh"

cat > "${PACKAGE_DIR}/cli.sh" << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"${SCRIPT_DIR}/bin/chaingraph-cli" --data-dir "${SCRIPT_DIR}/data" "$@"
EOF
chmod +x "${PACKAGE_DIR}/cli.sh"

# 创建版本文件
echo "${VERSION}" > "${PACKAGE_DIR}/VERSION"

# 创建 QUICKSTART.md
cat > "${PACKAGE_DIR}/QUICKSTART.md" << 'EOF'
# ChainGraph 快速开始

## 启动服务器

```bash
# 使用默认配置启动
./start-server.sh

# 指定端口
./start-server.sh --port 9090

# 指定数据目录
./start-server.sh --data-dir /path/to/data
```

## 使用 CLI

```bash
# 交互式 CLI
./cli.sh

# 执行单条查询
./cli.sh -c "MATCH (n:Account) RETURN n LIMIT 10"
```

## 数据导入

```bash
# 导入 CSV 数据
./bin/chaingraph-import --data-dir ./data --file examples/sample_transfers.csv
```

## REST API

```bash
# 执行查询
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (n:Account) RETURN n LIMIT 10"}'

# 健康检查
curl http://localhost:8080/health
```

## 文档

- [README](README.md) - 项目概述
- [产品手册](docs/manual.md) - 完整文档
- [HTML 文档](docs/html/index.html) - 在浏览器中阅读

## GQL 示例

```gql
-- 查询账户
MATCH (n:Account) RETURN n LIMIT 100

-- 查询转账
MATCH (a:Account)-[t:Transfer]->(b:Account) RETURN a, t, b

-- 最短路径
CALL shortest_path(1, 100)

-- 变量绑定
LET x = 10, name = "Alice"

-- 分组统计
SELECT n.type, COUNT(*) GROUP BY n.type HAVING COUNT(*) > 5
```
EOF

# 打包
echo ""
echo "📦 创建压缩包..."
cd "${RELEASE_DIR}"
tar -czvf "${PACKAGE_NAME}-macos.tar.gz" "${PACKAGE_NAME}"

# 创建 zip 包
zip -r "${PACKAGE_NAME}-macos.zip" "${PACKAGE_NAME}"

# 计算校验和
echo ""
echo "🔐 计算校验和..."
shasum -a 256 "${PACKAGE_NAME}-macos.tar.gz" > "${PACKAGE_NAME}-macos.tar.gz.sha256"
shasum -a 256 "${PACKAGE_NAME}-macos.zip" > "${PACKAGE_NAME}-macos.zip.sha256"

# 显示结果
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ✅ Release 打包完成！"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "📁 输出目录: ${RELEASE_DIR}"
echo ""
echo "📦 生成的文件:"
ls -lh "${RELEASE_DIR}"/*.tar.gz "${RELEASE_DIR}"/*.zip 2>/dev/null
echo ""
echo "📄 目录结构:"
find "${PACKAGE_DIR}" -type f | head -20
echo ""
echo "🚀 使用方法:"
echo "   tar -xzf ${PACKAGE_NAME}-macos.tar.gz"
echo "   cd ${PACKAGE_NAME}"
echo "   ./start-server.sh"
