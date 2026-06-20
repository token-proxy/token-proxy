//! 加密服务接口 — domain/shared/
//!
//! 定义 `EncryptionService` trait，提供对称加密/解密契约。

use crate::shared::error::AppError;
use async_trait::async_trait;

/// 加密服务接口，收口所有对称加密/解密操作
#[async_trait]
pub trait EncryptionService: Send + Sync {
    async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, AppError>;
    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, AppError>;
}
