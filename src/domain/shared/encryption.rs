use crate::shared::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait EncryptionService: Send + Sync {
    async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, AppError>;
    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, AppError>;
}
