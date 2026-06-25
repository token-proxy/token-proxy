//! Dashboard 应用层模块（个人视角）。
//!
//! 提供面向当前登录用户的数据洞察视图，包括：
//! - KPI 卡片：请求数 / 词元量 / 词元构成 / 缓存命中率（含 sparkline）
//! - 近 1 年日级热力图（按用户时区分桶）
//! - 模型 Top 8 / 接入点 Top 5
//! - 调用质量指标（成功率、错误率、中断率、p95 耗时）
//!
//! 所有数据源均按 `user_id` 维度过滤，支持统一时间范围过滤器（今日 / 7 天 / 30 天 / 自定义）。
//!
//! ## 分层职责
//!
//! - `dto/` —— 请求 / 响应 DTO（无业务逻辑）
//! - `dashboard_service.rs` —— Service 编排层
//! - `time_window.rs` —— 时间窗口解析纯函数
//! - `timezone.rs` —— IANA 时区白名单校验（热力图端点专用）

pub mod dashboard_service;
pub mod dto;
pub mod time_window;
pub mod timezone;

pub use dashboard_service::DashboardService;
pub use dto::*;
