# ─── 阶段 1: 构建前端 ─────────────────────────────
FROM node:26-alpine AS frontend-builder

WORKDIR /app

# 安装 pnpm，版本与 package.json 中 packageManager 声明的 pnpm@11.5.2 保持一致
RUN npm install -g pnpm@11

# 清单文件优先复制，配合 pnpm store 缓存挂载加速依赖安装
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && \
    pnpm install --frozen-lockfile --ignore-scripts

COPY index.html tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts eslint.config.js ./
COPY public/ public/
COPY src-dashboard/ src-dashboard/
RUN pnpm run build

# ─── 阶段 2: 构建后端 ─────────────────────────────
FROM rust:1.96-alpine AS backend-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# 复制清单和源码，Cargo.lock 确保依赖版本与 CI 的 rust-cache 一致
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY --from=frontend-builder /app/dist dist/

# 通过 cache mount 持久化 Cargo registry 和编译产物，跨 Docker 层共享
# 版本号变更时 cargo 做增量重编译而非全量重建
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    install -D /app/target/release/token-proxy /out/token-proxy

# ─── 阶段 3: 运行时镜像 ────────────────────────────
FROM alpine:3.24

# wget 用于 HEALTHCHECK（Alpine 默认带 BusyBox wget，比 curl 更轻）
RUN apk add --no-cache ca-certificates tzdata libgcc

ENV TZ=Asia/Shanghai
ENV SERVER_PORT=3000
ENV LOG_LEVEL=info

COPY --from=backend-builder /out/token-proxy /usr/local/bin/token-proxy

EXPOSE ${SERVER_PORT}

# 健康检查：调用 /api/health 端点
# - 关闭中时端点返回 {"status":"shutting_down"}，仍返回 200，HEALTHCHECK 视为健康
#   （避免 Docker 在优雅关闭期间重启容器；K8s 用单独的 /api/ready 实现摘除流量）
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:${SERVER_PORT}/api/health || exit 1

ENTRYPOINT ["token-proxy"]
