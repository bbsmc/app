//! 云账户（Yunzhanghu）对接模块。
//!
//! 包含：
//! - [`crypto`]：3DES/CBC/PKCS7、RSA-SHA256 签名/验签、AES-256-GCM
//! - [`secrets`]：从环境变量与文件加载凭据并缓存
//! - [`client`]：HTTP 客户端 + 回调解码
//! - [`error`]：统一错误类型 [`YzhError`]
//!
//! ## 用法
//!
//! ```rust,ignore
//! use crate::util::yunzhanghu::YzhClient;
//!
//! let client = YzhClient::new();
//! let resp: serde_json::Value = client
//!     .get("/api/payment/v1/query-balance", serde_json::json!({}))
//!     .await?;
//! ```

pub mod api;
pub mod client;
pub mod crypto;
pub mod error;
pub mod secrets;

pub use client::{NotifyEnvelope, YzhClient};
pub use error::YzhError;
