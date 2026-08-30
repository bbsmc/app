//! 云账户加密与签名实现
//!
//! - **3DES/CBC/PKCS7**：加密请求 Body 中的 `data` 字段（IV 取密钥前 8 字节）
//! - **RSA-SHA256**：签名整个请求 + 验证云账户回调签名
//!
//! 本地 PII（如身份证号）的对称加密由 [`crate::util::encrypt`] 统一管理，
//! 与云账户协议无关。
//!
//! 算法参考：<https://open.yunzhanghu.com/docs/接入指引/接口规范/2022032500000013>

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cbc::{Decryptor, Encryptor};
use cipher::{
    BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7,
};
use des::TdesEde3;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;

use super::error::YzhError;

type Des3CbcEnc = Encryptor<TdesEde3>;
type Des3CbcDec = Decryptor<TdesEde3>;

// ============================================================================
// 3DES / CBC / PKCS7
// ============================================================================

/// 3DES 加密 + Base64 编码。
///
/// 与 Java `DESede/CBC/PKCS5Padding`、PHP `des-ede3-cbc`、Python `pycryptodome` 等
/// 默认实现完全兼容。IV 取密钥前 8 字节。
pub fn encrypt_3des(key: &[u8], plaintext: &[u8]) -> Result<String, YzhError> {
    if key.len() != 24 {
        return Err(YzhError::InvalidDesKeyLength(key.len()));
    }
    let iv: [u8; 8] = key[..8].try_into().unwrap();
    let ct = Des3CbcEnc::new(key.into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(plaintext);
    Ok(BASE64.encode(&ct))
}

/// Base64 解码 + 3DES 解密。
pub fn decrypt_3des(
    key: &[u8],
    ciphertext_b64: &str,
) -> Result<Vec<u8>, YzhError> {
    if key.len() != 24 {
        return Err(YzhError::InvalidDesKeyLength(key.len()));
    }
    let ct = BASE64.decode(ciphertext_b64.as_bytes())?;
    let iv: [u8; 8] = key[..8].try_into().unwrap();
    Des3CbcDec::new(key.into(), &iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&ct)
        .map_err(|e| YzhError::DesDecrypt(e.to_string()))
}

// ============================================================================
// RSA-SHA256 签名 / 验签
// ============================================================================

/// 用平台企业私钥对待签名串做 RSA-SHA256，返回 Base64。
///
/// 待签名串拼接顺序固定为：`data=<>&mess=<>&timestamp=<>&key=<App Key>`。
pub fn rsa_sign_sha256(
    content: &str,
    private_key_pem: &str,
) -> Result<String, YzhError> {
    let key = RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|e| {
        YzhError::KeyParse(format!("私钥 PKCS8 PEM 解析失败: {}", e))
    })?;
    let signing_key = SigningKey::<Sha256>::new(key);
    let signature = signing_key
        .try_sign(content.as_bytes())
        .map_err(|e| YzhError::RsaSign(e.to_string()))?;
    Ok(BASE64.encode(signature.to_bytes()))
}

/// 用云账户公钥验证回调签名。
pub fn rsa_verify_sha256(
    content: &str,
    sign_b64: &str,
    public_key_pem: &str,
) -> Result<(), YzhError> {
    let key = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|e| YzhError::KeyParse(format!("公钥 PEM 解析失败: {}", e)))?;
    let verifying_key = VerifyingKey::<Sha256>::new(key);
    let sig_bytes = BASE64.decode(sign_b64.as_bytes())?;
    let signature = Signature::try_from(sig_bytes.as_slice())
        .map_err(|_| YzhError::RsaVerify)?;
    verifying_key
        .verify(content.as_bytes(), &signature)
        .map_err(|_| YzhError::RsaVerify)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 来自云账户官方示例的测试向量（auth.md）。
    /// 3DES Key 24 字节，加密一段 JSON，验证可往返。
    #[test]
    fn test_3des_roundtrip() {
        let key = b"123456788765432112345678"; // 24 字节
        let plaintext =
            r#"{"order_id":"1234567890987654321","real_name":"张三"}"#
                .as_bytes();
        let encrypted = encrypt_3des(key, plaintext).expect("encrypt");
        let decrypted = decrypt_3des(key, &encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_3des_key_length_check() {
        let bad_key = b"short_key";
        assert!(matches!(
            encrypt_3des(bad_key, b"x"),
            Err(YzhError::InvalidDesKeyLength(_))
        ));
    }

    /// 用临时生成的 RSA 密钥签名 + 验签，验证算法正确性。
    #[test]
    fn test_rsa_sign_verify_roundtrip() {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("gen priv");
        let pub_key = RsaPublicKey::from(&priv_key);
        let priv_pem = priv_key.to_pkcs8_pem(LineEnding::LF).unwrap();
        let pub_pem = pub_key.to_public_key_pem(LineEnding::LF).unwrap();

        let content = "data=abc&mess=12313&timestamp=1660289578&key=78f9b4fad3481fbce1df0b30eee58577";
        let sig = rsa_sign_sha256(content, &priv_pem).expect("sign");
        rsa_verify_sha256(content, &sig, &pub_pem).expect("verify ok");

        // 改一个字节验签必须失败
        let bad_content = "data=abd&mess=12313&timestamp=1660289578&key=78f9b4fad3481fbce1df0b30eee58577";
        assert!(rsa_verify_sha256(bad_content, &sig, &pub_pem).is_err());
    }
}
