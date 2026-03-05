//! 高级创作者审核路由
//!
//! 提供管理员审核高级创作者申请的功能。

use super::ApiError;
use crate::auth::check_is_admin_from_headers;
use crate::database::models::UserId as DBUserId;
use crate::database::models::creator_application_item::{
    ApplicationStatus, CreatorApplication,
};
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::redis::RedisPool;
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("creator/applications", web::get().to(list_applications));
    cfg.route(
        "creator/applications/{id}/approve",
        web::post().to(approve_application),
    );
    cfg.route(
        "creator/applications/{id}/reject",
        web::post().to(reject_application),
    );
}

// ==================== 请求/响应结构 ====================

/// 申请列表查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationsQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// 状态筛选：pending, approved, rejected（不传则返回所有）
    pub status: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// 管理员审核请求
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewApplicationRequest {
    /// 审核备注
    pub review_note: Option<String>,
}

/// 分页申请列表响应
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedApplications {
    pub applications: Vec<AdminApplicationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// 管理员查看的申请响应（包含用户信息）
#[derive(Debug, Clone, Serialize)]
pub struct AdminApplicationResponse {
    pub id: i64,
    pub user_id: String,
    pub username: String,
    pub status: String,
    pub real_name: String,
    pub contact_info: String,
    pub id_card_number: Option<String>,
    pub portfolio_links: Option<String>,
    pub application_reason: Option<String>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// 关联的对话线程 ID
    pub thread_id: Option<String>,
}

// ==================== 管理员路由 ====================

/// 获取申请列表
///
/// GET /_internal/creator/applications
pub async fn list_applications(
    req: HttpRequest,
    query: web::Query<ApplicationsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    // 解析状态筛选
    let status_filter = query.status.as_deref().and_then(|s| match s {
        "pending" => Some(ApplicationStatus::Pending),
        "approved" => Some(ApplicationStatus::Approved),
        "rejected" => Some(ApplicationStatus::Rejected),
        _ => None,
    });

    let applications =
        CreatorApplication::get_list(status_filter, limit, offset, &**pool)
            .await?;
    let total =
        CreatorApplication::count_by_status(status_filter, &**pool).await?;

    // 获取用户信息
    let user_ids: Vec<DBUserId> =
        applications.iter().map(|a| a.user_id).collect();
    let users =
        crate::database::models::User::get_many_ids(&user_ids, &**pool, &redis)
            .await?;

    let responses: Vec<AdminApplicationResponse> = applications
        .into_iter()
        .map(|app| {
            let username = users
                .iter()
                .find(|u| u.id == app.user_id)
                .map(|u| u.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            AdminApplicationResponse {
                id: app.id,
                user_id: crate::models::ids::UserId(app.user_id.0 as u64)
                    .to_string(),
                username,
                status: app.status.to_string(),
                real_name: app.real_name,
                contact_info: app.contact_info,
                id_card_number: app.id_card_number,
                portfolio_links: app.portfolio_links,
                application_reason: app.application_reason,
                review_note: app.review_note,
                created_at: app.created_at,
                reviewed_at: app.reviewed_at,
                thread_id: app.thread_id.map(|id| {
                    crate::models::ids::ThreadId::from(id).to_string()
                }),
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(PaginatedApplications {
        applications: responses,
        total,
        limit,
        offset,
    }))
}

/// 批准申请
///
/// POST /_internal/creator/applications/{id}/approve
pub async fn approve_application(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<ReviewApplicationRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let application_id: i64 = info
        .0
        .parse()
        .map_err(|_| ApiError::InvalidInput("无效的申请 ID".to_string()))?;

    // 检查申请是否存在且为 pending 状态
    let application = CreatorApplication::get_by_id(application_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("申请不存在".to_string()))?;

    if application.status != ApplicationStatus::Pending {
        return Err(ApiError::InvalidInput("该申请已被处理".to_string()));
    }

    // 提前获取用户名，用于后续清除缓存（避免事务后额外查询）
    let username = crate::database::models::User::get_id(
        application.user_id,
        &**pool,
        &redis,
    )
    .await?
    .map(|u| u.username);

    let reviewer_id = DBUserId(user.id.0 as i64);
    let review_note = body.review_note.as_deref();

    let mut transaction = pool.begin().await?;

    CreatorApplication::approve(
        application_id,
        reviewer_id,
        review_note,
        &mut transaction,
    )
    .await?;

    // 发送通知给申请用户
    NotificationBuilder {
        body: NotificationBody::CreatorApplicationApproved { application_id },
    }
    .insert(application.user_id, &mut transaction, &redis)
    .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除用户缓存（包括用户名缓存）
    crate::database::models::User::clear_caches(
        &[(application.user_id, username)],
        &redis,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "申请已批准"
    })))
}

/// 拒绝申请
///
/// POST /_internal/creator/applications/{id}/reject
pub async fn reject_application(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<ReviewApplicationRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let application_id: i64 = info
        .0
        .parse()
        .map_err(|_| ApiError::InvalidInput("无效的申请 ID".to_string()))?;

    // 检查申请是否存在且为 pending 状态
    let application = CreatorApplication::get_by_id(application_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("申请不存在".to_string()))?;

    if application.status != ApplicationStatus::Pending {
        return Err(ApiError::InvalidInput("该申请已被处理".to_string()));
    }

    let reviewer_id = DBUserId(user.id.0 as i64);
    let review_note = body.review_note.as_deref();

    let mut transaction = pool.begin().await?;

    CreatorApplication::reject(
        application_id,
        reviewer_id,
        review_note,
        &mut transaction,
    )
    .await?;

    // 发送通知给申请用户
    NotificationBuilder {
        body: NotificationBody::CreatorApplicationRejected {
            application_id,
            reason: review_note.map(|s| s.to_string()),
        },
    }
    .insert(application.user_id, &mut transaction, &redis)
    .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "申请已拒绝"
    })))
}
