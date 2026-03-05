//! 高级创作者 V2 路由
//!
//! 提供 V2 兼容的高级创作者申请和审核路由。
//! 实际逻辑调用 internal 或 v3 的实现。

use super::ApiError;
use crate::database::redis::RedisPool;
use crate::queue::session::AuthQueue;
use crate::routes::{internal, v2_reroute, v3};
use actix_web::{HttpRequest, HttpResponse, web};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("creator")
            // 用户路由
            .route("apply", web::post().to(apply_creator))
            .route("application", web::get().to(get_my_application))
            // 管理员路由
            .route("applications", web::get().to(list_applications))
            .route(
                "applications/{id}/approve",
                web::post().to(approve_application),
            )
            .route(
                "applications/{id}/reject",
                web::post().to(reject_application),
            ),
    );
}

// ==================== 用户路由 ====================

/// 提交高级创作者申请
///
/// POST /v2/creator/apply
pub async fn apply_creator(
    req: HttpRequest,
    body: web::Json<v3::creator::ApplyCreatorRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::creator::apply_creator(req, body, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 获取当前用户的申请状态
///
/// GET /v2/creator/application
pub async fn get_my_application(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::creator::get_my_application(req, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

// ==================== 管理员路由 ====================

/// 获取待审核申请列表
///
/// GET /v2/creator/applications
pub async fn list_applications(
    req: HttpRequest,
    query: web::Query<internal::creator::ApplicationsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    internal::creator::list_applications(req, query, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

/// 批准申请
///
/// POST /v2/creator/applications/{id}/approve
pub async fn approve_application(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<internal::creator::ReviewApplicationRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    internal::creator::approve_application(
        req,
        info,
        body,
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

/// 拒绝申请
///
/// POST /v2/creator/applications/{id}/reject
pub async fn reject_application(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<internal::creator::ReviewApplicationRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    internal::creator::reject_application(
        req,
        info,
        body,
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}
