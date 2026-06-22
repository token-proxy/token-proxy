# ─── 阶段 1: 构建前端 ─────────────────────────────
FROM node:26-alpine AS frontend-builder

WORKDIR /app

# 清单文件优先复制，配合 npm 缓存挂载加速依赖安装
COPY package.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm install

COPY index.html tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts eslint.config.js ./
COPY public/ public/
COPY src-dashboard/ src-dashboard/
RUN npm run build

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

RUN apk add --no-cache ca-certificates tzdata libgcc

ENV TZ=Asia/Shanghai
ENV SERVER_PORT=3000
ENV LOG_LEVEL=info

COPY --from=backend-builder /out/token-proxy /usr/local/bin/token-proxy

EXPOSE ${SERVER_PORT}

ENTRYPOINT ["token-proxy"]
