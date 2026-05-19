use crate::shared::error::AppError;
use async_trait::async_trait;

/// 加密服务 trait，定义加密和解密接口
/// 实现层负责具体的加密算法（如 AES-256-GCM）
#[async_trait]
pub trait EncryptionService: Send + Sync {
    /// 加密明文数据
    ///
    /// # 参数
    /// - `plaintext`: 明文字节数组
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: 加密后的密文（包含认证标签和 nonce）
    /// - `Err(AppError)`: 加密失败
    async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, AppError>;

    /// 解密密文数据
    ///
    /// # 参数
    /// - `ciphertext`: 密文字节数组（包含认证标签和 nonce）
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: 解密后的明文
    /// - `Err(AppError)`: 解密失败（如认证失败或数据损坏）
    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, AppError>;
}