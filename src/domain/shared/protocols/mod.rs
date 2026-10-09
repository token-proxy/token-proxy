//! 协议适配子模块 — domain/shared/protocols/
//!
//! 各 LLM API 协议的解析与变换实现，按协议一文件组织。
//! 对外不导出，由请求级协议值对象 `ApiProtocol` 的方法通过 `match self`
//! 分发到具体协议函数（见 `../api_protocol.rs`）。
//!
//! 添加新协议只需：
//! 1. 新建 `<protocol_name>.rs` 实现一组 `pub(in crate::domain::shared) fn`
//! 2. 在 `mod.rs` 中 `pub(super) mod <protocol_name>;`
//! 3. 在 `ApiProtocol` 的对应方法中补充 match 分支
//!    （编译器会自动指出所有需要补分支的位置）
//!
//! 注意：协议家族（`AccessPointType`）与请求级协议（`ApiProtocol`）是两个正交概念。
//! OpenAI 的 Chat Completions 与 Responses 是**两个** `ApiProtocol` variant，
//! 共用同一个 `openai` 家族，因此同一文件（`openai.rs`）内提供两组解析入口。

pub(super) mod anthropic;
pub(super) mod openai;
