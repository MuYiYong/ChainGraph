#!/bin/bash
# ============================================================
# ChainGraph 示例数据导入脚本
# ============================================================
# 用法: ./import_sample_data.sh [数据目录]
# 默认数据目录: ./data
# ============================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DATA_DIR="${1:-$PROJECT_DIR/data}"
CSV_FILE="$SCRIPT_DIR/sample_transfers.csv"

echo "=============================================="
echo "ChainGraph 示例数据导入"
echo "=============================================="
echo "项目目录: $PROJECT_DIR"
echo "数据目录: $DATA_DIR"
echo "数据文件: $CSV_FILE"
echo "=============================================="

# 检查 CSV 文件是否存在
if [ ! -f "$CSV_FILE" ]; then
    echo "错误: 找不到样例数据文件 $CSV_FILE"
    exit 1
fi

# 创建数据目录
mkdir -p "$DATA_DIR"

# 检查是否需要编译
IMPORT_BIN="$PROJECT_DIR/target/release/chaingraph-import"
if [ ! -f "$IMPORT_BIN" ]; then
    echo ""
    echo ">>> 正在编译..."
    cd "$PROJECT_DIR"
    cargo build --release --bin chaingraph-import
fi

echo ""
echo ">>> 正在导入数据..."

# 执行导入
"$IMPORT_BIN" \
    --input "$CSV_FILE" \
    --data-dir "$DATA_DIR" \
    --format csv

echo ""
echo "=============================================="
echo "导入完成!"
echo "=============================================="
echo ""
echo "现在可以使用以下命令启动服务:"
echo ""
echo "  # 启动 CLI"
echo "  $PROJECT_DIR/target/release/chaingraph-cli -d $DATA_DIR"
echo ""
echo "  # 启动 HTTP 服务器"
echo "  $PROJECT_DIR/target/release/chaingraph-server -d $DATA_DIR"
echo ""
echo "  # 运行演示"
echo "  cargo run --release --example demo"
echo ""
