use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

const DEFAULT_API_BASE: &str = "https://api-v4.mysubmail.com";
const REQUEST_TIMEOUT_SECONDS: u64 = 15;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
        .expect("build submail http client")
});

static RE_MAINLAND_PHONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^1[3-9]\d{9}$").unwrap());

#[derive(Debug, Error)]
pub enum SubmailError {
    #[error("缺少环境变量 {0}")]
    MissingEnv(String),
    #[error("SUBMAIL 签名模式无效: {0}（可选 normal/md5/sha1）")]
    InvalidSignType(String),
    #[error("SUBMAIL 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SUBMAIL 返回错误 code={code} msg={msg}")]
    Api { code: String, msg: String },
    #[error("SUBMAIL 响应异常 status={status} body={body}")]
    UnexpectedResponse { status: u16, body: String },
}

/// 签名模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmailSignType {
    /// 明文 AppKey（官方 SDK 默认，仅 HTTPS 下传输）
    #[default]
    Normal,
    /// 数字签名 md5
    Md5,
    /// 数字签名 sha1
    Sha1,
}

impl SubmailSignType {
    fn parse(value: &str) -> Result<Self, SubmailError> {
        match value.trim().to_lowercase().as_str() {
            "" | "normal" => Ok(Self::Normal),
            "md5" => Ok(Self::Md5),
            "sha1" => Ok(Self::Sha1),
            other => Err(SubmailError::InvalidSignType(other.to_string())),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            SubmailSignType::Normal => "normal",
            SubmailSignType::Md5 => "md5",
            SubmailSignType::Sha1 => "sha1",
        }
    }
}

/// 发送成功的结果。
#[derive(Debug)]
pub struct SubmailSendSuccess {
    pub send_id: String,
    /// 计费条数
    pub fee: Option<i64>,
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum SubmailResponse {
    Success {
        send_id: String,
        #[serde(default)]
        fee: Option<i64>,
    },
    Error {
        code: String,
        msg: String,
    },
}

/// 赛邮短信客户端（无状态，按需从环境变量构造）。
pub struct SubmailSmsClient {
    app_id: String,
    app_key: String,
    api_base: String,
    sign_type: SubmailSignType,
    /// 激励（云账户）结算成功短信的模板 project ID
    incentive_settle_project: String,
}

/// 云账户结算成功后给用户发短信通知（异步 fire-and-forget，不阻塞调用方）。
///
/// `amount` 应传**实际到账净额**（提现总额 − 手续费），
/// 与提交云账户的打款金额口径一致（`calculate_yunzhanghu_submitted_pay_amount`）。
///
/// 手机号优先取云账户实名手机号（收款人本人），缺失时回退 `users.phone_number`；
/// 未配置 SUBMAIL 或无可用手机号时静默跳过。发送失败仅记日志，不影响结算主流程。
pub fn notify_payout_success(
    pool: &sqlx::PgPool,
    payout_db_id: i64,
    user_id: crate::database::models::UserId,
    amount: rust_decimal::Decimal,
) {
    let pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = send_payout_success_sms(&pool, user_id, amount).await {
            log::warn!(
                "提现成功短信发送失败 payout_id={} user_id={}: {}",
                payout_db_id,
                user_id.0,
                e
            );
        }
    });
}

async fn send_payout_success_sms(
    pool: &sqlx::PgPool,
    user_id: crate::database::models::UserId,
    amount: rust_decimal::Decimal,
) -> Result<(), SubmailError> {
    let client = match SubmailSmsClient::from_env() {
        Ok(client) => client,
        Err(SubmailError::MissingEnv(name)) => {
            log::info!("未配置 SUBMAIL 短信（缺少 {name}），跳过提现成功短信");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let mut phone: Option<String> =
        crate::database::models::yunzhanghu_profile_item::YunzhanghuProfile::get(
            user_id, pool,
        )
        .await
        .map_err(|e| {
            log::warn!("查询云账户实名手机号失败 user_id={}: {}", user_id.0, e);
        })
        .ok()
        .flatten()
        .and_then(|profile| profile.phone);

    if phone.is_none() {
        // 不走 `User::get_id`：其 redis 缓存路径的 future 非 Send，会污染 scheduler 轮询链
        // 两次 flatten：行存在性 + 列可空（phone_number 为 NULL）
        phone = sqlx::query_scalar::<_, Option<String>>(
            "SELECT phone_number FROM users WHERE id = $1",
        )
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            log::warn!("查询账号绑定手机号失败 user_id={}: {}", user_id.0, e);
        })
        .ok()
        .flatten()
        .flatten();
    }

    let Some(phone) = phone
        .map(|p| p.trim().to_string())
        .filter(|p| is_valid_mainland_phone(p))
    else {
        log::info!("用户 {} 无可用手机号，跳过提现成功短信", user_id.0);
        return Ok(());
    };

    // 云账户打款当前仅支持支付宝；后续新增微信 / 银行卡渠道时需按 payout method 映射
    client
        .send_incentive_settlement_sms(&phone, amount, "支付宝")
        .await?;
    Ok(())
}

/// 中国大陆手机号预检（与云账户实名手机号入库校验同规则），避免脏数据外呼。
fn is_valid_mainland_phone(phone: &str) -> bool {
    RE_MAINLAND_PHONE.is_match(phone)
}

impl SubmailSmsClient {
    /// 从环境变量构造：
    ///
    /// | 变量 | 必填 | 说明 |
    /// |---|---|---|
    /// | `SUBMAIL_APP_ID` | 是 | 短信应用 AppID |
    /// | `SUBMAIL_APP_KEY` | 是 | 短信应用 AppKey |
    /// | `SUBMAIL_INCENTIVE_SETTLE_TEMPLATE_ID` | 是 | 激励结算成功模板 ID |
    /// | `SUBMAIL_SIGN_TYPE` | 否 | `normal`（默认）/ `md5` / `sha1` |
    /// | `SUBMAIL_API_BASE` | 否 | 默认 `https://api-v4.mysubmail.com` |
    pub fn from_env() -> Result<Self, SubmailError> {
        Ok(Self {
            app_id: dotenvy::var("SUBMAIL_APP_ID").map_err(|_| {
                SubmailError::MissingEnv("SUBMAIL_APP_ID".into())
            })?,
            app_key: dotenvy::var("SUBMAIL_APP_KEY").map_err(|_| {
                SubmailError::MissingEnv("SUBMAIL_APP_KEY".into())
            })?,
            api_base: dotenvy::var("SUBMAIL_API_BASE")
                .unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
            sign_type: SubmailSignType::parse(
                &dotenvy::var("SUBMAIL_SIGN_TYPE").unwrap_or_default(),
            )?,
            incentive_settle_project: dotenvy::var(
                "SUBMAIL_INCENTIVE_SETTLE_TEMPLATE_ID",
            )
            .map_err(|_| {
                SubmailError::MissingEnv(
                    "SUBMAIL_INCENTIVE_SETTLE_TEMPLATE_ID".into(),
                )
            })?,
        })
    }

    /// 激励（云账户）结算成功通知短信。
    ///
    /// `amount` 为实际到账净额（已扣手续费）。
    /// 模板变量：`@var(money)`（如 `12.34`）、`@var(pay_channel)`（如 `支付宝`）。
    pub async fn send_incentive_settlement_sms(
        &self,
        phone: &str,
        amount: rust_decimal::Decimal,
        pay_channel: &str,
    ) -> Result<SubmailSendSuccess, SubmailError> {
        let vars = serde_json::json!({
            "money": format!("{:.2}", amount.round_dp(2)),
            "pay_channel": pay_channel,
        });
        self.xsend(phone, &self.incentive_settle_project, &vars)
            .await
    }

    /// 模板短信发送（`sms/xsend`，单发）。
    ///
    /// `vars` 须为 JSON 对象，key 对应模板中的 `@var(key)` 占位符。
    pub async fn xsend(
        &self,
        to: &str,
        project: &str,
        vars: &serde_json::Value,
    ) -> Result<SubmailSendSuccess, SubmailError> {
        // 数字签名模式需先取 SUBMAIL 服务器时间戳（normal 模式忽略 timestamp/sign_type）
        let timestamp = match self.sign_type {
            SubmailSignType::Normal => None,
            _ => Some(self.get_server_timestamp().await?.to_string()),
        };

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("appid".into(), self.app_id.clone());
        params.insert("to".into(), to.to_string());
        params.insert("project".into(), project.to_string());
        params.insert("vars".into(), vars.to_string());
        if let Some(timestamp) = timestamp.as_deref() {
            params.insert("timestamp".into(), timestamp.to_string());
            params.insert(
                "sign_type".into(),
                self.sign_type.as_str().to_string(),
            );
        }
        let signature = self.build_signature(&params);
        params.insert("signature".into(), signature);

        // SUBMAIL 对余额不足、模板未审核等场景返回 4xx，但 body 仍是
        // {"status":"error","code","msg"} 结构，先取 body 再分类，避免丢排障信息
        let resp = HTTP_CLIENT
            .post(format!("{}/sms/xsend.json", self.api_base))
            .json(&params)
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;

        match serde_json::from_slice::<SubmailResponse>(&bytes) {
            Ok(SubmailResponse::Error { code, msg }) => {
                Err(SubmailError::Api { code, msg })
            }
            Ok(SubmailResponse::Success { send_id, fee })
                if status.is_success() =>
            {
                Ok(SubmailSendSuccess { send_id, fee })
            }
            _ => Err(SubmailError::UnexpectedResponse {
                status: status.as_u16(),
                body: truncate_body(&bytes),
            }),
        }
    }

    /// 获取 SUBMAIL 服务器时间戳（秒级，数字签名模式要求与服务器时差 < 6 秒）。
    async fn get_server_timestamp(&self) -> Result<i64, SubmailError> {
        #[derive(Deserialize)]
        struct TimestampResponse {
            timestamp: i64,
        }

        let resp: TimestampResponse = HTTP_CLIENT
            .get(format!("{}/service/timestamp.json", self.api_base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.timestamp)
    }

    /// 计算 `signature`。`signature` / `tag` / `sms_signature` 不参与计算
    /// （调用时传入的 `params` 尚未包含 `signature`）。
    fn build_signature(&self, params: &BTreeMap<String, String>) -> String {
        match self.sign_type {
            SubmailSignType::Normal => self.app_key.clone(),
            SubmailSignType::Md5 => {
                let raw = self.wrap_signature_base(params);
                format!("{:x}", md5::compute(raw.as_bytes()))
            }
            SubmailSignType::Sha1 => {
                use sha1::{Digest, Sha1};

                let raw = self.wrap_signature_base(params);
                let mut hasher = Sha1::new();
                hasher.update(raw.as_bytes());
                format!("{:x}", hasher.finalize())
            }
        }
    }

    /// `appid + appkey + 升序参数串 + appid + appkey`
    fn wrap_signature_base(&self, params: &BTreeMap<String, String>) -> String {
        let joined = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        format!(
            "{}{}{}{}{}",
            self.app_id, self.app_key, joined, self.app_id, self.app_key
        )
    }
}

/// 响应 body 片段（截断），用于排障日志；响应不含 AppKey 等敏感信息。
fn truncate_body(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut truncated: String = text.chars().take(150).collect();
    if truncated.chars().count() == 150 {
        truncated.push('…');
    }
    truncated
}
