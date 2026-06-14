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
use chrono::{DateTime, Utc};
use hex::ToHex;
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

#[derive(Clone, Copy)]
pub struct OrderStatusEvidence<'a> {
    require_callback_fields: bool,
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
    pub signed_at: Option<DateTime<Utc>>,
    /// 是否存在云账户支付宝提现中的订单。存在时禁止修改资料、重新签约或解约。
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
                signed_at: None,
                has_active_payout,
            };
        };
        let kyc_completed = p.real_name.is_some()
            && p.id_card_encrypted.is_some()
            && p.phone.is_some()
            && p.alipay_account.is_some();
        Self {
            kyc_completed,
            real_name: p.real_name.as_deref().map(mask_real_name),
            id_card_last4: p.id_card_last4,
            phone_masked: p.phone.as_deref().map(mask_phone),
            alipay_account_masked: p.alipay_account.as_deref().map(mask_alipay),
            sign_status: p.sign_status.as_str().to_string(),
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
    let has_active_payout =
        has_active_yunzhanghu_payout(user_id, &pool).await?;

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
    ensure_no_active_yunzhanghu_payout(user_id, &pool).await?;

    let mut tx = pool.begin().await?;
    let existing = YunzhanghuProfile::get(user_id, &mut *tx).await?;
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
    let has_active_payout =
        has_active_yunzhanghu_payout(user_id, &pool).await?;
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
    ensure_no_active_yunzhanghu_payout(user_id, &pool).await?;

    let (real_name, id_card) = load_kyc_for_yzh(user_id, &pool).await?;

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

    let self_addr =
        dotenvy::var("SELF_ADDR")?.trim_end_matches('/').to_string();
    let site_url = dotenvy::var("SITE_URL")
        .unwrap_or_else(|_| "https://bbsmc.net".to_string())
        .trim_end_matches('/')
        .to_string();

    let sign_nonce = Uuid::new_v4().simple().to_string();
    let event_callback_url = format!(
        "{}/v3/yunzhanghu/_webhook/sign/{}/{}",
        self_addr, user.id.0, sign_nonce
    );
    let redirect_url = format!("{}/yunzhanghu-result?action=sign", site_url);

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

    let mut tx = pool.begin().await?;
    YunzhanghuProfile::update_sign_status(
        &mut *tx,
        user_id,
        YzhSignStatus::Signing,
        Some(&sign.url),
        Some(&sign_nonce),
    )
    .await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(json!({ "url": sign.url })))
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
    let current_status = YunzhanghuProfile::get(user_id, &**pool)
        .await?
        .map(|p| p.sign_status)
        .unwrap_or(YzhSignStatus::Unsigned);

    let (real_name, id_card) = load_kyc_for_yzh(user_id, &pool).await?;

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
        return Ok(HttpResponse::Ok().json(json!({
            "sign_status": current_status.as_str(),
            "remote_status": resp.status,
            "signed_at": resp.signed_at,
            "requires_new_sign": true,
        })));
    }

    let mut tx = pool.begin().await?;
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
    ensure_no_active_yunzhanghu_payout(user_id, &pool).await?;

    let (real_name, id_card) = load_kyc_for_yzh(user_id, &pool).await?;

    let site_url = dotenvy::var("SITE_URL")
        .unwrap_or_else(|_| "https://bbsmc.net".to_string())
        .trim_end_matches('/')
        .to_string();
    let redirect_url = format!("{}/yunzhanghu-result?action=release", site_url);

    let client = YzhClient::new();
    let resp = yzh_api::h5_release_apply(
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
    .await
    .map_err(yzh_to_api_error)?;

    Ok(HttpResponse::Ok().json(json!({
        "url": resp.url,
        "remote_status": resp.status,
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

    if new_status == YzhSignStatus::Terminated
        && has_active_yunzhanghu_payout(user_id, &pool).await?
    {
        log::warn!(
            "忽略有处理中提现用户的云账户签约/解约事件回调 user_id={}",
            user_id.0
        );
        mark_notify_replay(&redis, &replay_key).await?;
        return Ok(HttpResponse::Ok().body("success"));
    }

    let mut tx = pool.begin().await?;
    YunzhanghuProfile::update_sign_status(
        &mut *tx, user_id, new_status, None, None,
    )
    .await?;
    tx.commit().await?;

    notify_yunzhanghu_sign_status_change(
        &pool,
        &redis,
        user_id,
        profile.sign_status,
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
        #[serde(default)]
        #[allow(dead_code)]
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

    // 在候选里逐个解密匹配（同末 4 位的用户很少）
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
        if real_name_matches
            && profile
                .decrypt_id_card()
                .ok()
                .flatten()
                .as_deref()
                .is_some_and(|id_card| id_card == notify.id_card)
        {
            if has_active_yunzhanghu_payout(profile.user_id, &pool).await? {
                log::warn!(
                    "忽略有处理中提现用户的云账户解约回调 user_id={}",
                    profile.user_id.0
                );
                mark_notify_replay(&redis, &replay_key).await?;
                return Ok(HttpResponse::Ok().body("success"));
            }

            let mut tx = pool.begin().await?;
            YunzhanghuProfile::update_sign_status(
                &mut *tx,
                profile.user_id,
                YzhSignStatus::Terminated,
                None,
                None,
            )
            .await?;
            tx.commit().await?;
            notify_yunzhanghu_sign_status_change(
                &pool,
                &redis,
                profile.user_id,
                profile.sign_status,
                YzhSignStatus::Terminated,
            )
            .await;
            break;
        }
    }

    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
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

    apply_order_status(
        &pool,
        &redis,
        &notify.order_id,
        &notify.status,
        &notify.status_detail_message,
        Some(&notify.ref_id),
        OrderStatusEvidence {
            require_callback_fields: true,
            pay: non_empty_str(&notify.pay),
            dealer_id: non_empty_str(&notify.dealer_id),
            broker_id: non_empty_str(&notify.broker_id),
            real_name: non_empty_str(&notify.real_name),
            id_card: non_empty_str(&notify.id_card),
            phone_no: non_empty_str(&notify.phone_no),
            card_no: non_empty_str(&notify.card_no),
        },
    )
    .await?;

    mark_notify_replay(&redis, &replay_key).await?;
    Ok(HttpResponse::Ok().body("success"))
}

/// 退款回调（用户支付失败被退款，或银行/支付宝退汇）。
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
    #[derive(Deserialize)]
    struct RefundNotifyData {
        order_id: String,
        #[serde(default)]
        pay: String,
        #[serde(default)]
        refund_amount: String,
        #[serde(default)]
        refund_status: String,
    }

    let notify: RefundNotifyData = form.decode().map_err(yzh_to_api_error)?;
    let replay_key = notify_replay_key("refund", &form);
    if notify_replay_seen(&redis, &replay_key).await? {
        return Ok(HttpResponse::Ok().body("success"));
    }

    // 退款成功 → 把对应 payouts 改为 Cancelled，用户余额自动回退
    if notify.refund_status.eq_ignore_ascii_case("success")
        || notify.refund_status == "1"
    {
        let pay_for_validation = non_empty_str(&notify.pay)
            .or_else(|| non_empty_str(&notify.refund_amount));
        apply_order_status(
            &pool,
            &redis,
            &notify.order_id,
            "cancelled",
            "用户退款",
            None,
            OrderStatusEvidence {
                require_callback_fields: false,
                pay: pay_for_validation,
                ..OrderStatusEvidence::empty()
            },
        )
        .await?;
    }

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

    apply_order_status(
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
) -> Result<(), ApiError> {
    let Some(new_status) =
        crate::util::yunzhanghu::api::map_order_status(remote_status)
    else {
        log::warn!(
            "未识别的云账户订单状态 order_id={} status={}",
            order_id,
            remote_status
        );
        return Ok(());
    };

    // 找到对应 payout 记录。我们的 order_id = "bbsmc-{base62 payout_id}"。
    let Some(payout_id_str) = order_id.strip_prefix("bbsmc-") else {
        log::warn!("忽略非本站云账户订单号回调 order_id={}", order_id);
        return Ok(());
    };
    let payout_db_id: i64 =
        match crate::models::ids::base62_impl::parse_base62(payout_id_str) {
            Ok(n) => n as i64,
            Err(_) => {
                log::warn!("无法解析 order_id={}", order_id);
                return Ok(());
            }
        };

    let mut tx = pool.begin().await?;

    // 锁定 payout 行，避免并发回调重复通知或终态竞争。
    let row = sqlx::query!(
        "
        SELECT user_id, status, amount, method, method_address,
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
        return Ok(());
    };

    if row.method.as_deref()
        != Some(
            crate::models::payouts::PayoutMethodType::YunzhanghuAlipay.as_str(),
        )
    {
        log::warn!("忽略非云账户支付宝提现订单回调 order_id={}", order_id);
        return Ok(());
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
        return Ok(());
    }

    if row.yunzhanghu_order_id.is_none()
        && row.yunzhanghu_submit_started_at.is_none()
    {
        log::warn!(
            "忽略未提交云账户的提现订单回调 payout_id={} order_id={}",
            payout_db_id,
            order_id
        );
        return Ok(());
    }

    if !validate_order_status_evidence(
        &mut tx,
        payout_db_id,
        crate::database::models::UserId(row.user_id),
        row.amount,
        row.method_address.as_deref(),
        evidence,
    )
    .await?
    {
        return Ok(());
    }

    // 幂等：终态不能回滚
    let current =
        crate::models::payouts::PayoutStatus::from_string(&row.status);
    if matches!(
        current,
        crate::models::payouts::PayoutStatus::Success
            | crate::models::payouts::PayoutStatus::Cancelled
            | crate::models::payouts::PayoutStatus::Failed
    ) && current != new_status
    {
        log::info!(
            "忽略状态回退 order_id={} 当前={} 收到={}",
            order_id,
            current,
            new_status
        );
        return Ok(());
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

    Ok(())
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
    let mut succeeded = 0usize;
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

        if let Err(e) = apply_order_status(
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
            failed += 1;
            log::warn!("更新订单状态失败 order_id={}: {:?}", order_id, e);
        } else {
            succeeded += 1;
        }
    }

    log::info!("云账户轮询：成功 {}，失败 {}", succeeded, failed);
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

async fn ensure_no_active_yunzhanghu_payout(
    user_id: UserId,
    pool: &PgPool,
) -> Result<(), ApiError> {
    if has_active_yunzhanghu_payout(user_id, pool).await? {
        return Err(ApiError::InvalidInput(
            "您有提现正在处理中，请等待提现完成或由管理员退回后再修改实名资料/收款账号、签约或解约。"
                .to_string(),
        ));
    }

    Ok(())
}

async fn has_active_yunzhanghu_payout(
    user_id: UserId,
    pool: &PgPool,
) -> Result<bool, ApiError> {
    let active = sqlx::query_scalar!(
        "
        SELECT id
        FROM payouts
        WHERE user_id = $1
          AND status = 'in-transit'
          AND method = 'yunzhanghu_alipay'
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
    payout_account: Option<&str>,
    evidence: OrderStatusEvidence<'_>,
) -> Result<bool, ApiError> {
    if evidence.require_callback_fields
        && (evidence.pay.is_none()
            || evidence.dealer_id.is_none()
            || evidence.broker_id.is_none())
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
        if remote_amount.round_dp(2) != amount.round_dp(2) {
            log::warn!(
                "云账户订单回调金额不匹配 payout_id={} local={} remote={}",
                payout_db_id,
                amount.round_dp(2),
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
        && evidence.card_no.is_none()
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

    if let Some(card_no) = evidence.card_no
        && profile.alipay_account.as_deref().map(str::trim)
            != Some(card_no.trim())
    {
        log::warn!("云账户订单回调支付宝账号不匹配 payout_id={}", payout_db_id);
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

/// 加载用户已绑定的真实姓名 + 身份证号（解密后）。
/// 缺失时返回 [`ApiError::InvalidInput`]，提示前端先完善 KYC。
async fn load_kyc_for_yzh(
    user_id: UserId,
    pool: &PgPool,
) -> Result<(String, String), ApiError> {
    let profile =
        YunzhanghuProfile::get(user_id, pool)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "请先完善实名信息与支付宝账号".to_string(),
                )
            })?;

    let id_card = profile
        .decrypt_id_card()
        .map_err(|e| {
            ApiError::InvalidInput(format!("身份证号解密失败: {}", e))
        })?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善身份证号".to_string())
        })?;

    let real_name = profile.real_name.ok_or_else(|| {
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
}
