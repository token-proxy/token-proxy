//! 协议适配子模块 — domain/shared/protocols/
//!
//! 各 LLM API 协议的解析与变换实现，按协议一文件组织。
//! 对外不导出，由 `AccessPointType` 的方法通过 match 分发到具体协议函数。
//!
//! 添加新协议（如 OpenAI）只需：
//! 1. 新建 `<protocol_name>.rs` 实现一组 `pub(super) fn`（与 anthropic.rs 签名一致）
//! 2. 在 `mod.rs` 中 `pub(super) mod <protocol_name>;`
//! 3. 在 `AccessPointType` 的对应方法中补充 match 分支
//!    （编译器会自动指出所有需要补分支的位置）

pub(super) mod anthropic;
pub(super) mod openai;
