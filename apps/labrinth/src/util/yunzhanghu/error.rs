//! 云账户接口错误类型与响应码映射
//!
//! 文档：<https://open.yunzhanghu.com/docs/API/实时支付/2022032500000002>

use thiserror::Error;

#[derive(Debug, Error)]
pub enum YzhError {
    #[error("缺少环境变量: {0}")]
    MissingEnv(&'static str),

    #[error("无法读取密钥文件 {path}: {source}")]
    KeyFileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("密钥解析失败: {0}")]
    KeyParse(String),

    #[error("3DES Key 长度必须为 24 字节，当前 {0} 字节")]
    InvalidDesKeyLength(usize),

    #[error("3DES 加密失败: {0}")]
    DesEncrypt(String),

    #[error("3DES 解密失败: {0}")]
    DesDecrypt(String),

    #[error("RSA 签名失败: {0}")]
    RsaSign(String),

    #[error("RSA 验签失败")]
    RsaVerify,

    #[error("JSON 序列化失败: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Base64 解码失败: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("响应解析失败: {0}")]
    BadResponse(String),

    /// 云账户业务响应错误。`code` 是云账户返回的状态码，例如 `2002`、`1000` 等
    #[error("云账户业务错误 [{code}] {message}")]
    Business { code: String, message: String },
}

impl YzhError {
    /// 业务码 `0000` 表示成功，`2002` 表示订单处理中（可重试），其余均视为失败。
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            YzhError::Business { code, .. } if code == "2002"
        )
    }
}
