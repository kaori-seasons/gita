# Rust Edge Compute - Production Dockerfile
# 使用多阶段构建优化镜像大小

# 构建阶段
FROM rust:1.70-slim as builder

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 复制源代码
COPY src ./src

# 构建应用
RUN cargo build --release

# 生产镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户
RUN useradd -r -s /bin/false rustapp

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/rust-edge-compute /app/rust-edge-compute

# 复制配置文件模板
COPY docker/config ./config/

# 复制启动脚本
COPY docker/start.sh ./start.sh
RUN chmod +x ./start.sh

# 创建数据目录
RUN mkdir -p ./data ./logs ./certs

# 设置权限
RUN chown -R rustapp:rustapp /app
USER rustapp

# 暴露端口
EXPOSE 3000
EXPOSE 443

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/v1/health || exit 1

# 启动应用
CMD ["./start.sh"]
