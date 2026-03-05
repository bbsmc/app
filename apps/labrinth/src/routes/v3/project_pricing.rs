//! 项目定价 API
//!
//! 提供付费资源的定价设置和查询功能。
//! 只有高级创作者且项目 is_paid=true 时才能设置定价。
//!
//! 权限要求：
//! - GET: 公开访问（定价信息是公开的，用户需要知道价格才能购买）
//! - POST/PATCH: 需要 PROJECT_WRITE scope、项目成员权限 EDIT_DETAILS、且是高级创作者

use super::ApiError;
use crate::auth::get_user_from_headers;
use crate::database::models::UserId as DBUserId;
use crate::database::models::user_purchase_item::UserPurchase;
use crate::database::models::{self, ProjectPricing};
use crate::database::redis::RedisPool;
use crate::models::ids::UserId;
use crate::models::pats::Scopes;
use crate::models::teams::ProjectPermissions;
use crate::queue::session::AuthQueue;
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// 定价请求数据
#[derive(Deserialize)]
pub struct PricingRequest {
    /// 价格（单位：元，整数 1-1000）
    pub price: i32,
    /// 授权有效期天数（None 表示永久）
    pub validity_days: Option<i32>,
}

/// 定价响应数据
#[derive(Serialize)]
pub struct PricingResponse {
    pub project_id: String,
    pub price: i32,
    pub validity_days: Option<i32>,
    pub is_permanent: bool,
}

/// 购买用户信息
#[derive(Serialize)]
pub struct PurchaserInfo {
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub amount: Decimal,
    pub purchased_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: String,
    pub is_active: bool,
}

/// 购买用户列表响应
#[derive(Serialize)]
pub struct PurchasersResponse {
    pub project_id: String,
    pub total: usize,
    pub purchasers: Vec<PurchaserInfo>,
}

/// 获取项目定价信息
///
/// GET /v3/project/{id}/pricing
pub async fn get_pricing(
    _req: HttpRequest,
    info: web::Path<String>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    _session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let project_id_str = info.into_inner();

    // 获取项目信息
    let project = models::Project::get(&project_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    // 检查项目是否为付费资源
    if !project.inner.is_paid {
        return Err(ApiError::InvalidInput("该项目不是付费资源".to_string()));
    }

    // 获取定价信息
    let pricing = ProjectPricing::get(project.inner.id, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("定价信息不存在".to_string()))?;

    // 将 Decimal 安全转换为 i32
    let price_i32 = decimal_to_i32(pricing.price)
        .map_err(|_| ApiError::InvalidInput("价格数据异常".to_string()))?;

    Ok(HttpResponse::Ok().json(PricingResponse {
        project_id: project_id_str,
        price: price_i32,
        validity_days: pricing.validity_days,
        is_permanent: pricing.validity_days.is_none(),
    }))
}

/// 设置项目定价（首次设置）
///
/// POST /v3/project/{id}/pricing
pub async fn set_pricing(
    req: HttpRequest,
    info: web::Path<String>,
    body: web::Json<PricingRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户身份
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    let project_id_str = info.into_inner();

    // 获取项目信息
    let project = models::Project::get(&project_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    // 转换用户 ID 类型
    let db_user_id = DBUserId(current_user.id.0 as i64);

    // 验证项目权限（必须是项目成员且有编辑权限）
    let team_member = models::TeamMember::get_from_user_id_project(
        project.inner.id,
        db_user_id,
        false,
        &**pool,
    )
    .await?;

    let member = team_member.ok_or_else(|| {
        ApiError::CustomAuthentication("您没有权限修改此项目".to_string())
    })?;

    // 检查成员是否已接受邀请且有 EDIT_DETAILS 权限
    if !member.accepted {
        return Err(ApiError::CustomAuthentication(
            "您的成员邀请尚未接受".to_string(),
        ));
    }

    if !member
        .permissions
        .contains(ProjectPermissions::EDIT_DETAILS)
    {
        return Err(ApiError::CustomAuthentication(
            "您没有编辑此项目的权限".to_string(),
        ));
    }

    // 验证用户是否为高级创作者
    let db_user = models::User::get_id(db_user_id, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("用户不存在".to_string()))?;

    if !db_user.is_premium_creator {
        return Err(ApiError::CustomAuthentication(
            "只有高级创作者才能设置定价".to_string(),
        ));
    }

    // 检查项目是否为付费资源
    if !project.inner.is_paid {
        return Err(ApiError::InvalidInput(
            "只有付费资源才能设置定价。请在创建项目时设置为付费资源。"
                .to_string(),
        ));
    }

    // 验证价格（整数 1-1000）
    validate_price(body.price)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;

    // 验证有效期
    validate_validity_days(body.validity_days)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;

    // 开启事务
    let mut transaction = pool.begin().await?;

    // 将 i32 转换为 Decimal
    let price_decimal = Decimal::from(body.price);

    // 设置定价
    ProjectPricing::upsert(
        project.inner.id,
        price_decimal,
        body.validity_days,
        &mut transaction,
    )
    .await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(PricingResponse {
        project_id: project_id_str,
        price: body.price,
        validity_days: body.validity_days,
        is_permanent: body.validity_days.is_none(),
    }))
}

/// 更新项目定价
///
/// PATCH /v3/project/{id}/pricing
pub async fn update_pricing(
    req: HttpRequest,
    info: web::Path<String>,
    body: web::Json<PricingRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 更新定价与设置定价逻辑相同（使用 upsert）
    set_pricing(req, info, body, pool, redis, session_queue).await
}

/// 获取项目的购买用户列表
///
/// GET /v3/project/{id}/pricing/purchasers
///
/// 权限要求：项目成员（EDIT_DETAILS 权限）
pub async fn get_purchasers(
    req: HttpRequest,
    info: web::Path<String>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户身份
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    let project_id_str = info.into_inner();

    // 获取项目信息
    let project = models::Project::get(&project_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    // 转换用户 ID 类型
    let db_user_id = DBUserId(current_user.id.0 as i64);

    // 验证项目权限（必须是项目成员且有编辑权限）
    let team_member = models::TeamMember::get_from_user_id_project(
        project.inner.id,
        db_user_id,
        false,
        &**pool,
    )
    .await?;

    let member = team_member.ok_or_else(|| {
        ApiError::CustomAuthentication(
            "您没有权限查看此项目的购买用户".to_string(),
        )
    })?;

    if !member.accepted {
        return Err(ApiError::CustomAuthentication(
            "您的成员邀请尚未接受".to_string(),
        ));
    }

    if !member
        .permissions
        .contains(ProjectPermissions::EDIT_DETAILS)
    {
        return Err(ApiError::CustomAuthentication(
            "您没有查看购买用户的权限".to_string(),
        ));
    }

    // 检查项目是否为付费资源
    if !project.inner.is_paid {
        return Err(ApiError::InvalidInput("该项目不是付费资源".to_string()));
    }

    // 获取购买用户列表
    let purchases =
        UserPurchase::get_project_purchasers(project.inner.id, &**pool).await?;

    // 获取所有购买用户的详细信息
    let user_ids: Vec<DBUserId> = purchases.iter().map(|p| p.user_id).collect();
    let users = models::User::get_many_ids(&user_ids, &**pool, &redis).await?;

    // 构建用户 ID 到信息的映射
    let user_map: std::collections::HashMap<i64, &models::User> =
        users.iter().map(|u| (u.id.0, u)).collect();

    let now = Utc::now();

    // 构建响应
    let purchaser_infos: Vec<PurchaserInfo> = purchases
        .into_iter()
        .map(|purchase| {
            let user = user_map.get(&purchase.user_id.0);
            let is_active = purchase.status
                == crate::database::models::user_purchase_item::PurchaseStatus::Active
                && purchase.expires_at.is_none_or(|exp| exp > now);

            PurchaserInfo {
                user_id: UserId::from(purchase.user_id).to_string(),
                username: user
                    .map(|u| u.username.clone())
                    .unwrap_or_else(|| "未知用户".to_string()),
                avatar_url: user.and_then(|u| u.avatar_url.clone()),
                amount: purchase.amount,
                purchased_at: purchase.purchased_at,
                expires_at: purchase.expires_at,
                status: purchase.status.as_str().to_string(),
                is_active,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(PurchasersResponse {
        project_id: project_id_str,
        total: purchaser_infos.len(),
        purchasers: purchaser_infos,
    }))
}

/// 撤销用户的购买授权
///
/// DELETE /v3/project/{id}/pricing/purchasers/{user_id}
///
/// 权限要求：项目成员（EDIT_DETAILS 权限）
pub async fn revoke_purchase(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户身份
    let current_user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_WRITE]),
    )
    .await?
    .1;

    let (project_id_str, target_user_id_str) = info.into_inner();

    // 获取项目信息
    let project = models::Project::get(&project_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("项目不存在".to_string()))?;

    // 转换用户 ID 类型
    let db_user_id = DBUserId(current_user.id.0 as i64);

    // 验证项目权限（必须是项目成员且有编辑权限）
    let team_member = models::TeamMember::get_from_user_id_project(
        project.inner.id,
        db_user_id,
        false,
        &**pool,
    )
    .await?;

    let member = team_member.ok_or_else(|| {
        ApiError::CustomAuthentication(
            "您没有权限管理此项目的购买用户".to_string(),
        )
    })?;

    if !member.accepted {
        return Err(ApiError::CustomAuthentication(
            "您的成员邀请尚未接受".to_string(),
        ));
    }

    if !member
        .permissions
        .contains(ProjectPermissions::EDIT_DETAILS)
    {
        return Err(ApiError::CustomAuthentication(
            "您没有撤销授权的权限".to_string(),
        ));
    }

    // 检查项目是否为付费资源
    if !project.inner.is_paid {
        return Err(ApiError::InvalidInput("该项目不是付费资源".to_string()));
    }

    // 获取目标用户
    let target_user = models::User::get(&target_user_id_str, &**pool, &redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("用户不存在".to_string()))?;
    let target_db_user_id = target_user.id;

    // 开启事务
    let mut transaction = pool.begin().await?;

    // 撤销授权
    let deleted = UserPurchase::revoke(
        target_db_user_id,
        project.inner.id,
        &mut transaction,
    )
    .await?;

    if !deleted {
        return Err(ApiError::InvalidInput("该用户没有购买此项目".to_string()));
    }

    // 清除用户购买缓存
    UserPurchase::clear_user_purchase_cache(target_db_user_id, &redis).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "授权已撤销"
    })))
}

/// 验证价格是否在有效范围内 (1-1000)
pub fn validate_price(price: i32) -> Result<(), &'static str> {
    if price < 1 {
        return Err("价格不能小于 1");
    }
    if price > 1000 {
        return Err("价格不能大于 1000");
    }
    Ok(())
}

/// 验证授权有效期（1-3650 天，即最多 10 年）
pub fn validate_validity_days(days: Option<i32>) -> Result<(), &'static str> {
    if let Some(d) = days {
        if d <= 0 {
            return Err("授权有效期必须大于 0 天");
        }
        if d > 3650 {
            return Err("授权有效期不能超过 3650 天（10年）");
        }
    }
    Ok(())
}

/// 将 Decimal 安全转换为 i32
pub fn decimal_to_i32(value: Decimal) -> Result<i32, &'static str> {
    value.try_into().map_err(|_| "Decimal 转换 i32 失败")
}

/// 注册定价路由
/// 注意：实际路由在 projects.rs 中的 web::scope("{id}") 内注册
/// 这里保留空配置以保持模块结构一致
pub fn config(_cfg: &mut web::ServiceConfig) {
    // 路由已在 projects.rs 中注册：
    // - GET    project/{id}/pricing
    // - POST   project/{id}/pricing
    // - PATCH  project/{id}/pricing
    // - GET    project/{id}/pricing/purchasers
}
