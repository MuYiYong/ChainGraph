# ============================================
# ChainGraph - Multi-stage Docker Build
# ============================================

# Stage 1: Build
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# 安装依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 先复制 Cargo 文件以利用缓存
COPY Cargo.toml Cargo.lock ./

# 创建虚拟源文件以构建依赖
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "fn main() {}" > src/bin/cli.rs && \
    echo "fn main() {}" > src/bin/import.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# 复制实际源码并构建
COPY src ./src
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 chaingraph

WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/chaingraph-server /usr/local/bin/
COPY --from=builder /app/target/release/chaingraph-cli /usr/local/bin/
COPY --from=builder /app/target/release/chaingraph-import /usr/local/bin/

# 创建数据目录
RUN mkdir -p /data && chown chaingraph:chaingraph /data

# 切换到非 root 用户
USER chaingraph

# 数据卷
VOLUME ["/data"]

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 默认启动服务器
ENTRYPOINT ["chaingraph-server"]
CMD ["-d", "/data", "-p", "8080", "-H", "0.0.0.0"]
