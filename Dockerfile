# ─── 阶段 1: 构建前端 ─────────────────────────────
FROM node:22-alpine AS frontend-builder

WORKDIR /app
COPY package.json ./
RUN npm install

COPY index.html tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts eslint.config.js ./
COPY public/ public/
COPY src-dashboard/ src-dashboard/
RUN npm run build

# ─── 阶段 2: 构建后端 ─────────────────────────────
FROM rust:1.96-alpine AS backend-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY src/ src/
COPY --from=frontend-builder /app/dist frontend/dist/

RUN cargo build --release

# ─── 阶段 3: 运行时镜像 ────────────────────────────
FROM alpine:3.22

RUN apk add --no-cache ca-certificates tzdata libgcc

ENV TZ=Asia/Shanghai
ENV SERVER_PORT=3000
ENV LOG_LEVEL=info

COPY --from=backend-builder /app/target/release/token-proxy /usr/local/bin/token-proxy

EXPOSE ${SERVER_PORT}

ENTRYPOINT ["token-proxy"]
