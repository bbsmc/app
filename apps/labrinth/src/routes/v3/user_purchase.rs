//! 用户购买记录路由
//!
//! 提供用户查看已购买项目的功能。

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

use crate::auth::get_user_from_headers;
use crate::database::models::UserId as DBUserId;
use crate::database::models::ids::ProjectId as DbProjectId;
use crate::database::models::project_item::Project;
use crate::database::models::user_purchase_item::{
    PurchaseStatus, UserPurchase,
};
use crate::database::redis::RedisPool;
use crate::models::ids::ProjectId;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("user/purchases")
            .route("", web::get().to(list_user_purchases))
            .route("/{project_id}", web::get().to(check_purchase)),
    );
}

// ==================== 响应结构 ====================

/// 用户购买列表响应
#[derive(Debug, Clone, Serialize)]
pub struct UserPurchasesResponse {
    pub purchases: Vec<PurchaseSummary>,
}

/// 购买记录摘要
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseSummary {
    pub project_id: String,
    pub project_title: Option<String>,
    pub project_slug: Option<String>,
    pub project_description: Option<String>,
    pub project_type: Option<String>,
    pub icon_url: Option<String>,
    pub amount: Decimal,
    pub purchased_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub status: String,
    pub is_active: bool,
}

/// 购买检查响应
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseCheckResponse {
    pub purchased: bool,
    pub is_active: bool,
    pub purchase: Option<PurchaseDetail>,
}

/// 购买详情
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseDetail {
    pub project_id: String,
    pub amount: Decimal,
    pub purchased_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub status: String,
}

// ==================== 路由处理 ====================

/// 获取用户购买列表
///
/// GET /v3/user/purchases
///
/// 返回当前用户购买的所有项目
pub async fn list_user_purchases(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 获取用户的所有购买记录
    let purchases =
        UserPurchase::get_user_purchases(db_user_id, &**pool).await?;

    // 获取所有相关项目的信息
    let project_ids: Vec<DbProjectId> =
        purchases.iter().map(|p| p.project_id).collect();
    let projects = Project::get_many_ids(&project_ids, &**pool, &redis).await?;

    // 构建项目 ID 到信息的映射
    struct ProjectInfo {
        name: String,
        slug: Option<String>,
        description: String,
        project_type: String,
        icon_url: Option<String>,
    }

    let project_map: std::collections::HashMap<i64, ProjectInfo> = projects
        .into_iter()
        .map(|p| {
            (
                p.inner.id.0,
                ProjectInfo {
                    name: p.inner.name,
                    slug: p.inner.slug,
                    description: p.inner.description,
                    project_type: p
                        .project_types
                        .first()
                        .cloned()
                        .unwrap_or_default(),
                    icon_url: p.inner.icon_url,
                },
            )
        })
        .collect();

    let now = Utc::now();

    // 构建响应
    let purchase_summaries: Vec<PurchaseSummary> = purchases
        .into_iter()
        .map(|purchase| {
            let project_info = project_map.get(&purchase.project_id.0);
            let is_active = purchase.status == PurchaseStatus::Active
                && purchase.expires_at.is_none_or(|exp| exp > now);

            PurchaseSummary {
                project_id: ProjectId::from(purchase.project_id).to_string(),
                project_title: project_info.map(|p| p.name.clone()),
                project_slug: project_info.and_then(|p| p.slug.clone()),
                project_description: project_info
                    .map(|p| p.description.clone()),
                project_type: project_info.map(|p| p.project_type.clone()),
                icon_url: project_info.and_then(|p| p.icon_url.clone()),
                amount: purchase.amount,
                purchased_at: purchase.purchased_at,
                expires_at: purchase.expires_at,
                status: purchase.status.as_str().to_string(),
                is_active,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(UserPurchasesResponse {
        purchases: purchase_summaries,
    }))
}

/// 检查用户是否购买了某个项目
///
/// GET /v3/user/purchases/{project_id}
///
/// 返回用户对指定项目的购买状态
pub async fn check_purchase(
    req: HttpRequest,
    path: web::Path<String>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let project_id_str = path.into_inner();

    // 验证用户登录
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 获取项目信息
    let project = Project::get(&project_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    let project_id = project.inner.id;

    // 获取购买记录
    let purchase = UserPurchase::get(db_user_id, project_id, &**pool).await?;

    let now = Utc::now();

    let response = match purchase {
        Some(p) => {
            let is_active = p.status == PurchaseStatus::Active
                && p.expires_at.is_none_or(|exp| exp > now);

            PurchaseCheckResponse {
                purchased: true,
                is_active,
                purchase: Some(PurchaseDetail {
                    project_id: ProjectId::from(p.project_id).to_string(),
                    amount: p.amount,
                    purchased_at: p.purchased_at,
                    expires_at: p.expires_at,
                    status: p.status.as_str().to_string(),
                }),
            }
        }
        None => PurchaseCheckResponse {
            purchased: false,
            is_active: false,
            purchase: None,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}
