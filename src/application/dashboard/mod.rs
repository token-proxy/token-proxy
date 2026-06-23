//! Dashboard 应用层模块。
//!
//! 提供面向技术主管的数据洞察视图，包括：
//! - 顶部 4 张 KPI 卡（请求数 / Token 量 / 活跃成员数 / 缓存命中率）
//! - 三条 sparkline 时间序列（内嵌于 KpiResponse）
//! - 成员请求量排行 Top 10
//! - 上游账号 Token 消耗排行 Top 10
//!
//! 所有数据源支持统一时间范围过滤器（今日 / 7 天 / 30 天 / 自定义）。
//!
//! ## 分层职责
//!
//! - `dto/` —— 请求 / 响应 DTO（无业务逻辑）
//! - `dashboard_service.rs` —— Service 编排层
//! - `time_window.rs` —— 时间窗口解析纯函数

pub mod dashboard_service;
pub mod dto;
pub mod time_window;

pub use dashboard_service::DashboardService;
pub use dto::*;
