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
/// 云账户支付宝单笔提现上限（人民币元）。
const MAX_WITHDRAW_AMOUNT: Decimal =
    Decimal::from_parts(50_000, 0, 0, false, 0);
const WITHDRAW_SERVICE_FEE_RATE: Decimal =
    Decimal::from_parts(3, 0, 0, false, 2);
/// 云账户待转账金额之外由平台承担的额外服务费率（6.8%）。
const YUNZHANGHU_EXTRA_SERVICE_FEE_RATE: Decimal =
    Decimal::from_parts(68, 0, 0, false, 3);
const BATCH_PAYOUT_ADMIN_USERNAME: &str = "BBSMC";
const BATCH_PAYOUT_CONFIRMATION: &str = "我确认批量申请提现";
const BATCH_PAYOUT_PREVIEW_MINUTES: i64 = 15;
const BATCH_PAYOUT_HEARTBEAT_TIMEOUT_SECONDS: i64 = 5 * 60;
const BATCH_PAYOUT_PREVIEW_CLEANUP_LIMIT: i64 = 100;

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
            .service(admin_batch_payout_preview)
            .service(admin_batch_payout_apply)
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
    pub pending_transfer_summary: AdminPendingTransferSummary,
}

#[derive(Serialize)]
pub struct AdminPendingTransferSummary {
    pub order_count: i64,
    #[serde(with = "rust_decimal::serde::str")]
    pub transfer_amount: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub additional_service_fee: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_with_service_fee: Decimal,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdminBatchPayoutSource {
    pub project_id: Option<crate::models::ids::ProjectId>,
    pub slug: Option<String>,
    pub title: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminBatchPayoutUser {
    pub user_id: crate::models::ids::UserId,
    pub username: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub order_count: usize,
    pub orders: Vec<AdminBatchPayoutPlannedOrder>,
    pub sources: Vec<AdminBatchPayoutSource>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminBatchPayoutPlannedOrder {
    pub chunk_index: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminBatchPayoutExcludedUser {
    pub user_id: crate::models::ids::UserId,
    pub username: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub reason_code: String,
}

#[derive(Serialize)]
pub struct AdminBatchPayoutPreview {
    pub batch_id: String,
    pub expires_at: DateTime<Utc>,
    pub source_attribution: &'static str,
    pub user_count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_amount: Decimal,
    pub users: Vec<AdminBatchPayoutUser>,
    pub excluded_count: usize,
    pub excluded_users: Vec<AdminBatchPayoutExcludedUser>,
}

#[derive(Deserialize)]
pub struct AdminBatchPayoutApplyRequest {
    pub confirmation: String,
}

#[derive(Serialize)]
pub struct AdminBatchPayoutApplyItem {
    pub user_id: crate::models::ids::UserId,
    pub username: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub status: String,
    /// 兼容旧管理端：拆单时取第一笔，完整结果以 `payout_ids` 为准。
    pub payout_id: Option<crate::models::ids::PayoutId>,
    pub payout_ids: Vec<crate::models::ids::PayoutId>,
    pub reason_code: Option<String>,
    pub retryable: bool,
}

#[derive(Serialize)]
pub struct AdminBatchPayoutApplyResponse {
    pub batch_id: String,
    pub status: String,
    pub requested_count: usize,
    pub created_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_previewed: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_created: Decimal,
    pub items: Vec<AdminBatchPayoutApplyItem>,
}

struct BatchPayoutCandidate {
    user_id: crate::database::models::UserId,
    username: String,
    amount: Decimal,
    profile_updated_at: DateTime<Utc>,
    sources: Vec<AdminBatchPayoutSource>,
    payout_amounts: Vec<Decimal>,
}

struct BatchPayoutPreviewData {
    candidates: Vec<BatchPayoutCandidate>,
    excluded_users: Vec<AdminBatchPayoutExcludedUser>,
}

#[derive(Clone, Debug)]
struct BatchLedgerValue {
    mod_id: Option<i64>,
    amount: Decimal,
    project_name: Option<String>,
    project_slug: Option<String>,
}

struct BatchSourceAccumulator {
    mod_id: Option<i64>,
    project_name: Option<String>,
    project_slug: Option<String>,
    exact_amount: Decimal,
    first_order: usize,
}

struct BatchBalanceSnapshot {
    available: Decimal,
    positive_earned: Decimal,
}

#[derive(Debug)]
enum BatchItemApplyOutcome {
    Created(Vec<crate::database::models::PayoutId>),
    Skipped,
    Failed,
}

fn batch_item_event_type(status: &str) -> Option<&'static str> {
    match status {
        "skipped" => Some("item_skipped"),
        "failed" => Some("item_failed"),
        _ => None,
    }
}

fn batch_terminal_state(
    total_count: i64,
    created_count: i64,
) -> (&'static str, &'static str) {
    if created_count == total_count {
        ("completed", "batch_completed")
    } else {
        ("partial", "batch_partial")
    }
}

fn batch_processing_kind(
    previous_status: &str,
    takeover: bool,
) -> &'static str {
    if takeover {
        "takeover"
    } else if previous_status == "partial" {
        "retry"
    } else {
        "initial"
    }
}

#[post("admin/batch/preview")]
pub async fn admin_batch_payout_preview(
    req: HttpRequest,
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
    ensure_batch_payout_admin(&admin.username)?;
    cleanup_expired_batch_previews(&pool).await?;

    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;

    let preview_data = build_batch_payout_candidates(&mut tx).await?;
    let batch_id = uuid::Uuid::new_v4().simple().to_string();
    let expires_at =
        Utc::now() + Duration::minutes(BATCH_PAYOUT_PREVIEW_MINUTES);
    let admin_id = crate::database::models::UserId::from(admin.id);

    sqlx::query!(
        "
        INSERT INTO admin_payout_batches (
            id, requested_by, requested_by_username, expires_at
        )
        VALUES ($1, $2, $3, $4)
        ",
        batch_id,
        admin_id.0,
        admin.username,
        expires_at,
    )
    .execute(&mut *tx)
    .await?;

    for candidate in &preview_data.candidates {
        let sources = serde_json::to_value(&candidate.sources)?;
        sqlx::query!(
            "
            INSERT INTO admin_payout_batch_items (
                batch_id, user_id, username, amount, profile_updated_at, sources
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
            batch_id,
            candidate.user_id.0,
            candidate.username,
            candidate.amount,
            candidate.profile_updated_at,
            sources,
        )
        .execute(&mut *tx)
        .await?;

        for (chunk_index, amount) in candidate.payout_amounts.iter().enumerate()
        {
            sqlx::query!(
                "
                INSERT INTO admin_payout_batch_orders (
                    batch_id, user_id, chunk_index, amount
                )
                VALUES ($1, $2, $3, $4)
                ",
                batch_id,
                candidate.user_id.0,
                chunk_index as i32,
                amount,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    for excluded in &preview_data.excluded_users {
        let user_id: crate::database::models::UserId = excluded.user_id.into();
        sqlx::query!(
            "
            INSERT INTO admin_payout_batch_exclusions (
                batch_id, user_id, username, amount, reason_code
            )
            VALUES ($1, $2, $3, $4, $5)
            ",
            batch_id,
            user_id.0,
            excluded.username,
            excluded.amount,
            excluded.reason_code,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let users = preview_data
        .candidates
        .into_iter()
        .map(|candidate| AdminBatchPayoutUser {
            user_id: crate::models::ids::UserId::from(candidate.user_id),
            username: candidate.username,
            amount: candidate.amount,
            order_count: candidate.payout_amounts.len(),
            orders: candidate
                .payout_amounts
                .into_iter()
                .enumerate()
                .map(|(chunk_index, amount)| AdminBatchPayoutPlannedOrder {
                    chunk_index,
                    amount,
                })
                .collect(),
            sources: candidate.sources,
        })
        .collect::<Vec<_>>();
    let total_amount = users.iter().map(|user| user.amount).sum();
    let excluded_count = preview_data.excluded_users.len();

    Ok(HttpResponse::Ok().json(AdminBatchPayoutPreview {
        batch_id,
        expires_at,
        source_attribution: "fifo_reconstructed",
        user_count: users.len(),
        total_amount,
        users,
        excluded_count,
        excluded_users: preview_data.excluded_users,
    }))
}

async fn append_batch_payout_event_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    batch_id: &str,
    event_type: &str,
    owner_token: &str,
    user_id: Option<crate::database::models::UserId>,
    details: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let user_id = user_id.map(|id| id.0);
    let result = sqlx::query!(
        "
        INSERT INTO admin_payout_batch_events (
            batch_id,
            event_type,
            actor_user_id,
            actor_username,
            processing_owner,
            user_id,
            details
        )
        SELECT
            id,
            $2,
            requested_by,
            requested_by_username,
            $3,
            $4,
            $5
        FROM admin_payout_batches
        WHERE id = $1
        ",
        batch_id,
        event_type,
        owner_token,
        user_id,
        details,
    )
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

#[post("admin/batch/{batch_id}/apply")]
pub async fn admin_batch_payout_apply(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<AdminBatchPayoutApplyRequest>,
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
    ensure_batch_payout_admin(&admin.username)?;
    if body.confirmation != BATCH_PAYOUT_CONFIRMATION {
        return Err(ApiError::InvalidInput(
            "请输入完整确认短语后再执行批量提现".to_string(),
        ));
    }

    let batch_id = path.into_inner();
    let admin_id = crate::database::models::UserId::from(admin.id);
    let owner_token = uuid::Uuid::new_v4().simple().to_string();
    let mut batch_tx = pool.begin().await?;
    let batch = sqlx::query!(
        "
        SELECT requested_by, status, expires_at, processing_owner, heartbeat_at
        FROM admin_payout_batches
        WHERE id = $1
        FOR UPDATE
        ",
        batch_id,
    )
    .fetch_optional(&mut *batch_tx)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("批量提现预览不存在".to_string()))?;

    if batch.requested_by != admin_id.0 {
        return Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }
    append_batch_payout_event_in_tx(
        &mut batch_tx,
        &batch_id,
        "apply_requested",
        &owner_token,
        None,
        serde_json::json!({
            "observed_status": batch.status.as_str(),
            "expires_at": batch.expires_at,
            "processing_owner": batch.processing_owner.as_deref(),
            "heartbeat_at": batch.heartbeat_at,
        }),
    )
    .await?;
    if batch.status == "completed" {
        batch_tx.commit().await?;
        return Ok(HttpResponse::Ok()
            .json(build_batch_payout_apply_response(&pool, &batch_id).await?));
    }
    if batch.status == "processing"
        && batch.heartbeat_at.is_some_and(|heartbeat_at| {
            Utc::now().signed_duration_since(heartbeat_at)
                < Duration::seconds(BATCH_PAYOUT_HEARTBEAT_TIMEOUT_SECONDS)
        })
    {
        batch_tx.commit().await?;
        return Err(ApiError::InvalidInput(
            "该批次正在处理中，请稍后重试".to_string(),
        ));
    }
    let item_state = sqlx::query!(
        r#"
        SELECT
            COUNT(*)::bigint AS "total!",
            COUNT(*) FILTER (WHERE status = 'created')::bigint AS "created!",
            COUNT(*) FILTER (WHERE status = 'failed')::bigint AS "failed!"
        FROM admin_payout_batch_items
        WHERE batch_id = $1
        "#,
        batch_id,
    )
    .fetch_one(&mut *batch_tx)
    .await?;

    // 兼容历史并发异常：若所有 item 均已创建，锁住 batch 后原子修复终态。
    if batch.status != "previewed" && item_state.created == item_state.total {
        sqlx::query!(
            "
            UPDATE admin_payout_batches
            SET status = 'completed',
                processing_owner = NULL,
                heartbeat_at = NULL,
                finished_at = COALESCE(finished_at, NOW())
            WHERE id = $1
            ",
            batch_id,
        )
        .execute(&mut *batch_tx)
        .await?;
        append_batch_payout_event_in_tx(
            &mut batch_tx,
            &batch_id,
            "batch_completed",
            &owner_token,
            None,
            serde_json::json!({
                "recovered_terminal_state": true,
                "created_count": item_state.created,
                "total_count": item_state.total,
            }),
        )
        .await?;
        batch_tx.commit().await?;
        return Ok(HttpResponse::Ok()
            .json(build_batch_payout_apply_response(&pool, &batch_id).await?));
    }
    if batch.status == "partial" && item_state.failed == 0 {
        batch_tx.commit().await?;
        return Ok(HttpResponse::Ok()
            .json(build_batch_payout_apply_response(&pool, &batch_id).await?));
    }
    if batch.status == "previewed" && batch.expires_at < Utc::now() {
        batch_tx.commit().await?;
        return Err(ApiError::InvalidInput(
            "批量提现预览已过期，请重新打开弹窗计算".to_string(),
        ));
    }
    let is_takeover = batch.status == "processing";
    if is_takeover {
        append_batch_payout_event_in_tx(
            &mut batch_tx,
            &batch_id,
            "processing_taken_over",
            &owner_token,
            None,
            serde_json::json!({
                "previous_owner": batch.processing_owner.as_deref(),
                "previous_heartbeat_at": batch.heartbeat_at,
            }),
        )
        .await?;
    }
    let processing_kind = batch_processing_kind(&batch.status, is_takeover);
    sqlx::query!(
        "
        UPDATE admin_payout_batches
        SET status = 'processing',
            processing_started_at = NOW(),
            processing_owner = $2,
            heartbeat_at = NOW(),
            finished_at = NULL
        WHERE id = $1
        ",
        batch_id,
        owner_token,
    )
    .execute(&mut *batch_tx)
    .await?;
    append_batch_payout_event_in_tx(
        &mut batch_tx,
        &batch_id,
        "processing_started",
        &owner_token,
        None,
        serde_json::json!({
            "kind": processing_kind,
            "previous_status": batch.status.as_str(),
        }),
    )
    .await?;
    batch_tx.commit().await?;

    let items = sqlx::query!(
        "
        SELECT user_id, username, amount, profile_updated_at
        FROM admin_payout_batch_items
        WHERE batch_id = $1 AND status IN ('pending', 'failed')
        ORDER BY user_id
        ",
        batch_id,
    )
    .fetch_all(&**pool)
    .await?;

    let mut created_user_ids = Vec::new();
    let yzh_client = YzhClient::new();
    for item in items {
        refresh_batch_payout_heartbeat(&pool, &batch_id, &owner_token).await?;
        match apply_batch_payout_item(
            &pool,
            &batch_id,
            &owner_token,
            crate::database::models::UserId(item.user_id),
            item.amount,
            item.profile_updated_at,
            &yzh_client,
        )
        .await
        {
            Ok(BatchItemApplyOutcome::Created(payout_ids)) => {
                created_user_ids.push((
                    crate::database::models::UserId(item.user_id),
                    None,
                ));
                for payout_id in payout_ids {
                    log::info!(
                        "批量提现申请已创建 batch_id={} payout_id={} user_id={}",
                        batch_id,
                        payout_id.0,
                        item.user_id
                    );
                }
            }
            Ok(BatchItemApplyOutcome::Skipped) => {}
            Ok(BatchItemApplyOutcome::Failed) => {}
            Err(err) => {
                log::error!(
                    "批量提现申请失败 batch_id={} user_id={}: {}",
                    batch_id,
                    item.user_id,
                    err
                );
                mark_batch_payout_item(
                    &pool,
                    &batch_id,
                    &owner_token,
                    crate::database::models::UserId(item.user_id),
                    "failed",
                    Some("database_error"),
                )
                .await?;
            }
        }
        refresh_batch_payout_heartbeat(&pool, &batch_id, &owner_token).await?;
    }

    finalize_batch_payout(&pool, &batch_id, &owner_token).await?;

    if !created_user_ids.is_empty() {
        if let Err(err) = crate::database::models::User::clear_caches(
            &created_user_ids,
            &redis,
        )
        .await
        {
            log::warn!(
                "批量提现已落库，但清理用户缓存失败 batch_id={}: {}",
                batch_id,
                err
            );
        }
        crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
            .await;
    }

    Ok(HttpResponse::Ok()
        .json(build_batch_payout_apply_response(&pool, &batch_id).await?))
}

async fn cleanup_expired_batch_previews(
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
        DELETE FROM admin_payout_batches
        WHERE id IN (
            SELECT id
            FROM admin_payout_batches
            WHERE status = 'previewed'
              AND expires_at < NOW()
              AND NOT EXISTS (
                  SELECT 1
                  FROM admin_payout_batch_events e
                  WHERE e.batch_id = admin_payout_batches.id
              )
            ORDER BY expires_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
          AND status = 'previewed'
        ",
        BATCH_PAYOUT_PREVIEW_CLEANUP_LIMIT,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn refresh_batch_payout_heartbeat(
    pool: &PgPool,
    batch_id: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    let result = sqlx::query!(
        "
        UPDATE admin_payout_batches
        SET heartbeat_at = NOW()
        WHERE id = $1
          AND status = 'processing'
          AND processing_owner = $2
        ",
        batch_id,
        owner_token,
    )
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(ApiError::InvalidInput(
            "批量提现处理权已失效，请刷新批次状态".to_string(),
        ));
    }
    Ok(())
}

async fn finalize_batch_payout(
    pool: &PgPool,
    batch_id: &str,
    owner_token: &str,
) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    let batch = sqlx::query!(
        "
        SELECT status, processing_owner
        FROM admin_payout_batches
        WHERE id = $1
        FOR UPDATE
        ",
        batch_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("批量提现预览不存在".to_string()))?;
    if batch.status != "processing"
        || batch.processing_owner.as_deref() != Some(owner_token)
    {
        return Err(ApiError::InvalidInput(
            "批量提现处理权已失效，请刷新批次状态".to_string(),
        ));
    }

    let item_state = sqlx::query!(
        r#"
        SELECT
            COUNT(*)::bigint AS "total!",
            COUNT(*) FILTER (WHERE status = 'created')::bigint AS "created!",
            COUNT(*) FILTER (WHERE status = 'skipped')::bigint AS "skipped!",
            COUNT(*) FILTER (WHERE status = 'failed')::bigint AS "failed!",
            COUNT(*) FILTER (WHERE status = 'pending')::bigint AS "pending!"
        FROM admin_payout_batch_items
        WHERE batch_id = $1
        "#,
        batch_id,
    )
    .fetch_one(&mut *tx)
    .await?;
    let (final_status, final_event_type) =
        batch_terminal_state(item_state.total, item_state.created);
    let updated = sqlx::query!(
        "
        UPDATE admin_payout_batches
        SET status = $3,
            processing_owner = NULL,
            heartbeat_at = NULL,
            finished_at = NOW()
        WHERE id = $1
          AND status = 'processing'
          AND processing_owner = $2
        ",
        batch_id,
        owner_token,
        final_status,
    )
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::InvalidInput(
            "批量提现处理权已失效，请刷新批次状态".to_string(),
        ));
    }
    append_batch_payout_event_in_tx(
        &mut tx,
        batch_id,
        final_event_type,
        owner_token,
        None,
        serde_json::json!({
            "total_count": item_state.total,
            "created_count": item_state.created,
            "skipped_count": item_state.skipped,
            "failed_count": item_state.failed,
            "pending_count": item_state.pending,
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn ensure_batch_payout_admin(username: &str) -> Result<(), ApiError> {
    if username == BATCH_PAYOUT_ADMIN_USERNAME {
        Ok(())
    } else {
        Err(ApiError::Authentication(
            AuthenticationError::InvalidCredentials,
        ))
    }
}

fn batch_profile_is_complete(profile: &YunzhanghuProfile) -> bool {
    let is_present = |value: Option<&str>| {
        value.is_some_and(|value| !value.trim().is_empty())
    };

    profile.sign_status == YzhSignStatus::Signed
        && profile.sign_nonce.is_none()
        && is_present(profile.real_name.as_deref())
        && is_present(profile.phone.as_deref())
        && is_present(profile.alipay_account.as_deref())
        && profile
            .decrypt_id_card()
            .ok()
            .flatten()
            .is_some_and(|value| !value.trim().is_empty())
}

fn batch_profile_exclusion_reason(
    profile: &YunzhanghuProfile,
) -> Option<&'static str> {
    if profile.sign_status != YzhSignStatus::Signed {
        Some("not_signed")
    } else if profile.sign_nonce.is_some() {
        Some("sign_operation_pending")
    } else if !batch_profile_is_complete(profile) {
        Some("incomplete_profile")
    } else {
        None
    }
}

fn batch_payout_exclusion(
    user_id: crate::database::models::UserId,
    username: &str,
    amount: Decimal,
    reason_code: &str,
) -> AdminBatchPayoutExcludedUser {
    AdminBatchPayoutExcludedUser {
        user_id: crate::models::ids::UserId::from(user_id),
        username: username.to_string(),
        amount: amount.max(Decimal::ZERO),
        reason_code: reason_code.to_string(),
    }
}

async fn build_batch_payout_candidates(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<BatchPayoutPreviewData, ApiError> {
    let balances = sqlx::query!(
        r#"
        WITH earnings AS (
            SELECT
                user_id,
                COALESCE(SUM(amount), 0)::numeric AS earned,
                COALESCE(SUM(amount) FILTER (WHERE amount > 0), 0)::numeric
                    AS positive_earned
            FROM payouts_values
            WHERE date_available <= NOW()
            GROUP BY user_id
        ), withdrawn AS (
            SELECT
                user_id,
                COALESCE(SUM(amount), 0)::numeric AS amount,
                COALESCE(SUM(
                    CASE
                        WHEN method = 'yunzhanghu_alipay' THEN 0
                        ELSE COALESCE(fee, 0)
                    END
                ), 0)::numeric AS old_channel_fees
            FROM payouts
            WHERE status IN ('success', 'in-transit')
            GROUP BY user_id
        )
        SELECT
            u.id,
            u.username,
            COALESCE(e.earned, 0)::numeric AS "earned!",
            COALESCE(e.positive_earned, 0)::numeric AS "positive_earned!",
            COALESCE(w.amount, 0)::numeric AS "withdrawn!",
            COALESCE(w.old_channel_fees, 0)::numeric AS "old_channel_fees!",
            EXISTS (
                SELECT 1
                FROM user_bans ub
                WHERE ub.user_id = u.id
                  AND ub.ban_type IN ('global', 'resource')
                  AND ub.is_active = TRUE
                  AND (ub.expires_at IS NULL OR ub.expires_at > NOW())
            ) AS "is_banned!"
        FROM earnings e
        INNER JOIN users u ON u.id = e.user_id
        LEFT JOIN withdrawn w ON w.user_id = u.id
        WHERE u.id <> $1
        ORDER BY u.id
        "#,
        crate::models::users::DELETED_USER.0 as i64,
    )
    .fetch_all(&mut **tx)
    .await?;

    struct EligibleBalance {
        user_id: crate::database::models::UserId,
        username: String,
        available: Decimal,
        amount: Decimal,
        positive_earned: Decimal,
        profile_updated_at: DateTime<Utc>,
    }

    let mut eligible = Vec::new();
    let mut excluded_users = Vec::new();
    for balance in balances {
        let available = balance.earned.round_dp(16)
            - balance.withdrawn.round_dp(16)
            - balance.old_channel_fees.round_dp(16);
        let amount = truncate_money_to_cents(available);
        if available <= Decimal::ZERO {
            continue;
        }
        let user_id = crate::database::models::UserId(balance.id);
        if amount < MIN_WITHDRAW_AMOUNT {
            excluded_users.push(batch_payout_exclusion(
                user_id,
                &balance.username,
                amount,
                "below_minimum",
            ));
            continue;
        }
        if balance.is_banned {
            excluded_users.push(batch_payout_exclusion(
                user_id,
                &balance.username,
                amount,
                "banned",
            ));
            continue;
        }

        let profile = match YunzhanghuProfile::get(user_id, &mut **tx).await {
            Ok(Some(profile)) => profile,
            Ok(None) => {
                excluded_users.push(batch_payout_exclusion(
                    user_id,
                    &balance.username,
                    amount,
                    "missing_profile",
                ));
                continue;
            }
            Err(crate::database::models::DatabaseError::SchemaError(_)) => {
                log::warn!("批量提现资料无法解密 user_id={}", user_id.0);
                excluded_users.push(batch_payout_exclusion(
                    user_id,
                    &balance.username,
                    amount,
                    "profile_unreadable",
                ));
                continue;
            }
            Err(err) => return Err(err.into()),
        };
        if let Some(reason) = batch_profile_exclusion_reason(&profile) {
            excluded_users.push(batch_payout_exclusion(
                user_id,
                &balance.username,
                amount,
                reason,
            ));
            continue;
        }

        eligible.push(EligibleBalance {
            user_id,
            username: balance.username,
            available,
            amount,
            positive_earned: balance.positive_earned,
            profile_updated_at: profile.updated_at,
        });
    }

    let eligible_ids = eligible
        .iter()
        .map(|balance| balance.user_id.0)
        .collect::<Vec<_>>();
    let ledger_rows = sqlx::query!(
        r#"
        SELECT
            pv.user_id,
            pv.mod_id,
            pv.amount,
            m.name AS "project_name?",
            m.slug AS "project_slug?"
        FROM payouts_values pv
        LEFT JOIN mods m ON m.id = pv.mod_id
        WHERE pv.user_id = ANY($1::bigint[])
          AND pv.date_available <= NOW()
          AND pv.amount > 0
        ORDER BY pv.user_id, pv.date_available, pv.created, pv.id
        "#,
        &eligible_ids,
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut ledger_by_user = HashMap::<i64, Vec<BatchLedgerValue>>::new();
    for row in ledger_rows {
        ledger_by_user
            .entry(row.user_id)
            .or_default()
            .push(BatchLedgerValue {
                mod_id: row.mod_id,
                amount: row.amount,
                project_name: row.project_name,
                project_slug: row.project_slug,
            });
    }

    let mut candidates = Vec::with_capacity(eligible.len());
    for balance in eligible {
        let amount = balance.amount;
        let values = ledger_by_user
            .remove(&balance.user_id.0)
            .unwrap_or_default();
        let sources = match attribute_batch_payout_sources(
            balance.positive_earned,
            balance.available,
            amount,
            &values,
        ) {
            Ok(sources) => sources,
            Err(reason) => {
                log::warn!(
                    "批量提现收益来源无法对齐 user_id={}: {}",
                    balance.user_id.0,
                    reason
                );
                excluded_users.push(AdminBatchPayoutExcludedUser {
                    user_id: crate::models::ids::UserId::from(balance.user_id),
                    username: balance.username,
                    amount,
                    reason_code: "source_attribution_failed".to_string(),
                });
                continue;
            }
        };
        let payout_amounts =
            split_batch_payout_amount(amount).map_err(|reason| {
                ApiError::InvalidInput(format!("批量提现拆单失败: {reason}"))
            })?;

        candidates.push(BatchPayoutCandidate {
            user_id: balance.user_id,
            username: balance.username,
            amount,
            profile_updated_at: balance.profile_updated_at,
            sources,
            payout_amounts,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .amount
            .cmp(&left.amount)
            .then_with(|| left.username.cmp(&right.username))
    });
    excluded_users.sort_by(|left, right| {
        right
            .amount
            .cmp(&left.amount)
            .then_with(|| left.username.cmp(&right.username))
    });
    Ok(BatchPayoutPreviewData {
        candidates,
        excluded_users,
    })
}

fn split_batch_payout_amount(
    amount: Decimal,
) -> Result<Vec<Decimal>, &'static str> {
    if amount != truncate_money_to_cents(amount) {
        return Err("批量提现金额必须精确到分");
    }
    if amount < MIN_WITHDRAW_AMOUNT {
        return Err("批量提现金额低于最低限额");
    }

    let mut remaining = amount;
    let mut chunks = Vec::new();
    while remaining > MAX_WITHDRAW_AMOUNT {
        chunks.push(MAX_WITHDRAW_AMOUNT);
        remaining -= MAX_WITHDRAW_AMOUNT;
    }
    if remaining > Decimal::ZERO {
        chunks.push(remaining);
    }

    if chunks.len() > 1
        && chunks
            .last()
            .is_some_and(|last| *last < MIN_WITHDRAW_AMOUNT)
    {
        let last_index = chunks.len() - 1;
        let deficit = MIN_WITHDRAW_AMOUNT - chunks[last_index];
        chunks[last_index - 1] -= deficit;
        chunks[last_index] = MIN_WITHDRAW_AMOUNT;
    }

    if chunks.is_empty()
        || chunks.iter().any(|chunk| {
            *chunk < MIN_WITHDRAW_AMOUNT || *chunk > MAX_WITHDRAW_AMOUNT
        })
        || chunks.iter().copied().sum::<Decimal>() != amount
    {
        return Err("无法按云账户单笔限额完整拆分提现金额");
    }
    Ok(chunks)
}

fn attribute_batch_payout_sources(
    positive_earned: Decimal,
    available: Decimal,
    payout_amount: Decimal,
    values: &[BatchLedgerValue],
) -> Result<Vec<AdminBatchPayoutSource>, &'static str> {
    if payout_amount <= Decimal::ZERO {
        return Err("提现金额必须大于零");
    }

    let fifo_consumed = (positive_earned - available).max(Decimal::ZERO);
    let payout_end = fifo_consumed + payout_amount;
    let mut cursor = Decimal::ZERO;
    let mut grouped = HashMap::<Option<i64>, BatchSourceAccumulator>::new();

    for (order, value) in values.iter().enumerate() {
        if value.amount <= Decimal::ZERO {
            continue;
        }
        let value_start = cursor;
        let value_end = cursor + value.amount;
        cursor = value_end;
        let overlap = (value_end.min(payout_end)
            - value_start.max(fifo_consumed))
        .max(Decimal::ZERO);
        if overlap <= Decimal::ZERO {
            continue;
        }

        let entry = grouped.entry(value.mod_id).or_insert_with(|| {
            BatchSourceAccumulator {
                mod_id: value.mod_id,
                project_name: value.project_name.clone(),
                project_slug: value.project_slug.clone(),
                exact_amount: Decimal::ZERO,
                first_order: order,
            }
        });
        entry.exact_amount += overlap;
    }

    let exact_total = grouped
        .values()
        .map(|source| source.exact_amount)
        .sum::<Decimal>();
    if (exact_total - payout_amount).abs() >= Decimal::new(1, 2) {
        return Err("FIFO 来源合计与提现金额相差至少一分钱");
    }
    if grouped.is_empty() {
        return Err("没有可归因的收益来源");
    }

    struct RoundedSource {
        source: BatchSourceAccumulator,
        amount: Decimal,
        remainder: Decimal,
    }

    let mut rounded = grouped
        .into_values()
        .map(|source| {
            let amount = truncate_money_to_cents(source.exact_amount);
            let remainder = source.exact_amount - amount;
            RoundedSource {
                source,
                amount,
                remainder,
            }
        })
        .collect::<Vec<_>>();
    let base_total =
        rounded.iter().map(|source| source.amount).sum::<Decimal>();
    let mut missing = payout_amount - base_total;
    let cent = Decimal::new(1, 2);
    rounded.sort_by(|left, right| {
        right
            .remainder
            .cmp(&left.remainder)
            .then_with(|| {
                left.source.first_order.cmp(&right.source.first_order)
            })
            .then_with(|| left.source.mod_id.cmp(&right.source.mod_id))
    });
    for source in &mut rounded {
        if missing < cent {
            break;
        }
        source.amount += cent;
        missing -= cent;
    }
    if missing != Decimal::ZERO {
        return Err("无法将来源尾差分配到分");
    }

    let mut sources = rounded
        .into_iter()
        .filter(|source| source.amount > Decimal::ZERO)
        .map(|source| AdminBatchPayoutSource {
            project_id: source.source.mod_id.map(|id| {
                crate::models::ids::ProjectId::from(
                    crate::database::models::ProjectId(id),
                )
            }),
            slug: source.source.project_slug,
            title: source
                .source
                .project_name
                .unwrap_or_else(|| "已删除或无资源归属".to_string()),
            amount: source.amount,
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        right
            .amount
            .cmp(&left.amount)
            .then_with(|| left.title.cmp(&right.title))
    });

    if sources.iter().map(|source| source.amount).sum::<Decimal>()
        != payout_amount
    {
        return Err("来源分配结果与提现金额不一致");
    }
    Ok(sources)
}

async fn apply_batch_payout_item(
    pool: &PgPool,
    batch_id: &str,
    owner_token: &str,
    user_id: crate::database::models::UserId,
    amount: Decimal,
    expected_profile_updated_at: DateTime<Utc>,
    yzh_client: &YzhClient,
) -> Result<BatchItemApplyOutcome, ApiError> {
    let planned_orders = sqlx::query!(
        "
        SELECT chunk_index, amount, status, payout_id
        FROM admin_payout_batch_orders
        WHERE batch_id = $1 AND user_id = $2
        ORDER BY chunk_index
        ",
        batch_id,
        user_id.0,
    )
    .fetch_all(pool)
    .await?;
    if planned_orders.is_empty()
        || planned_orders
            .iter()
            .map(|order| order.amount)
            .sum::<Decimal>()
            != amount
        || planned_orders.iter().any(|order| {
            order.amount < MIN_WITHDRAW_AMOUNT
                || order.amount > MAX_WITHDRAW_AMOUNT
        })
    {
        return Err(ApiError::InvalidInput(
            "批量提现拆单快照不完整".to_string(),
        ));
    }
    if planned_orders
        .iter()
        .all(|order| order.status == "created" && order.payout_id.is_some())
    {
        return Ok(BatchItemApplyOutcome::Created(
            planned_orders
                .into_iter()
                .filter_map(|order| {
                    order.payout_id.map(crate::database::models::PayoutId)
                })
                .collect(),
        ));
    }
    if planned_orders
        .iter()
        .any(|order| order.status != "planned" || order.payout_id.is_some())
    {
        return Err(ApiError::InvalidInput(
            "批量提现拆单状态不一致".to_string(),
        ));
    }

    let profile = match YunzhanghuProfile::get(user_id, pool).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            mark_batch_payout_item(
                pool,
                batch_id,
                owner_token,
                user_id,
                "skipped",
                Some("missing_profile"),
            )
            .await?;
            return Ok(BatchItemApplyOutcome::Skipped);
        }
        Err(crate::database::models::DatabaseError::SchemaError(_)) => {
            mark_batch_payout_item(
                pool,
                batch_id,
                owner_token,
                user_id,
                "skipped",
                Some("profile_unreadable"),
            )
            .await?;
            return Ok(BatchItemApplyOutcome::Skipped);
        }
        Err(err) => return Err(err.into()),
    };
    if profile.updated_at != expected_profile_updated_at {
        mark_batch_payout_item(
            pool,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("profile_changed"),
        )
        .await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }
    if let Some(reason) = batch_profile_exclusion_reason(&profile) {
        mark_batch_payout_item(
            pool,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some(reason),
        )
        .await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let mut quotes = Vec::with_capacity(planned_orders.len());
    for order in &planned_orders {
        refresh_batch_payout_heartbeat(pool, batch_id, owner_token).await?;
        match quote_yunzhanghu_payout_with_client(
            order.amount,
            &profile,
            yzh_client,
        )
        .await
        {
            Ok(quote) => quotes.push(quote),
            Err(_) => {
                log::warn!(
                    "批量提现云账户试算失败 user_id={} chunk_index={}",
                    user_id.0,
                    order.chunk_index
                );
                mark_batch_payout_item(
                    pool,
                    batch_id,
                    owner_token,
                    user_id,
                    "failed",
                    Some("quote_failed"),
                )
                .await?;
                return Ok(BatchItemApplyOutcome::Failed);
            }
        }
    }
    refresh_batch_payout_heartbeat(pool, batch_id, owner_token).await?;

    let mut tx = pool.begin().await?;
    let item = sqlx::query!(
        "
        SELECT status, sources
        FROM admin_payout_batch_items
        WHERE batch_id = $1 AND user_id = $2
        FOR UPDATE
        ",
        batch_id,
        user_id.0,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::InvalidInput("批量提现用户快照不存在".to_string())
    })?;
    if item.status == "created" {
        let payout_ids = sqlx::query!(
            r#"
            SELECT payout_id AS "payout_id!"
            FROM admin_payout_batch_orders
            WHERE batch_id = $1
              AND user_id = $2
              AND status = 'created'
            ORDER BY chunk_index
            "#,
            batch_id,
            user_id.0,
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|order| crate::database::models::PayoutId(order.payout_id))
        .collect();
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Created(payout_ids));
    }
    if item.status == "skipped" {
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }
    let expected_sources =
        serde_json::from_value::<Vec<AdminBatchPayoutSource>>(item.sources)?;

    // 与管理员退回、用户注销保持 payouts -> users 的锁顺序，避免死锁。
    let _payout_locks = sqlx::query_scalar!(
        "
        SELECT id
        FROM payouts
        WHERE user_id = $1 AND status IN ('success', 'in-transit')
        FOR SHARE
        ",
        user_id.0,
    )
    .fetch_all(&mut *tx)
    .await?;

    let user_exists = sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user_id.0,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if user_exists.is_none() {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("user_missing"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let ban = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_bans
            WHERE user_id = $1
              AND ban_type IN ('global', 'resource')
              AND is_active = TRUE
              AND (expires_at IS NULL OR expires_at > NOW())
        ) AS "is_banned!"
        "#,
        user_id.0,
    )
    .fetch_one(&mut *tx)
    .await?;
    if ban.is_banned {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("banned"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let profile_lock = sqlx::query_scalar!(
        "
        SELECT user_id
        FROM user_yunzhanghu_profiles
        WHERE user_id = $1
        FOR UPDATE
        ",
        user_id.0,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if profile_lock.is_none() {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("missing_profile"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let locked_profile = match YunzhanghuProfile::get(user_id, &mut *tx).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            mark_batch_payout_item_in_tx(
                &mut tx,
                batch_id,
                owner_token,
                user_id,
                "skipped",
                Some("missing_profile"),
            )
            .await?;
            tx.commit().await?;
            return Ok(BatchItemApplyOutcome::Skipped);
        }
        Err(crate::database::models::DatabaseError::SchemaError(_)) => {
            mark_batch_payout_item_in_tx(
                &mut tx,
                batch_id,
                owner_token,
                user_id,
                "skipped",
                Some("profile_unreadable"),
            )
            .await?;
            tx.commit().await?;
            return Ok(BatchItemApplyOutcome::Skipped);
        }
        Err(err) => return Err(err.into()),
    };
    if locked_profile.updated_at != expected_profile_updated_at {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("profile_changed"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }
    if let Some(reason) = batch_profile_exclusion_reason(&locked_profile) {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some(reason),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let current_balance = get_batch_balance_in_tx(&mut tx, user_id).await?;
    if truncate_money_to_cents(current_balance.available) != amount {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("balance_changed"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let current_values = get_batch_ledger_in_tx(&mut tx, user_id).await?;
    let current_sources = match attribute_batch_payout_sources(
        current_balance.positive_earned,
        current_balance.available,
        amount,
        &current_values,
    ) {
        Ok(sources) => sources,
        Err(_) => {
            mark_batch_payout_item_in_tx(
                &mut tx,
                batch_id,
                owner_token,
                user_id,
                "skipped",
                Some("sources_changed"),
            )
            .await?;
            tx.commit().await?;
            return Ok(BatchItemApplyOutcome::Skipped);
        }
    };
    if !batch_payout_sources_match(&expected_sources, &current_sources) {
        mark_batch_payout_item_in_tx(
            &mut tx,
            batch_id,
            owner_token,
            user_id,
            "skipped",
            Some("sources_changed"),
        )
        .await?;
        tx.commit().await?;
        return Ok(BatchItemApplyOutcome::Skipped);
    }

    let alipay_account =
        locked_profile.alipay_account.clone().ok_or_else(|| {
            ApiError::InvalidInput("KYC 信息异常：缺少支付宝账号".to_string())
        })?;

    let locked_orders = sqlx::query!(
        "
        SELECT chunk_index, amount, status, payout_id
        FROM admin_payout_batch_orders
        WHERE batch_id = $1 AND user_id = $2
        ORDER BY chunk_index
        FOR UPDATE
        ",
        batch_id,
        user_id.0,
    )
    .fetch_all(&mut *tx)
    .await?;
    if locked_orders.len() != planned_orders.len()
        || locked_orders
            .iter()
            .zip(&planned_orders)
            .any(|(locked, planned)| {
                locked.chunk_index != planned.chunk_index
                    || locked.amount != planned.amount
                    || locked.status != "planned"
                    || locked.payout_id.is_some()
            })
    {
        return Err(ApiError::InvalidInput(
            "批量提现拆单快照已变化".to_string(),
        ));
    }

    let mut payout_ids = Vec::with_capacity(locked_orders.len());
    for (order, quote) in locked_orders.iter().zip(&quotes) {
        let payout_id = generate_payout_id(&mut tx).await?;
        let payout_item = crate::database::models::payout_item::Payout {
            id: payout_id,
            user_id,
            created: Utc::now(),
            status: PayoutStatus::InTransit,
            amount: order.amount,
            fee: Some(quote.user_fee),
            method: Some(PayoutMethodType::YunzhanghuAlipay),
            method_address: Some(alipay_account.clone()),
            platform_id: None,
            admin_reject_reason: None,
        };
        payout_item.insert(&mut tx).await?;
        let updated = sqlx::query!(
            "
            UPDATE admin_payout_batch_orders
            SET status = 'created', payout_id = $4
            WHERE batch_id = $1
              AND user_id = $2
              AND chunk_index = $3
              AND status = 'planned'
              AND payout_id IS NULL
            ",
            batch_id,
            user_id.0,
            order.chunk_index,
            payout_id.0,
        )
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(ApiError::InvalidInput(
                "批量提现子订单状态已变化".to_string(),
            ));
        }
        payout_ids.push(payout_id);
    }
    let item_updated = sqlx::query!(
        "
        UPDATE admin_payout_batch_items
        SET status = 'created', error_code = NULL
        WHERE batch_id = $1
          AND user_id = $2
          AND status IN ('pending', 'failed')
        ",
        batch_id,
        user_id.0,
    )
    .execute(&mut *tx)
    .await?;
    if item_updated.rows_affected() != 1 {
        return Err(ApiError::InvalidInput(
            "批量提现用户状态已变化".to_string(),
        ));
    }
    append_batch_payout_event_in_tx(
        &mut tx,
        batch_id,
        "item_created",
        owner_token,
        Some(user_id),
        serde_json::json!({
            "amount": amount.to_string(),
            "payout_ids": payout_ids.iter().map(|id| id.0).collect::<Vec<_>>(),
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(BatchItemApplyOutcome::Created(payout_ids))
}

async fn get_batch_balance_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: crate::database::models::UserId,
) -> Result<BatchBalanceSnapshot, sqlx::Error> {
    let balance = sqlx::query!(
        "
        SELECT
            COALESCE((
                SELECT SUM(amount)
                FROM payouts_values
                WHERE user_id = $1 AND date_available <= NOW()
            ), 0)::numeric AS \"earned!\",
            COALESCE((
                SELECT SUM(amount)
                FROM payouts_values
                WHERE user_id = $1
                  AND date_available <= NOW()
                  AND amount > 0
            ), 0)::numeric AS \"positive_earned!\",
            COALESCE((
                SELECT SUM(amount)
                FROM payouts
                WHERE user_id = $1 AND status IN ('success', 'in-transit')
            ), 0)::numeric AS \"withdrawn!\",
            COALESCE((
                SELECT SUM(
                    CASE
                        WHEN method = 'yunzhanghu_alipay' THEN 0
                        ELSE COALESCE(fee, 0)
                    END
                )
                FROM payouts
                WHERE user_id = $1 AND status IN ('success', 'in-transit')
            ), 0)::numeric AS \"old_channel_fees!\"
        ",
        user_id.0,
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(BatchBalanceSnapshot {
        available: balance.earned.round_dp(16)
            - balance.withdrawn.round_dp(16)
            - balance.old_channel_fees.round_dp(16),
        positive_earned: balance.positive_earned,
    })
}

async fn get_batch_ledger_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: crate::database::models::UserId,
) -> Result<Vec<BatchLedgerValue>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            pv.mod_id,
            pv.amount,
            m.name AS "project_name?",
            m.slug AS "project_slug?"
        FROM payouts_values pv
        LEFT JOIN mods m ON m.id = pv.mod_id
        WHERE pv.user_id = $1
          AND pv.date_available <= NOW()
          AND pv.amount > 0
        ORDER BY pv.date_available, pv.created, pv.id
        "#,
        user_id.0,
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BatchLedgerValue {
            mod_id: row.mod_id,
            amount: row.amount,
            project_name: row.project_name,
            project_slug: row.project_slug,
        })
        .collect())
}

fn batch_payout_sources_match(
    expected: &[AdminBatchPayoutSource],
    current: &[AdminBatchPayoutSource],
) -> bool {
    expected.len() == current.len()
        && expected.iter().all(|expected_source| {
            current.iter().any(|current_source| {
                current_source.project_id == expected_source.project_id
                    && current_source.amount == expected_source.amount
            })
        })
}

async fn mark_batch_payout_item(
    pool: &PgPool,
    batch_id: &str,
    owner_token: &str,
    user_id: crate::database::models::UserId,
    status: &str,
    error_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    mark_batch_payout_item_in_tx(
        &mut tx,
        batch_id,
        owner_token,
        user_id,
        status,
        error_code,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn mark_batch_payout_item_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    batch_id: &str,
    owner_token: &str,
    user_id: crate::database::models::UserId,
    status: &str,
    error_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    let event_type = batch_item_event_type(status).ok_or_else(|| {
        sqlx::Error::Protocol(format!(
            "unsupported audited batch item status: {status}"
        ))
    })?;
    let result = sqlx::query!(
        "
        UPDATE admin_payout_batch_items
        SET status = $3, error_code = $4
        WHERE batch_id = $1
          AND user_id = $2
          AND status IN ('pending', 'failed')
        ",
        batch_id,
        user_id.0,
        status,
        error_code,
    )
    .execute(&mut **tx)
    .await?;
    if result.rows_affected() == 1 {
        append_batch_payout_event_in_tx(
            tx,
            batch_id,
            event_type,
            owner_token,
            Some(user_id),
            serde_json::json!({
                "item_status": status,
                "reason_code": error_code,
            }),
        )
        .await?;
    }
    Ok(())
}

async fn build_batch_payout_apply_response(
    pool: &PgPool,
    batch_id: &str,
) -> Result<AdminBatchPayoutApplyResponse, ApiError> {
    let status = sqlx::query_scalar!(
        "SELECT status FROM admin_payout_batches WHERE id = $1",
        batch_id,
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("批量提现预览不存在".to_string()))?;
    let rows = sqlx::query!(
        "
        SELECT user_id, username, amount, status, error_code
        FROM admin_payout_batch_items
        WHERE batch_id = $1
        ORDER BY amount DESC, username ASC
        ",
        batch_id,
    )
    .fetch_all(pool)
    .await?;
    let order_rows = sqlx::query!(
        r#"
        SELECT user_id, payout_id AS "payout_id!"
        FROM admin_payout_batch_orders
        WHERE batch_id = $1 AND status = 'created'
        ORDER BY user_id, chunk_index
        "#,
        batch_id,
    )
    .fetch_all(pool)
    .await?;
    let mut payout_ids_by_user =
        HashMap::<i64, Vec<crate::models::ids::PayoutId>>::new();
    for order in order_rows {
        payout_ids_by_user.entry(order.user_id).or_default().push(
            crate::models::ids::PayoutId::from(
                crate::database::models::PayoutId(order.payout_id),
            ),
        );
    }

    let requested_count = rows.len();
    let created_count =
        rows.iter().filter(|row| row.status == "created").count();
    let skipped_count =
        rows.iter().filter(|row| row.status == "skipped").count();
    let failed_count = rows.iter().filter(|row| row.status == "failed").count();
    let total_previewed = rows.iter().map(|row| row.amount).sum();
    let total_created = rows
        .iter()
        .filter(|row| row.status == "created")
        .map(|row| row.amount)
        .sum();
    let items = rows
        .into_iter()
        .map(|row| {
            let payout_ids =
                payout_ids_by_user.remove(&row.user_id).unwrap_or_default();
            AdminBatchPayoutApplyItem {
                user_id: crate::models::ids::UserId::from(
                    crate::database::models::UserId(row.user_id),
                ),
                username: row.username,
                amount: row.amount,
                retryable: row.status == "failed",
                status: row.status,
                payout_id: payout_ids.first().copied(),
                payout_ids,
                reason_code: row.error_code,
            }
        })
        .collect();

    Ok(AdminBatchPayoutApplyResponse {
        batch_id: batch_id.to_string(),
        status,
        requested_count,
        created_count,
        skipped_count,
        failed_count,
        total_previewed,
        total_created,
        items,
    })
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

    let totals = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (
                WHERE $1::text IS NULL OR p.status = $1
            )::bigint AS "filtered_total!",
            COUNT(*) FILTER (
                WHERE p.status = 'in-transit'
                  AND p.platform_id IS NULL
            )::bigint AS "pending_order_count!",
            COALESCE(SUM(
                TRUNC(p.amount - COALESCE(p.fee, 0), 2)
            ) FILTER (
                WHERE p.status = 'in-transit'
                  AND p.platform_id IS NULL
            ), 0)::numeric AS "pending_transfer_amount!"
        FROM payouts p
        WHERE p.method = 'yunzhanghu_alipay'
        "#,
        status_filter,
    )
    .fetch_one(&**pool)
    .await?;
    let additional_service_fee =
        calculate_yunzhanghu_extra_service_fee(totals.pending_transfer_amount);

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
        total: totals.filtered_total,
        page,
        page_size,
        pending_transfer_summary: AdminPendingTransferSummary {
            order_count: totals.pending_order_count,
            transfer_amount: totals.pending_transfer_amount,
            additional_service_fee,
            total_with_service_fee: totals.pending_transfer_amount
                + additional_service_fee,
        },
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
    if profile.sign_nonce.is_some() {
        return Err(ApiError::InvalidInput(
            "用户签约或解约操作正在处理中，不能确认转账".to_string(),
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
    let pay_amount = calculate_yunzhanghu_submitted_pay_amount(
        payout.amount,
        payout.fee.unwrap_or_default(),
    );
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

pub(super) fn calculate_yunzhanghu_submitted_pay_amount(
    amount: Decimal,
    fee: Decimal,
) -> Decimal {
    truncate_money_to_cents(amount - fee)
}

fn calculate_yunzhanghu_extra_service_fee(amount: Decimal) -> Decimal {
    (amount * YUNZHANGHU_EXTRA_SERVICE_FEE_RATE)
        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

fn calculate_yunzhanghu_pay_amount(
    amount: Decimal,
) -> Result<Decimal, ApiError> {
    let fee = calculate_withdraw_service_fee(amount);
    let pay_amount = calculate_yunzhanghu_submitted_pay_amount(amount, fee);
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
    if amount > MAX_WITHDRAW_AMOUNT {
        return Err(ApiError::InvalidInput(format!(
            "单笔提现金额不能超过 ¥{}",
            MAX_WITHDRAW_AMOUNT
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

fn ensure_yunzhanghu_profile_ready(
    profile: &YunzhanghuProfile,
) -> Result<(), ApiError> {
    if profile.sign_status != YzhSignStatus::Signed {
        return Err(ApiError::InvalidInput(
            "您尚未完成签约，无法提现。请到「实名认证 & 收款账号」卡片完成签约。"
                .to_string(),
        ));
    }
    if profile.sign_nonce.is_some() {
        return Err(ApiError::InvalidInput(
            "签约或解约操作正在处理中，请完成操作后再提现。".to_string(),
        ));
    }
    Ok(())
}

async fn quote_yunzhanghu_payout(
    amount: Decimal,
    profile: &YunzhanghuProfile,
) -> Result<PayoutQuote, ApiError> {
    let client = YzhClient::new();
    quote_yunzhanghu_payout_with_client(amount, profile, &client).await
}

async fn quote_yunzhanghu_payout_with_client(
    amount: Decimal,
    profile: &YunzhanghuProfile,
    client: &YzhClient,
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

    let resp = yzh_api::calc_tax(
        client,
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
    ensure_yunzhanghu_profile_ready(&profile)?;

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
    ensure_yunzhanghu_profile_ready(&profile)?;
    if profile.real_name.is_none() {
        return Err(ApiError::InvalidInput(
            "KYC 信息异常：缺少真实姓名".to_string(),
        ));
    }
    if profile.alipay_account.is_none() {
        return Err(ApiError::InvalidInput(
            "KYC 信息异常：缺少支付宝账号".to_string(),
        ));
    }
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
    let user_lock = sqlx::query!(
        "SELECT balance FROM users WHERE id = $1 FOR UPDATE",
        user.id.0
    )
    .fetch_optional(&mut *transaction)
    .await?;
    if user_lock.is_none() {
        return Err(ApiError::InvalidInput("用户不存在".to_string()));
    }

    let profile_lock = sqlx::query_scalar!(
        "
        SELECT user_id
        FROM user_yunzhanghu_profiles
        WHERE user_id = $1
        FOR UPDATE
        ",
        user.id.0,
    )
    .fetch_optional(&mut *transaction)
    .await?;
    if profile_lock.is_none() {
        return Err(ApiError::InvalidInput(
            "请先完善实名信息与支付宝账号".to_string(),
        ));
    }
    let locked_profile = YunzhanghuProfile::get(user.id, &mut *transaction)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("请先完善实名信息与支付宝账号".to_string())
        })?;
    if locked_profile.updated_at != profile.updated_at {
        return Err(ApiError::InvalidInput(
            "实名资料或签约状态已变化，请重新试算".to_string(),
        ));
    }
    ensure_yunzhanghu_profile_ready(&locked_profile)?;
    let alipay_account =
        locked_profile.alipay_account.clone().ok_or_else(|| {
            ApiError::InvalidInput("KYC 信息异常：缺少支付宝账号".to_string())
        })?;

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
            // 云账户/支付宝实际限额仍以通道风控为准。
            max: MAX_WITHDRAW_AMOUNT,
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

    fn ledger_value(
        mod_id: Option<i64>,
        title: &str,
        amount: Decimal,
    ) -> BatchLedgerValue {
        BatchLedgerValue {
            mod_id,
            amount,
            project_name: Some(title.to_string()),
            project_slug: Some(title.to_lowercase()),
        }
    }

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
    fn yunzhanghu_extra_service_fee_is_six_point_eight_percent_rounded() {
        assert_eq!(
            calculate_yunzhanghu_extra_service_fee(Decimal::from(97)),
            Decimal::from_parts(660, 0, 0, false, 2)
        );
        assert_eq!(
            calculate_yunzhanghu_extra_service_fee(Decimal::from_parts(
                125, 0, 0, false, 2
            )),
            Decimal::from_parts(9, 0, 0, false, 2)
        );
    }

    #[test]
    fn yunzhanghu_pay_amount_deducts_fee_from_requested_amount() {
        assert_eq!(
            calculate_yunzhanghu_pay_amount(Decimal::from(100)).unwrap(),
            Decimal::from(97)
        );
    }

    #[test]
    fn yunzhanghu_submitted_pay_matches_production_orders() {
        assert_eq!(
            calculate_yunzhanghu_submitted_pay_amount(
                Decimal::new(109_091, 2),
                Decimal::new(3_272, 2),
            ),
            Decimal::new(105_819, 2)
        );
        assert_eq!(
            calculate_yunzhanghu_submitted_pay_amount(
                Decimal::new(61_476, 2),
                Decimal::new(1_844, 2),
            ),
            Decimal::new(59_632, 2)
        );
    }

    #[test]
    fn yunzhanghu_submitted_pay_keeps_legacy_orders_without_fee() {
        assert_eq!(
            calculate_yunzhanghu_submitted_pay_amount(
                Decimal::new(10_091, 2),
                Decimal::ZERO,
            ),
            Decimal::new(10_091, 2)
        );
    }

    #[test]
    fn batch_payout_admin_username_must_match_exactly() {
        assert!(ensure_batch_payout_admin("BBSMC").is_ok());
        assert!(ensure_batch_payout_admin("bbsmc").is_err());
        assert!(ensure_batch_payout_admin("OtherAdmin").is_err());
    }

    #[test]
    fn batch_audit_event_types_cover_item_and_batch_terminal_states() {
        assert_eq!(batch_item_event_type("skipped"), Some("item_skipped"));
        assert_eq!(batch_item_event_type("failed"), Some("item_failed"));
        assert_eq!(batch_item_event_type("pending"), None);
        assert_eq!(batch_item_event_type("created"), None);

        assert_eq!(
            batch_terminal_state(3, 3),
            ("completed", "batch_completed")
        );
        assert_eq!(batch_terminal_state(3, 2), ("partial", "batch_partial"));
        assert_eq!(
            batch_terminal_state(0, 0),
            ("completed", "batch_completed")
        );
    }

    #[test]
    fn batch_audit_processing_kind_distinguishes_retry_and_takeover() {
        assert_eq!(batch_processing_kind("previewed", false), "initial");
        assert_eq!(batch_processing_kind("partial", false), "retry");
        assert_eq!(batch_processing_kind("processing", true), "takeover");
    }

    #[test]
    fn ordinary_payout_rejects_amount_over_single_order_limit() {
        assert!(ensure_supported_payout_amount(MAX_WITHDRAW_AMOUNT).is_ok());
        assert!(
            ensure_supported_payout_amount(
                MAX_WITHDRAW_AMOUNT + Decimal::new(1, 2)
            )
            .is_err()
        );
    }

    #[test]
    fn batch_amount_at_limit_stays_single_order() {
        assert_eq!(
            split_batch_payout_amount(MAX_WITHDRAW_AMOUNT).unwrap(),
            vec![MAX_WITHDRAW_AMOUNT]
        );
    }

    #[test]
    fn batch_amount_over_limit_is_fully_split() {
        let amount = Decimal::from(100_002);
        let chunks = split_batch_payout_amount(amount).unwrap();

        assert_eq!(
            chunks,
            vec![
                Decimal::from(50_000),
                Decimal::from(49_997),
                Decimal::from(5)
            ]
        );
        assert_eq!(chunks.iter().copied().sum::<Decimal>(), amount);
        assert!(chunks.iter().all(|chunk| {
            *chunk >= MIN_WITHDRAW_AMOUNT && *chunk <= MAX_WITHDRAW_AMOUNT
        }));
    }

    #[test]
    fn batch_split_moves_subminimum_remainder_into_final_order() {
        let amount = MAX_WITHDRAW_AMOUNT + Decimal::new(1, 2);
        assert_eq!(
            split_batch_payout_amount(amount).unwrap(),
            vec![Decimal::new(4_999_501, 2), Decimal::from(5)]
        );
    }

    #[test]
    fn batch_sources_consume_oldest_earnings_first() {
        let values = vec![
            ledger_value(Some(1), "A", Decimal::from(10)),
            ledger_value(Some(2), "B", Decimal::from(20)),
        ];

        let sources = attribute_batch_payout_sources(
            Decimal::from(30),
            Decimal::from(20),
            Decimal::from(20),
            &values,
        )
        .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "B");
        assert_eq!(sources[0].amount, Decimal::from(20));
    }

    #[test]
    fn batch_sources_allocate_cent_remainder_deterministically() {
        let values = vec![
            ledger_value(Some(1), "A", Decimal::new(2335, 3)),
            ledger_value(Some(2), "B", Decimal::new(2335, 3)),
            ledger_value(Some(3), "C", Decimal::new(2335, 3)),
        ];

        let sources = attribute_batch_payout_sources(
            Decimal::new(7005, 3),
            Decimal::new(7005, 3),
            Decimal::from(7),
            &values,
        )
        .unwrap();

        assert_eq!(
            sources.iter().map(|source| source.amount).sum::<Decimal>(),
            Decimal::from(7)
        );
        assert_eq!(sources[0].title, "A");
        assert_eq!(sources[0].amount, Decimal::new(234, 2));
        assert_eq!(sources[1].amount, Decimal::new(233, 2));
        assert_eq!(sources[2].amount, Decimal::new(233, 2));
    }

    #[test]
    fn batch_sources_absorb_sub_cent_balance_rounding() {
        let exact = Decimal::from_str_exact("4.99999999999999996").unwrap();
        let values = vec![ledger_value(None, "无资源", exact)];

        let sources = attribute_batch_payout_sources(
            exact,
            exact.round_dp(16),
            Decimal::from(5),
            &values,
        )
        .unwrap();

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].amount, Decimal::from(5));
    }

    #[test]
    fn batch_source_snapshot_matches_ids_and_amounts() {
        let expected = vec![AdminBatchPayoutSource {
            project_id: None,
            slug: None,
            title: "预览标题".to_string(),
            amount: Decimal::from(5),
        }];
        let mut current = expected.clone();
        current[0].title = "改名后的标题".to_string();
        assert!(batch_payout_sources_match(&expected, &current));

        current[0].amount = Decimal::new(499, 2);
        assert!(!batch_payout_sources_match(&expected, &current));
    }
}
