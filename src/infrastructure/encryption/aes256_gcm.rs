use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use async_trait::async_trait;

use crate::domain::shared::EncryptionService;
use crate::shared::error::AppError;

/// AES-256-GCM 加密服务
///
/// 加密格式: [12-byte nonce][ciphertext]
/// - nonce: 随机生成的 12 字节初始化向量
/// - ciphertext: 包含认证标签的加密数据
pub struct Aes256GcmEncryptionService {
    key: [u8; 32],
}

impl Aes256GcmEncryptionService {
    pub fn new(key: [u8; 32]) -> Self {
        Aes256GcmEncryptionService { key }
    }
}

#[async_trait]
impl EncryptionService for Aes256GcmEncryptionService {
    async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::Encryption(format!("无法创建加密器: {}", e)))?;

        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::Encryption(format!("加密失败: {}", e)))?;

        // 拼接: [12-byte nonce][ciphertext]
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    async fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, AppError> {
        if data.len() < 12 {
            return Err(AppError::Encryption(
                "加密数据长度不足，缺少 nonce".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::Encryption(format!("无法创建解密器: {}", e)))?;

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Encryption(format!("解密失败: {}", e)))?;

        Ok(plaintext)
    }
}

impl Aes256GcmEncryptionService {
    fn generate_nonce() -> [u8; 12] {
        use rand::RngCore;
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}
