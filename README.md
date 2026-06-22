# Token Proxy

企业级 LLM API 资源管理平台——统一入口、集中管理、完全可控。

## 解决的问题

将 LLM API 开放给团队或外部用户时，你会面临三个痛点：

- **密钥泄漏风险**：上游 API Key 直接暴露给使用者，难以轮换和吊销
- **模型不可控**：使用者可以随意切换模型，无法限制可用范围和映射关系
- **用量黑盒**：缺乏统一的请求日志和 Token 消耗统计

Token Proxy 在你与上游 LLM 提供商之间建立一个代理层。你配置接入点和映射规则，使用者拿到一个短码 URL 即可调用——上游密钥、模型策略、用量追踪全部由你掌控。

## 核心能力

### 接入点管理

为不同团队或场景创建独立的接入点。每个接入点绑定一个账户池（多个 API 账号），支持按优先级或权重分配流量，单个账号故障时自动切换到下一个可用账号。

### 模型路由

通过二维路由网格精确控制模型映射。每个接入点可针对不同 Provider 定义差异化的目标模型，支持精确匹配、前缀匹配和通配符兜底三级策略。使用者请求的模型会被自动映射为你指定的上游模型，全程无感知。

### 账户池与故障转移

一个接入点可配置多个 API 账号，系统按优先级排序后依次尝试。遇到 429（限流）、402（余额不足）、502/503（上游故障）时自动重试下一个账号。检测到故障的账号会被自动禁用，并在冷却期后自动恢复。

### 用户 API Key

为每个使用者签发独立的 API Key，支持随时吊销。Key 泄漏后无需更换上游账号密钥，吊销对应 Key 即可止损。

### 用量统计

完整的请求日志、会话时间线和 Token 消耗统计。Dashboard 提供总览面板和趋势图表，支持按接入点、模型、用户等维度查看用量分布。

### 管理面板

Web 界面一站式管理 Provider、账号、接入点、用户和 API Key，查看请求日志和会话详情。无需直接操作数据库或调用 API。

## 快速开始

### Docker 部署

镜像通过 GitHub Container Registry 发布：

```bash
docker run -d --name token-proxy \
  -p 3000:3000 \
  -e DATABASE_URL='postgres://user:pass@host:5432/tokenproxy' \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e ENCRYPTION_KEY="$(openssl rand -hex 32)" \
  ghcr.io/your-org/token-proxy:latest
```

服务启动后访问 `http://localhost:3000`，首次启动会在容器日志中输出默认管理员密码，使用 `admin` 账号登录即可。

数据库需自行准备（推荐 PostgreSQL 17）；应用启动时自动执行迁移。

### Kubernetes 部署

应用在收到 `SIGTERM` 后会：先把 `/api/ready` 返回 503 让 K8s 摘除流量，然后等所有正在进行的请求（含 SSE 长连接）自然完成，并确保所有异步日志写入落库后才退出。要让 K8s 配合这个流程，至少需要在 Pod 模板中加入以下字段：

```yaml
spec:
  # 不设硬性超时；600 秒给最长 SSE 流足够空间（默认 30 秒会过早强杀）
  terminationGracePeriodSeconds: 600
  containers:
    - name: token-proxy
      image: ghcr.io/your-org/token-proxy:latest
      ports:
        - containerPort: 3000
      # 关闭信号到达后立即返回 503，K8s 从 Service 摘除 Pod
      readinessProbe:
        httpGet:
          path: /api/ready
          port: 3000
        periodSeconds: 5
      # 关闭中仍返回 200，避免被 K8s 误判重启
      livenessProbe:
        httpGet:
          path: /api/health
          port: 3000
        periodSeconds: 15
```

配合 `Deployment.spec.strategy.rollingUpdate.maxUnavailable: 0` 可实现零中断滚动更新。

### 配置流程

```
创建 Provider → 添加 API 账号 → 创建接入点 → 分配账号 → 配置模型路由 → 生成短码
                                                                              ↓
使用者拿到短码 + 用户 API Key → 调用 /ap/{short_code}/v1/messages → 代理转发到上游
```

1. **创建 Provider**：填写提供商名称和 API 地址（支持 Anthropic 等）
2. **添加账号**：录入上游 API Key（加密存储），配置权重和优先级
3. **创建接入点**：指定短码、API 类型，选择账户池中的账号
4. **配置模型路由**：设置源模型到目标模型的映射规则
5. **签发用户 Key**：为使用者创建 API Key，关联到对应接入点

### 调用示例

假设接入点短码为 `my-team`，映射规则为 `claude-sonnet-* → claude-sonnet-4-6`：

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

使用者请求的 `claude-sonnet-4-7` 会被自动映射为 `claude-sonnet-4-6`，上游请求使用你配置的账号密钥。

## 环境变量

| 变量                            | 说明                                            | 默认值 |
| ------------------------------- | ----------------------------------------------- | ------ |
| `DATABASE_URL`                  | PostgreSQL 连接串                               | 必填   |
| `JWT_SECRET`                    | JWT 签名密钥                                    | 必填   |
| `ENCRYPTION_KEY`                | API Key 加密密钥（64 位十六进制）               | 必填   |
| `SERVER_PORT`                   | 监听端口                                        | `3000` |
| `LOG_LEVEL`                     | 日志级别（trace / debug / info / warn / error） | `info` |
| `PARTITION_CHECK_INTERVAL_SECS` | 分区检查间隔（秒）                              | `3600` |
| `PARTITION_PREMAKE_MONTHS`      | 预创建未来分区月数                              | `3`    |
| `PARTITION_RETENTION_MONTHS`    | 日志分区保留月数（可在管理面板中动态调整）      | `12`   |

## 项目状态

当前已完成 Phase 1 MVP，功能稳定可用：

- 代理转发：多账号故障转移、会话粘滞、流式/非流式透明代理
- 管理面板：Provider 管理、接入点管理、用户管理、API Key 管理
- 日志系统：请求日志、会话时间线、Token 用量统计、Dashboard 概览
- 安全机制：AES-256-GCM 加密存储、JWT 双令牌认证、API Key SHA-256 哈希

## 开发者文档

- [ARCHITECTURE.md](./ARCHITECTURE.md) — 架构设计与技术细节
- [CLAUDE.md](./CLAUDE.md) — AI 辅助开发配置

## License

MIT
