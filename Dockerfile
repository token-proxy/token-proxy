# ─── 阶段 1: 构建前端 ─────────────────────────────
FROM node:22-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ─── 阶段 2: 构建后端 ─────────────────────────────
FROM rust:1.89-alpine AS backend-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY migration/ migration/
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY src/ src/
COPY --from=frontend-builder /app/frontend/dist frontend/dist/

RUN cargo build --release

# ─── 阶段 3: 运行时镜像 ────────────────────────────
FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata libgcc

ENV TZ=Asia/Shanghai
ENV SERVER_PORT=3000
ENV LOG_LEVEL=info

COPY --from=backend-builder /app/target/release/token-proxy /usr/local/bin/token-proxy

EXPOSE ${SERVER_PORT}

ENTRYPOINT ["token-proxy"]