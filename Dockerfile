# ============================================
# ChainGraph - Multi-stage Docker Build
# 使用依赖缓存优化，减少重复下载
# ============================================

# Stage 1: Build
FROM rust:latest AS builder

WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 1. 先只复制 Cargo 配置文件
COPY Cargo.toml Cargo.lock ./

# 2. 创建虚拟源文件结构，让 cargo 下载依赖
RUN mkdir -p src/bin && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "fn main() {}" > src/bin/cli.rs && \
    echo "fn main() {}" > src/bin/import.rs

# 3. 预编译依赖（这一层会被缓存）
RUN cargo build --release || true
RUN rm -rf src

# 4. 复制实际源码
COPY src ./src

# 5. 重新编译（只编译项目代码，依赖已缓存）
RUN touch src/lib.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
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
