# CHANGELOG

本项目的所有重要变更均会记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [1.2.0] - 2026-06-23

### Added

- 审计日志全面标准化改造
- 实现 SSE 实时日志推送，支持列表自动刷新和会话详情增量更新
- 实现账号自动恢复定时任务及审计日志
- 支持 OpenAI 协议渲染和客户端类型展示
- 新增 client_type 列并清理冗余客户端解析字段
- 集成管线、Token 解析、接入点更新 API 和字段清理
- 新增 OpenAI 协议适配、ClientType 识别和客户端字段清理

### Changed

- 请求日志详情 OpenAI Input 结构化渲染
- 优化 OpenAI 响应体解析和会话轮次展示
- 重构 execute 控制流为骨架式调度
- 重构日志记录器和后台写入调度
- 重构协议适配层和请求侧数据结构

### Documentation

- 同步架构文档和 CLI 配置
- 同步代理转发链路重构后的文档

### Fixed

- Resolve clippy warnings (too_many_arguments + unnecessary_map_or)

### Miscellaneous

- Bump version to 1.2.0
- Add CHANGELOG for 1.1.0

## [1.1.0] - 2026-06-23

### Added

- 重做 Dashboard 数据分析功能 **BREAKING**
- 实现优雅关闭以支持 K8s 零中断滚动更新

### Changed

- ProfilePage 路由从 /settings/profile 移为 /profile

### Fixed

- 修复上游响应超时控制并增强响应体格式支持
- 修复创建接入点表单账号池无法选择服务商及模型路由未自动添加未匹配行
- 补充 CHANGELOG 缺失的版本号头部并修复 body 模板

### Miscellaneous

- Bump version to 1.1.0
- 完善 git-cliff commit_parsers 配置，关闭 filter_commits
- Add CHANGELOG for 1.0.2
- 修复新 tag 发布时 Docker 构建缓存完全失效的问题
- Add CHANGELOG for 1.0.1
- Add CHANGELOG for 1.0.0

## [1.0.2] - 2026-06-22

### Fixed

- 修复上游响应超时控制并增强响应体格式支持

### Miscellaneous

- Bump version to 1.0.2

## [1.0.1] - 2026-06-22

### Fixed

- 修复创建接入点表单账号池无法选择服务商及模型路由未自动添加未匹配行

### Miscellaneous

- Bump version to 1.0.1

## [1.0.0] - 2026-06-22

### Added

- 实现账户池模式，支持多账户路由、会话粘滞和自动故障检测
- 日志列表 token 字段补齐至 6 类，提取 TokenCell 组件
- 支持动态设置日志保留月数，log_contents 按月分区
- 接入点表格增加复制链接和复制命令按钮
- 嵌入前端静态文件至后端二进制，配置 SPA fallback 路由
- 支持修改备注与管理端吊销
- 实现 JWT 自动刷新与过期令牌清理
- 模型列表与默认模型选择器统一蓝色 Tag 展示
- 日志详情改为新标签页打开
- 接入点表格 ID 可复制 + 侧栏路由高亮 + Modal 布局优化
- 重构日志解析与展示系统
- 添加请求详情内容复制按钮
- 优化请求日志表格标识字段展示
- 重构日志事件流和用量统计
- 添加日志页面刷新按钮
- 添加主题切换
- 优化模型映射配置交互
- 支持默认模型兜底映射
- 添加个人设置和用户 API key 认证
- 登录响应返回用户名和显示名称
- 实现管理后台全部功能页面
- 添加统计接口、审计日志和模型自动发现
- 实现管理后台前端
- 实现后端核心系统 (DDD 四层架构)

### Changed

- 优化请求日志详情展示
- ProxyLogger 迁入基础设施层，引入 ProxyLogData DTO，LogEntry 更名为 LogMetadata
- 透明代理管道重构，ProxyLogger 统一日志积累
- 统一文件命名与模块组织，对齐 domain 层惯例
- 按聚合重组应用层并引入 LogContext 防腐层
- 用 RequestSnapshot 值对象替代 ApiProtocol 并改进日志 Schema
- 提取 ProcessedRequest 并重构 ApiProtocol 封装
- 适配 default_model 迁移，移除 __default_model__ 哨兵
- 将 default_model 从 Provider 迁移到 AccessPoint，删除 __default_model__ 哨兵
- 按聚合边界重组领域层
- 将代理转发管道从过程式重构为 DDD 结构
- 重构 API 路由为 RESTful 资源导向风格
- 合并 domain 和 infrastructure 实体层，消除手动类型转换
- 拆分日志页面组件架构
- 将日志内容解析从后端迁移至前端
- SideSheet 改用 size 替代固定宽度
- 提升前端项目到根目录
- 拆分页面组件结构
- 适配后端 API 契约并改进交互体验
- 使用应用层分区管理替代 pg_partman 扩展
- 迁移系统移入主项目并添加 CLI 支持

### Documentation

- Bump the docker-images group with 2 updates
- 同步 PR #9 依赖升级后的架构变更
- 同步 CLAUDE.md 和 ARCHITECTURE.md，记录新工具链与发布流程
- 同步项目文档，反映代理管道重构
- 重写 README，替换 Vite 模板内容为项目文档
- 将所有英文注释替换为中文注释
- 同步模型映射架构说明
- 同步用户 API key 架构说明
- 添加项目架构文档

### Fixed

- 修复 jsonwebtoken CryptoProvider 未安装导致 panic
- 消除所有 TypeScript 和 ESLint 检查错误
- 添加缺失的 prismjs 依赖，修复 Vite 预构建错误
- 修正 lint-staged 中 cargo fmt 参数为 cargo fmt --
- 修复 SeaORM 2.0 Model.into() 导致 UPDATE 空操作的 bug
- 修复密码修改时 audit_logs.details 为 NULL 违反 NOT NULL 约束
- 修复公开路由处理器因状态类型擦除被 JWT 中间件拦截
- 修复 models JSON 列类型不匹配
- 修正 refresh_token 落库时使用 access 寿命
- 修复 Claude Haiku 前缀规则匹配
- 优化 Provider 默认模型表单交互
- 优化表单弹窗关闭和宽度
- 完善账号 API key 解密流程
- 调整上游请求认证头构造
- 优化布局和表单交互
- 调整接入点表格映射规则列宽
- 防止异步按钮重复触发
- Table 组件添加水平滚动支持并补齐列宽
- 修复 Account 加密 Key 存储并完善模型自动发现
- 修复迁移文件中的列定义错误

### Miscellaneous

- Bump version to 1.0.0
- 新增 Docker 镜像自动更新监控
- Bump the npm-deps group across 1 directory with 2 updates
- 重构 CI 流水线为按路径触发的独立工作流
- 将 Cargo.lock 提交到版本控制
- 重构发布流程为单提交模式，修复 git-cliff 配置
- 应用 Prettier 格式化所有前端源文件
- 应用 cargo fmt 格式化所有 Rust 源文件
- 跟踪 .claude/skills, 添加 release 技能, 更新 Cargo.toml 元数据
- 重构工作流为 PR 检查与发布管道，添加 Dependabot
- 添加 rust-toolchain.toml, cliff.toml, .dockerignore
- 添加 Prettier, simple-git-hooks, lint-staged 配置
- 添加 Apache 2.0 许可证
- 关闭 provenance 和 sbom 产出，仅推送单镜像
- Cargo fmt 代码格式化
- 开发环境使用 cargo watch 自动重编译，日志变量改用 RUST_LOG
- 整合迁移文件，移除已废弃解析器列和会话事件表
- 为表格名称和模型列设置固定宽度
- 简化触发条件和镜像标签策略，仅保留语义化版本标签
- 优化工作流触发条件和镜像标签策略
- 优化多阶段构建流程并添加 CI 发布工作流
- SessionListView 请求次数列宽调整
- 删除已合并的旧迁移文件
- 抑制 LogMetadata 枚举的未使用变体警告
- 初始化项目配置和构建系统

