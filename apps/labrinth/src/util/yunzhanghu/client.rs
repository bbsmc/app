//! 云账户 HTTP 客户端。
//!
//! 单次请求流程：
//! 1. 业务参数 → JSON
//! 2. JSON → 3DES 加密 + Base64 → `data`
//! 3. 生成 `mess`（UUID）、`timestamp`（秒级）
//! 4. 拼待签名串：`data=<>&mess=<>&timestamp=<>&key=<App Key>`
//! 5. RSA-SHA256 签名 → `sign`
//! 6. POST `application/x-www-form-urlencoded`，Headers 带 `dealer-id`、`request-id`
//! 7. 响应 form 字段 `data` 同样需要 3DES 解密

use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use super::crypto::{decrypt_3des, encrypt_3des, rsa_sign_sha256};
use super::error::YzhError;
use super::secrets::{YzhCredentials, load as load_credentials};

const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const NOTIFY_TIMESTAMP_TOLERANCE_SECONDS: i64 = 60 * 60;

/// 通用响应外壳。云账户大部分接口返回结构为：
/// ```json
/// { "code": "0000", "message": "成功", "request_id": "...", "data": "<3DES 加密的业务数据>" }
/// ```
#[derive(Deserialize)]
struct EnvelopeRaw {
    code: String,
    message: Option<String>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// 云账户客户端（无状态，可放进 `web::Data` 共享）。
#[derive(Clone, Default)]
pub struct YzhClient {
    http: reqwest::Client,
}

impl YzhClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// 调用云账户接口。`payload` 是业务参数（不含 `dealer_id`/`broker_id`，本函数自动注入）。
    ///
    /// `T` 应实现 [`serde::Deserialize`]，对应**业务 data 字段解密后**的 JSON 结构。
    pub async fn call<T>(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: serde_json::Value,
        is_form: bool,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.call_inner(method, path, payload, is_form, true).await
    }

    /// 与 [`call`] 类似，但不自动注入 `dealer_id` / `broker_id`。
    /// 适用于云账户严格校验字段集合的接口（如 query-order）。
    pub async fn call_raw<T>(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: serde_json::Value,
        is_form: bool,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.call_inner(method, path, payload, is_form, false).await
    }

    async fn call_inner<T>(
        &self,
        method: reqwest::Method,
        path: &str,
        mut payload: serde_json::Value,
        is_form: bool,
        auto_inject_ids: bool,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let creds = load_credentials()?;
        if auto_inject_ids && let Some(obj) = payload.as_object_mut() {
            obj.entry("dealer_id").or_insert_with(|| {
                serde_json::Value::String(creds.dealer_id.clone())
            });
            obj.entry("broker_id").or_insert_with(|| {
                serde_json::Value::String(creds.broker_id.clone())
            });
        }

        let body = self.build_body(creds, &payload)?;
        let url = format!("{}{}", creds.api_url, path);
        let request_id = Uuid::new_v4().to_string();

        let builder = self
            .http
            .request(method, &url)
            .header("dealer-id", &creds.dealer_id)
            .header("request-id", &request_id);

        log::debug!(
            "云账户请求 [{}] {} request_id={} payload_keys={}",
            if is_form { "POST" } else { "GET" },
            url,
            request_id,
            payload_keys(&payload)
        );

        let resp = if is_form {
            builder.form(&body).send().await?
        } else {
            builder.query(&body).send().await?
        };

        let status = resp.status();
        let text = resp.text().await?;

        log::debug!(
            "云账户响应 [{}] {} request_id={} body_len={}",
            status,
            url,
            request_id,
            text.len()
        );

        let env: EnvelopeRaw = serde_json::from_str(&text).map_err(|e| {
            YzhError::BadResponse(format!(
                "HTTP {} 解析 envelope 失败: {}",
                status, e
            ))
        })?;

        if env.code != "0000" {
            return Err(YzhError::Business {
                code: env.code,
                message: env.message.unwrap_or_default(),
            });
        }

        // data 字段可能是密文字符串（实时支付/查询类）也可能是 JSON 对象（某些查询）
        let payload_value = match env.data {
            Some(serde_json::Value::String(cipher)) if !cipher.is_empty() => {
                let pt = decrypt_3des(creds.des_key.as_bytes(), &cipher)?;
                serde_json::from_slice::<serde_json::Value>(&pt).map_err(
                    |e| {
                        YzhError::BadResponse(format!(
                            "data 解密后解析失败: {}",
                            e
                        ))
                    },
                )?
            }
            Some(v) => v,
            None => serde_json::Value::Null,
        };

        serde_json::from_value::<T>(payload_value).map_err(|e| {
            YzhError::BadResponse(format!("data 反序列化失败: {}", e))
        })
    }

    /// POST 请求（application/x-www-form-urlencoded）—— 云账户大部分写接口
    pub async fn post<T>(
        &self,
        path: &str,
        payload: serde_json::Value,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.call::<T>(reqwest::Method::POST, path, payload, true)
            .await
    }

    /// GET 请求（Query String）—— 云账户大部分查询接口
    pub async fn get<T>(
        &self,
        path: &str,
        payload: serde_json::Value,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.call::<T>(reqwest::Method::GET, path, payload, false)
            .await
    }

    /// GET 请求但**不自动注入 dealer_id / broker_id**
    pub async fn get_raw<T>(
        &self,
        path: &str,
        payload: serde_json::Value,
    ) -> Result<T, YzhError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.call_raw::<T>(reqwest::Method::GET, path, payload, false)
            .await
    }

    /// 构造 Body 五件套：data / mess / timestamp / sign / sign_type
    fn build_body(
        &self,
        creds: &YzhCredentials,
        payload: &serde_json::Value,
    ) -> Result<Vec<(&'static str, String)>, YzhError> {
        let plaintext = serde_json::to_vec(payload)?;
        let data = encrypt_3des(creds.des_key.as_bytes(), &plaintext)?;
        let mess = Uuid::new_v4().simple().to_string();
        let timestamp = Utc::now().timestamp().to_string();

        let signing_str = format!(
            "data={}&mess={}&timestamp={}&key={}",
            data, mess, timestamp, creds.app_key
        );
        let sign =
            rsa_sign_sha256(&signing_str, &creds.dealer_private_key_pem)?;

        Ok(vec![
            ("data", data),
            ("mess", mess),
            ("timestamp", timestamp),
            ("sign", sign),
            ("sign_type", "rsa".to_string()),
        ])
    }
}

// ============================================================================
// 回调解码工具：用云账户公钥验签 + 用 3DES 解密 data。
// ============================================================================

/// 回调 Body 反序列化结构（form-urlencoded → serde_urlencoded 解析后的字段）。
#[derive(Deserialize)]
pub struct NotifyEnvelope {
    pub data: String,
    pub mess: String,
    pub timestamp: String,
    pub sign: String,
    #[serde(default)]
    pub sign_type: String,
}

impl NotifyEnvelope {
    /// 验签 + 解密 data，返回明文 JSON。
    ///
    /// 注意签名串里的 `key` 仍然是平台企业自己的 App Key，
    /// 签名用的是**云账户私钥**，所以验签用云账户公钥。
    pub fn decode<T: for<'de> Deserialize<'de>>(&self) -> Result<T, YzhError> {
        self.validate()?;
        let creds = load_credentials()?;
        let signing_str = format!(
            "data={}&mess={}&timestamp={}&key={}",
            self.data, self.mess, self.timestamp, creds.app_key
        );
        super::crypto::rsa_verify_sha256(
            &signing_str,
            &self.sign,
            &creds.platform_public_key_pem,
        )?;
        let plaintext =
            super::crypto::decrypt_3des(creds.des_key.as_bytes(), &self.data)?;
        serde_json::from_slice::<T>(&plaintext).map_err(YzhError::Serde)
    }

    fn validate(&self) -> Result<(), YzhError> {
        if !self.sign_type.is_empty()
            && !self.sign_type.eq_ignore_ascii_case("rsa")
        {
            return Err(YzhError::BadResponse(format!(
                "回调 sign_type 非法: {}",
                self.sign_type
            )));
        }

        let timestamp = self.timestamp.parse::<i64>().map_err(|e| {
            YzhError::BadResponse(format!("回调 timestamp 解析失败: {}", e))
        })?;
        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > NOTIFY_TIMESTAMP_TOLERANCE_SECONDS {
            return Err(YzhError::BadResponse(
                "回调 timestamp 超出允许时间窗".to_string(),
            ));
        }

        Ok(())
    }
}

fn payload_keys(payload: &serde_json::Value) -> String {
    payload
        .as_object()
        .map(|obj| obj.keys().map(String::as_str).collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| "<non-object>".to_string())
}

// ============================================================================
// 联调测试：调真实云账户接口验证整个链路（加密 → 签名 → HTTP → 解密 → 反序列化）
//
// 跑法：
//   cd apps/labrinth
//   cargo test --lib util::yunzhanghu::client::integration -- --ignored --nocapture
//
// 需要 .env 已配置 YUNZHANGHU_* 凭据。
// ============================================================================
