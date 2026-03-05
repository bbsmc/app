//! 敏感数据加密工具模块
//!
//! 使用 AES-256-GCM 加密算法对敏感信息进行加密存储
//! 如身份证号、银行卡号等个人隐私信息

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::LazyLock;
use thiserror::Error;

/// 加密密钥环境变量名
pub const ENCRYPTION_KEY_ENV: &str = "ENCRYPTION_KEY";

/// 加密密钥（32 字节，256 位）
/// 格式：Base64 编码的 32 字节密钥
static ENCRYPTION_KEY: LazyLock<Option<[u8; 32]>> = LazyLock::new(|| {
    match std::env::var(ENCRYPTION_KEY_ENV) {
        Ok(key_base64) => match BASE64.decode(&key_base64) {
            Ok(key_bytes) => {
                if key_bytes.len() != 32 {
                    log::error!(
                        "{} 必须是 32 字节（Base64 编码后约 44 字符），当前长度: {} 字节",
                        ENCRYPTION_KEY_ENV,
                        key_bytes.len()
                    );
                    return None;
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                Some(key)
            }
            Err(e) => {
                log::error!("{} Base64 解码失败: {}", ENCRYPTION_KEY_ENV, e);
                None
            }
        },
        Err(_) => {
            log::warn!(
                "{} 未设置，敏感数据加密功能将不可用",
                ENCRYPTION_KEY_ENV
            );
            None
        }
    }
});

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("加密密钥未配置，请设置 {} 环境变量", ENCRYPTION_KEY_ENV)]
    KeyNotConfigured,

    #[error("加密失败: {0}")]
    EncryptionFailed(String),

    #[error("解密失败: {0}")]
    DecryptionFailed(String),

    #[error("数据格式错误: {0}")]
    InvalidFormat(String),
}

/// 检查加密功能是否可用
pub fn is_encryption_available() -> bool {
    ENCRYPTION_KEY.is_some()
}

/// 确保加密密钥已配置（用于启动时强制检查）
/// 如果未配置则 panic，防止服务在不安全状态下运行
pub fn ensure_encryption_key_configured() {
    if ENCRYPTION_KEY.is_none() {
        panic!(
            "严重错误: {} 未配置或无效！\n\
            敏感数据加密功能必须可用才能启动服务。\n\
            请设置有效的 {} 环境变量（Base64 编码的 32 字节密钥）。\n\
            生成密钥示例: openssl rand -base64 32",
            ENCRYPTION_KEY_ENV, ENCRYPTION_KEY_ENV
        );
    }
    log::info!("加密密钥已配置，敏感数据加密功能可用");
}

/// 加密敏感数据
///
/// 返回格式：Base64(nonce || ciphertext || tag)
/// - nonce: 12 字节随机数
/// - ciphertext: 加密后的数据
/// - tag: 16 字节认证标签
pub fn encrypt(plaintext: &str) -> Result<String, EncryptionError> {
    let key_bytes = ENCRYPTION_KEY
        .as_ref()
        .ok_or(EncryptionError::KeyNotConfigured)?;

    let unbound_key =
        UnboundKey::new(&AES_256_GCM, key_bytes).map_err(|e| {
            EncryptionError::EncryptionFailed(format!("创建密钥失败: {}", e))
        })?;
    let key = LessSafeKey::new(unbound_key);

    // 生成随机 nonce（12 字节）
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes).map_err(|_| {
        EncryptionError::EncryptionFailed("生成随机数失败".to_string())
    })?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    // 加密数据
    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| {
            EncryptionError::EncryptionFailed("加密失败".to_string())
        })?;

    // 组合：nonce + ciphertext + tag
    let mut result = Vec::with_capacity(12 + in_out.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&in_out);

    Ok(BASE64.encode(&result))
}

/// 解密敏感数据
///
/// 输入格式：Base64(nonce || ciphertext || tag)
pub fn decrypt(ciphertext: &str) -> Result<String, EncryptionError> {
    let key_bytes = ENCRYPTION_KEY
        .as_ref()
        .ok_or(EncryptionError::KeyNotConfigured)?;

    let data = BASE64.decode(ciphertext).map_err(|e| {
        EncryptionError::InvalidFormat(format!("Base64 解码失败: {}", e))
    })?;

    // 最小长度：12 (nonce) + 0 (plaintext) + 16 (tag) = 28
    if data.len() < 28 {
        return Err(EncryptionError::InvalidFormat("密文数据太短".to_string()));
    }

    let (nonce_bytes, encrypted) = data.split_at(12);
    let nonce_array: [u8; 12] = nonce_bytes.try_into().map_err(|_| {
        EncryptionError::InvalidFormat("Nonce 长度错误".to_string())
    })?;

    let unbound_key =
        UnboundKey::new(&AES_256_GCM, key_bytes).map_err(|e| {
            EncryptionError::DecryptionFailed(format!("创建密钥失败: {}", e))
        })?;
    let key = LessSafeKey::new(unbound_key);

    let nonce = Nonce::assume_unique_for_key(nonce_array);

    // 解密数据
    let mut encrypted_data = encrypted.to_vec();
    let decrypted = key
        .open_in_place(nonce, Aad::empty(), &mut encrypted_data)
        .map_err(|_| {
            EncryptionError::DecryptionFailed(
                "解密失败，密钥或数据错误".to_string(),
            )
        })?;

    String::from_utf8(decrypted.to_vec()).map_err(|e| {
        EncryptionError::DecryptionFailed(format!("UTF-8 解码失败: {}", e))
    })
}
