use crate::auth::validate::{
    check_is_admin_from_headers, get_user_record_from_bearer_token,
};
use crate::auth::{AuthenticationError, get_user_from_headers};
use crate::database::models::generate_payout_id;
use crate::database::models::yunzhanghu_profile_item::{
    YunzhanghuProfile, YzhSignStatus,
};
use crate::database::redis::RedisPool;
use crate::models::ids::PayoutId;
use crate::models::pats::Scopes;
use crate::models::payouts::{
    PayoutInterval, PayoutMethod, PayoutMethodFee, PayoutMethodType,
    PayoutStatus,
};
use crate::queue::payouts::{PayoutsQueue, make_aditude_request};
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::yunzhanghu::{NotifyEnvelope, YzhClient, api as yzh_api};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, web};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

/// 提现最低额度（人民币元）
const MIN_WITHDRAW_AMOUNT: Decimal = Decimal::from_parts(5, 0, 0, false, 0);
const WITHDRAW_SERVICE_FEE_RATE: Decimal =
    Decimal::from_parts(3, 0, 0, false, 2);

/// 云账户合规要求 - 平台企业名称
const DEALER_PLATFORM_NAME: &str = "青岛柒兮网络科技";

const YZH_SUBMIT_IN_PROGRESS_LOCK_SECONDS: i64 = 60;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("payout")
            .service(user_payouts)
            .service(quote_payout)
            .service(create_payout)
            .service(admin_payouts)
            .service(admin_processing_payouts)
            .service(admin_processing_payout_detail)
            .service(admin_confirm_payout)
            .service(admin_reject_payout)
            .service(cancel_payout)
            .service(yunzhanghu_order_callback_alias)
            .service(yunzhanghu_prepay_callback_alias)
            .service(yunzhanghu_balance_callback_alias)
            .service(yunzhanghu_unsign_callback_alias)
            .service(yunzhanghu_refund_callback_alias)
            .service(payment_methods)
            .service(get_balance)
            .service(platform_revenue),
    );
}

// 云账户后台历史配置使用 /v3/payout/_yunzhanghu/*，这里保留兼容入口。
#[post("_yunzhanghu/order")]
pub async fn yunzhanghu_order_callback_alias(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    crate::routes::v3::yunzhanghu::handle_order_callback(form, pool, redis)
        .await
}

#[post("_yunzhanghu/prepay")]
pub async fn yunzhanghu_prepay_callback_alias(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    crate::routes::v3::yunzhanghu::handle_prepay_callback(form, redis).await
}

#[post("_yunzhanghu/balance")]
pub async fn yunzhanghu_balance_callback_alias(
    form: web::Form<NotifyEnvelope>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    crate::routes::v3::yunzhanghu::handle_balance_callback(form, redis).await
}

#[post("_yunzhanghu/unsign")]
pub async fn yunzhanghu_unsign_callback_alias(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    crate::routes::v3::yunzhanghu::handle_unsign_callback(form, pool, redis)
        .await
}

#[post("_yunzhanghu/refund")]
pub async fn yunzhanghu_refund_callback_alias(
    form: web::Form<NotifyEnvelope>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    crate::routes::v3::yunzhanghu::handle_refund_callback(form, pool, redis)
        .await
}

#[get("")]
pub async fn user_payouts(
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

    let payout_ids =
        crate::database::models::payout_item::Payout::get_all_for_user(
            user.id.into(),
            &**pool,
        )
        .await?;
    let payouts = crate::database::models::payout_item::Payout::get_many(
        &payout_ids,
        &**pool,
    )
    .await?;
    let payout_db_ids = payout_ids.iter().map(|id| id.0).collect::<Vec<_>>();
    let detail_rows = sqlx::query!(
        "
        SELECT
            d.payout_id,
            COALESCE(d.user_real_amount, d.pay) AS received_amount,
            CASE
                WHEN COALESCE(p.fee, 0) <> 0 THEN p.fee
                WHEN d.received_user_fee <> 0 THEN d.received_user_fee
                ELSE d.user_fee
            END AS service_fee,
            CASE
                WHEN (
                    d.user_received_personal_tax
                    + d.user_received_additional_tax
                ) <> 0 THEN (
                    d.user_received_personal_tax
                    + d.user_received_additional_tax
                )
                ELSE (
                    d.user_personal_tax
                    + d.user_additional_tax
                )
            END AS user_tax
        FROM payout_yunzhanghu_order_details d
        INNER JOIN payouts p ON p.id = d.payout_id
        WHERE d.payout_id = ANY($1)
        ",
        &payout_db_ids
    )
    .fetch_all(&**pool)
    .await?;
    let details = detail_rows
        .into_iter()
        .map(|row| {
            (
                row.payout_id,
                crate::models::payouts::PayoutYunzhanghuDetails {
                    received_amount: row.received_amount.unwrap_or_default(),
                    service_fee: row.service_fee.unwrap_or_default(),
                    tax: row.user_tax.unwrap_or_default(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(HttpResponse::Ok().json(
        payouts
            .into_iter()
            .map(|payout| {
                let detail = details.get(&payout.id.0).cloned();
                crate::models::payouts::Payout::from_with_yunzhanghu_details(
                    payout, detail,
                )
            })
            .collect::<Vec<_>>(),
    ))
}

#[derive(Deserialize)]
pub struct Withdrawal {
    /// 提现金额（元）
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    /// 支付通道，目前仅 [`PayoutMethodType::YunzhanghuAlipay`]
    pub method: PayoutMethodType,
}

#[derive(Serialize)]
pub struct PayoutQuote {
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub arrival_amount: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub user_fee: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub required_balance: Decimal,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub after_tax_amount: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub user_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub dealer_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub broker_tax: Option<Decimal>,
    pub tax_detail: PayoutQuoteTaxDetail,
    pub status_message: Option<String>,
}

#[derive(Serialize, Default)]
pub struct PayoutQuoteTaxDetail {
    #[serde(with = "rust_decimal::serde::float_option")]
    pub personal_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub value_added_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub additional_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub user_personal_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub user_value_added_tax: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub user_additional_tax: Option<Decimal>,
}

#[derive(Serialize)]
pub struct AdminProcessingPayout {
    pub id: crate::models::ids::PayoutId,
    pub user_id: crate::models::ids::UserId,
    pub username: String,
    pub created: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub fee: Option<Decimal>,
    pub status: PayoutStatus,
    pub method: Option<PayoutMethodType>,
    pub order_id: String,
    pub platform_id: Option<String>,
    pub submit_started_at: Option<DateTime<Utc>>,
    pub submit_error: Option<String>,
    pub submit_attempts: i32,
    pub real_name: Option<String>,
    pub id_card_last4: Option<String>,
    pub phone_masked: Option<String>,
    pub alipay_account_masked: Option<String>,
    pub sign_status: String,
    pub kyc_matches_payout: bool,
}

#[derive(Deserialize)]
pub struct AdminPayoutsQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Serialize)]
pub struct AdminPayoutsResponse {
    pub items: Vec<AdminProcessingPayout>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Serialize)]
pub struct AdminProcessingPayoutDetail {
    pub id: crate::models::ids::PayoutId,
    pub user_id: crate::models::ids::UserId,
    pub username: String,
    pub created: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub fee: Option<Decimal>,
    pub status: PayoutStatus,
    pub method: Option<PayoutMethodType>,
    pub order_id: String,
    pub platform_id: Option<String>,
    pub submit_started_at: Option<DateTime<Utc>>,
    pub submit_error: Option<String>,
    pub submit_attempts: i32,
    pub real_name: Option<String>,
    pub id_card_last4: Option<String>,
    pub phone_masked: Option<String>,
    /// 管理员确认转账使用的完整支付宝账号，仅单条详情接口返回。
    pub alipay_account: Option<String>,
    pub sign_status: String,
    pub kyc_matches_payout: bool,
}

#[derive(Deserialize)]
pub struct AdminRejectPayout {
    pub reason: Option<String>,
}

#[get("admin")]
pub async fn admin_payouts(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    query: web::Query<AdminPayoutsQuery>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?;

    let status_filter = normalize_admin_payout_status(query.status.as_deref())?;
    let page = query.page.unwrap_or(1).clamp(1, 10_000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let total = sqlx::query_scalar!(
        "
        SELECT COUNT(*)
        FROM payouts p
        WHERE p.method = 'yunzhanghu_alipay'
          AND ($1::text IS NULL OR p.status = $1)
        ",
        status_filter,
    )
    .fetch_one(&**pool)
    .await?
    .unwrap_or(0);

    let rows = sqlx::query!(
        "
        SELECT p.id, p.user_id, p.created, p.amount, p.fee, p.status,
               p.method, p.method_address, p.platform_id,
               p.yunzhanghu_submit_started_at, p.yunzhanghu_submit_error,
               p.yunzhanghu_submit_attempts, u.username
        FROM payouts p
        INNER JOIN users u ON u.id = p.user_id
        WHERE p.method = 'yunzhanghu_alipay'
          AND ($1::text IS NULL OR p.status = $1)
        ORDER BY
          CASE WHEN p.status = 'in-transit' THEN 0 ELSE 1 END ASC,
          p.created DESC
        LIMIT $2 OFFSET $3
        ",
        status_filter,
        page_size,
        offset,
    )
    .fetch_all(&**pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let profile = YunzhanghuProfile::get(
            crate::database::models::UserId(row.user_id),
            &**pool,
        )
        .await?;
        let payout_account = row.method_address;
        let (
            real_name,
            id_card_last4,
            phone_masked,
            sign_status,
            kyc_matches_payout,
        ) = if let Some(profile) = profile {
            let kyc_matches_payout = profile.sign_status
                == YzhSignStatus::Signed
                && profile.alipay_account.as_deref()
                    == payout_account.as_deref();
            (
                profile.real_name,
                profile.id_card_last4,
                profile.phone.as_deref().map(mask_admin_phone),
                profile.sign_status.as_str().to_string(),
                kyc_matches_payout,
            )
        } else {
            (
                None,
                None,
                None,
                YzhSignStatus::Unsigned.as_str().to_string(),
                false,
            )
        };

        let id = crate::database::models::PayoutId(row.id);
        let public_id = crate::models::ids::PayoutId::from(id);
        items.push(AdminProcessingPayout {
            id: public_id,
            user_id: crate::models::ids::UserId::from(
                crate::database::models::UserId(row.user_id),
            ),
            username: row.username,
            created: row.created,
            amount: row.amount,
            fee: row.fee,
            status: PayoutStatus::from_string(&row.status),
            method: row
                .method
                .map(|method| PayoutMethodType::from_string(&method)),
            order_id: format!("bbsmc-{}", public_id),
            platform_id: row.platform_id,
            submit_started_at: row.yunzhanghu_submit_started_at,
            submit_error: row.yunzhanghu_submit_error,
            submit_attempts: row.yunzhanghu_submit_attempts,
            real_name,
            id_card_last4,
            phone_masked,
            alipay_account_masked: payout_account
                .as_deref()
                .map(mask_alipay_account),
            sign_status,
            kyc_matches_payout,
        });
    }

    Ok(HttpResponse::Ok().json(AdminPayoutsResponse {
        items,
        total,
        page,
        page_size,
    }))
}

#[get("admin/processing")]
pub async fn admin_processing_payouts(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await?;

    let rows = sqlx::query!(
        "
        SELECT p.id, p.user_id, p.created, p.amount, p.fee, p.status,
               p.method, p.method_address, p.platform_id,
               p.yunzhanghu_submit_started_at, p.yunzhanghu_submit_error,
               p.yunzhanghu_submit_attempts, u.username
        FROM payouts p
        INNER JOIN users u ON u.id = p.user_id
        WHERE p.status = 'in-transit'
          AND p.method = 'yunzhanghu_alipay'
          AND p.platform_id IS NULL
        ORDER BY p.created ASC
        LIMIT 200
        "
    )
    .fetch_all(&**pool)
    .await?;

    let mut payouts = Vec::with_capacity(rows.len());
    for row in rows {
        let profile = YunzhanghuProfile::get(
            crate::database::models::UserId(row.user_id),
            &**pool,
        )
        .await?;
        let payout_account = row.method_address;
        let (
            real_name,
            id_card_last4,
            phone_masked,
            sign_status,
            kyc_matches_payout,
        ) = if let Some(profile) = profile {
            let kyc_matches_payout = profile.sign_status
                == YzhSignStatus::Signed
                && profile.alipay_account.as_deref()
                    == payout_account.as_deref();
            (
                profile.real_name,
                profile.id_card_last4,
                profile.phone.as_deref().map(mask_admin_phone),
                profile.sign_status.as_str().to_string(),
                kyc_matches_payout,
            )
        } else {
            (
                None,
                None,
                None,
                YzhSignStatus::Unsigned.as_str().to_string(),
                false,
            )
        };

        let id = crate::database::models::PayoutId(row.id);
        let public_id = crate::models::ids::PayoutId::from(id);
        payouts.push(AdminProcessingPayout {
            id: public_id,
            user_id: crate::models::ids::UserId::from(
                crate::database::models::UserId(row.user_id),
            ),
            username: row.username,
            created: row.created,
            amount: row.amount,
            fee: row.fee,
            status: PayoutStatus::from_string(&row.status),
            method: row
                .method
                .map(|method| PayoutMethodType::from_string(&method)),
            order_id: format!("bbsmc-{}", public_id),
            platform_id: row.platform_id,
            submit_started_at: row.yunzhanghu_submit_started_at,
            submit_error: row.yunzhanghu_submit_error,
            submit_attempts: row.yunzhanghu_submit_attempts,
            real_name,
            id_card_last4,
            phone_masked,
            alipay_account_masked: payout_account
                .as_deref()
                .map(mask_alipay_account),
            sign_status,
            kyc_matches_payout,
        });
    }

    Ok(HttpResponse::Ok().json(payouts))
}

#[get("admin/{id}")]
pub async fn admin_processing_payout_detail(
    req: HttpRequest,
    path: web::Path<crate::models::ids::PayoutId>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?;

    let public_id = path.into_inner();
    let payout_id: crate::database::models::PayoutId = public_id.into();
    let row = sqlx::query!(
        "
        SELECT p.id, p.user_id, p.created, p.amount, p.fee, p.status,
               p.method, p.method_address, p.platform_id,
               p.yunzhanghu_submit_started_at, p.yunzhanghu_submit_error,
               p.yunzhanghu_submit_attempts, u.username
        FROM payouts p
        INNER JOIN users u ON u.id = p.user_id
        WHERE p.id = $1
          AND p.status = 'in-transit'
          AND p.method = 'yunzhanghu_alipay'
          AND p.platform_id IS NULL
        ",
        payout_id.0
    )
    .fetch_optional(&**pool)
    .await?
    .ok_or_else(|| {
        ApiError::InvalidInput("提现记录不存在或无需确认".to_string())
    })?;

    let profile = YunzhanghuProfile::get(
        crate::database::models::UserId(row.user_id),
        &**pool,
    )
    .await?;
    let (
        real_name,
        id_card_last4,
        phone_masked,
        sign_status,
        kyc_matches_payout,
    ) = if let Some(profile) = profile {
        let kyc_matches_payout = profile.sign_status == YzhSignStatus::Signed
            && profile.alipay_account.as_deref()
                == row.method_address.as_deref();
        (
            profile.real_name,
            profile.id_card_last4,
            profile.phone.as_deref().map(mask_admin_phone),
            profile.sign_status.as_str().to_string(),
            kyc_matches_payout,
        )
    } else {
        (
            None,
            None,
            None,
            YzhSignStatus::Unsigned.as_str().to_string(),
            false,
        )
    };

    Ok(HttpResponse::Ok().json(AdminProcessingPayoutDetail {
        id: public_id,
        user_id: crate::models::ids::UserId::from(
            crate::database::models::UserId(row.user_id),
        ),
        username: row.username,
        created: row.created,
        amount: row.amount,
        fee: row.fee,
        status: PayoutStatus::from_string(&row.status),
        method: row
            .method
            .map(|method| PayoutMethodType::from_string(&method)),
        order_id: format!("bbsmc-{}", public_id),
        platform_id: row.platform_id,
        submit_started_at: row.yunzhanghu_submit_started_at,
        submit_error: row.yunzhanghu_submit_error,
        submit_attempts: row.yunzhanghu_submit_attempts,
        real_name,
        id_card_last4,
        phone_masked,
        alipay_account: row.method_address,
        sign_status,
        kyc_matches_payout,
    }))
}

#[post("admin/{id}/confirm")]
pub async fn admin_confirm_payout(
    req: HttpRequest,
    path: web::Path<crate::models::ids::PayoutId>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let admin = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?;

    let public_id = path.into_inner();
    let payout_id: crate::database::models::PayoutId = public_id.into();
    let order_id = format!("bbsmc-{}", public_id);
    let admin_id = crate::database::models::UserId::from(admin.id);

    let (
        pay_amount,
        user_id,
        username,
        real_name,
        id_card,
        alipay_account,
        phone,
    ) = prepare_yunzhanghu_submit(&pool, payout_id, &order_id, admin_id)
        .await?;

    let resp = match submit_yunzhanghu_alipay_order(
        &order_id,
        pay_amount,
        user_id,
        &username,
        &real_name,
        &id_card,
        &alipay_account,
        &phone,
    )
    .await
    {
        Ok(resp) => resp,
        Err(err) => {
            if let Err(update_err) =
                record_yunzhanghu_submit_error(&pool, payout_id, &err).await
            {
                log::warn!(
                    "记录云账户提交失败状态失败 payout_id={}: {}",
                    payout_id.0,
                    update_err
                );
            }
            return Err(err);
        }
    };

    sqlx::query!(
        "
        UPDATE payouts
        SET platform_id = $2,
            yunzhanghu_submit_finished_at = NOW(),
            yunzhanghu_submit_error = NULL
        WHERE id = $1 AND platform_id IS NULL AND status = 'in-transit'
        ",
        payout_id.0,
        resp.ref_id,
    )
    .execute(&**pool)
    .await?;

    crate::database::models::User::clear_caches(&[(user_id, None)], &redis)
        .await?;
    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "payout_id": public_id,
        "order_id": order_id,
        "ref": resp.ref_id,
        "amount": resp.pay,
        "status": PayoutStatus::InTransit.as_str(),
    })))
}

#[post("admin/{id}/reject")]
pub async fn admin_reject_payout(
    req: HttpRequest,
    path: web::Path<crate::models::ids::PayoutId>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<AdminRejectPayout>,
) -> Result<HttpResponse, ApiError> {
    let admin = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?;

    let public_id = path.into_inner();
    let payout_id: crate::database::models::PayoutId = public_id.into();
    let admin_id = crate::database::models::UserId::from(admin.id);
    let reason = body
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(|reason| truncate_for_db(reason, 500));

    let mut tx = pool.begin().await?;
    let payout = sqlx::query!(
        "
        SELECT id, user_id, status, amount, method, platform_id,
               yunzhanghu_submit_started_at
        FROM payouts
        WHERE id = $1
        FOR UPDATE
        ",
        payout_id.0
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("提现记录不存在".to_string()))?;

    if PayoutStatus::from_string(&payout.status) != PayoutStatus::InTransit {
        return Err(ApiError::InvalidInput(
            "该提现记录不是处理中状态".to_string(),
        ));
    }
    if payout.method.as_deref()
        != Some(PayoutMethodType::YunzhanghuAlipay.as_str())
    {
        return Err(ApiError::InvalidInput(
            "该提现记录不是云账户支付宝通道".to_string(),
        ));
    }
    if payout.platform_id.is_some()
        || payout.yunzhanghu_submit_started_at.is_some()
    {
        return Err(ApiError::InvalidInput(
            "该提现已提交或正在提交云账户，不能直接退回".to_string(),
        ));
    }

    sqlx::query!(
        "
        UPDATE payouts
        SET status = 'cancelled',
            admin_rejected_at = NOW(),
            admin_rejected_by = $2,
            admin_reject_reason = $3
        WHERE id = $1
        ",
        payout_id.0,
        admin_id.0,
        reason.as_deref(),
    )
    .execute(&mut *tx)
    .await?;

    insert_payout_admin_rejected_notification(
        &mut tx,
        &redis,
        crate::database::models::UserId(payout.user_id),
        payout.amount,
        reason.as_deref(),
    )
    .await?;

    tx.commit().await?;

    let user_id = crate::database::models::UserId(payout.user_id);
    crate::database::models::User::clear_caches(&[(user_id, None)], &redis)
        .await?;
    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "payout_id": public_id,
        "status": PayoutStatus::Cancelled.as_str(),
    })))
}

async fn prepare_yunzhanghu_submit(
    pool: &PgPool,
    payout_id: crate::database::models::PayoutId,
    order_id: &str,
    admin_id: crate::database::models::UserId,
) -> Result<
    (
        Decimal,
        crate::database::models::UserId,
        String,
        String,
        String,
        String,
        String,
    ),
    ApiError,
> {
    let mut tx = pool.begin().await?;
    let payout = sqlx::query!(
        "
        SELECT p.id, p.user_id, p.amount, p.fee, p.status, p.method, p.method_address,
               p.platform_id, p.yunzhanghu_submit_started_at,
               p.yunzhanghu_submit_finished_at, u.username
        FROM payouts p
        INNER JOIN users u ON u.id = p.user_id
        WHERE p.id = $1
        FOR UPDATE OF p
        ",
        payout_id.0
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("提现记录不存在".to_string()))?;

    if PayoutStatus::from_string(&payout.status) != PayoutStatus::InTransit {
        return Err(ApiError::InvalidInput(
            "该提现记录不是处理中状态".to_string(),
        ));
    }
    if payout.method.as_deref()
        != Some(PayoutMethodType::YunzhanghuAlipay.as_str())
    {
        return Err(ApiError::InvalidInput(
            "该提现记录不是云账户支付宝通道".to_string(),
        ));
    }
    if payout.platform_id.is_some() {
        return Err(ApiError::InvalidInput(
            "该提现记录已提交云账户，无需重复确认".to_string(),
        ));
    }
    if let Some(started_at) = payout.yunzhanghu_submit_started_at
        && payout.yunzhanghu_submit_finished_at.is_none()
        && Utc::now().signed_duration_since(started_at)
            < Duration::seconds(YZH_SUBMIT_IN_PROGRESS_LOCK_SECONDS)
    {
        return Err(ApiError::InvalidInput(
            "该提现正在提交云账户，请稍后刷新或查单".to_string(),
        ));
    }

    let user_id = crate::database::models::UserId(payout.user_id);
    let profile = YunzhanghuProfile::get(user_id, &mut *tx)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("用户缺少云账户实名资料".to_string())
        })?;

    if profile.sign_status != YzhSignStatus::Signed {
        return Err(ApiError::InvalidInput(
            "用户当前未完成云账户签约，不能确认转账".to_string(),
        ));
    }

    let real_name = profile.real_name.clone().ok_or_else(|| {
        ApiError::InvalidInput("KYC 信息异常：缺少真实姓名".to_string())
    })?;
    let alipay_account = profile.alipay_account.clone().ok_or_else(|| {
        ApiError::InvalidInput("KYC 信息异常：缺少支付宝账号".to_string())
    })?;
    let payout_account = payout.method_address.as_deref().ok_or_else(|| {
        ApiError::InvalidInput("提现记录缺少支付宝账号".to_string())
    })?;
    if alipay_account != payout_account {
        return Err(ApiError::InvalidInput(
            "用户收款账号已变更，请不要确认该笔旧账号提现".to_string(),
        ));
    }
    let phone = profile.phone.clone().ok_or_else(|| {
        ApiError::InvalidInput("KYC 信息异常：缺少手机号".to_string())
    })?;
    let id_card = profile
        .decrypt_id_card()
        .map_err(|e| {
            ApiError::InvalidInput(format!("身份证号解密失败: {}", e))
        })?
        .ok_or_else(|| {
            ApiError::InvalidInput("KYC 信息异常：缺少身份证号".to_string())
        })?;
    let pay_amount =
        truncate_money_to_cents(payout.amount - payout.fee.unwrap_or_default());
    if pay_amount <= Decimal::ZERO {
        return Err(ApiError::InvalidInput(
            "扣除手续费后到账金额必须大于 0".to_string(),
        ));
    }

    sqlx::query!(
        "
        UPDATE payouts
        SET yunzhanghu_order_id = $2,
            yunzhanghu_submit_started_at = NOW(),
            yunzhanghu_submit_finished_at = NULL,
            yunzhanghu_submit_attempts = yunzhanghu_submit_attempts + 1,
            yunzhanghu_submit_error = NULL,
            yunzhanghu_confirmed_by = $3
        WHERE id = $1 AND platform_id IS NULL AND status = 'in-transit'
        ",
        payout_id.0,
        order_id,
        admin_id.0,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        pay_amount,
        user_id,
        payout.username,
        real_name,
        id_card,
        alipay_account,
        phone,
    ))
}

async fn record_yunzhanghu_submit_error(
    pool: &PgPool,
    payout_id: crate::database::models::PayoutId,
    err: &ApiError,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
        UPDATE payouts
        SET yunzhanghu_submit_finished_at = NOW(),
            yunzhanghu_submit_error = $2
        WHERE id = $1 AND platform_id IS NULL AND status = 'in-transit'
        ",
        payout_id.0,
        truncate_for_db(&err.to_string(), 1000),
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_payout_admin_rejected_notification(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
    user_id: crate::database::models::UserId,
    amount: Decimal,
    reason: Option<&str>,
) -> Result<(), crate::database::models::DatabaseError> {
    let reason_text = reason
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(|reason| format!("原因：{}", reason))
        .unwrap_or_else(|| "请在转账记录中查看详情。".to_string());

    crate::database::models::notification_item::NotificationBuilder {
        body: crate::models::notifications::NotificationBody::LegacyMarkdown {
            notification_type: Some("payout_cancelled".to_string()),
            name: "提现已退回".to_string(),
            text: format!(
                "您的 {} 提现已被管理员退回，金额已退回到可提现余额。{}",
                format_withdraw_amount(amount),
                reason_text
            ),
            link: "/dashboard/revenue/transfers".to_string(),
            actions: vec![],
        },
    }
    .insert(user_id, tx, redis)
    .await?;

    Ok(())
}

fn format_withdraw_amount(amount: Decimal) -> String {
    format!("¥{:.2}", amount.round_dp(2))
}

#[allow(clippy::too_many_arguments)]
async fn submit_yunzhanghu_alipay_order(
    order_id: &str,
    amount: Decimal,
    user_id: crate::database::models::UserId,
    username: &str,
    real_name: &str,
    id_card: &str,
    alipay_account: &str,
    phone: &str,
) -> Result<yzh_api::AlipayOrderResponse, ApiError> {
    let self_addr =
        dotenvy::var("SELF_ADDR")?.trim_end_matches('/').to_string();
    let notify_url = format!("{}/v3/yunzhanghu/_webhook/order", self_addr);
    let pay_str = format!("{amount:.2}");
    let user_id_str = user_id.0.to_string();

    let client = YzhClient::new();
    yzh_api::alipay_order(
        &client,
        &yzh_api::AlipayOrderRequest {
            order_id,
            real_name,
            card_no: alipay_account,
            id_card,
            phone_no: phone,
            pay: &pay_str,
            pay_remark: Some("BBSMC 创作者激励"),
            order_title: None,
            check_name: Some("Check"),
            notify_url: Some(&notify_url),
            dealer_platform_name: DEALER_PLATFORM_NAME,
            dealer_user_nickname: username,
            dealer_user_id: &user_id_str,
        },
    )
    .await
    .map_err(|e| {
        log::error!("云账户支付宝下单失败 order_id={}: {}", order_id, e);
        ApiError::InvalidInput(format!("云账户接口失败: {}", e))
    })
}

fn mask_admin_phone(phone: &str) -> String {
    let chars = phone.chars().collect::<Vec<_>>();
    if chars.len() < 7 {
        return "*".repeat(chars.len());
    }
    format!(
        "{}****{}",
        chars.iter().take(3).collect::<String>(),
        chars.iter().skip(chars.len() - 4).collect::<String>()
    )
}

fn mask_alipay_account(account: &str) -> String {
    if let Some((local, domain)) = account.split_once('@') {
        let local_chars = local.chars().collect::<Vec<_>>();
        let visible = local_chars.iter().take(1).collect::<String>();
        let masked_local = if visible.is_empty() {
            "***".to_string()
        } else {
            format!("{}***", visible)
        };
        return format!("{}@{}", masked_local, domain);
    }

    let chars = account.chars().collect::<Vec<_>>();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    if chars.len() <= 7 {
        return format!(
            "{}***{}",
            chars.iter().take(1).collect::<String>(),
            chars.iter().skip(chars.len() - 2).collect::<String>()
        );
    }

    format!(
        "{}****{}",
        chars.iter().take(3).collect::<String>(),
        chars.iter().skip(chars.len() - 4).collect::<String>()
    )
}

fn normalize_admin_payout_status(
    status: Option<&str>,
) -> Result<Option<&'static str>, ApiError> {
    let Some(status) =
        status.map(str::trim).filter(|status| !status.is_empty())
    else {
        return Ok(None);
    };

    match status {
        "all" => Ok(None),
        "in-transit" => Ok(Some("in-transit")),
        "success" => Ok(Some("success")),
        "failed" => Ok(Some("failed")),
        "cancelled" => Ok(Some("cancelled")),
        "cancelling" => Ok(Some("cancelling")),
        _ => Err(ApiError::InvalidInput("未知的提现状态筛选".to_string())),
    }
}

fn truncate_for_db(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn decimal_from_yzh_field(
    value: &str,
    field: &str,
) -> Result<Option<Decimal>, ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    value.parse::<Decimal>().map(Some).map_err(|e| {
        ApiError::InvalidInput(format!(
            "云账户试算返回字段 {} 格式异常: {}",
            field, e
        ))
    })
}

fn truncate_money_to_cents(amount: Decimal) -> Decimal {
    amount.round_dp_with_strategy(2, RoundingStrategy::ToZero)
}

fn calculate_withdraw_service_fee(amount: Decimal) -> Decimal {
    truncate_money_to_cents(amount * WITHDRAW_SERVICE_FEE_RATE)
}

fn calculate_yunzhanghu_pay_amount(
    amount: Decimal,
) -> Result<Decimal, ApiError> {
    let fee = calculate_withdraw_service_fee(amount);
    let pay_amount = truncate_money_to_cents(amount - fee);
    if pay_amount <= Decimal::ZERO {
        return Err(ApiError::InvalidInput(
            "扣除手续费后到账金额必须大于 0".to_string(),
        ));
    }
    Ok(pay_amount)
}

fn ensure_supported_payout_amount(
    amount: Decimal,
) -> Result<Decimal, ApiError> {
    if amount <= Decimal::ZERO {
        return Err(ApiError::InvalidInput("请输入合法的提现金额".to_string()));
    }
    if amount < MIN_WITHDRAW_AMOUNT {
        return Err(ApiError::InvalidInput(format!(
            "提现金额不能低于 ¥{}",
            MIN_WITHDRAW_AMOUNT
        )));
    }
    Ok(amount)
}

fn normalize_requested_payout_amount(
    input: Decimal,
) -> Result<Decimal, ApiError> {
    let amount = input.round_dp(2);
    if input != amount {
        return Err(ApiError::InvalidInput(
            "提现金额最多支持 2 位小数".to_string(),
        ));
    }
    ensure_supported_payout_amount(amount)
}

async fn quote_yunzhanghu_payout(
    amount: Decimal,
    profile: &YunzhanghuProfile,
) -> Result<PayoutQuote, ApiError> {
    let real_name = profile.real_name.as_deref().ok_or_else(|| {
        ApiError::InvalidInput("KYC 信息异常：缺少真实姓名".to_string())
    })?;
    let id_card = profile
        .decrypt_id_card()
        .map_err(|e| {
            ApiError::InvalidInput(format!("身份证号解密失败: {}", e))
        })?
        .ok_or_else(|| {
            ApiError::InvalidInput("KYC 信息异常：缺少身份证号".to_string())
        })?;
    let user_fee = calculate_withdraw_service_fee(amount);
    let arrival_amount = calculate_yunzhanghu_pay_amount(amount)?;
    let pay_str = format!("{arrival_amount:.2}");

    let client = YzhClient::new();
    let resp = yzh_api::calc_tax(
        &client,
        &yzh_api::CalcTaxRequest {
            real_name,
            id_card: &id_card,
            pay: &pay_str,
            tax_type: Some("before_tax"),
            before_tax_amount_type: Some("max"),
            include_recovery_amount: Some(1),
            include_user_service_fee: Some(2),
        },
    )
    .await
    .map_err(|e| {
        log::warn!(
            "云账户订单税费试算失败 user_id={}: {}",
            profile.user_id.0,
            e
        );
        ApiError::InvalidInput(format!("云账户试算失败: {}", e))
    })?;

    if !(resp.status == "1" || resp.status.eq_ignore_ascii_case("success"))
        || !(resp.status_detail.is_empty() || resp.status_detail == "0")
    {
        let message = [
            resp.status_message.as_str(),
            resp.status_detail_message.as_str(),
        ]
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("：");
        return Err(ApiError::InvalidInput(if message.is_empty() {
            "云账户税费试算未通过".to_string()
        } else {
            format!("云账户税费试算未通过：{}", message)
        }));
    }

    let required_balance = amount;
    let after_tax_amount =
        decimal_from_yzh_field(&resp.after_tax_amount, "after_tax_amount")?
            .or(Some(arrival_amount));

    Ok(PayoutQuote {
        amount,
        arrival_amount,
        user_fee,
        required_balance,
        after_tax_amount,
        tax: decimal_from_yzh_field(&resp.tax, "tax")?,
        user_tax: decimal_from_yzh_field(&resp.user_tax, "user_tax")?,
        dealer_tax: decimal_from_yzh_field(&resp.dealer_tax, "dealer_tax")?,
        broker_tax: decimal_from_yzh_field(&resp.broker_tax, "broker_tax")?,
        tax_detail: PayoutQuoteTaxDetail {
            personal_tax: decimal_from_yzh_field(
                &resp.tax_detail.personal_tax,
                "tax_detail.personal_tax",
            )?,
            value_added_tax: decimal_from_yzh_field(
                &resp.tax_detail.value_added_tax,
                "tax_detail.value_added_tax",
            )?,
            additional_tax: decimal_from_yzh_field(
                &resp.tax_detail.additional_tax,
                "tax_detail.additional_tax",
            )?,
            user_personal_tax: decimal_from_yzh_field(
                &resp.tax_detail.user_personal_tax,
                "tax_detail.user_personal_tax",
            )?,
            user_value_added_tax: decimal_from_yzh_field(
                &resp.tax_detail.user_value_added_tax,
                "tax_detail.user_value_added_tax",
            )?,
            user_additional_tax: decimal_from_yzh_field(
                &resp.tax_detail.user_additional_tax,
                "tax_detail.user_additional_tax",
            )?,
        },
        status_message: {
            let message = [
                resp.status_message.trim(),
                resp.status_detail_message.trim(),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("：");
            if message.is_empty() {
                None
            } else {
                Some(message)
            }
        },
    })
}

#[post("quote")]
pub async fn quote_payout(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    body: web::Json<Withdrawal>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (scopes, user) = get_user_record_from_bearer_token(
        &req,
        None,
        &**pool,
        &redis,
        &session_queue,
    )
    .await?
    .ok_or_else(|| {
        ApiError::Authentication(AuthenticationError::InvalidCredentials)
    })?;

    if !scopes.contains(Scopes::PAYOUTS_WRITE) {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    if body.method != PayoutMethodType::YunzhanghuAlipay {
        return Err(ApiError::InvalidInput(
            "暂仅支持云账户·支付宝通道".to_string(),
        ));
    }

    let amount = normalize_requested_payout_amount(body.amount)?;
    let profile =
        YunzhanghuProfile::get(user.id, &**pool)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "请先完善实名信息与支付宝账号".to_string(),
                )
            })?;
    if profile.sign_status != YzhSignStatus::Signed {
        return Err(ApiError::InvalidInput(
            "您尚未完成签约，无法提现。请到「实名认证 & 收款账号」卡片完成签约。"
                .to_string(),
        ));
    }

    Ok(HttpResponse::Ok()
        .json(quote_yunzhanghu_payout(amount, &profile).await?))
}

/// 发起一笔提现。流程：
/// 1. 验证 KYC 完整 + 已签约
/// 2. 调云账户订单税费试算，获取用户服务费/预计到账等展示信息
/// 3. 落库 `payouts` 记录，状态 `in-transit`，等待管理员确认转账
#[post("")]
pub async fn create_payout(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    body: web::Json<Withdrawal>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (scopes, user) = get_user_record_from_bearer_token(
        &req,
        None,
        &**pool,
        &redis,
        &session_queue,
    )
    .await?
    .ok_or_else(|| {
        ApiError::Authentication(AuthenticationError::InvalidCredentials)
    })?;

    if !scopes.contains(Scopes::PAYOUTS_WRITE) {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    // 仅支持云账户·支付宝
    if body.method != PayoutMethodType::YunzhanghuAlipay {
        return Err(ApiError::InvalidInput(
            "暂仅支持云账户·支付宝通道".to_string(),
        ));
    }

    let amount = normalize_requested_payout_amount(body.amount)?;

    // 加载 KYC + 签约状态
    let profile =
        YunzhanghuProfile::get(user.id, &**pool)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "请先完善实名信息与支付宝账号".to_string(),
                )
            })?;
    if profile.sign_status != YzhSignStatus::Signed {
        return Err(ApiError::InvalidInput(
            "您尚未完成签约，无法提现。请到「实名认证 & 收款账号」卡片完成签约。"
                .to_string(),
        ));
    }
    if profile.real_name.is_none() {
        return Err(ApiError::InvalidInput(
            "KYC 信息异常：缺少真实姓名".to_string(),
        ));
    }
    let alipay_account = profile.alipay_account.clone().ok_or_else(|| {
        ApiError::InvalidInput("KYC 信息异常：缺少支付宝账号".to_string())
    })?;
    if profile.phone.is_none() {
        return Err(ApiError::InvalidInput(
            "KYC 信息异常：缺少手机号".to_string(),
        ));
    }
    if profile.id_card_encrypted.is_none() {
        return Err(ApiError::InvalidInput(
            "KYC 信息异常：缺少身份证号".to_string(),
        ));
    }

    let quote = quote_yunzhanghu_payout(amount, &profile).await?;

    // 事务开启
    let mut transaction = pool.begin().await?;

    // 锁住用户行避免并发提现
    sqlx::query!(
        "SELECT balance FROM users WHERE id = $1 FOR UPDATE",
        user.id.0
    )
    .fetch_optional(&mut *transaction)
    .await?;

    // 校验可用余额：用户输入金额就是本次从余额扣除的总额，服务费只影响预计到账。
    let balance = get_user_balance(user.id, &pool).await?;
    if balance.available < quote.required_balance {
        return Err(ApiError::InvalidInput(format!(
            "您的余额不足，本次提现需要 ¥{:.2}",
            quote.required_balance
        )));
    }

    // 生成订单号
    let payout_id = generate_payout_id(&mut transaction).await?;
    let order_id =
        format!("bbsmc-{}", crate::models::ids::PayoutId::from(payout_id));

    // 落库 payout 记录：InTransit 表示已占用余额，等待管理员确认转账。
    let payout_item = crate::database::models::payout_item::Payout {
        id: payout_id,
        user_id: user.id,
        created: Utc::now(),
        status: PayoutStatus::InTransit,
        amount,
        // 云账户通道手续费从提现金额内扣，不作为额外余额扣除项。
        fee: Some(quote.user_fee),
        method: Some(PayoutMethodType::YunzhanghuAlipay),
        method_address: Some(alipay_account.clone()),
        platform_id: None,
        admin_reject_reason: None,
    };

    payout_item.insert(&mut transaction).await?;
    transaction.commit().await?;

    crate::database::models::User::clear_caches(&[(user.id, None)], &redis)
        .await?;
    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "payout_id": crate::models::ids::PayoutId::from(payout_id),
        "order_id": order_id,
        "amount": amount.to_string(),
        "arrival_amount": quote.arrival_amount.to_string(),
        "fee": quote.user_fee.to_string(),
        "required_balance": quote.required_balance.to_string(),
        "status": PayoutStatus::InTransit.as_str(),
        "requires_admin_confirmation": true,
    })))
}

#[delete("{id}")]
pub async fn cancel_payout(
    info: web::Path<(PayoutId,)>,
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    _payouts_queue: web::Data<PayoutsQueue>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let _user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_WRITE]),
    )
    .await?
    .1;

    let _ = info.into_inner().0;
    // 云账户实时支付一旦提交无法直接取消，需要联系云账户运营
    Err(ApiError::InvalidInput(
        "云账户实时支付订单一经提交无法取消，请联系客服处理。".to_string(),
    ))
}

#[derive(Deserialize)]
pub struct MethodFilter {
    pub country: Option<String>,
}

/// 返回可用提现通道列表。
///
/// 当前仅向中国大陆用户开放支付宝；其他国家空数组。
#[get("methods")]
pub async fn payment_methods(
    _payouts_queue: web::Data<PayoutsQueue>,
    filter: web::Query<MethodFilter>,
) -> Result<HttpResponse, ApiError> {
    let country = filter.country.as_deref().unwrap_or("CN");
    if country != "CN" {
        return Ok(HttpResponse::Ok().json(Vec::<PayoutMethod>::new()));
    }

    let methods = vec![PayoutMethod {
        id: "yunzhanghu_alipay".to_string(),
        type_: PayoutMethodType::YunzhanghuAlipay,
        name: "支付宝".to_string(),
        supported_countries: vec!["CN".to_string()],
        image_url: None,
        interval: PayoutInterval::Standard {
            min: MIN_WITHDRAW_AMOUNT,
            max: Decimal::from(50000), // 单笔上限，云账户/支付宝实际限额以风控为准
        },
        fee: PayoutMethodFee {
            // 手续费从提现金额中内扣，实际到账金额由 /payout/quote 返回。
            percentage: WITHDRAW_SERVICE_FEE_RATE,
            min: Decimal::ZERO,
            max: None,
        },
    }];

    Ok(HttpResponse::Ok().json(methods))
}

#[derive(Serialize)]
pub struct UserBalance {
    pub available: Decimal,
    pub pending: Decimal,
}

#[get("balance")]
pub async fn get_balance(
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

    let balance = get_user_balance(user.id.into(), &pool).await?;

    Ok(HttpResponse::Ok().json(balance))
}

async fn get_user_balance(
    user_id: crate::database::models::ids::UserId,
    pool: &PgPool,
) -> Result<UserBalance, sqlx::Error> {
    let available = sqlx::query!(
        "
        SELECT SUM(amount)
        FROM payouts_values
        WHERE user_id = $1 AND date_available <= NOW()
        ",
        user_id.0
    )
    .fetch_optional(pool)
    .await?;

    let pending = sqlx::query!(
        "
        SELECT SUM(amount)
        FROM payouts_values
        WHERE user_id = $1 AND date_available > NOW()
        ",
        user_id.0
    )
    .fetch_optional(pool)
    .await?;

    let withdrawn = sqlx::query!(
        "
        SELECT
            SUM(amount) amount,
            SUM(
                CASE
                    WHEN method = 'yunzhanghu_alipay' THEN 0
                    ELSE COALESCE(fee, 0)
                END
            ) fee
        FROM payouts
        WHERE user_id = $1 AND (status = 'success' OR status = 'in-transit')
        ",
        user_id.0
    )
    .fetch_optional(pool)
    .await?;

    let available = available
        .map(|x| x.sum.unwrap_or(Decimal::ZERO))
        .unwrap_or(Decimal::ZERO);
    let pending = pending
        .map(|x| x.sum.unwrap_or(Decimal::ZERO))
        .unwrap_or(Decimal::ZERO);
    let (withdrawn, fees) = withdrawn
        .map(|x| {
            (
                x.amount.unwrap_or(Decimal::ZERO),
                x.fee.unwrap_or(Decimal::ZERO),
            )
        })
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));

    Ok(UserBalance {
        available: available.round_dp(16)
            - withdrawn.round_dp(16)
            - fees.round_dp(16),
        pending,
    })
}

#[derive(Serialize, Deserialize)]
pub struct RevenueResponse {
    pub all_time: Decimal,
    pub data: Vec<RevenueData>,
}

#[derive(Serialize, Deserialize)]
pub struct RevenueData {
    pub time: u64,
    pub revenue: Decimal,
    pub creator_revenue: Decimal,
}

#[get("platform_revenue")]
pub async fn platform_revenue(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let mut redis = redis.connect().await?;

    const PLATFORM_REVENUE_NAMESPACE: &str = "platform_revenue";

    let res: Option<RevenueResponse> = redis
        .get_deserialized_from_json(PLATFORM_REVENUE_NAMESPACE, "0")
        .await?;

    if let Some(res) = res {
        return Ok(HttpResponse::Ok().json(res));
    }

    let all_time_payouts = sqlx::query!(
        "
        SELECT SUM(amount) from payouts_values
        ",
    )
    .fetch_optional(&**pool)
    .await?
    .and_then(|x| x.sum)
    .unwrap_or(Decimal::ZERO);

    let points = make_aditude_request(
        &["METRIC_REVENUE", "METRIC_IMPRESSIONS"],
        "30d",
        "1d",
    )
    .await?;

    let mut points_map = HashMap::new();

    for point in points {
        for point in point.points_list {
            let entry =
                points_map.entry(point.time.seconds).or_insert((None, None));

            if let Some(revenue) = point.metric.revenue {
                entry.0 = Some(revenue);
            }

            if let Some(impressions) = point.metric.impressions {
                entry.1 = Some(impressions);
            }
        }
    }

    let mut revenue_data = Vec::new();
    let now = Utc::now();

    for i in 1..=30 {
        let time = now - Duration::days(i);
        let start = time
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        if let Some((revenue, impressions)) = points_map.remove(&(start as u64))
        {
            // 2024/9/5 之前，旧版提现机制生效期间
            if start >= 1725494400 {
                let revenue = revenue.unwrap_or(Decimal::ZERO);
                let impressions = impressions.unwrap_or(0);

                // BBSMC 的广告收入分成
                let platform_cut = Decimal::from(1) / Decimal::from(4);
                // Clean.io 费用（广告反恶意软件），按每千次展示计算
                let clean_io_fee = Decimal::from(8) / Decimal::from(1000);

                let net_revenue = revenue
                    - (clean_io_fee * Decimal::from(impressions)
                        / Decimal::from(1000));

                let payout = net_revenue * (Decimal::from(1) - platform_cut);

                revenue_data.push(RevenueData {
                    time: start as u64,
                    revenue: net_revenue,
                    creator_revenue: payout,
                });

                continue;
            }
        }

        revenue_data.push(get_legacy_data_point(start as u64));
    }

    let res = RevenueResponse {
        all_time: all_time_payouts,
        data: revenue_data,
    };

    redis
        .set_serialized_to_json(
            PLATFORM_REVENUE_NAMESPACE,
            0,
            &res,
            Some(60 * 60),
        )
        .await?;

    Ok(HttpResponse::Ok().json(res))
}

fn get_legacy_data_point(timestamp: u64) -> RevenueData {
    let start = Utc.timestamp_opt(timestamp as i64, 0).unwrap();

    let old_payouts_budget = Decimal::from(10_000);

    let days = Decimal::from(28);
    let weekdays = Decimal::from(20);
    let weekend_bonus = Decimal::from(5) / Decimal::from(4);

    let weekday_amount =
        old_payouts_budget / (weekdays + (weekend_bonus) * (days - weekdays));
    let weekend_amount = weekday_amount * weekend_bonus;

    let payout = match start.weekday() {
        Weekday::Sat | Weekday::Sun => weekend_amount,
        _ => weekday_amount,
    };

    RevenueData {
        time: timestamp,
        revenue: payout,
        creator_revenue: payout * (Decimal::from(9) / Decimal::from(10)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_truncates_to_cents() {
        assert_eq!(
            truncate_money_to_cents(Decimal::from_parts(165, 0, 0, false, 3)),
            Decimal::from_parts(16, 0, 0, false, 2)
        );
    }

    #[test]
    fn withdraw_service_fee_is_three_percent_truncated() {
        assert_eq!(
            calculate_withdraw_service_fee(Decimal::from_parts(
                501, 0, 0, false, 2
            )),
            Decimal::from_parts(15, 0, 0, false, 2)
        );
    }

    #[test]
    fn yunzhanghu_pay_amount_deducts_fee_from_requested_amount() {
        assert_eq!(
            calculate_yunzhanghu_pay_amount(Decimal::from(100)).unwrap(),
            Decimal::from(97)
        );
    }
}
