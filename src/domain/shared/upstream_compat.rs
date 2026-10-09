//! 上游兼容策略值对象 — domain/shared/
//!
//! 描述「本服务商上游端点与最新协议规范的落差」，以及针对该落差的**确定性适配行为**。
//!
//! ## 为什么需要它
//!
//! 代理的默认契约是**透明转发**：不改动请求体语义。但现实中第三方中转/自建网关
//! 常常使用落后于 OpenAI 规范的服务端 SDK，对较新的枚举值直接反序列化失败，例如：
//!
//! ```text
//! messages[0].role: unknown variant `developer`,
//! expected one of `system`, `user`, `assistant`, `tool`
//! ```
//!
//! 这不是代理的缺陷（实测 `async-openai 0.41.1` 接受 `developer`），而是上游无法
//! 跟上规范。此时唯一可用的补救是**按能力降级请求体**。
//!
//! 降级必须是**显式、可控、可审计**的：静默改写请求语义会让代理行为不可预测，
//! 因此 [`UpstreamCompat::default`] 为「完全透明」，降级只在服务商显式开启后生效。
//!
//! ## 归属决策
//!
//! 这是「值对象数据 + 外部数据」的组合判断（落差等级 → 是否改写），
//! 故建模为值对象；具体改写动作按协议不同，委托给 [`crate::domain::shared::ApiProtocol`]。

use serde::{Deserialize, Serialize};

/// 上游兼容策略（服务商级配置）
///
/// 决定代理在转发前是否需要对请求体做**能力降级**。默认全透明，
/// 不改变任何请求语义；仅当服务商显式开启时才改写已知的不兼容取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UpstreamCompat {
    /// 是否把上游不支持的较新消息 role 降级为等价旧 role
    ///
    /// 当前唯一规则：OpenAI 家族的 `developer` → `system`。
    /// OpenAI 规范中 `developer` 与 `system` 语义等价（前者为后者的新命名），
    /// 对只认 `system` 的上游做此替换不改变指令语义。
    pub normalize_legacy_roles: bool,
}

impl UpstreamCompat {
    /// 完全透明策略（默认）：不做任何请求体降级
    pub fn transparent() -> Self {
        Self::default()
    }

    /// 开启旧 role 降级
    pub fn with_normalize_legacy_roles(mut self, enabled: bool) -> Self {
        self.normalize_legacy_roles = enabled;
        self
    }

    /// 从数据库布尔列构造
    pub fn from_db(normalize_legacy_roles: bool) -> Self {
        Self {
            normalize_legacy_roles,
        }
    }

    /// 是否处于完全透明状态（未启用任何降级）
    pub fn is_transparent(&self) -> bool {
        !self.normalize_legacy_roles
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_transparent() {
        let compat = UpstreamCompat::default();
        assert!(compat.is_transparent());
        assert!(!compat.normalize_legacy_roles);
    }

    #[test]
    fn enabling_normalization_breaks_transparency() {
        let compat = UpstreamCompat::transparent().with_normalize_legacy_roles(true);
        assert!(!compat.is_transparent());
    }

    #[test]
    fn from_db_maps_column() {
        assert!(UpstreamCompat::from_db(true).normalize_legacy_roles);
        assert!(UpstreamCompat::from_db(false).is_transparent());
    }
}
