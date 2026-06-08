# Token Proxy

企业级 LLM API 代理网关 — 统一入口、集中管理、完全可控。

## 解决的问题

当你需要将 LLM API 开放给团队或外部用户时，会面临三个痛点：

- **密钥泄漏风险**：上游 API Key 直接暴露给使用者，难以轮换和吊销
- **模型不可控**：使用者可以随意切换模型，无法限制可用范围和映射关系
- **用量黑盒**：缺乏统一的请求日志和 Token 消耗统计

Token Proxy 在你的使用者与上游 LLM 提供商之间建立一个代理层，你配置接入点和映射规则，使用者只需拿到一个短码 URL 即可调用，上游密钥、模型策略、用量追踪全部由你掌控。

## 能力

- **接入点管理**：为不同团队或场景创建独立的接入点，每个接入点指向特定的提供商和账号
- **模型映射**：将使用者请求的模型按规则映射到上游实际模型，支持精确匹配、前缀匹配和通配符兜底
- **用户 API Key**：为每个使用者签发独立的 API Key，支持随时吊销，泄漏后可快速止损
- **用量统计**：完整的请求日志、会话时间线和 Token 消耗统计，按天聚合、按月归档
- **管理面板**：Web 界面管理 Provider、Account、AccessPoint、用户和 API Key，查看日志和会话详情

## 使用流程

```
管理员配置 Provider → 添加 Account → 创建 AccessPoint → 生成短码
                                                              ↓
使用者拿到短码 + 用户 API Key → 调用 /ap/{short_code}/v1/messages → Token Proxy 转发到上游
```

使用者无需知道上游是哪家提供商、使用什么 API Key，只需一个短码 URL 和用户 API Key 即可调用。

### 接入示例

假设你配置了一个接入点，短码为 `my-team`，映射规则为 `claude-sonnet-* → claude-sonnet-4-6`：

```bash
curl https://your-domain/ap/my-team/v1/messages \
  -H "Authorization: Bearer tp_xxxxxxxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-7",
    "max_tokens": 1024,
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

使用者请求的 `claude-sonnet-4-7` 会被自动映射为 `claude-sonnet-4-6`，上游请求使用你配置的账号 Key，使用者全程无感知。

## 部署

### Docker Compose（推荐）

```bash
git clone https://github.com/your-org/token-proxy.git
cd token-proxy

# 配置环境变量
cat > .env << EOF
DATABASE_URL=postgres://tokenproxy:password@db:5432/tokenproxy
JWT_SECRET=$(openssl rand -hex 32)
ENCRYPTION_KEY=$(openssl rand -hex 32)
EOF

docker compose up -d
```

服务启动后访问 `http://localhost:3000`，使用默认管理员账号登录。

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `DATABASE_URL` | PostgreSQL 连接串 | 必填 |
| `JWT_SECRET` | JWT 签名密钥 | 必填 |
| `ENCRYPTION_KEY` | API Key 加密密钥 (64 位十六进制) | 必填 |
| `SERVER_PORT` | 监听端口 | `3000` |
| `LOG_LEVEL` | 日志级别 | `info` |
| `PARTITION_CHECK_INTERVAL_SECS` | 分区检查间隔 | `3600` |
| `PARTITION_PREMAKE_MONTHS` | 预创建未来分区数 | `3` |
| `PARTITION_RETENTION_MONTHS` | 分区保留月数 | `12` |

## 开发者

- [ARCHITECTURE.md](./ARCHITECTURE.md) — 架构设计与开发指南
- [CLAUDE.md](./CLAUDE.md) — AI 辅助开发配置

## License

MIT
