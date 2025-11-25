//! V2 封禁管理路由 - 调用 V3 实现

use crate::database::redis::RedisPool;
use crate::models::v3::bans::{
    AppealsQueryParams, BansQueryParams, BatchBansQuery, CreateAppealRequest,
    CreateBanRequest, ReviewAppealRequest, RevokeBanRequest, UpdateBanRequest,
};
use crate::queue::session::AuthQueue;
use crate::routes::{ApiError, v2_reroute, v3};
use actix_web::{HttpRequest, HttpResponse, web};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("bans")
            // 管理员路由
            .route("", web::get().to(list_bans))
            .route("batch", web::get().to(get_bans_batch))
            .route("user/{user_id}", web::post().to(create_ban))
            .route("user/{user_id}", web::get().to(get_user_bans))
            // 申诉管理（必须在 {ban_id} 之前，否则 appeals 会被当作 ban_id）
            .route("appeals", web::get().to(list_appeals))
            .route("appeals/{appeal_id}", web::patch().to(review_appeal))
            // 单个封禁操作
            .route("{ban_id}", web::get().to(get_ban))
            .route("{ban_id}", web::patch().to(update_ban))
            .route("{ban_id}", web::delete().to(revoke_ban))
            .route("{ban_id}/history", web::get().to(get_ban_history)),
    );
}

/// 获取封禁列表
pub async fn list_bans(
    req: HttpRequest,
    query: web::Query<BansQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::list_bans(req, query, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 批量获取封禁详情
pub async fn get_bans_batch(
    req: HttpRequest,
    query: web::Query<BatchBansQuery>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_bans_batch(req, query, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 创建封禁
pub async fn create_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::create_ban(req, info, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取用户封禁记录
pub async fn get_user_bans(
    req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<BansQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_user_bans(req, info, query, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取封禁详情
pub async fn get_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_ban(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 修改封禁
pub async fn update_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<UpdateBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::update_ban(req, info, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 解除封禁
pub async fn revoke_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<RevokeBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::revoke_ban(req, info, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取封禁历史
pub async fn get_ban_history(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_ban_history(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取申诉列表
pub async fn list_appeals(
    req: HttpRequest,
    query: web::Query<AppealsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::list_appeals(req, query, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 审核申诉
pub async fn review_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<ReviewAppealRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::review_appeal(req, info, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

// ==================== 用户路由（在 users.rs 中添加） ====================

/// 获取自己的封禁状态
pub async fn get_my_bans(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_my_bans(req, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 创建申诉
pub async fn create_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateAppealRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::create_appeal(req, info, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取自己的申诉详情
pub async fn get_my_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::bans::get_my_appeal(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}
