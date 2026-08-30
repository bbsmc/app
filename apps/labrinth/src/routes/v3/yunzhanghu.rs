//! 云账户(Yunzhanghu)用户资料接口
//!
//! 提供创作者实名信息 + 支付宝账号绑定的查询/提交端点。
//! 签约相关接口在 [`super::payouts`] 中专门处理。

use crate::auth::get_user_from_headers;
use crate::database::models::UserId;
use crate::database::models::yunzhanghu_profile_item::{
    YunzhanghuProfile, YzhSignStatus,
};
use crate::database::redis::RedisPool;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::yunzhanghu::api::CERTIFICATE_TYPE_IDCARD;
use crate::util::yunzhanghu::{NotifyEnvelope, YzhClient, api as yzh_api};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use chrono::{DateTime, Duration, TimeZone, Utc};
use hex::ToHex;
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

const YZH_NOTIFY_REPLAY_NAMESPACE: &str = "yunzhanghu_notify_seen";
const YZH_NOTIFY_REPLAY_TTL_SECONDS: i64 = 26 * 60 * 60;
const YZH_H5_OPERATION_VALID_HOURS: i64 = 2;
const YZH_UNSIGN_RECONCILE_LEASE_SECONDS: i64 = 2 * 60;
const YZH_UNSIGN_RECONCILE_RETRY_SECONDS: i64 = 5 * 60;
const YZH_UNSIGN_RECONCILE_BATCH_SIZE: i64 = 50;

#[derive(Clone, Copy)]
enum UnsignReconcileClaimMode {
    DueOnly,
    Force,
}

enum UnsignReconcileOutcome {
    NotClaimed,
    Ignored,
    Pending {
        remote_status: Option<i32>,
    },
    Resolved {
        status: YzhSignStatus,
        remote_status: i32,
        signed_at: Option<DateTime<Utc>>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderStatusApplyOutcome {
    Applied,
    Ignored,
    EvidenceRejected,
}

fn ensure_order_status_evidence_accepted(
    outcome: OrderStatusApplyOutcome,
) -> Result<(), ApiError> {
    if outcome == OrderStatusApplyOutcome::EvidenceRejected {
        return Err(ApiError::InvalidInput(
            "云账户订单金额或身份信息与本地提交记录不一致".to_string(),
        ));
    }
    Ok(())
}

fn ensure_order_callback_status_applied(
    outcome: OrderStatusApplyOutcome,
) -> Result<(), ApiError> {
    match outcome {
        OrderStatusApplyOutcome::Applied => Ok(()),
        OrderStatusApplyOutcome::EvidenceRejected => {
            ensure_order_status_evidence_accepted(outcome)
        }
        OrderStatusApplyOutcome::Ignored => Err(ApiError::InvalidInput(
            "云账户订单状态未能应用到本地提现记录，请稍后重试通知".to_string(),
        )),
    }
}

fn ensure_refund_status_applied(
    outcome: OrderStatusApplyOutcome,
) -> Result<(), ApiError> {
    match outcome {
        OrderStatusApplyOutcome::Applied => Ok(()),
        OrderStatusApplyOutcome::EvidenceRejected => {
            ensure_order_status_evidence_accepted(outcome)
        }
        OrderStatusApplyOutcome::Ignored => Err(ApiError::InvalidInput(
            "云账户退款未能应用到本地提现状态，请人工核对".to_string(),
        )),
    }
}

fn should_ignore_terminal_order_transition(
    current: crate::models::payouts::PayoutStatus,
    new_status: crate::models::payouts::PayoutStatus,
    allow_success_refund: bool,
) -> bool {
    let is_terminal = matches!(
        current,
        crate::models::payouts::PayoutStatus::Success
            | crate::models::payouts::PayoutStatus::Cancelled
            | crate::models::payouts::PayoutStatus::Failed
    );
    let is_verified_refund = allow_success_refund
        && current == crate::models::payouts::PayoutStatus::Success
        && new_status == crate::models::payouts::PayoutStatus::Cancelled;

    is_terminal && current != new_status && !is_verified_refund
}

fn is_verified_channel_return(status: &str, refund_origin: &str) -> bool {
    matches!(status.trim(), "4" | "refund") && refund_origin.trim() == "2"
}

fn channel_return_query_matches_callback(
    notify: &OrderNotifyData,
    resp: &yzh_api::QueryOrderResponse,
) -> bool {
    let amounts_match = notify
        .pay
        .trim()
        .parse::<rust_decimal::Decimal>()
        .ok()
        .zip(resp.pay.trim().parse::<rust_decimal::Decimal>().ok())
        .is_some_and(|(callback_pay, queried_pay)| {
            callback_pay.round_dp(2) == queried_pay.round_dp(2)
        });

    resp.order_id == notify.order_id
        && is_verified_channel_return(&resp.status, &resp.refund_origin)
        && amounts_match
}

fn order_callback_evidence(
    notify: &OrderNotifyData,
    verified_channel_return: bool,
) -> OrderStatusEvidence<'_> {
    let (real_name, id_card, phone_no) = if verified_channel_return {
        // 历史订单只能绑定不可变快照；提现成功后当前实名资料允许修改。
        (None, None, None)
    } else {
        (
            non_empty_str(&notify.real_name),
            non_empty_str(&notify.id_card),
            non_empty_str(&notify.phone_no),
        )
    };

    OrderStatusEvidence {
        require_callback_fields: true,
        allow_success_refund: verified_channel_return,
        pay: non_empty_str(&notify.pay),
        dealer_id: non_empty_str(&notify.dealer_id),
        broker_id: non_empty_str(&notify.broker_id),
        real_name,
        id_card,
        phone_no,
        card_no: non_empty_str(&notify.card_no),
    }
}

fn ensure_full_refund_amount(
    original_pay: rust_decimal::Decimal,
    refund_amount: rust_decimal::Decimal,
) -> Result<(), ApiError> {
    if refund_amount.round_dp(2) != original_pay.round_dp(2) {
        return Err(ApiError::InvalidInput(
            "云账户部分退款不能自动取消整笔提现，请人工核对".to_string(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct OrderStatusEvidence<'a> {
    require_callback_fields: bool,
    allow_success_refund: bool,
    pay: Option<&'a str>,
    dealer_id: Option<&'a str>,
    broker_id: Option<&'a str>,
    real_name: Option<&'a str>,
    id_card: Option<&'a str>,
    phone_no: Option<&'a str>,
    card_no: Option<&'a str>,
}

impl<'a> OrderStatusEvidence<'a> {
    fn empty() -> Self {
        Self {
            require_callback_fields: false,
            allow_success_refund: false,
            pay: None,
            dealer_id: None,
            broker_id: None,
            real_name: None,
            id_card: None,
            phone_no: None,
            card_no: None,
        }
    }
}

fn required_order_callback_fields_present(
    evidence: &OrderStatusEvidence<'_>,
) -> bool {
    evidence.pay.is_some()
        && evidence.dealer_id.is_some()
        && evidence.broker_id.is_some()
        && evidence.card_no.is_some()
}

#[derive(Deserialize)]
struct OrderNotifyData {
    order_id: String,
    #[serde(default)]
    pay: String,
    #[serde(default)]
    dealer_id: String,
    #[serde(default)]
    broker_id: String,
    #[serde(default)]
    real_name: String,
    #[serde(default)]
    card_no: String,
    #[serde(default)]
    id_card: String,
    #[serde(default)]
    phone_no: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    status_detail: String,
    #[serde(default)]
    status_detail_message: String,
    #[serde(default, rename = "ref")]
    ref_id: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OrderNotifyPayload {
    Wrapped {
        #[serde(default)]
        notify_id: String,
        #[allow(dead_code)]
        #[serde(default)]
        notify_time: String,
        data: OrderNotifyData,
    },
    Flat(OrderNotifyData),
}

#[derive(Deserialize)]
struct RefundNotifyData {
    order_id: String,
    #[serde(default)]
    broker_id: String,
    #[serde(default)]
    dealer_id: String,
    #[serde(default, rename = "ref")]
    ref_id: String,
    #[serde(default)]
    refund_ref: String,
    #[serde(default)]
    real_name: String,
    #[serde(default)]
    card_no: String,
    #[serde(default)]
    id_card: String,
    #[serde(default)]
    refund_type: String,
    #[serde(default)]
    refund_total_amount: String,
    // 兼容旧版回调字段。
    #[serde(default)]
    pay: String,
    #[serde(default)]
    refund_amount: String,
    #[serde(default)]
    refund_status: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RefundNotifyPayload {
    Wrapped {
        #[serde(default)]
        notify_id: String,
        #[allow(dead_code)]
        #[serde(default)]
        notify_time: String,
        data: RefundNotifyData,
    },
    Flat(RefundNotifyData),
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("yunzhanghu")
            .service(get_profile)
            .service(submit_profile)
            .service(initiate_sign)
            .service(refresh_sign_status)
            .service(release_sign)
            .service(sign_callback)
            .service(unsign_callback)
            .service(order_callback)
            .service(refund_callback)
            .service(prepay_callback)
            .service(balance_callback)
            .service(refresh_payout_status),
    );
}

// ============================================================================
// 输入输出 DTO
// ============================================================================

lazy_static! {
    /// 中国身份证号，18 位，末位可能是 X / x
    static ref RE_ID_CARD: Regex = Regex::new(
        r"^[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[0-9Xx]$"
    ).unwrap();
    /// 中国大陆手机号（11 位，1[3-9] 开头）
    static ref RE_PHONE: Regex = Regex::new(r"^1[3-9]\d{9}$").unwrap();
    /// 支付宝账号：手机号或邮箱
    static ref RE_ALIPAY: Regex =
        Regex::new(r"^(1[3-9]\d{9}|[^\s@]+@[^\s@]+\.[^\s@]+)$").unwrap();
}

#[derive(Deserialize, Validate)]
pub struct ProfileSubmit {
    /// 真实姓名（与身份证一致）
    #[validate(length(min = 2, max = 30))]
    pub real_name: String,

    /// 18 位身份证号，末位可大写 X
    #[validate(regex(
        path = *RE_ID_CARD,
        message = "身份证号格式错误，请输入 18 位居民身份证号"
    ))]
    pub id_card: String,

    /// 11 位中国大陆手机号
    #[validate(regex(path = *RE_PHONE, message = "手机号格式错误"))]
    pub phone: String,

    /// 支付宝账号（手机号或邮箱）
    #[validate(regex(
        path = *RE_ALIPAY,
        message = "支付宝账号应为手机号或邮箱"
    ))]
    pub alipay_account: String,
}

#[derive(Serialize)]
pub struct ProfileResponse {
    /// 是否已填写完整 KYC 信息
    pub kyc_completed: bool,
    /// 真实姓名脱敏展示值
    pub real_name: Option<String>,
    /// 身份证号末 4 位（脱敏）
    pub id_card_last4: Option<String>,
    /// 手机号脱敏，例 138****8888
    pub phone_masked: Option<String>,
    /// 支付宝账号脱敏
    pub alipay_account_masked: Option<String>,
    /// 签约状态：unsigned / signing / signed / terminated
    pub sign_status: String,
    /// 当前 H5 操作：sign / release。用于把 signing 状态正确展示为签约中或解约中。
    pub sign_operation: Option<String>,
    /// 当前仍在有效期内的 H5 操作链接，仅返回给资料所属用户。
    pub sign_url: Option<String>,
    /// 当前 H5 操作的到期时间。云账户签约 token 与解约链接有效期均为 2 小时。
    pub sign_operation_expires_at: Option<DateTime<Utc>>,
    /// H5 操作是否已过期；过期后需先刷新远端状态，才能安全重新发起。
    pub sign_operation_expired: bool,
    pub signed_at: Option<DateTime<Utc>>,
    /// 是否存在仍在处理或取消中的提现订单。存在时禁止修改资料、重新签约或解约。
    pub has_active_payout: bool,
}

#[derive(Deserialize)]
struct SignNotifyData {
    #[serde(default)]
    dealer_id: String,
    #[serde(default)]
    broker_id: String,
    real_name: String,
    id_card: String,
    #[serde(default)]
    phone: String,
    #[serde(default)]
    status: i32,
    #[serde(default)]
    #[allow(dead_code)]
    event_type: String,
    #[serde(default)]
    #[allow(dead_code)]
    event_status: String,
}

impl ProfileResponse {
    fn from_db(p: Option<YunzhanghuProfile>, has_active_payout: bool) -> Self {
        let Some(p) = p else {
            return Self {
                kyc_completed: false,
                real_name: None,
                id_card_last4: None,
                phone_masked: None,
                alipay_account_masked: None,
                sign_status: YzhSignStatus::Unsigned.as_str().to_string(),
                sign_operation: None,
                sign_url: None,
                sign_operation_expires_at: None,
                sign_operation_expired: false,
                signed_at: None,
                has_active_payout,
            };
        };
        let kyc_completed = p.real_name.is_some()
            && p.id_card_encrypted.is_some()
            && p.phone.is_some()
            && p.alipay_account.is_some();
        let sign_operation = if p.sign_status == YzhSignStatus::Signing {
            Some(
                if p.sign_nonce
                    .as_deref()
                    .is_some_and(|nonce| nonce.starts_with("release:"))
                {
                    "release"
                } else {
                    "sign"
                }
                .to_string(),
            )
        } else {
            None
        };
        let is_unsign_reconcile = p
            .sign_nonce
            .as_deref()
            .and_then(unsign_reconcile_event_at)
            .is_some();
        let sign_operation_expires_at = sign_operation
            .as_ref()
            .filter(|_| !is_unsign_reconcile)
            .map(|_| {
                p.updated_at + Duration::hours(YZH_H5_OPERATION_VALID_HOURS)
            });
        let sign_operation_expired = sign_operation_expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now());
        let sign_url = if sign_operation.is_some() && !sign_operation_expired {
            p.sign_url.clone()
        } else {
            None
        };
        Self {
            kyc_completed,
            real_name: p.real_name.as_deref().map(mask_real_name),
            id_card_last4: p.id_card_last4,
            phone_masked: p.phone.as_deref().map(mask_phone),
            alipay_account_masked: p.alipay_account.as_deref().map(mask_alipay),
            sign_status: p.sign_status.as_str().to_string(),
            sign_operation,
            sign_url,
            sign_operation_expires_at,
            sign_operation_expired,
            signed_at: p.signed_at,
            has_active_payout,
        }
    }
}

// ============================================================================
// 路由
// ============================================================================

/// 查询当前用户的云账户资料 + 签约状态
#[get("profile")]
pub async fn get_profile(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?
    .1;

    let user_id = UserId::from(user.id);
    let profile = YunzhanghuProfile::get(user_id, &**pool).await?;
    let has_active_payout = has_processing_payout(user_id, &pool).await?;

    Ok(HttpResponse::Ok()
        .json(ProfileResponse::from_db(profile, has_active_payout)))
}

/// 提交或更新实名信息 + 支付宝账号。
///
/// 更新身份要素或收款账号后会强制清空签约状态，用户必须重新签约。
#[post("profile")]
pub async fn submit_profile(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<ProfileSubmit>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;

    body.validate().map_err(|err| {
        ApiError::Validation(
            crate::util::validate::validation_errors_to_string(err, None),
        )
    })?;

    // 身份证末位统一大写（云账户接口要求）
    let id_card = body.id_card.to_uppercase();
    let real_name = body.real_name.trim();
    let phone = body.phone.trim();
    let alipay_account = body.alipay_account.trim();
    let user_id = UserId::from(user.id);

    let mut tx = pool.begin().await?;
    lock_user_and_ensure_no_processing_payout(&mut tx, user_id).await?;
    let existing = YunzhanghuProfile::get(user_id, &mut *tx).await?;
    if existing
        .as_ref()
        .is_some_and(profile_has_pending_sign_operation)
    {
        return Err(ApiError::InvalidInput(
            "云账户签约或解约正在处理中，请先刷新签约状态后再修改资料。"
                .to_string(),
        ));
    }
    let kyc_changed = existing.as_ref().is_some_and(|p| {
        !profile_matches_kyc(p, real_name, &id_card, phone, alipay_account)
    });

    YunzhanghuProfile::upsert_kyc(
        &mut *tx,
        user_id,
        real_name,
        &id_card,
        phone,
        alipay_account,
    )
    .await?;
    if kyc_changed {
        YunzhanghuProfile::reset_sign_status_for_kyc_change(&mut *tx, user_id)
            .await?;
    }
    tx.commit().await?;

    let profile = YunzhanghuProfile::get(user_id, &**pool).await?;
    let has_active_payout = has_processing_payout(user_id, &pool).await?;
    Ok(HttpResponse::Ok()
        .json(ProfileResponse::from_db(profile, has_active_payout)))
}

// ============================================================================
// H5 签约
// ============================================================================

/// 发起 H5 签约，返回签约 URL（前端跳转）。
///
/// 流程：用本地 KYC 信息 → 云账户 presign 拿 token → sign 拿 H5 URL →
/// 写入本地 `sign_status=signing`、`sign_url`。
#[post("sign")]
pub async fn initiate_sign(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;
    let user_id = UserId::from(user.id);
    let sign_nonce = Uuid::new_v4().simple().to_string();
    let self_addr =
        dotenvy::var("SELF_ADDR")?.trim_end_matches('/').to_string();
    let site_url = dotenvy::var("SITE_URL")
        .unwrap_or_else(|_| "https://bbsmc.net".to_string())
        .trim_end_matches('/')
        .to_string();
    let event_callback_url = format!(
        "{}/v3/yunzhanghu/_webhook/sign/{}/{}",
        self_addr, user.id.0, sign_nonce
    );
    let redirect_url = format!("{}/yunzhanghu-result?action=sign", site_url);

    // 先用短事务写入本地签约意图，阻止其间创建新的提现；
    // 云账户 HTTP 必须在事务提交后执行。
    let mut tx = pool.begin().await?;
    lock_user_and_ensure_no_processing_payout(&mut tx, user_id).await?;
    let profile = YunzhanghuProfile::get(user_id, &mut *tx)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善实名信息与支付宝账号".to_string())
        })?;
    if profile_has_pending_sign_operation(&profile) {
        return Err(ApiError::InvalidInput(
            "已有云账户签约或解约操作正在处理中，请先刷新状态。".to_string(),
        ));
    }
    if !matches!(
        profile.sign_status,
        YzhSignStatus::Unsigned | YzhSignStatus::Terminated
    ) {
        return Err(ApiError::InvalidInput(
            "当前签约状态不允许重新发起签约。".to_string(),
        ));
    }
    let previous_status = profile.sign_status;
    let (real_name, id_card) = kyc_for_yzh_from_profile(&profile)?;
    sqlx::query!(
        "
        UPDATE user_yunzhanghu_profiles
        SET sign_status = 'signing',
            sign_url = NULL,
            sign_nonce = $2,
            updated_at = NOW()
        WHERE user_id = $1
        ",
        user_id.0,
        &sign_nonce,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let remote_result: Result<String, ApiError> = async {
        let client = YzhClient::new();
        let presign = yzh_api::h5_presign(
            &client,
            &yzh_api::PresignRequest {
                real_name: &real_name,
                id_card: &id_card,
                certificate_type: CERTIFICATE_TYPE_IDCARD,
                collect_phone_no: Some(0),
            },
        )
        .await
        .map_err(yzh_to_api_error)?;

        let sign = yzh_api::h5_sign_apply(
            &client,
            &yzh_api::SignApplyRequest {
                token: &presign.token,
                color: None,
                url: None,
                redirect_url: Some(&redirect_url),
                event_callback_url: Some(&event_callback_url),
            },
        )
        .await
        .map_err(yzh_to_api_error)?;
        Ok(sign.url)
    }
    .await;

    let sign_url = match remote_result {
        Ok(url) => url,
        Err(err) => {
            restore_failed_sign_operation(
                &pool,
                user_id,
                &sign_nonce,
                previous_status,
            )
            .await;
            return Err(err);
        }
    };

    let url_persisted = store_sign_operation_url(
        &pool,
        user_id,
        &sign_nonce,
        &sign_url,
        "签约",
    )
    .await?;

    Ok(HttpResponse::Ok().json(json!({
        "url": sign_url,
        "url_persisted": url_persisted,
    })))
}

/// 主动从云账户拉取签约状态，并同步到本地。
/// 适用于回调未到、用户刷新页面等场景。
#[post("sign/refresh")]
pub async fn refresh_sign_status(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?
    .1;
    let user_id = UserId::from(user.id);
    let requested_profile = YunzhanghuProfile::get(user_id, &**pool)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善实名信息与支付宝账号".to_string())
        })?;
    if let Some(reconcile_nonce) = requested_profile
        .sign_nonce
        .as_deref()
        .filter(|nonce| unsign_reconcile_event_at(nonce).is_some())
    {
        let outcome = reconcile_pending_unsign_nonce(
            &pool,
            &redis,
            reconcile_nonce,
            UnsignReconcileClaimMode::Force,
        )
        .await?;
        return match outcome {
            UnsignReconcileOutcome::Resolved {
                status,
                remote_status,
                signed_at,
            } => Ok(HttpResponse::Ok().json(json!({
                "sign_status": status.as_str(),
                "remote_status": remote_status,
                "signed_at": signed_at,
                "operation_pending": false,
            }))),
            UnsignReconcileOutcome::Pending { remote_status } => {
                Ok(HttpResponse::Ok().json(json!({
                    "sign_status": YzhSignStatus::Signing.as_str(),
                    "remote_status": remote_status,
                    "operation_pending": true,
                })))
            }
            UnsignReconcileOutcome::Ignored => {
                let current = YunzhanghuProfile::get(user_id, &**pool)
                    .await?
                    .ok_or_else(|| {
                    ApiError::InvalidInput(
                        "云账户资料不存在，请刷新页面。".to_string(),
                    )
                })?;
                Ok(HttpResponse::Ok().json(json!({
                    "sign_status": current.sign_status.as_str(),
                    "signed_at": current.signed_at,
                    "operation_pending": profile_has_pending_sign_operation(&current),
                })))
            }
            UnsignReconcileOutcome::NotClaimed => {
                let current = YunzhanghuProfile::get(user_id, &**pool)
                    .await?
                    .ok_or_else(|| {
                    ApiError::InvalidInput(
                        "云账户资料不存在，请刷新页面。".to_string(),
                    )
                })?;
                Ok(HttpResponse::Ok().json(json!({
                    "sign_status": current.sign_status.as_str(),
                    "operation_pending": profile_has_pending_sign_operation(&current),
                })))
            }
        };
    }
    let requested_profile_updated_at = requested_profile.updated_at;
    let (real_name, id_card) = kyc_for_yzh_from_profile(&requested_profile)?;

    let client = YzhClient::new();
    let resp = yzh_api::h5_sign_status(
        &client,
        &yzh_api::SignStatusRequest {
            real_name: &real_name,
            id_card: &id_card,
        },
    )
    .await
    .map_err(yzh_to_api_error)?;

    // 0=未签约 / 1=已签约 / 2=已解约
    let new_status = match resp.status {
        1 => YzhSignStatus::Signed,
        2 => YzhSignStatus::Terminated,
        _ => YzhSignStatus::Unsigned,
    };

    let mut tx = pool.begin().await?;
    lock_yunzhanghu_user(&mut tx, user_id).await?;
    let locked_profile = YunzhanghuProfile::get(user_id, &mut *tx)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("云账户资料不存在，请刷新页面。".to_string())
        })?;
    if locked_profile.updated_at != requested_profile_updated_at {
        return Err(ApiError::InvalidInput(
            "实名或签约资料已变化，请刷新页面后重新查询。".to_string(),
        ));
    }
    let current_status = locked_profile.sign_status;
    let was_release_operation = locked_profile
        .sign_nonce
        .as_deref()
        .is_some_and(|nonce| nonce.starts_with("release:"));
    let has_pending_operation =
        profile_has_pending_sign_operation(&locked_profile);
    let operation_expired =
        sign_operation_is_expired(&locked_profile, Utc::now());
    let unsign_reconcile_at = locked_profile
        .sign_nonce
        .as_deref()
        .and_then(unsign_reconcile_event_at);
    if new_status == YzhSignStatus::Signed
        && !matches!(
            current_status,
            YzhSignStatus::Signing | YzhSignStatus::Signed
        )
    {
        log::warn!(
            "忽略非签约中用户的云账户签约状态刷新 user_id={} current_status={}",
            user_id.0,
            current_status
        );
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(json!({
            "sign_status": current_status.as_str(),
            "remote_status": resp.status,
            "signed_at": resp.signed_at,
            "requires_new_sign": true,
        })));
    }
    let remote_still_before_operation = if was_release_operation {
        new_status == YzhSignStatus::Signed
    } else {
        new_status != YzhSignStatus::Signed
    };
    let remote_signed_at = parse_yzh_event_time(&resp.signed_at);
    let keep_unsign_reconcile = unsign_reconcile_at.is_some_and(|release_at| {
        new_status != YzhSignStatus::Terminated
            && !(new_status == YzhSignStatus::Signed
                && remote_signed_at
                    .is_some_and(|signed_at| signed_at > release_at))
    });
    if keep_unsign_reconcile
        || (unsign_reconcile_at.is_none()
            && should_keep_pending_sign_operation(
                has_pending_operation,
                was_release_operation,
                new_status,
                operation_expired,
            ))
    {
        // H5 仍在有效期内且远端尚未完成本次操作时，必须保留本地 intent。
        // 否则旧链接稍后完成操作时，可能与资料修改或新提现并发。
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(json!({
            "sign_status": YzhSignStatus::Signing.as_str(),
            "remote_sign_status": new_status.as_str(),
            "remote_status": resp.status,
            "signed_at": resp.signed_at,
            "operation_pending": true,
        })));
    }
    if new_status == YzhSignStatus::Terminated
        && has_processing_payout_in_tx(&mut tx, user_id).await?
    {
        // 远端终态优先落库，阻止继续创建新提现；现有订单交由查单/回调完成，
        // 同时留下高优先级日志供人工核对。
        log::error!(
            "云账户已解约但仍有处理中提现，需要人工核对 user_id={}",
            user_id.0
        );
    }
    YunzhanghuProfile::update_sign_status(
        &mut *tx, user_id, new_status, None, None,
    )
    .await?;
    tx.commit().await?;

    notify_yunzhanghu_sign_status_change(
        &pool,
        &redis,
        user_id,
        current_status,
        new_status,
    )
    .await;

    Ok(HttpResponse::Ok().json(json!({
        "sign_status": new_status.as_str(),
        "remote_status": resp.status,
        "signed_at": resp.signed_at,
        "operation_expired": has_pending_operation
            && remote_still_before_operation
            && operation_expired,
    })))
}

/// 申请解约：返回 H5 解约页面 URL，前端弹二维码让用户扫码完成手机号/人脸验证。
/// 解约结果由云账户通过解约回调（`_webhook/unsign`）通知 BBSMC。
#[post("sign/release")]
pub async fn release_sign(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;
    let user_id = UserId::from(user.id);
    let site_url = dotenvy::var("SITE_URL")
        .unwrap_or_else(|_| "https://bbsmc.net".to_string())
        .trim_end_matches('/')
        .to_string();
    let redirect_url = format!("{}/yunzhanghu-result?action=release", site_url);
    let release_nonce = format!("release:{}", Uuid::new_v4().simple());

    // 写入本地解约意图后再调用云账户。提现创建会要求 signed 且无 operation nonce，
    // 因而解约 H5 有效期间不会出现新提现。
    let mut tx = pool.begin().await?;
    lock_user_and_ensure_no_processing_payout(&mut tx, user_id).await?;
    let profile = YunzhanghuProfile::get(user_id, &mut *tx)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善实名信息与支付宝账号".to_string())
        })?;
    if profile_has_pending_sign_operation(&profile) {
        return Err(ApiError::InvalidInput(
            "已有云账户签约或解约操作正在处理中，请先刷新状态。".to_string(),
        ));
    }
    if profile.sign_status != YzhSignStatus::Signed {
        return Err(ApiError::InvalidInput(
            "当前用户尚未完成签约，不能申请解约。".to_string(),
        ));
    }
    let (real_name, id_card) = kyc_for_yzh_from_profile(&profile)?;
    sqlx::query!(
        "
        UPDATE user_yunzhanghu_profiles
        SET sign_status = 'signing',
            sign_url = NULL,
            sign_nonce = $2,
            updated_at = NOW()
        WHERE user_id = $1
        ",
        user_id.0,
        &release_nonce,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let client = YzhClient::new();
    let remote_result = yzh_api::h5_release_apply(
        &client,
        &yzh_api::SignReleaseApplyRequest {
            real_name: &real_name,
            id_card: &id_card,
            color: None,
            // 不传 url，使用云账户后台配置的「解约回调地址」(/_webhook/unsign)
            url: None,
            redirect_url: Some(&redirect_url),
        },
    )
    .await;
    let resp = match remote_result {
        Ok(resp) => resp,
        Err(err) => {
            restore_failed_sign_operation(
                &pool,
                user_id,
                &release_nonce,
                YzhSignStatus::Signed,
            )
            .await;
            return Err(yzh_to_api_error(err));
        }
    };
    let url_persisted = store_sign_operation_url(
        &pool,
        user_id,
        &release_nonce,
        &resp.url,
        "解约",
    )
    .await?;

    Ok(HttpResponse::Ok().json(json!({
        "url": resp.url,
        "remote_status": resp.status,
        "url_persisted": url_persisted,
    })))
}

// ============================================================================
// 回调接收
// ============================================================================

/// 签约事件异步通知。云账户 POST form-urlencoded 到本端点。
/// `user_id` 在发起签约时编码进 event_callback_url 路径，回调时取出定位用户。
///
/// 新版回调字段：`status` 是 Int（0=未签约 / 1=已签约 / 2=已解约），
/// 还会带 `event_type` + `event_status` 描述具体事件。
#[post("_webhook/sign/{user_id}/{nonce}")]
pub async fn sign_callback(
    path: web::Path<(u64, String)>,
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let (user_id_raw, nonce) = path.into_inner();
    let notify: SignNotifyData = form.decode().map_err(yzh_to_api_error)?;

    let replay_key = format!(
        "path:{}:{}:{}",
        user_id_raw,
        nonce,
        notify_replay_key("sign", &form)
    );
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }

    let user_id = UserId(user_id_raw as i64);
    let new_status = match notify.status {
        1 => YzhSignStatus::Signed,
        2 => YzhSignStatus::Terminated,
        // 0 或其他：可能是签约事件失败但状态尚未变化，先不更新本地
        _ => {
            mark_notify_replay(&redis, &replay_key).await?;
            return Ok(HttpResponse::Ok().body("success"));
        }
    };

    let Some(profile) = YunzhanghuProfile::get(user_id, &**pool).await? else {
        log::warn!("收到未知用户的云账户签约回调 user_id={}", user_id.0);
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    };

    if profile.sign_nonce.as_deref() != Some(nonce.as_str()) {
        log::warn!(
            "云账户签约回调 nonce 不匹配 user_id={} status={}",
            user_id.0,
            notify.status
        );
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    let creds =
        crate::util::yunzhanghu::secrets::load().map_err(yzh_to_api_error)?;
    if !sign_notify_matches_profile(&notify, &profile, creds) {
        log::warn!(
            "云账户签约回调身份不匹配 user_id={} status={}",
            user_id.0,
            notify.status
        );
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    if new_status == YzhSignStatus::Signed
        && !matches!(
            profile.sign_status,
            YzhSignStatus::Signing | YzhSignStatus::Signed
        )
    {
        log::warn!(
            "忽略非签约中用户的云账户签约成功回调 user_id={} current_status={}",
            user_id.0,
            profile.sign_status
        );
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    let mut tx = pool.begin().await?;
    lock_yunzhanghu_user(&mut tx, user_id).await?;
    let Some(locked_profile) =
        YunzhanghuProfile::get(user_id, &mut *tx).await?
    else {
        tx.commit().await?;
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    };
    if locked_profile.sign_nonce.as_deref() != Some(nonce.as_str())
        || !sign_notify_matches_profile(&notify, &locked_profile, creds)
    {
        log::warn!(
            "忽略锁定后资料已变化的云账户签约回调 user_id={} status={}",
            user_id.0,
            notify.status
        );
        tx.commit().await?;
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }
    let previous_status = locked_profile.sign_status;
    if new_status == YzhSignStatus::Terminated
        && has_processing_payout_in_tx(&mut tx, user_id).await?
    {
        // 远端终态必须落库以阻止后续新提现；已存在订单继续依赖订单回调/查单，
        // 并通过错误日志进入人工核对队列。
        log::error!(
            "云账户解约回调到达时仍有处理中提现，需要人工核对 user_id={}",
            user_id.0
        );
    }

    YunzhanghuProfile::update_sign_status(
        &mut *tx, user_id, new_status, None, None,
    )
    .await?;
    tx.commit().await?;

    notify_yunzhanghu_sign_status_change(
        &pool,
        &redis,
        user_id,
        previous_status,
        new_status,
    )
    .await;

    mark_notify_replay(&redis, &replay_key).await?;

    // 云账户协议要求返回 "success" 字符串，否则会重试
    Ok(HttpResponse::Ok().body("success"))
}

/// 解约异步通知。云账户后台配置的"解约回调地址"指向本端点。
/// 与签约回调不同，解约回调里没有 user_id（不通过 notify_url 传），
/// 我们通过身份证末 4 位筛候选，再解密完整身份要素定位用户。
#[post("_webhook/unsign")]
pub async fn unsign_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    handle_unsign_callback(form, pool, redis).await
}

pub async fn handle_unsign_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    #[derive(Deserialize)]
    struct UnsignNotifyData {
        #[serde(default)]
        dealer_id: String,
        #[serde(default)]
        broker_id: String,
        real_name: String,
        id_card: String,
        #[serde(default)]
        #[allow(dead_code)]
        release_type: String,
        #[serde(default)]
        #[allow(dead_code)]
        release_reason: String,
        #[serde(default, alias = "cancellation_time")]
        release_time: String,
    }

    let notify: UnsignNotifyData = form.decode().map_err(yzh_to_api_error)?;
    let replay_key = notify_replay_key("unsign", &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }

    let creds =
        crate::util::yunzhanghu::secrets::load().map_err(yzh_to_api_error)?;
    if !notify_party_matches_credentials(
        &notify.dealer_id,
        &notify.broker_id,
        creds,
    ) {
        log::warn!("云账户解约回调 dealer/broker 不匹配");
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    let id_card_chars: Vec<char> = notify.id_card.chars().collect();
    let last4: String = if id_card_chars.len() >= 4 {
        id_card_chars[id_card_chars.len() - 4..].iter().collect()
    } else {
        notify.id_card.clone()
    };

    // 候选用户：先按身份证末 4 位筛选，再解密完整身份要素匹配。
    let candidates = sqlx::query!(
        "
        SELECT user_id
        FROM user_yunzhanghu_profiles
        WHERE id_card_last4 = $1 AND sign_status IN ('signed', 'signing')
        ",
        last4,
    )
    .fetch_all(&**pool)
    .await?;

    // 在候选里逐个解密匹配（同末 4 位的用户很少）。同一身份可能因历史
    // 数据绑定多个本地账号，云账户签约状态按身份生效，因此必须全部同步。
    let mut matching_profiles = Vec::new();
    for row in candidates {
        let Some(profile) =
            YunzhanghuProfile::get(UserId(row.user_id), &**pool).await?
        else {
            continue;
        };
        let real_name_matches = profile
            .real_name
            .as_deref()
            .is_some_and(|name| name.trim() == notify.real_name.trim());
        let id_card_matches = match profile.decrypt_id_card() {
            Ok(Some(id_card)) => id_card == notify.id_card,
            Ok(None) => false,
            Err(err) => {
                log::error!(
                    "云账户解约回调匹配候选资料时解密失败 user_id={}: {}",
                    profile.user_id.0,
                    err
                );
                return Err(ApiError::InvalidInput(
                    "云账户实名资料暂时无法解密，请稍后重试通知".to_string(),
                ));
            }
        };
        if real_name_matches && id_card_matches {
            matching_profiles.push(profile);
        }
    }

    if matching_profiles.is_empty() {
        log::warn!("云账户解约回调未匹配到本地实名资料");
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    matching_profiles.sort_by_key(|profile| profile.user_id.0);
    let release_at =
        parse_yzh_event_time(&notify.release_time).ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户解约回调缺少可解析的 release_time，请重试通知"
                    .to_string(),
            )
        })?;
    // envelope 的 timestamp/sign 可能在重试时变化，不能拿 replay key 当业务
    // nonce。使用已验签解密的身份 + 解约时间和应用密钥生成稳定 HMAC，同一业务
    // 事件在多实例与多次重试中始终竞争同一 reconciliation 行。
    let reconcile_nonce = unsign_reconciliation_nonce(
        creds,
        &notify.real_name,
        &notify.id_card,
        release_at,
    );

    // 可信解约回调一到达，先在一个短事务内锁定同一身份的全部本地账号并写入
    // pending marker。提现创建也会锁用户并要求 signed + 无 nonce，因此远端状态
    // 尚未最终一致时不会出现新的提现。事件时间晚于最近签约事件的账号才进入核对，
    // 旧回调不会覆盖后来完成的新签约。
    let all_user_ids = matching_profiles
        .iter()
        .map(|profile| profile.user_id.0)
        .collect::<Vec<_>>();
    let mut block_tx = pool.begin().await?;
    sqlx::query!(
        "
        INSERT INTO yunzhanghu_unsign_reconciliations (
            nonce, release_at, next_attempt_at
        )
        VALUES ($1, $2, NOW())
        ON CONFLICT (nonce) DO NOTHING
        ",
        &reconcile_nonce,
        release_at,
    )
    .execute(&mut *block_tx)
    .await?;
    let reconciliation = sqlx::query!(
        "
        SELECT status, release_at
        FROM yunzhanghu_unsign_reconciliations
        WHERE nonce = $1
        FOR UPDATE
        ",
        &reconcile_nonce,
    )
    .fetch_one(&mut *block_tx)
    .await?;
    if reconciliation.release_at != release_at {
        return Err(ApiError::InvalidInput(
            "云账户解约核对事件时间不一致".to_string(),
        ));
    }
    if reconciliation.status == "resolved" {
        block_tx.commit().await?;
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }
    lock_yunzhanghu_users(&mut block_tx, &all_user_ids).await?;
    let mut reconcile_user_ids = Vec::new();
    for snapshot in &matching_profiles {
        let Some(locked_profile) =
            YunzhanghuProfile::get(snapshot.user_id, &mut *block_tx).await?
        else {
            return Err(ApiError::InvalidInput(
                "签约资料并发变化，请稍后重试解约通知".to_string(),
            ));
        };
        if locked_profile.updated_at != snapshot.updated_at
            && locked_profile.sign_nonce.as_deref()
                != Some(reconcile_nonce.as_str())
        {
            return Err(ApiError::InvalidInput(
                "签约资料并发变化，请稍后重试解约通知".to_string(),
            ));
        }
        if !yunzhanghu_identity_matches(
            &locked_profile,
            &notify.real_name,
            &notify.id_card,
        ) {
            return Err(ApiError::InvalidInput(
                "签约身份并发变化，请稍后重试解约通知".to_string(),
            ));
        }
        if unsign_event_is_stale(&locked_profile, release_at, &reconcile_nonce)
        {
            log::warn!(
                "忽略早于最近本地签约事件的解约回调 user_id={} release_time={}",
                locked_profile.user_id.0,
                notify.release_time
            );
            continue;
        }
        if has_processing_payout_in_tx(&mut block_tx, locked_profile.user_id)
            .await?
        {
            log::error!(
                "云账户解约回调到达时仍有处理中提现，需要人工核对 user_id={}",
                locked_profile.user_id.0
            );
        }
        reconcile_user_ids.push(locked_profile.user_id.0);
    }
    if !reconcile_user_ids.is_empty() {
        sqlx::query!(
            r#"
            UPDATE user_yunzhanghu_profiles
            SET sign_status = 'signing',
                sign_url = NULL,
                sign_nonce = $2,
                updated_at = NOW()
            WHERE user_id = ANY($1::bigint[])
            "#,
            &reconcile_user_ids,
            &reconcile_nonce,
        )
        .execute(&mut *block_tx)
        .await?;
    } else {
        sqlx::query!(
            "
            UPDATE yunzhanghu_unsign_reconciliations
            SET status = 'resolved',
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_error = 'stale_event',
                resolved_status = 'ignored',
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE nonce = $1 AND status = 'pending'
            ",
            &reconcile_nonce,
        )
        .execute(&mut *block_tx)
        .await?;
    }
    block_tx.commit().await?;

    if reconcile_user_ids.is_empty() {
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    match reconcile_pending_unsign_nonce(
        &pool,
        &redis,
        &reconcile_nonce,
        UnsignReconcileClaimMode::Force,
    )
    .await?
    {
        UnsignReconcileOutcome::Resolved { .. }
        | UnsignReconcileOutcome::Ignored => {
            mark_notify_replay(&redis, &replay_key).await?;
            Ok(HttpResponse::Ok().body("success"))
        }
        UnsignReconcileOutcome::Pending { .. }
        | UnsignReconcileOutcome::NotClaimed => Err(ApiError::InvalidInput(
            "云账户解约状态尚未同步，请稍后重试通知".to_string(),
        )),
    }
}

// ============================================================================
// 实时支付：订单状态回调 + 主动查单
// ============================================================================

/// 订单状态异步通知。云账户 POST form-urlencoded 到本端点。
/// 验签 + 解密后根据 `status` 字段更新 `payouts.status`。
#[post("_webhook/order")]
pub async fn order_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    handle_order_callback(form, pool, redis).await
}

pub async fn handle_order_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let payload: OrderNotifyPayload =
        form.decode().map_err(yzh_to_api_error)?;
    let (notify_id, notify) = match payload {
        OrderNotifyPayload::Wrapped {
            notify_id, data, ..
        } => (notify_id, data),
        OrderNotifyPayload::Flat(data) => (String::new(), data),
    };

    let replay_key = notify_replay_key_with_id("order", &notify_id, &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }

    // 支付渠道可能在订单成功后退汇。先主动查单确认同一订单、同一提交净额且
    // refund_origin=2，再改用历史订单不可变快照校验并放行 Success -> Cancelled。
    let verified_channel_return =
        if matches!(notify.status.trim(), "4" | "refund") {
            let client = YzhClient::new();
            let resp = yzh_api::query_order(
                &client,
                &yzh_api::QueryOrderRequest {
                    order_id: &notify.order_id,
                    channel: "支付宝",
                },
            )
            .await
            .map_err(yzh_to_api_error)?;
            if !channel_return_query_matches_callback(&notify, &resp) {
                return Err(ApiError::InvalidInput(
                    "云账户退汇回调未通过主动查单确认，请稍后重试通知"
                        .to_string(),
                ));
            }
            true
        } else {
            false
        };

    let outcome = apply_order_status(
        &pool,
        &redis,
        &notify.order_id,
        &notify.status,
        &notify.status_detail_message,
        non_empty_ref(&notify.ref_id),
        order_callback_evidence(&notify, verified_channel_return),
    )
    .await?;

    ensure_order_callback_status_applied(outcome)?;

    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
}

/// 劳动者主动退款回调；云账户当前仅通知全额退款（refund_type=0）。
#[post("_webhook/refund")]
pub async fn refund_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    handle_refund_callback(form, pool, redis).await
}

pub async fn handle_refund_callback(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let payload: RefundNotifyPayload =
        form.decode().map_err(yzh_to_api_error)?;
    let (notify_id, notify) = match payload {
        RefundNotifyPayload::Wrapped {
            notify_id, data, ..
        } => (notify_id, data),
        RefundNotifyPayload::Flat(data) => (String::new(), data),
    };
    let replay_key = notify_replay_key_with_id("refund", &notify_id, &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }

    let is_current_full_refund = notify.refund_type.trim() == "0";
    let is_legacy_success = notify.refund_type.trim().is_empty()
        && (notify.refund_status.eq_ignore_ascii_case("success")
            || notify.refund_status == "1");

    let evidence = if is_current_full_refund {
        let refund_total_amount_raw =
            non_empty_str(&notify.refund_total_amount).ok_or_else(|| {
                ApiError::InvalidInput(
                    "云账户退款回调缺少退款总金额".to_string(),
                )
            })?;
        let refund_total_amount = parse_yzh_decimal_option(
            refund_total_amount_raw,
            "refund.refund_total_amount",
        )?
        .filter(|amount| *amount > rust_decimal::Decimal::ZERO)
        .ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户退款回调缺少合法的退款总金额".to_string(),
            )
        })?;
        let dealer_id = non_empty_str(&notify.dealer_id).ok_or_else(|| {
            ApiError::InvalidInput("云账户退款回调缺少平台企业 ID".to_string())
        })?;
        let broker_id = non_empty_str(&notify.broker_id).ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户退款回调缺少综合服务主体 ID".to_string(),
            )
        })?;
        let _real_name = non_empty_str(&notify.real_name).ok_or_else(|| {
            ApiError::InvalidInput("云账户退款回调缺少劳动者姓名".to_string())
        })?;
        let _id_card = non_empty_str(&notify.id_card).ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户退款回调缺少劳动者身份证号".to_string(),
            )
        })?;
        let card_no = non_empty_str(&notify.card_no).ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户退款回调缺少劳动者收款账号".to_string(),
            )
        })?;

        log::info!(
            "收到云账户全额退款 order_id={} refund_total_amount={} refund_ref={}",
            notify.order_id,
            refund_total_amount.round_dp(2),
            notify.refund_ref
        );

        OrderStatusEvidence {
            require_callback_fields: false,
            allow_success_refund: true,
            // 仅在平台实际收回的退款总额等于本站提交 pay 时自动整单回退。
            pay: Some(refund_total_amount_raw),
            dealer_id: Some(dealer_id),
            broker_id: Some(broker_id),
            // 回调身份属于原订单；提现成功后当前 profile 允许变更，不能拿可变
            // 资料做历史订单绑定。原收款账号使用 payout.method_address 快照校验。
            real_name: None,
            id_card: None,
            phone_no: None,
            card_no: Some(card_no),
        }
    } else if is_legacy_success {
        let pay_for_validation =
            non_empty_str(&notify.pay).ok_or_else(|| {
                ApiError::InvalidInput(
                    "云账户退款回调缺少原订单金额 pay".to_string(),
                )
            })?;
        let original_pay =
            parse_yzh_decimal_option(pay_for_validation, "refund.pay")?
                .ok_or_else(|| {
                    ApiError::InvalidInput(
                        "云账户退款回调缺少原订单金额 pay".to_string(),
                    )
                })?;
        let refund_amount = parse_yzh_decimal_option(
            &notify.refund_amount,
            "refund.refund_amount",
        )?
        .ok_or_else(|| {
            ApiError::InvalidInput(
                "云账户退款回调缺少退款金额 refund_amount".to_string(),
            )
        })?;
        ensure_full_refund_amount(original_pay, refund_amount)?;

        OrderStatusEvidence {
            require_callback_fields: false,
            allow_success_refund: true,
            pay: Some(pay_for_validation),
            ..OrderStatusEvidence::empty()
        }
    } else {
        return Err(ApiError::InvalidInput(
            "无法识别云账户退款回调状态或退款类型".to_string(),
        ));
    };

    // 云账户当前只通知 refund_type=0 的全额退款。状态改为 Cancelled 后，
    // 用户余额按现有 payout 账务规则自动回退。
    let outcome = apply_order_status(
        &pool,
        &redis,
        &notify.order_id,
        "cancelled",
        "用户全额退款",
        non_empty_ref(&notify.ref_id),
        evidence,
    )
    .await?;
    ensure_refund_status_applied(outcome)?;

    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
}

/// 预付业务服务费充值通知（平台向云账户充值业务服务费后回调）。
/// 当前 BBSMC 无需特殊处理，仅校验签名后日志记录。
#[post("_webhook/prepay")]
pub async fn prepay_callback(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    handle_prepay_callback(form, redis).await
}

pub async fn handle_prepay_callback(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let body: serde_json::Value = form.decode().map_err(yzh_to_api_error)?;
    let replay_key = notify_replay_key("prepay", &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }
    log::info!(
        "收到云账户预付服务费回调 payload_keys={}",
        json_payload_keys(&body)
    );
    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
}

/// 余额提现回调（平台从云账户对公账户提现到自己银行卡）。
/// BBSMC 不主动发起此操作，仅记录。
#[post("_webhook/balance")]
pub async fn balance_callback(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    handle_balance_callback(form, redis).await
}

pub async fn handle_balance_callback(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let body: serde_json::Value = form.decode().map_err(yzh_to_api_error)?;
    let replay_key = notify_replay_key("balance", &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }
    log::info!(
        "收到云账户余额提现回调 payload_keys={}",
        json_payload_keys(&body)
    );
    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
}

/// 主动从云账户查询某笔 payout 的最新状态并同步到本地。
/// 用户可在转账记录页手动触发；定时任务可遍历 in-transit 订单兜底。
#[post("/payouts/{id}/refresh")]
pub async fn refresh_payout_status(
    path: web::Path<crate::models::ids::PayoutId>,
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?
    .1;

    let pid = path.into_inner();
    let payout_db_id: crate::database::models::PayoutId = pid.into();

    let payout = crate::database::models::payout_item::Payout::get(
        payout_db_id,
        &**pool,
    )
    .await?
    .ok_or_else(|| ApiError::InvalidInput("提现订单不存在".to_string()))?;

    // 仅订单所有人或管理员可刷新
    if payout.user_id != user.id.into() && !user.role.is_admin() {
        return Err(ApiError::CustomAuthentication(
            "无权访问此提现订单".to_string(),
        ));
    }

    // 已终态不再刷新
    use crate::models::payouts::PayoutStatus;
    if matches!(
        payout.status,
        PayoutStatus::Success | PayoutStatus::Cancelled | PayoutStatus::Failed
    ) {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": payout.status.as_str(),
            "message": "订单已是终态，无需刷新",
        })));
    }

    let submit_row = if payout.platform_id.is_none() {
        sqlx::query!(
            "
            SELECT yunzhanghu_order_id, yunzhanghu_submit_started_at
            FROM payouts
            WHERE id = $1
            ",
            payout_db_id.0
        )
        .fetch_optional(&**pool)
        .await?
    } else {
        None
    };

    if payout.platform_id.is_none()
        && submit_row
            .as_ref()
            .and_then(|row| row.yunzhanghu_submit_started_at)
            .is_none()
    {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": payout.status.as_str(),
            "message": "提现正在等待管理员确认转账",
            "requires_admin_confirmation": true,
        })));
    }

    let order_id = submit_row
        .as_ref()
        .and_then(|row| row.yunzhanghu_order_id.clone())
        .unwrap_or_else(|| format!("bbsmc-{}", pid));
    let client = YzhClient::new();
    let resp = yzh_api::query_order(
        &client,
        &yzh_api::QueryOrderRequest {
            order_id: &order_id,
            channel: "支付宝",
        },
    )
    .await
    .map_err(yzh_to_api_error)?;

    let outcome = apply_order_status(
        &pool,
        &redis,
        &resp.order_id,
        &resp.status,
        &resp.status_detail_message,
        non_empty_ref(&resp.ref_id),
        OrderStatusEvidence {
            require_callback_fields: false,
            pay: non_empty_str(&resp.pay),
            ..OrderStatusEvidence::empty()
        },
    )
    .await?;
    ensure_order_status_evidence_accepted(outcome)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "order_id": resp.order_id,
        "remote_status": resp.status,
        "status_detail_message": resp.status_detail_message,
        "ref": resp.ref_id,
    })))
}

/// 公共：根据云账户订单状态字符串更新本地 payouts.status
pub async fn apply_order_status(
    pool: &PgPool,
    redis: &RedisPool,
    order_id: &str,
    remote_status: &str,
    status_message: &str,
    ref_id: Option<&str>,
    evidence: OrderStatusEvidence<'_>,
) -> Result<OrderStatusApplyOutcome, ApiError> {
    let Some(new_status) =
        crate::util::yunzhanghu::api::map_order_status(remote_status)
    else {
        log::warn!(
            "未识别的云账户订单状态 order_id={} status={}",
            order_id,
            remote_status
        );
        return Ok(OrderStatusApplyOutcome::Ignored);
    };

    // 找到对应 payout 记录。我们的 order_id = "bbsmc-{base62 payout_id}"。
    let Some(payout_id_str) = order_id.strip_prefix("bbsmc-") else {
        log::warn!("忽略非本站云账户订单号回调 order_id={}", order_id);
        return Ok(OrderStatusApplyOutcome::Ignored);
    };
    let payout_db_id: i64 =
        match crate::models::ids::base62_impl::parse_base62(payout_id_str) {
            Ok(n) => n as i64,
            Err(_) => {
                log::warn!("无法解析 order_id={}", order_id);
                return Ok(OrderStatusApplyOutcome::Ignored);
            }
        };

    let mut tx = pool.begin().await?;

    // 锁定 payout 行，避免并发回调重复通知或终态竞争。
    let row = sqlx::query!(
        "
        SELECT user_id, status, amount, fee, method, method_address,
               yunzhanghu_order_id, yunzhanghu_submit_started_at
        FROM payouts
        WHERE id = $1
        FOR UPDATE
        ",
        payout_db_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        log::warn!("收到未知 order_id 的回调: {}", order_id);
        return Ok(OrderStatusApplyOutcome::Ignored);
    };

    if row.method.as_deref()
        != Some(
            crate::models::payouts::PayoutMethodType::YunzhanghuAlipay.as_str(),
        )
    {
        log::warn!("忽略非云账户支付宝提现订单回调 order_id={}", order_id);
        return Ok(OrderStatusApplyOutcome::Ignored);
    }

    if let Some(stored_order_id) = row.yunzhanghu_order_id.as_deref()
        && stored_order_id != order_id
    {
        log::warn!(
            "云账户订单回调 order_id 与本地记录不匹配 payout_id={} stored_order_id={} callback_order_id={}",
            payout_db_id,
            stored_order_id,
            order_id
        );
        return Ok(OrderStatusApplyOutcome::Ignored);
    }

    if row.yunzhanghu_order_id.is_none()
        && row.yunzhanghu_submit_started_at.is_none()
    {
        log::warn!(
            "忽略未提交云账户的提现订单回调 payout_id={} order_id={}",
            payout_db_id,
            order_id
        );
        return Ok(OrderStatusApplyOutcome::Ignored);
    }

    if !validate_order_status_evidence(
        &mut tx,
        payout_db_id,
        crate::database::models::UserId(row.user_id),
        row.amount,
        row.fee.unwrap_or_default(),
        row.method_address.as_deref(),
        evidence,
    )
    .await?
    {
        return Ok(OrderStatusApplyOutcome::EvidenceRejected);
    }

    // 幂等：终态不能回滚
    let current =
        crate::models::payouts::PayoutStatus::from_string(&row.status);
    if should_ignore_terminal_order_transition(
        current,
        new_status,
        evidence.allow_success_refund,
    ) {
        log::info!(
            "忽略状态回退 order_id={} 当前={} 收到={}",
            order_id,
            current,
            new_status
        );
        return Ok(OrderStatusApplyOutcome::Ignored);
    }

    let should_notify_success = current
        != crate::models::payouts::PayoutStatus::Success
        && new_status == crate::models::payouts::PayoutStatus::Success;
    let should_notify_terminal_failure = !matches!(
        current,
        crate::models::payouts::PayoutStatus::Failed
            | crate::models::payouts::PayoutStatus::Cancelled
    ) && matches!(
        new_status,
        crate::models::payouts::PayoutStatus::Failed
            | crate::models::payouts::PayoutStatus::Cancelled
    );

    let new_status_str = new_status.as_str();
    sqlx::query!(
        "
        UPDATE payouts
        SET status = $1::text,
            platform_id = COALESCE(NULLIF($2, ''), platform_id),
            yunzhanghu_submit_finished_at = CASE
                WHEN NULLIF($2, '') IS NOT NULL OR $1::text <> 'in-transit'
                THEN NOW()
                ELSE yunzhanghu_submit_finished_at
            END,
            yunzhanghu_submit_error = NULL
        WHERE id = $3
        ",
        new_status_str,
        ref_id,
        payout_db_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // 用户余额可能因为 payout 被改为非 in-transit 而回退，清缓存让前端刷新
    crate::database::models::User::clear_caches(
        &[(crate::database::models::UserId(row.user_id), None)],
        redis,
    )
    .await?;

    if should_notify_success
        && let Err(e) = insert_payout_success_notification(
            pool,
            redis,
            crate::database::models::UserId(row.user_id),
            row.amount,
        )
        .await
    {
        log::warn!(
            "提现成功通知写入失败 payout_id={} user_id={}: {}",
            payout_db_id,
            row.user_id,
            e
        );
    }

    if should_notify_terminal_failure
        && let Err(e) = insert_payout_terminal_notification(
            pool,
            redis,
            crate::database::models::UserId(row.user_id),
            row.amount,
            new_status,
            status_message,
        )
        .await
    {
        log::warn!(
            "提现终态通知写入失败 payout_id={} user_id={} status={}: {}",
            payout_db_id,
            row.user_id,
            new_status,
            e
        );
    }

    if new_status == crate::models::payouts::PayoutStatus::Success
        && let Err(e) = sync_yunzhanghu_order_details(pool, order_id).await
    {
        log::warn!(
            "云账户成功订单费用明细回填失败 order_id={}: {}",
            order_id,
            e
        );
    }

    Ok(OrderStatusApplyOutcome::Applied)
}

async fn insert_payout_terminal_notification(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: crate::database::models::UserId,
    amount: rust_decimal::Decimal,
    status: crate::models::payouts::PayoutStatus,
    status_message: &str,
) -> Result<(), crate::database::models::DatabaseError> {
    let (notification_type, name, status_text) = match status {
        crate::models::payouts::PayoutStatus::Failed => (
            "payout_failed",
            "提现失败",
            "提现失败，金额已退回到可提现余额。",
        ),
        crate::models::payouts::PayoutStatus::Cancelled => (
            "payout_cancelled",
            "提现已取消",
            "提现已取消，金额已退回到可提现余额。",
        ),
        _ => return Ok(()),
    };
    let detail = status_message.trim();
    let detail_text = if detail.is_empty() {
        "请在转账记录中查看详情。".to_string()
    } else {
        format!("原因：{}", detail)
    };

    let mut tx = pool.begin().await?;
    crate::database::models::notification_item::NotificationBuilder {
        body: crate::models::notifications::NotificationBody::LegacyMarkdown {
            notification_type: Some(notification_type.to_string()),
            name: name.to_string(),
            text: format!(
                "您的 {} {}{}",
                format_payout_amount(amount),
                status_text,
                detail_text
            ),
            link: "/dashboard/revenue/transfers".to_string(),
            actions: vec![],
        },
    }
    .insert(user_id, &mut tx, redis)
    .await?;
    tx.commit().await?;

    Ok(())
}

async fn insert_payout_success_notification(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: crate::database::models::UserId,
    amount: rust_decimal::Decimal,
) -> Result<(), crate::database::models::DatabaseError> {
    let mut tx = pool.begin().await?;
    crate::database::models::notification_item::NotificationBuilder {
        body: crate::models::notifications::NotificationBody::LegacyMarkdown {
            notification_type: Some("payout_success".to_string()),
            name: "提现已到账".to_string(),
            text: format!(
                "您的 {} 提现已成功到账，请在转账记录中查看详情。",
                format_payout_amount(amount)
            ),
            link: "/dashboard/revenue/transfers".to_string(),
            actions: vec![],
        },
    }
    .insert(user_id, &mut tx, redis)
    .await?;
    tx.commit().await?;

    Ok(())
}

fn format_payout_amount(amount: rust_decimal::Decimal) -> String {
    format!("¥{:.2}", amount.round_dp(2))
}

async fn sync_yunzhanghu_order_details(
    pool: &PgPool,
    order_id: &str,
) -> Result<(), ApiError> {
    let client = YzhClient::new();
    let resp = yzh_api::query_order(
        &client,
        &yzh_api::QueryOrderRequest {
            order_id,
            channel: "支付宝",
        },
    )
    .await
    .map_err(yzh_to_api_error)?;

    if crate::util::yunzhanghu::api::map_order_status(&resp.status)
        != Some(crate::models::payouts::PayoutStatus::Success)
    {
        return Ok(());
    }

    upsert_yunzhanghu_order_details(pool, &resp).await
}

async fn upsert_yunzhanghu_order_details(
    pool: &PgPool,
    resp: &yzh_api::QueryOrderResponse,
) -> Result<(), ApiError> {
    let Some(payout_id_str) = resp.order_id.strip_prefix("bbsmc-") else {
        log::warn!("忽略非本站云账户订单费用明细 order_id={}", resp.order_id);
        return Ok(());
    };
    let payout_db_id: i64 =
        match crate::models::ids::base62_impl::parse_base62(payout_id_str) {
            Ok(n) => n as i64,
            Err(_) => {
                log::warn!(
                    "无法解析云账户订单费用明细 order_id={}",
                    resp.order_id
                );
                return Ok(());
            }
        };

    let pay = parse_yzh_decimal_or_zero(&resp.pay, "pay")?;
    let user_real_amount =
        parse_yzh_decimal_option(&resp.user_real_amount, "user_real_amount")?;
    let user_real_excluding_vat_amount = parse_yzh_decimal_option(
        &resp.user_real_excluding_vat_amount,
        "user_real_excluding_vat_amount",
    )?;
    let user_fee = parse_yzh_decimal_or_zero(&resp.user_fee, "user_fee")?;
    let received_user_fee = parse_yzh_decimal_or_zero(
        &resp.received_user_fee,
        "received_user_fee",
    )?;
    let tax = parse_yzh_decimal_or_zero(&resp.tax, "tax")?;
    let received_tax_amount = parse_yzh_decimal_or_zero(
        &resp.received_tax_amount,
        "received_tax_amount",
    )?;
    let personal_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.personal_tax,
        "tax_detail.personal_tax",
    )?;
    let value_added_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.value_added_tax,
        "tax_detail.value_added_tax",
    )?;
    let additional_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.additional_tax,
        "tax_detail.additional_tax",
    )?;
    let user_personal_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_personal_tax,
        "tax_detail.user_personal_tax",
    )?;
    let user_value_added_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_value_added_tax,
        "tax_detail.user_value_added_tax",
    )?;
    let user_additional_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_additional_tax,
        "tax_detail.user_additional_tax",
    )?;
    let received_personal_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.received_personal_tax,
        "tax_detail.received_personal_tax",
    )?;
    let user_received_personal_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_received_personal_tax,
        "tax_detail.user_received_personal_tax",
    )?;
    let received_value_added_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.received_value_added_tax,
        "tax_detail.received_value_added_tax",
    )?;
    let user_received_value_added_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_received_value_added_tax,
        "tax_detail.user_received_value_added_tax",
    )?;
    let received_additional_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.received_additional_tax,
        "tax_detail.received_additional_tax",
    )?;
    let user_received_additional_tax = parse_yzh_decimal_or_zero(
        &resp.tax_detail.user_received_additional_tax,
        "tax_detail.user_received_additional_tax",
    )?;

    sqlx::query!(
        "
        INSERT INTO payout_yunzhanghu_order_details (
            payout_id, order_id, platform_id, pay,
            user_real_amount, user_real_excluding_vat_amount,
            user_fee, received_user_fee,
            tax, received_tax_amount,
            personal_tax, value_added_tax, additional_tax,
            user_personal_tax, user_value_added_tax, user_additional_tax,
            received_personal_tax, user_received_personal_tax,
            received_value_added_tax, user_received_value_added_tax,
            received_additional_tax, user_received_additional_tax,
            raw_status, raw_status_detail, queried_at
        )
        VALUES (
            $1, $2, NULLIF($3, ''), $4,
            $5, $6,
            $7, $8,
            $9, $10,
            $11, $12, $13,
            $14, $15, $16,
            $17, $18,
            $19, $20,
            $21, $22,
            $23, $24, NOW()
        )
        ON CONFLICT (payout_id) DO UPDATE
        SET order_id = EXCLUDED.order_id,
            platform_id = EXCLUDED.platform_id,
            pay = EXCLUDED.pay,
            user_real_amount = EXCLUDED.user_real_amount,
            user_real_excluding_vat_amount = EXCLUDED.user_real_excluding_vat_amount,
            user_fee = EXCLUDED.user_fee,
            received_user_fee = EXCLUDED.received_user_fee,
            tax = EXCLUDED.tax,
            received_tax_amount = EXCLUDED.received_tax_amount,
            personal_tax = EXCLUDED.personal_tax,
            value_added_tax = EXCLUDED.value_added_tax,
            additional_tax = EXCLUDED.additional_tax,
            user_personal_tax = EXCLUDED.user_personal_tax,
            user_value_added_tax = EXCLUDED.user_value_added_tax,
            user_additional_tax = EXCLUDED.user_additional_tax,
            received_personal_tax = EXCLUDED.received_personal_tax,
            user_received_personal_tax = EXCLUDED.user_received_personal_tax,
            received_value_added_tax = EXCLUDED.received_value_added_tax,
            user_received_value_added_tax = EXCLUDED.user_received_value_added_tax,
            received_additional_tax = EXCLUDED.received_additional_tax,
            user_received_additional_tax = EXCLUDED.user_received_additional_tax,
            raw_status = EXCLUDED.raw_status,
            raw_status_detail = EXCLUDED.raw_status_detail,
            queried_at = NOW()
        ",
        payout_db_id,
        &resp.order_id,
        &resp.ref_id,
        pay,
        user_real_amount,
        user_real_excluding_vat_amount,
        user_fee,
        received_user_fee,
        tax,
        received_tax_amount,
        personal_tax,
        value_added_tax,
        additional_tax,
        user_personal_tax,
        user_value_added_tax,
        user_additional_tax,
        received_personal_tax,
        user_received_personal_tax,
        received_value_added_tax,
        user_received_value_added_tax,
        received_additional_tax,
        user_received_additional_tax,
        &resp.status,
        &resp.status_detail,
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn parse_yzh_decimal_option(
    value: &str,
    field: &str,
) -> Result<Option<rust_decimal::Decimal>, ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    value
        .parse::<rust_decimal::Decimal>()
        .map(Some)
        .map_err(|e| {
            ApiError::InvalidInput(format!(
                "云账户订单字段 {} 金额格式异常: {}",
                field, e
            ))
        })
}

fn parse_yzh_decimal_or_zero(
    value: &str,
    field: &str,
) -> Result<rust_decimal::Decimal, ApiError> {
    Ok(parse_yzh_decimal_option(value, field)?.unwrap_or_default())
}

// ============================================================================
// 后台定时任务：每分钟扫描 in-transit 订单，主动调云账户 query-order 同步状态
// ============================================================================

/// 定时核对因解约回调与远端查询短暂不一致而留下的安全 marker。
///
/// 云账户只在约 25 小时内重试回调；该任务让远端长时间故障恢复后仍能自动解除
/// `signing` 阻断，而不依赖用户手动进入收益页刷新。
pub async fn poll_pending_unsign_reconciliations(
    pool: PgPool,
    redis: RedisPool,
) {
    let nonces = match sqlx::query_scalar!(
        r#"
        SELECT nonce AS "nonce!"
        FROM yunzhanghu_unsign_reconciliations
        WHERE status = 'pending'
          AND next_attempt_at <= NOW()
          AND (lease_expires_at IS NULL OR lease_expires_at <= NOW())
        ORDER BY next_attempt_at, created_at, nonce
        LIMIT $1
        "#,
        YZH_UNSIGN_RECONCILE_BATCH_SIZE,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(nonces) => nonces,
        Err(err) => {
            log::error!("拉取待核对云账户解约状态失败: {}", err);
            return;
        }
    };

    for nonce in nonces {
        if let Err(err) = reconcile_pending_unsign_nonce(
            &pool,
            &redis,
            &nonce,
            UnsignReconcileClaimMode::DueOnly,
        )
        .await
        {
            log::warn!("核对云账户解约 marker 失败 nonce={}: {}", nonce, err);
        }
    }
}

async fn reconcile_pending_unsign_nonce(
    pool: &PgPool,
    redis: &RedisPool,
    reconcile_nonce: &str,
    claim_mode: UnsignReconcileClaimMode,
) -> Result<UnsignReconcileOutcome, ApiError> {
    let Some((lease_owner, release_at)) =
        claim_unsign_reconciliation(pool, reconcile_nonce, claim_mode).await?
    else {
        return Ok(UnsignReconcileOutcome::NotClaimed);
    };

    let first_user_id = match sqlx::query_scalar!(
        "
        SELECT user_id
        FROM user_yunzhanghu_profiles
        WHERE sign_status = 'signing'
          AND sign_nonce = $1
        ORDER BY user_id
        LIMIT 1
        ",
        reconcile_nonce,
    )
    .fetch_optional(pool)
    .await
    {
        Ok(user_id) => user_id,
        Err(err) => {
            let err = ApiError::from(err);
            defer_unsign_reconciliation_after_error(
                pool,
                reconcile_nonce,
                &lease_owner,
                None,
                &err,
            )
            .await;
            return Err(err);
        }
    };
    let Some(first_user_id) = first_user_id else {
        let outcome = resolve_empty_claimed_unsign_reconciliation(
            pool,
            reconcile_nonce,
            &lease_owner,
        )
        .await;
        if let Err(err) = &outcome {
            defer_unsign_reconciliation_after_error(
                pool,
                reconcile_nonce,
                &lease_owner,
                None,
                err,
            )
            .await;
        }
        return outcome;
    };

    let kyc = async {
        let profile = YunzhanghuProfile::get(UserId(first_user_id), pool)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("云账户资料不存在".to_string())
            })?;
        kyc_for_yzh_from_profile(&profile)
    }
    .await;
    let (real_name, id_card) = match kyc {
        Ok(kyc) => kyc,
        Err(err) => {
            defer_unsign_reconciliation_after_error(
                pool,
                reconcile_nonce,
                &lease_owner,
                None,
                &err,
            )
            .await;
            return Err(err);
        }
    };

    let remote = match yzh_api::h5_sign_status(
        &YzhClient::new(),
        &yzh_api::SignStatusRequest {
            real_name: &real_name,
            id_card: &id_card,
        },
    )
    .await
    {
        Ok(remote) => remote,
        Err(err) => {
            let err = yzh_to_api_error(err);
            defer_unsign_reconciliation_after_error(
                pool,
                reconcile_nonce,
                &lease_owner,
                None,
                &err,
            )
            .await;
            return Err(err);
        }
    };
    let remote_signed_at = parse_yzh_event_time(&remote.signed_at);
    let final_status = if remote.status == 2 {
        Some(YzhSignStatus::Terminated)
    } else if remote.status == 1
        && remote_signed_at.is_some_and(|signed_at| signed_at > release_at)
    {
        Some(YzhSignStatus::Signed)
    } else {
        None
    };
    let Some(final_status) = final_status else {
        let reason = if remote.status == 1 {
            "远端仍是回调之前的签约状态"
        } else {
            "远端解约状态尚未收敛"
        };
        defer_claimed_unsign_reconciliation(
            pool,
            reconcile_nonce,
            &lease_owner,
            Some(remote.status),
            reason,
        )
        .await?;
        return Ok(UnsignReconcileOutcome::Pending {
            remote_status: Some(remote.status),
        });
    };

    let outcome = finalize_claimed_unsign_reconciliation(
        pool,
        redis,
        reconcile_nonce,
        &lease_owner,
        final_status,
        remote.status,
        remote_signed_at,
    )
    .await;
    if let Err(err) = &outcome {
        defer_unsign_reconciliation_after_error(
            pool,
            reconcile_nonce,
            &lease_owner,
            Some(remote.status),
            err,
        )
        .await;
    }
    outcome
}

async fn claim_unsign_reconciliation(
    pool: &PgPool,
    reconcile_nonce: &str,
    claim_mode: UnsignReconcileClaimMode,
) -> Result<Option<(String, DateTime<Utc>)>, ApiError> {
    let lease_owner = Uuid::new_v4().simple().to_string();
    let force = matches!(claim_mode, UnsignReconcileClaimMode::Force);
    let row = sqlx::query!(
        r#"
        UPDATE yunzhanghu_unsign_reconciliations
        SET lease_owner = $2,
            lease_expires_at = NOW() + ($3::double precision * INTERVAL '1 second'),
            attempt_count = attempt_count + 1,
            last_attempt_at = NOW(),
            updated_at = NOW()
        WHERE nonce = $1
          AND status = 'pending'
          AND ($4::boolean OR next_attempt_at <= NOW())
          AND (lease_expires_at IS NULL OR lease_expires_at <= NOW())
        RETURNING release_at
        "#,
        reconcile_nonce,
        &lease_owner,
        YZH_UNSIGN_RECONCILE_LEASE_SECONDS as f64,
        force,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| (lease_owner, row.release_at)))
}

async fn defer_claimed_unsign_reconciliation(
    pool: &PgPool,
    reconcile_nonce: &str,
    lease_owner: &str,
    remote_status: Option<i32>,
    last_error: &str,
) -> Result<bool, ApiError> {
    let last_error = last_error.chars().take(1000).collect::<String>();
    let result = sqlx::query!(
        r#"
        UPDATE yunzhanghu_unsign_reconciliations
        SET lease_owner = NULL,
            lease_expires_at = NULL,
            last_remote_status = COALESCE($3, last_remote_status),
            last_error = $4,
            next_attempt_at = NOW() + ($5::double precision * INTERVAL '1 second'),
            updated_at = NOW()
        WHERE nonce = $1
          AND status = 'pending'
          AND lease_owner = $2
        "#,
        reconcile_nonce,
        lease_owner,
        remote_status,
        &last_error,
        YZH_UNSIGN_RECONCILE_RETRY_SECONDS as f64,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn defer_unsign_reconciliation_after_error(
    pool: &PgPool,
    reconcile_nonce: &str,
    lease_owner: &str,
    remote_status: Option<i32>,
    error: &ApiError,
) {
    if let Err(release_error) = defer_claimed_unsign_reconciliation(
        pool,
        reconcile_nonce,
        lease_owner,
        remote_status,
        &error.to_string(),
    )
    .await
    {
        log::error!(
            "释放云账户解约核对租约失败 nonce={}: {}",
            reconcile_nonce,
            release_error
        );
    }
}

async fn resolve_empty_claimed_unsign_reconciliation(
    pool: &PgPool,
    reconcile_nonce: &str,
    lease_owner: &str,
) -> Result<UnsignReconcileOutcome, ApiError> {
    let mut tx = pool.begin().await?;
    let reconciliation = sqlx::query!(
        "
        SELECT status, lease_owner
        FROM yunzhanghu_unsign_reconciliations
        WHERE nonce = $1
        FOR UPDATE
        ",
        reconcile_nonce,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(reconciliation) = reconciliation else {
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::NotClaimed);
    };
    if reconciliation.status != "pending"
        || reconciliation.lease_owner.as_deref() != Some(lease_owner)
    {
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::NotClaimed);
    }

    let marker_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM user_yunzhanghu_profiles
            WHERE sign_status = 'signing'
              AND sign_nonce = $1
        ) AS "exists!"
        "#,
        reconcile_nonce,
    )
    .fetch_one(&mut *tx)
    .await?;
    if marker_exists {
        sqlx::query!(
            r#"
            UPDATE yunzhanghu_unsign_reconciliations
            SET lease_owner = NULL,
                lease_expires_at = NULL,
                last_error = 'marker_appeared_after_claim',
                next_attempt_at = NOW() + ($3::double precision * INTERVAL '1 second'),
                updated_at = NOW()
            WHERE nonce = $1
              AND status = 'pending'
              AND lease_owner = $2
            "#,
            reconcile_nonce,
            lease_owner,
            YZH_UNSIGN_RECONCILE_RETRY_SECONDS as f64,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::Pending {
            remote_status: None,
        });
    }

    let result = sqlx::query!(
        "
        UPDATE yunzhanghu_unsign_reconciliations
        SET status = 'resolved',
            lease_owner = NULL,
            lease_expires_at = NULL,
            last_error = 'no_pending_profiles',
            resolved_status = 'ignored',
            resolved_at = NOW(),
            updated_at = NOW()
        WHERE nonce = $1
          AND status = 'pending'
          AND lease_owner = $2
        ",
        reconcile_nonce,
        lease_owner,
    )
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(ApiError::InvalidInput(
            "云账户解约核对租约已变化".to_string(),
        ));
    }
    tx.commit().await?;
    Ok(UnsignReconcileOutcome::Ignored)
}

async fn finalize_claimed_unsign_reconciliation(
    pool: &PgPool,
    redis: &RedisPool,
    reconcile_nonce: &str,
    lease_owner: &str,
    final_status: YzhSignStatus,
    remote_status: i32,
    remote_signed_at: Option<DateTime<Utc>>,
) -> Result<UnsignReconcileOutcome, ApiError> {
    let mut tx = pool.begin().await?;
    let reconciliation = sqlx::query!(
        "
        SELECT status, lease_owner, release_at
        FROM yunzhanghu_unsign_reconciliations
        WHERE nonce = $1
        FOR UPDATE
        ",
        reconcile_nonce,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(reconciliation) = reconciliation else {
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::NotClaimed);
    };
    if reconciliation.status != "pending"
        || reconciliation.lease_owner.as_deref() != Some(lease_owner)
    {
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::NotClaimed);
    }

    // reconciliation 行锁必须先于 users 锁。回调也遵循相同顺序，因此在这次
    // 查询之后不会再有同 nonce 的用户加入，下面锁住的就是完整结算组。
    let rows = sqlx::query!(
        "
        SELECT user_id
        FROM user_yunzhanghu_profiles
        WHERE sign_status = 'signing'
          AND sign_nonce = $1
        ORDER BY user_id
        ",
        reconcile_nonce,
    )
    .fetch_all(&mut *tx)
    .await?;
    let user_ids = rows.into_iter().map(|row| row.user_id).collect::<Vec<_>>();
    if user_ids.is_empty() {
        let result = sqlx::query!(
            "
            UPDATE yunzhanghu_unsign_reconciliations
            SET status = 'resolved',
                lease_owner = NULL,
                lease_expires_at = NULL,
                last_remote_status = $3,
                last_error = 'no_pending_profiles',
                resolved_status = 'ignored',
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE nonce = $1
              AND status = 'pending'
              AND lease_owner = $2
            ",
            reconcile_nonce,
            lease_owner,
            remote_status,
        )
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(ApiError::InvalidInput(
                "云账户解约核对租约已变化".to_string(),
            ));
        }
        tx.commit().await?;
        return Ok(UnsignReconcileOutcome::Ignored);
    }

    lock_yunzhanghu_users(&mut tx, &user_ids).await?;
    let mut transitioned_user_ids = match final_status {
        YzhSignStatus::Terminated => sqlx::query!(
            r#"
            UPDATE user_yunzhanghu_profiles
            SET sign_status = 'terminated',
                sign_url = NULL,
                sign_nonce = NULL,
                signed_at = NULL,
                terminated_at = $3,
                updated_at = NOW()
            WHERE user_id = ANY($1::bigint[])
              AND sign_status = 'signing'
              AND sign_nonce = $2
            RETURNING user_id
            "#,
            &user_ids,
            reconcile_nonce,
            reconciliation.release_at,
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| row.user_id)
        .collect::<Vec<_>>(),
        YzhSignStatus::Signed => sqlx::query!(
            r#"
            UPDATE user_yunzhanghu_profiles
            SET sign_status = 'signed',
                sign_url = NULL,
                sign_nonce = NULL,
                signed_at = $3,
                terminated_at = NULL,
                updated_at = NOW()
            WHERE user_id = ANY($1::bigint[])
              AND sign_status = 'signing'
              AND sign_nonce = $2
            RETURNING user_id
            "#,
            &user_ids,
            reconcile_nonce,
            remote_signed_at,
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| row.user_id)
        .collect::<Vec<_>>(),
        _ => {
            return Err(ApiError::InvalidInput(
                "云账户解约核对终态无效".to_string(),
            ));
        }
    };
    transitioned_user_ids.sort_unstable();
    if transitioned_user_ids != user_ids {
        return Err(ApiError::InvalidInput(
            "云账户解约核对用户组发生并发变化".to_string(),
        ));
    }

    let result = sqlx::query!(
        "
        UPDATE yunzhanghu_unsign_reconciliations
        SET status = 'resolved',
            lease_owner = NULL,
            lease_expires_at = NULL,
            last_remote_status = $3,
            last_error = NULL,
            resolved_status = $4,
            resolved_at = NOW(),
            updated_at = NOW()
        WHERE nonce = $1
          AND status = 'pending'
          AND lease_owner = $2
        ",
        reconcile_nonce,
        lease_owner,
        remote_status,
        final_status.as_str(),
    )
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(ApiError::InvalidInput(
            "云账户解约核对租约已变化".to_string(),
        ));
    }
    tx.commit().await?;

    for user_id in &transitioned_user_ids {
        notify_yunzhanghu_sign_status_change(
            pool,
            redis,
            UserId(*user_id),
            YzhSignStatus::Signing,
            final_status,
        )
        .await;
    }
    Ok(UnsignReconcileOutcome::Resolved {
        status: final_status,
        remote_status,
        signed_at: if final_status == YzhSignStatus::Signed {
            remote_signed_at
        } else {
            None
        },
    })
}

/// 由 `lib.rs` 的 scheduler 每分钟调用一次。
pub async fn poll_in_transit_payouts(pool: PgPool, redis: RedisPool) {
    let in_transit = match sqlx::query!(
        "
        SELECT id, yunzhanghu_order_id
        FROM payouts
        WHERE status = 'in-transit'
          AND method = 'yunzhanghu_alipay'
          AND (
              platform_id IS NOT NULL
              OR yunzhanghu_submit_started_at IS NOT NULL
          )
          AND COALESCE(yunzhanghu_submit_started_at, created) < NOW() - INTERVAL '30 seconds'
        ORDER BY created ASC
        LIMIT 200
        "
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            log::error!("拉取 in-transit 订单失败: {}", e);
            return;
        }
    };

    if in_transit.is_empty() {
        return;
    }

    log::info!(
        "云账户轮询：开始同步 {} 笔 in-transit 订单",
        in_transit.len()
    );

    let client = YzhClient::new();
    let mut applied = 0usize;
    let mut ignored = 0usize;
    let mut rejected = 0usize;
    let mut failed = 0usize;

    for row in in_transit {
        let pid_model = crate::models::ids::PayoutId::from(
            crate::database::models::PayoutId(row.id),
        );
        let order_id = row
            .yunzhanghu_order_id
            .unwrap_or_else(|| format!("bbsmc-{}", pid_model));

        let resp = match yzh_api::query_order(
            &client,
            &yzh_api::QueryOrderRequest {
                order_id: &order_id,
                channel: "支付宝",
            },
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                failed += 1;
                log::warn!("查单失败 order_id={}: {}", order_id, e);
                continue;
            }
        };

        match apply_order_status(
            &pool,
            &redis,
            &resp.order_id,
            &resp.status,
            &resp.status_detail_message,
            non_empty_ref(&resp.ref_id),
            OrderStatusEvidence {
                require_callback_fields: false,
                pay: non_empty_str(&resp.pay),
                ..OrderStatusEvidence::empty()
            },
        )
        .await
        {
            Ok(OrderStatusApplyOutcome::Applied) => applied += 1,
            Ok(OrderStatusApplyOutcome::Ignored) => ignored += 1,
            Ok(OrderStatusApplyOutcome::EvidenceRejected) => {
                rejected += 1;
                log::warn!("订单状态证据校验拒绝 order_id={}", order_id);
            }
            Err(e) => {
                failed += 1;
                log::warn!("更新订单状态失败 order_id={}: {:?}", order_id, e);
            }
        }
    }

    log::info!(
        "云账户轮询：已同步 {}，已忽略 {}，校验拒绝 {}，失败 {}",
        applied,
        ignored,
        rejected,
        failed
    );
}

// ============================================================================
// 辅助
// ============================================================================

async fn notify_yunzhanghu_sign_status_change(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: UserId,
    old_status: YzhSignStatus,
    new_status: YzhSignStatus,
) {
    if old_status == new_status {
        return;
    }

    if let Err(e) = insert_yunzhanghu_sign_status_notification(
        pool, redis, user_id, new_status,
    )
    .await
    {
        log::warn!(
            "云账户签约状态通知写入失败 user_id={} old_status={} new_status={}: {}",
            user_id.0,
            old_status,
            new_status,
            e
        );
    }
}

async fn insert_yunzhanghu_sign_status_notification(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: UserId,
    new_status: YzhSignStatus,
) -> Result<(), crate::database::models::DatabaseError> {
    let (notification_type, name, text) = match new_status {
        YzhSignStatus::Signed => (
            "yunzhanghu_sign_signed",
            "云账户签约已完成",
            "你已完成云账户实名签约，现在可以发起提现。",
        ),
        YzhSignStatus::Terminated => (
            "yunzhanghu_sign_terminated",
            "云账户签约已解除",
            "你的云账户签约已解除。重新签约前，将无法发起新的提现。",
        ),
        YzhSignStatus::Unsigned => (
            "yunzhanghu_sign_unsigned",
            "云账户签约未完成",
            "你的云账户签约当前未完成。完成签约后才能发起提现。",
        ),
        YzhSignStatus::Signing => return Ok(()),
    };

    let mut tx = pool.begin().await?;
    crate::database::models::notification_item::NotificationBuilder {
        body: crate::models::notifications::NotificationBody::LegacyMarkdown {
            notification_type: Some(notification_type.to_string()),
            name: name.to_string(),
            text: text.to_string(),
            link: "/dashboard/revenue/withdraw".to_string(),
            actions: vec![],
        },
    }
    .insert(user_id, &mut tx, redis)
    .await?;
    tx.commit().await?;

    Ok(())
}

fn profile_has_pending_sign_operation(profile: &YunzhanghuProfile) -> bool {
    profile.sign_status == YzhSignStatus::Signing
        || profile
            .sign_nonce
            .as_deref()
            .is_some_and(|nonce| !nonce.is_empty())
}

fn sign_operation_is_expired(
    profile: &YunzhanghuProfile,
    now: DateTime<Utc>,
) -> bool {
    if profile
        .sign_nonce
        .as_deref()
        .and_then(unsign_reconcile_event_at)
        .is_some()
    {
        return false;
    }
    profile_has_pending_sign_operation(profile)
        && profile.updated_at + Duration::hours(YZH_H5_OPERATION_VALID_HOURS)
            <= now
}

fn should_keep_pending_sign_operation(
    has_pending_operation: bool,
    is_release_operation: bool,
    remote_status: YzhSignStatus,
    operation_expired: bool,
) -> bool {
    if !has_pending_operation || operation_expired {
        return false;
    }

    if is_release_operation {
        remote_status == YzhSignStatus::Signed
    } else {
        remote_status != YzhSignStatus::Signed
    }
}

fn parse_yzh_event_time(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(value) {
        return Some(value.with_timezone(&Utc));
    }

    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|value| {
            crate::util::date::app_tz()
                .from_local_datetime(&value)
                .single()
        })
        .map(|value| value.with_timezone(&Utc))
}

fn unsign_reconcile_event_at(nonce: &str) -> Option<DateTime<Utc>> {
    let timestamp = nonce
        .strip_prefix("release:webhook:")?
        .split(':')
        .next()?
        .parse::<i64>()
        .ok()?;
    Utc.timestamp_opt(timestamp, 0).single()
}

fn unsign_reconciliation_nonce(
    creds: &crate::util::yunzhanghu::secrets::YzhCredentials,
    real_name: &str,
    id_card: &str,
    release_at: DateTime<Utc>,
) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(creds.app_key.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(b"yunzhanghu-unsign-reconciliation\0");
    mac.update(real_name.trim().as_bytes());
    mac.update(b"\0");
    mac.update(id_card.trim().to_uppercase().as_bytes());
    mac.update(b"\0");
    mac.update(release_at.timestamp().to_string().as_bytes());
    let digest = mac.finalize().into_bytes().encode_hex::<String>();
    format!("release:webhook:{}:{}", release_at.timestamp(), digest)
}

fn yunzhanghu_identity_matches(
    profile: &YunzhanghuProfile,
    real_name: &str,
    id_card: &str,
) -> bool {
    profile
        .real_name
        .as_deref()
        .is_some_and(|name| name.trim() == real_name.trim())
        && profile
            .decrypt_id_card()
            .ok()
            .flatten()
            .as_deref()
            .is_some_and(|stored_id_card| stored_id_card == id_card)
}

fn unsign_event_is_stale(
    profile: &YunzhanghuProfile,
    release_at: DateTime<Utc>,
    reconcile_nonce: &str,
) -> bool {
    if profile.sign_nonce.as_deref() == Some(reconcile_nonce) {
        return false;
    }

    // 一个更早（或相同时间）的回调不能覆盖已经在核对中的更新解约事件。
    // 同 nonce 已在上方视为幂等重试；不同 nonce 仅在事件时间严格更新时替换。
    if profile
        .sign_nonce
        .as_deref()
        .and_then(unsign_reconcile_event_at)
        .is_some_and(|current_release_at| current_release_at >= release_at)
    {
        return true;
    }

    let latest_local_sign_intent = match profile.sign_status {
        YzhSignStatus::Signing
            if profile
                .sign_nonce
                .as_deref()
                .is_some_and(|nonce| !nonce.starts_with("release:")) =>
        {
            Some(profile.updated_at)
        }
        _ => None,
    };
    // Signed.signed_at 是本地处理签约回调的时间，不一定是远端事件时间，不能
    // 用它跳过可信解约回调；已签约状态必须进入 marker 后以远端 status/signed_at
    // 排序。只有尚未完成的新签约 intent 的创建时间可作为本地并发保护。
    latest_local_sign_intent.is_some_and(|event_at| event_at > release_at)
}

async fn lock_yunzhanghu_user(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
) -> Result<(), ApiError> {
    let user_exists = sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user_id.0,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if user_exists.is_none() {
        return Err(ApiError::InvalidInput("用户不存在".to_string()));
    }
    Ok(())
}

async fn lock_yunzhanghu_users(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_ids: &[i64],
) -> Result<(), ApiError> {
    if user_ids.is_empty() {
        return Ok(());
    }
    let locked_user_ids = sqlx::query_scalar!(
        r#"
        SELECT id AS "id!"
        FROM users
        WHERE id = ANY($1::bigint[])
        ORDER BY id
        FOR UPDATE
        "#,
        user_ids,
    )
    .fetch_all(&mut **tx)
    .await?;
    if locked_user_ids.len() != user_ids.len() {
        return Err(ApiError::InvalidInput("用户不存在".to_string()));
    }
    Ok(())
}

async fn lock_user_and_ensure_no_processing_payout(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
) -> Result<(), ApiError> {
    // 与批量落单/管理员退回保持 payouts -> users 的锁顺序。
    let _payout_locks = sqlx::query_scalar!(
        "
        SELECT id
        FROM payouts
        WHERE user_id = $1
          AND status IN ('in-transit', 'cancelling')
        FOR SHARE
        ",
        user_id.0,
    )
    .fetch_all(&mut **tx)
    .await?;
    lock_yunzhanghu_user(tx, user_id).await?;
    ensure_no_processing_payout_in_tx(tx, user_id).await
}

async fn ensure_no_processing_payout_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
) -> Result<(), ApiError> {
    if has_processing_payout_in_tx(tx, user_id).await? {
        return Err(ApiError::InvalidInput(
            "您有提现正在处理中，请等待提现完成或由管理员退回后再修改实名资料/收款账号、签约或解约。"
                .to_string(),
        ));
    }

    Ok(())
}

async fn has_processing_payout_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
) -> Result<bool, ApiError> {
    Ok(sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM payouts
            WHERE user_id = $1
              AND status IN ('in-transit', 'cancelling')
        ) AS "active!"
        "#,
        user_id.0,
    )
    .fetch_one(&mut **tx)
    .await?)
}

async fn restore_failed_sign_operation(
    pool: &PgPool,
    user_id: UserId,
    sign_nonce: &str,
    previous_status: YzhSignStatus,
) {
    if let Err(err) = sqlx::query!(
        "
        UPDATE user_yunzhanghu_profiles
        SET sign_status = $3,
            sign_url = NULL,
            sign_nonce = NULL,
            updated_at = NOW()
        WHERE user_id = $1
          AND sign_status = 'signing'
          AND sign_nonce = $2
        ",
        user_id.0,
        sign_nonce,
        previous_status.as_str(),
    )
    .execute(pool)
    .await
    {
        log::error!(
            "恢复云账户签约意图失败 user_id={} previous_status={}: {}",
            user_id.0,
            previous_status,
            err
        );
    }
}

/// 保存已经由云账户创建的 H5 操作链接。
///
/// 远端请求成功后，即使此处因瞬时数据库错误无法保存 URL，也应把链接返回给
/// 当前请求方；本地已提前写入的 nonce 仍会阻止重复发起，并可在有效期结束后
/// 通过状态刷新恢复。若 nonce 已变化，则说明本地状态已被另一个操作推进，不能
/// 把旧链接交给用户继续执行。
async fn store_sign_operation_url(
    pool: &PgPool,
    user_id: UserId,
    sign_nonce: &str,
    sign_url: &str,
    operation_name: &str,
) -> Result<bool, ApiError> {
    match sqlx::query!(
        "
        UPDATE user_yunzhanghu_profiles
        SET sign_url = $3,
            updated_at = NOW()
        WHERE user_id = $1
          AND sign_status = 'signing'
          AND sign_nonce = $2
        ",
        user_id.0,
        sign_nonce,
        sign_url,
    )
    .execute(pool)
    .await
    {
        Ok(result) if result.rows_affected() == 1 => Ok(true),
        Ok(_) => Err(ApiError::InvalidInput(format!(
            "本地云账户{}状态已变化，请刷新页面后重试。",
            operation_name
        ))),
        Err(err) => {
            log::error!(
                "保存云账户{} H5 链接失败 user_id={}: {}",
                operation_name,
                user_id.0,
                err
            );
            Ok(false)
        }
    }
}

async fn has_processing_payout(
    user_id: UserId,
    pool: &PgPool,
) -> Result<bool, ApiError> {
    let active = sqlx::query_scalar!(
        "
        SELECT id
        FROM payouts
        WHERE user_id = $1
          AND status IN ('in-transit', 'cancelling')
        LIMIT 1
        ",
        user_id.0
    )
    .fetch_optional(pool)
    .await?;

    Ok(active.is_some())
}

fn non_empty_ref(ref_id: &str) -> Option<&str> {
    let ref_id = ref_id.trim();
    if ref_id.is_empty() {
        None
    } else {
        Some(ref_id)
    }
}

fn non_empty_str(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}

async fn validate_order_status_evidence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    payout_db_id: i64,
    user_id: UserId,
    amount: rust_decimal::Decimal,
    fee: rust_decimal::Decimal,
    payout_account: Option<&str>,
    evidence: OrderStatusEvidence<'_>,
) -> Result<bool, ApiError> {
    if evidence.require_callback_fields
        && !required_order_callback_fields_present(&evidence)
    {
        log::warn!(
            "云账户订单回调缺少必要业务绑定字段 payout_id={}",
            payout_db_id
        );
        return Ok(false);
    }

    if let Some(pay) = evidence.pay {
        let Ok(remote_amount) = pay.parse::<rust_decimal::Decimal>() else {
            log::warn!(
                "云账户订单回调金额解析失败 payout_id={} pay={}",
                payout_db_id,
                pay
            );
            return Ok(false);
        };
        let expected_amount =
            super::payouts::calculate_yunzhanghu_submitted_pay_amount(
                amount, fee,
            );
        if remote_amount.round_dp(2) != expected_amount.round_dp(2) {
            log::warn!(
                "云账户订单金额不匹配 payout_id={} gross={} fee={} expected_pay={} remote_pay={}",
                payout_db_id,
                amount.round_dp(2),
                fee.round_dp(2),
                expected_amount.round_dp(2),
                remote_amount.round_dp(2)
            );
            return Ok(false);
        }
    }

    let creds =
        crate::util::yunzhanghu::secrets::load().map_err(yzh_to_api_error)?;
    if !notify_party_matches_credentials(
        evidence.dealer_id.unwrap_or(&creds.dealer_id),
        evidence.broker_id.unwrap_or(&creds.broker_id),
        creds,
    ) {
        log::warn!(
            "云账户订单回调 dealer/broker 不匹配 payout_id={}",
            payout_db_id
        );
        return Ok(false);
    }

    if let Some(card_no) = evidence.card_no
        && payout_account.map(str::trim) != Some(card_no.trim())
    {
        log::warn!("云账户订单回调收款账号不匹配 payout_id={}", payout_db_id);
        return Ok(false);
    }

    if evidence.real_name.is_none()
        && evidence.id_card.is_none()
        && evidence.phone_no.is_none()
    {
        return Ok(true);
    }

    let Some(profile) = YunzhanghuProfile::get(user_id, &mut **tx).await?
    else {
        log::warn!(
            "云账户订单回调无法加载用户 KYC payout_id={} user_id={}",
            payout_db_id,
            user_id.0
        );
        return Ok(false);
    };

    if let Some(real_name) = evidence.real_name
        && profile.real_name.as_deref().map(str::trim) != Some(real_name.trim())
    {
        log::warn!("云账户订单回调真实姓名不匹配 payout_id={}", payout_db_id);
        return Ok(false);
    }

    if let Some(phone_no) = evidence.phone_no
        && profile.phone.as_deref().map(str::trim) != Some(phone_no.trim())
    {
        log::warn!("云账户订单回调手机号不匹配 payout_id={}", payout_db_id);
        return Ok(false);
    }

    if let Some(id_card) = evidence.id_card {
        let local_id_card = profile
            .decrypt_id_card()
            .map_err(|e| {
                ApiError::InvalidInput(format!("身份证号解密失败: {}", e))
            })?
            .ok_or_else(|| {
                ApiError::InvalidInput("KYC 信息异常：缺少身份证号".to_string())
            })?;
        if local_id_card.trim().to_uppercase() != id_card.trim().to_uppercase()
        {
            log::warn!("云账户订单回调身份证不匹配 payout_id={}", payout_db_id);
            return Ok(false);
        }
    }

    Ok(true)
}

fn profile_matches_kyc(
    profile: &YunzhanghuProfile,
    real_name: &str,
    id_card: &str,
    phone: &str,
    alipay_account: &str,
) -> bool {
    let old_id_card = match profile.decrypt_id_card() {
        Ok(Some(id_card)) => id_card,
        Ok(None) => return false,
        Err(e) => {
            log::warn!(
                "比对云账户 KYC 时身份证解密失败 user_id={}: {}",
                profile.user_id.0,
                e
            );
            return false;
        }
    };

    profile.real_name.as_deref() == Some(real_name)
        && old_id_card == id_card
        && profile.phone.as_deref() == Some(phone)
        && profile.alipay_account.as_deref() == Some(alipay_account)
}

fn sign_notify_matches_profile(
    notify: &SignNotifyData,
    profile: &YunzhanghuProfile,
    creds: &crate::util::yunzhanghu::secrets::YzhCredentials,
) -> bool {
    if !notify_party_matches_credentials(
        &notify.dealer_id,
        &notify.broker_id,
        creds,
    ) {
        return false;
    }

    let Some(real_name) = profile.real_name.as_deref() else {
        return false;
    };
    let old_id_card = match profile.decrypt_id_card() {
        Ok(Some(id_card)) => id_card,
        Ok(None) => return false,
        Err(e) => {
            log::warn!(
                "校验云账户签约回调时身份证解密失败 user_id={}: {}",
                profile.user_id.0,
                e
            );
            return false;
        }
    };

    let phone_matches = notify.phone.trim().is_empty()
        || profile.phone.as_deref().map(str::trim) == Some(notify.phone.trim());

    real_name.trim() == notify.real_name.trim()
        && old_id_card.to_uppercase() == notify.id_card.trim().to_uppercase()
        && phone_matches
}

fn notify_party_matches_credentials(
    dealer_id: &str,
    broker_id: &str,
    creds: &crate::util::yunzhanghu::secrets::YzhCredentials,
) -> bool {
    dealer_id.trim() == creds.dealer_id && broker_id.trim() == creds.broker_id
}

fn notify_replay_key(kind: &str, envelope: &NotifyEnvelope) -> String {
    notify_replay_key_with_id(kind, "", envelope)
}

fn notify_replay_key_with_id(
    kind: &str,
    notify_id: &str,
    envelope: &NotifyEnvelope,
) -> String {
    if !notify_id.trim().is_empty() {
        return format!("{}:id:{}", kind, notify_id.trim());
    }

    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(envelope.data.as_bytes());
    hasher.update(b"\0");
    hasher.update(envelope.mess.as_bytes());
    hasher.update(b"\0");
    hasher.update(envelope.timestamp.as_bytes());
    hasher.update(b"\0");
    hasher.update(envelope.sign.as_bytes());
    let digest = hasher.finalize();
    format!("{}:hash:{}", kind, digest.encode_hex::<String>())
}

fn json_payload_keys(payload: &serde_json::Value) -> String {
    payload
        .as_object()
        .map(|obj| obj.keys().map(String::as_str).collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| "<non-object>".to_string())
}

async fn notify_replay_seen(
    redis: &RedisPool,
    replay_key: &str,
) -> Result<bool, ApiError> {
    let mut redis = redis.connect().await?;
    Ok(redis
        .get(YZH_NOTIFY_REPLAY_NAMESPACE, replay_key)
        .await?
        .is_some())
}

async fn mark_notify_replay(
    redis: &RedisPool,
    replay_key: &str,
) -> Result<(), ApiError> {
    let mut redis = redis.connect().await?;
    redis
        .set(
            YZH_NOTIFY_REPLAY_NAMESPACE,
            replay_key,
            "1",
            Some(YZH_NOTIFY_REPLAY_TTL_SECONDS),
        )
        .await?;
    Ok(())
}

/// 从已锁定/已读取的资料中取得云账户签约所需 KYC，避免再次跨连接读取。
fn kyc_for_yzh_from_profile(
    profile: &YunzhanghuProfile,
) -> Result<(String, String), ApiError> {
    let id_card = profile
        .decrypt_id_card()
        .map_err(|e| {
            ApiError::InvalidInput(format!("身份证号解密失败: {}", e))
        })?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善身份证号".to_string())
        })?;

    let real_name = profile.real_name.clone().ok_or_else(|| {
        ApiError::InvalidInput("请先完善实名信息".to_string())
    })?;

    Ok((real_name, id_card))
}

/// 把 [`crate::util::yunzhanghu::YzhError`] 转成对外的 [`ApiError`]。
fn yzh_to_api_error(err: crate::util::yunzhanghu::YzhError) -> ApiError {
    use crate::util::yunzhanghu::YzhError;
    match err {
        YzhError::Business { code, message } => ApiError::InvalidInput(
            format!("云账户接口错误 [{}] {}", code, message),
        ),
        e => ApiError::InvalidInput(format!("调用云账户失败: {}", e)),
    }
}

// ============================================================================
// 脱敏工具
// ============================================================================

/// 138****8888
fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() < 7 {
        return "*".repeat(chars.len());
    }
    let mut out = String::with_capacity(chars.len());
    out.extend(chars[..3].iter());
    out.push_str("****");
    out.extend(chars[chars.len() - 4..].iter());
    out
}

fn mask_real_name(name: &str) -> String {
    let chars = name.chars().collect::<Vec<_>>();
    match chars.len() {
        0 => String::new(),
        1 => "*".to_string(),
        2 => format!("{}*", chars[0]),
        _ => {
            let middle = "*".repeat(chars.len() - 2);
            format!("{}{}{}", chars[0], middle, chars[chars.len() - 1])
        }
    }
}

/// 邮箱：a***@b.com；手机号：复用 [`mask_phone`]
fn mask_alipay(account: &str) -> String {
    if let Some((local, domain)) = account.split_once('@') {
        let local_chars: Vec<char> = local.chars().collect();
        let head: String = local_chars.iter().take(1).collect();
        return format!("{}***@{}", head, domain);
    }
    mask_phone(account)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_phone() {
        assert_eq!(mask_phone("13812345678"), "138****5678");
        assert_eq!(mask_phone("13800"), "*****");
    }

    #[test]
    fn test_mask_real_name() {
        assert_eq!(mask_real_name("张三"), "张*");
        assert_eq!(mask_real_name("张三丰"), "张*丰");
        assert_eq!(mask_real_name("A"), "*");
    }

    #[test]
    fn test_mask_alipay() {
        assert_eq!(mask_alipay("alice@example.com"), "a***@example.com");
        assert_eq!(mask_alipay("13812345678"), "138****5678");
    }

    #[test]
    fn test_id_card_regex() {
        assert!(RE_ID_CARD.is_match("110105194912310029"));
        assert!(RE_ID_CARD.is_match("11010519491231002X"));
        assert!(RE_ID_CARD.is_match("11010519491231002x"));
        assert!(!RE_ID_CARD.is_match("123")); // 太短
        assert!(!RE_ID_CARD.is_match("01010519491231002X")); // 首位 0
        assert!(!RE_ID_CARD.is_match("11010521491231002X")); // 年份 21xx 不在 18/19/20
        assert!(!RE_ID_CARD.is_match("11010519491331002X")); // 月份 13
    }

    #[test]
    fn test_phone_regex() {
        assert!(RE_PHONE.is_match("13812345678"));
        assert!(!RE_PHONE.is_match("12812345678")); // 12 开头无效
        assert!(!RE_PHONE.is_match("1381234567")); // 10 位
    }

    #[test]
    fn test_alipay_regex() {
        assert!(RE_ALIPAY.is_match("13812345678"));
        assert!(RE_ALIPAY.is_match("alice@example.com"));
        assert!(!RE_ALIPAY.is_match("alice"));
        assert!(!RE_ALIPAY.is_match(""));
    }

    #[test]
    fn rejected_order_status_evidence_is_not_accepted() {
        assert!(
            ensure_order_status_evidence_accepted(
                OrderStatusApplyOutcome::EvidenceRejected,
            )
            .is_err()
        );
        assert!(
            ensure_order_status_evidence_accepted(
                OrderStatusApplyOutcome::Applied,
            )
            .is_ok()
        );
        assert!(
            ensure_order_status_evidence_accepted(
                OrderStatusApplyOutcome::Ignored,
            )
            .is_ok()
        );

        assert!(
            ensure_order_callback_status_applied(
                OrderStatusApplyOutcome::Applied,
            )
            .is_ok()
        );
        assert!(
            ensure_order_callback_status_applied(
                OrderStatusApplyOutcome::EvidenceRejected,
            )
            .is_err()
        );
        assert!(
            ensure_order_callback_status_applied(
                OrderStatusApplyOutcome::Ignored,
            )
            .is_err()
        );
    }

    #[test]
    fn successful_payout_allows_only_verified_full_refund_transition() {
        use crate::models::payouts::PayoutStatus;

        assert!(
            ensure_refund_status_applied(OrderStatusApplyOutcome::Applied)
                .is_ok()
        );
        assert!(
            ensure_refund_status_applied(OrderStatusApplyOutcome::Ignored)
                .is_err()
        );
        assert!(
            ensure_refund_status_applied(
                OrderStatusApplyOutcome::EvidenceRejected,
            )
            .is_err()
        );

        assert!(!should_ignore_terminal_order_transition(
            PayoutStatus::Success,
            PayoutStatus::Cancelled,
            true,
        ));
        assert!(should_ignore_terminal_order_transition(
            PayoutStatus::Success,
            PayoutStatus::Cancelled,
            false,
        ));
        assert!(should_ignore_terminal_order_transition(
            PayoutStatus::Success,
            PayoutStatus::Failed,
            true,
        ));

        assert!(is_verified_channel_return("4", "2"));
        assert!(is_verified_channel_return("refund", "2"));
        assert!(!is_verified_channel_return("4", "1"));
        assert!(!is_verified_channel_return("5", "2"));

        assert!(
            ensure_full_refund_amount(
                rust_decimal::Decimal::new(105_819, 2),
                rust_decimal::Decimal::new(105_819, 2),
            )
            .is_ok()
        );
        assert!(
            ensure_full_refund_amount(
                rust_decimal::Decimal::new(105_819, 2),
                rust_decimal::Decimal::new(50_000, 2),
            )
            .is_err()
        );
    }

    #[test]
    fn verified_channel_return_uses_immutable_order_evidence() {
        let notify: OrderNotifyData = serde_json::from_value(json!({
            "order_id": "bbsmc-test",
            "pay": "1058.19",
            "dealer_id": "dealer",
            "broker_id": "broker",
            "real_name": "原姓名",
            "card_no": "original-account",
            "id_card": "original-id-card",
            "phone_no": "13800000000",
            "status": "4"
        }))
        .unwrap();
        let mut queried: yzh_api::QueryOrderResponse =
            serde_json::from_value(json!({
                "order_id": "bbsmc-test",
                "pay": "1058.19",
                "status": "4",
                "refund_origin": "2"
            }))
            .unwrap();

        assert!(channel_return_query_matches_callback(&notify, &queried));

        let normal_evidence = order_callback_evidence(&notify, false);
        assert_eq!(normal_evidence.real_name, Some("原姓名"));
        assert_eq!(normal_evidence.id_card, Some("original-id-card"));
        assert_eq!(normal_evidence.phone_no, Some("13800000000"));

        let returned_evidence = order_callback_evidence(&notify, true);
        assert!(returned_evidence.allow_success_refund);
        assert_eq!(returned_evidence.card_no, Some("original-account"));
        assert!(returned_evidence.real_name.is_none());
        assert!(returned_evidence.id_card.is_none());
        assert!(returned_evidence.phone_no.is_none());
        assert!(required_order_callback_fields_present(&returned_evidence));
        assert!(!required_order_callback_fields_present(
            &OrderStatusEvidence {
                card_no: None,
                ..returned_evidence
            }
        ));

        queried.refund_origin = "1".to_string();
        assert!(!channel_return_query_matches_callback(&notify, &queried));
    }

    #[test]
    fn current_refund_callback_payload_uses_wrapped_full_refund_fields() {
        let payload: RefundNotifyPayload = serde_json::from_value(json!({
            "notify_id": "notify-1",
            "notify_time": "2026-08-02 18:00:00",
            "data": {
                "broker_id": "broker",
                "dealer_id": "dealer",
                "ref": "original-ref",
                "refund_ref": "refund-ref",
                "order_id": "bbsmc-test",
                "real_name": "测试用户",
                "card_no": "account",
                "id_card": "id-card",
                "refund_type": "0",
                "refund_total_amount": "1058.19"
            }
        }))
        .unwrap();

        let RefundNotifyPayload::Wrapped {
            notify_id, data, ..
        } = payload
        else {
            panic!("current refund callback must decode as wrapped payload");
        };
        assert_eq!(notify_id, "notify-1");
        assert_eq!(data.order_id, "bbsmc-test");
        assert_eq!(data.refund_type, "0");
        assert_eq!(data.refund_total_amount, "1058.19");
        assert_eq!(data.refund_ref, "refund-ref");
    }

    #[test]
    fn yunzhanghu_event_time_uses_application_timezone() {
        assert_eq!(
            parse_yzh_event_time("2026-08-02 12:34:56"),
            Some(Utc.with_ymd_and_hms(2026, 8, 2, 4, 34, 56).unwrap())
        );
        assert_eq!(
            unsign_reconcile_event_at(
                "release:webhook:1785645296:callback-digest"
            ),
            Utc.timestamp_opt(1785645296, 0).single()
        );
    }

    #[test]
    fn unsign_callback_preserves_newer_local_operations() {
        let release_at = Utc.with_ymd_and_hms(2026, 8, 2, 4, 34, 56).unwrap();
        let mut profile = YunzhanghuProfile {
            user_id: UserId(1),
            real_name: None,
            id_card_encrypted: None,
            id_card_last4: None,
            phone: None,
            alipay_account: None,
            sign_status: YzhSignStatus::Signed,
            sign_url: None,
            sign_nonce: None,
            signed_at: Some(release_at + Duration::minutes(1)),
            terminated_at: None,
            created_at: release_at - Duration::days(1),
            updated_at: release_at + Duration::minutes(1),
        };

        // signed_at 是本地处理回调的时间，不能据此丢弃可信解约事件。
        assert!(!unsign_event_is_stale(
            &profile,
            release_at,
            "release:webhook:1785645296:digest"
        ));
        profile.sign_status = YzhSignStatus::Signing;
        profile.sign_nonce = Some("new-sign-intent".to_string());
        assert!(unsign_event_is_stale(
            &profile,
            release_at,
            "release:webhook:1785645296:digest"
        ));
        assert!(!unsign_event_is_stale(
            &profile,
            release_at + Duration::minutes(2),
            "release:webhook:1785645416:digest"
        ));

        let newer_release_at = release_at + Duration::minutes(1);
        profile.sign_nonce = Some(format!(
            "release:webhook:{}:newer-release",
            newer_release_at.timestamp()
        ));
        assert!(unsign_event_is_stale(
            &profile,
            release_at,
            "release:webhook:1785645296:older-release"
        ));
        assert!(!unsign_event_is_stale(
            &profile,
            newer_release_at + Duration::minutes(1),
            "release:webhook:1785645416:newest-release"
        ));
    }

    #[test]
    fn unsign_reconciliation_nonce_is_stable_for_business_event() {
        let creds = crate::util::yunzhanghu::secrets::YzhCredentials {
            api_url: "https://example.invalid".to_string(),
            dealer_id: "dealer".to_string(),
            broker_id: "broker".to_string(),
            app_key: "test-app-key".to_string(),
            des_key: "000000000000000000000000".to_string(),
            dealer_private_key_pem: String::new(),
            platform_public_key_pem: String::new(),
        };
        let release_at = Utc.with_ymd_and_hms(2026, 8, 2, 4, 34, 56).unwrap();
        let nonce = unsign_reconciliation_nonce(
            &creds,
            "张三",
            "11010519491231002X",
            release_at,
        );
        assert_eq!(
            nonce,
            unsign_reconciliation_nonce(
                &creds,
                " 张三 ",
                "11010519491231002x",
                release_at,
            )
        );
        assert_ne!(
            nonce,
            unsign_reconciliation_nonce(
                &creds,
                "张三",
                "11010519491231002X",
                release_at + Duration::seconds(1),
            )
        );
    }

    #[test]
    fn pending_sign_operation_is_kept_until_remote_signs_or_it_expires() {
        assert!(should_keep_pending_sign_operation(
            true,
            false,
            YzhSignStatus::Unsigned,
            false,
        ));
        assert!(should_keep_pending_sign_operation(
            true,
            false,
            YzhSignStatus::Terminated,
            false,
        ));
        assert!(!should_keep_pending_sign_operation(
            true,
            false,
            YzhSignStatus::Signed,
            false,
        ));
        assert!(!should_keep_pending_sign_operation(
            true,
            false,
            YzhSignStatus::Unsigned,
            true,
        ));
    }

    #[test]
    fn pending_release_operation_is_kept_until_remote_terminates_or_it_expires()
    {
        assert!(should_keep_pending_sign_operation(
            true,
            true,
            YzhSignStatus::Signed,
            false,
        ));
        assert!(!should_keep_pending_sign_operation(
            true,
            true,
            YzhSignStatus::Terminated,
            false,
        ));
        assert!(!should_keep_pending_sign_operation(
            true,
            true,
            YzhSignStatus::Signed,
            true,
        ));
        assert!(!should_keep_pending_sign_operation(
            false,
            true,
            YzhSignStatus::Signed,
            false,
        ));
    }
}
