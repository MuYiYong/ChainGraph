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

# 复制所有源码
COPY . .

# 构建 release 版本
RUN cargo build --release

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
