//! 用户封禁管理路由
//!
//! 提供管理员封禁管理和用户封禁查看/申诉功能。

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use log::info;
use sqlx::PgPool;

use crate::auth::get_user_from_headers;
use crate::database::models::ids::{
    generate_ban_appeal_id, generate_ban_history_id, generate_user_ban_id,
};
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::user_ban_item::{
    AppealStatus, BanAppeal, BanAppealBuilder, BanHistory, BanHistoryBuilder,
    BanType, UserBan, UserBanBuilder,
};
use crate::database::models::{BanAppealId, UserBanId, UserId};
use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::parse_base62;
use crate::models::pats::Scopes;
use crate::models::users::Role;
use crate::models::v3::bans::{
    AppealsQueryParams, BansQueryParams, BatchBansQuery, CreateAppealRequest,
    CreateBanRequest, PaginatedAppeals, PaginatedBans, ReviewAppealRequest,
    RevokeBanRequest, UpdateBanRequest, UserActiveBans,
};
use crate::models::v3::notifications::NotificationBody;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;

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
    // 用户路由（get_my_bans, create_appeal, get_my_appeal）已移至 users.rs
}

// ==================== 辅助函数 ====================

/// 检查是否是管理员或版主
fn is_admin_or_moderator(role: &Role) -> bool {
    matches!(role, Role::Admin | Role::Moderator)
}

/// 解析用户 ID（支持 Base62 字符串）
fn parse_user_id(id_str: &str) -> Result<UserId, ApiError> {
    let id = parse_base62(id_str).map_err(|_| {
        ApiError::InvalidInput("无效的用户 ID 格式".to_string())
    })?;
    Ok(UserId(id as i64))
}

/// 解析封禁 ID
fn parse_ban_id(id_str: &str) -> Result<UserBanId, ApiError> {
    let id = parse_base62(id_str).map_err(|_| {
        ApiError::InvalidInput("无效的封禁 ID 格式".to_string())
    })?;
    Ok(UserBanId(id as i64))
}

/// 解析申诉 ID
fn parse_appeal_id(id_str: &str) -> Result<BanAppealId, ApiError> {
    let id = parse_base62(id_str).map_err(|_| {
        ApiError::InvalidInput("无效的申诉 ID 格式".to_string())
    })?;
    Ok(BanAppealId(id as i64))
}

/// 转换 API 层封禁类型到数据库层
fn convert_ban_type(api_type: &crate::models::v3::bans::BanType) -> BanType {
    match api_type {
        crate::models::v3::bans::BanType::Global => BanType::Global,
        crate::models::v3::bans::BanType::Resource => BanType::Resource,
        crate::models::v3::bans::BanType::Forum => BanType::Forum,
    }
}

/// 转换 API 层申诉状态到数据库层
fn convert_appeal_status(
    api_status: &crate::models::v3::bans::AppealStatus,
) -> AppealStatus {
    match api_status {
        crate::models::v3::bans::AppealStatus::Pending => AppealStatus::Pending,
        crate::models::v3::bans::AppealStatus::Approved => {
            AppealStatus::Approved
        }
        crate::models::v3::bans::AppealStatus::Rejected => {
            AppealStatus::Rejected
        }
    }
}

// ==================== 管理员路由 ====================

/// 创建封禁
///
/// POST /v3/bans/user/{user_id}
pub async fn create_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证管理员权限
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    // 输入验证
    if body.reason.is_empty() || body.reason.len() > 2000 {
        return Err(ApiError::InvalidInput(
            "封禁原因长度必须在 1-2000 字符之间".to_string(),
        ));
    }
    if let Some(ref internal_reason) = body.internal_reason
        && internal_reason.len() > 2000
    {
        return Err(ApiError::InvalidInput(
            "内部原因长度不能超过 2000 字符".to_string(),
        ));
    }
    if let Some(expires_at) = body.expires_at
        && expires_at <= Utc::now()
    {
        return Err(ApiError::InvalidInput(
            "过期时间必须是将来的时间".to_string(),
        ));
    }

    let target_user_id = parse_user_id(&info.0)?;
    let admin_user_id: UserId = user.id.into();

    // 不能封禁自己
    if target_user_id == admin_user_id {
        return Err(ApiError::InvalidInput("不能封禁自己".to_string()));
    }

    // 检查目标用户是否存在
    let target_user =
        crate::database::models::User::get_id(target_user_id, &**pool, &redis)
            .await?
            .ok_or_else(|| ApiError::NotFound)?;

    // 权限层级检查
    let target_role = Role::from_string(&target_user.role);
    match (&user.role, &target_role) {
        (Role::Moderator, Role::Admin) | (Role::Moderator, Role::Moderator) => {
            return Err(ApiError::CustomAuthentication(
                "版主无法封禁管理员或其他版主".to_string(),
            ));
        }
        _ => {}
    }

    // 预检查是否已存在相同类型的活跃封禁（用户体验优化，快速返回错误）
    let ban_type = convert_ban_type(&body.ban_type);
    let ban_type_display = match body.ban_type {
        crate::models::v3::bans::BanType::Global => "全局",
        crate::models::v3::bans::BanType::Resource => "资源",
        crate::models::v3::bans::BanType::Forum => "论坛",
    };
    if UserBan::is_user_banned(target_user_id, ban_type.clone(), &pool, &redis)
        .await?
    {
        return Err(ApiError::InvalidInput(format!(
            "用户已被{}封禁，请勿重复操作",
            ban_type_display
        )));
    }

    let mut transaction = pool.begin().await?;

    // 生成 ID
    let ban_id = generate_user_ban_id(&mut transaction).await?;
    let history_id = generate_ban_history_id(&mut transaction).await?;

    // 创建封禁记录（数据库有唯一索引 user_bans_unique_active_idx 防止竞态条件）
    let ban = match UserBan::insert(
        ban_id,
        UserBanBuilder {
            user_id: target_user_id,
            ban_type: ban_type.clone(),
            reason: body.reason.clone(),
            internal_reason: body.internal_reason.clone(),
            banned_by: admin_user_id,
            expires_at: body.expires_at,
            metadata: None,
        },
        &mut transaction,
    )
    .await
    {
        Ok(ban) => ban,
        Err(e) => {
            // 捕获唯一约束违规错误（竞态条件下的重复封禁）
            if let crate::database::models::DatabaseError::Database(
                sqlx::Error::Database(db_err),
            ) = &e
                && db_err.code().map(|c| c == "23505").unwrap_or(false)
            {
                return Err(ApiError::InvalidInput(format!(
                    "用户已被{}封禁，请勿重复操作",
                    ban_type_display
                )));
            }
            return Err(e.into());
        }
    };

    // 记录历史
    BanHistory::insert(
        history_id,
        BanHistoryBuilder {
            ban_id,
            action: "created".to_string(),
            operator_id: admin_user_id,
            old_data: None,
            new_data: serde_json::json!({
                "ban_type": ban.ban_type,
                "reason": ban.reason,
                "expires_at": ban.expires_at,
            }),
            reason: "创建封禁".to_string(),
        },
        &mut transaction,
    )
    .await?;

    // 发送通知
    if body.notify_user {
        NotificationBuilder {
            body: NotificationBody::UserBanned {
                ban_id: crate::models::v3::bans::UserBanId(ban_id.0 as u64),
                ban_type: ban_type.as_str().to_string(),
                reason: body.reason.clone(),
                expires_at: body.expires_at,
            },
        }
        .insert(target_user_id, &mut transaction, &redis)
        .await?;
    }

    transaction.commit().await?;

    // 清除用户缓存（包含active_bans字段）并清理锁键
    // 使用clear_caches_with_locks防止封禁后立即访问时的锁超时问题
    crate::database::models::User::clear_caches_with_locks(
        &[(target_user_id, Some(target_user.username.clone()))],
        &redis,
    )
    .await?;

    // 记录日志
    info!(
        "用户封禁创建: 管理员 {} 封禁了用户 {}, 类型: {:?}, 封禁ID: {}",
        admin_user_id.0, target_user_id.0, body.ban_type, ban_id.0
    );

    // 构建响应
    let response = crate::models::v3::bans::UserBan {
        id: crate::models::v3::bans::UserBanId(ban_id.0 as u64),
        user_id: crate::models::ids::UserId(target_user_id.0 as u64),
        ban_type: body.ban_type.clone(),
        reason: ban.reason,
        internal_reason: ban.internal_reason,
        banned_by: crate::models::ids::UserId(admin_user_id.0 as u64),
        banned_by_username: Some(user.username),
        banned_at: ban.banned_at,
        expires_at: ban.expires_at,
        is_active: true,
        can_appeal: Some(true),
        appeal: None,
    };

    Ok(HttpResponse::Created().json(response))
}

/// 获取封禁详情
///
/// GET /v3/bans/{ban_id}
pub async fn get_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    let ban_id = parse_ban_id(&info.0)?;
    let ban = UserBan::get(ban_id, &pool, &redis)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    // 获取执行人信息
    let banned_by_user =
        crate::database::models::User::get_id(ban.banned_by, &**pool, &redis)
            .await?;

    // 获取申诉信息
    let appeal = BanAppeal::get_by_ban_id(ban_id, &**pool).await?;

    // 只有 Admin 才能看到 internal_reason（信息安全控制）
    let internal_reason = if user.role == Role::Admin {
        ban.internal_reason.clone()
    } else {
        None
    };

    let response = crate::models::v3::bans::UserBan {
        id: crate::models::v3::bans::UserBanId(ban_id.0 as u64),
        user_id: crate::models::ids::UserId(ban.user_id.0 as u64),
        ban_type: crate::models::v3::bans::BanType::parse(&ban.ban_type)
            .unwrap_or(crate::models::v3::bans::BanType::Global),
        reason: ban.reason,
        internal_reason,
        banned_by: crate::models::ids::UserId(ban.banned_by.0 as u64),
        banned_by_username: banned_by_user.map(|u| u.username),
        banned_at: ban.banned_at,
        expires_at: ban.expires_at,
        is_active: ban.is_active,
        can_appeal: Some(appeal.is_none() && ban.is_active),
        appeal: appeal.map(|a| crate::models::v3::bans::BanAppeal {
            id: crate::models::v3::bans::BanAppealId(a.id.0 as u64),
            ban_id: crate::models::v3::bans::UserBanId(a.ban_id.0 as u64),
            user_id: crate::models::ids::UserId(a.user_id.0 as u64),
            reason: a.reason,
            status: crate::models::v3::bans::AppealStatus::parse(&a.status)
                .unwrap_or(crate::models::v3::bans::AppealStatus::Pending),
            created_at: a.created_at,
            reviewed_by: a
                .reviewed_by
                .map(|id| crate::models::ids::UserId(id.0 as u64)),
            reviewed_by_username: None,
            reviewed_at: a.reviewed_at,
            review_notes: a.review_notes,
            thread_id: a
                .thread_id
                .map(|t| crate::models::threads::ThreadId(t.0 as u64)),
        }),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// 修改封禁
///
/// PATCH /v3/bans/{ban_id}
pub async fn update_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<UpdateBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    let ban_id = parse_ban_id(&info.0)?;
    let admin_user_id: UserId = user.id.into();

    let ban = UserBan::get(ban_id, &pool, &redis)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    if !ban.is_active {
        return Err(ApiError::InvalidInput("此封禁已被解除".to_string()));
    }

    // 获取被封禁用户信息（用于清除缓存）
    let target_user =
        crate::database::models::User::get_id(ban.user_id, &**pool, &redis)
            .await?
            .ok_or_else(|| ApiError::NotFound)?;

    // 权限层级检查：版主无法修改管理员或其他版主的封禁
    let target_role = Role::from_string(&target_user.role);
    if user.role == Role::Moderator {
        // 检查被封禁用户的角色
        if matches!(target_role, Role::Admin | Role::Moderator) {
            return Err(ApiError::CustomAuthentication(
                "版主无法修改针对管理员或其他版主的封禁".to_string(),
            ));
        }
        // 检查封禁创建者的角色
        let banned_by_user = crate::database::models::User::get_id(
            ban.banned_by,
            &**pool,
            &redis,
        )
        .await?;
        if let Some(banned_by) = banned_by_user {
            let banned_by_role = Role::from_string(&banned_by.role);
            if matches!(banned_by_role, Role::Admin) {
                return Err(ApiError::CustomAuthentication(
                    "版主无法修改由管理员创建的封禁".to_string(),
                ));
            }
        }
    }

    // 输入验证：过期时间必须是将来的时间
    if let Some(Some(expires_at)) = &body.expires_at
        && *expires_at <= Utc::now()
    {
        return Err(ApiError::InvalidInput(
            "过期时间必须是将来的时间".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;

    // 记录旧数据
    let old_data = serde_json::json!({
        "reason": ban.reason,
        "internal_reason": ban.internal_reason,
        "expires_at": ban.expires_at,
    });

    // 更新封禁
    UserBan::update(
        ban_id,
        body.reason.clone(),
        body.internal_reason.clone(),
        body.expires_at,
        &mut transaction,
    )
    .await?;

    // 记录历史
    let history_id = generate_ban_history_id(&mut transaction).await?;
    BanHistory::insert(
        history_id,
        BanHistoryBuilder {
            ban_id,
            action: "modified".to_string(),
            operator_id: admin_user_id,
            old_data: Some(old_data),
            new_data: serde_json::json!({
                "reason": body.reason,
                "internal_reason": body.internal_reason,
                "expires_at": body.expires_at,
            }),
            reason: body
                .modification_reason
                .clone()
                .unwrap_or("修改封禁".to_string()),
        },
        &mut transaction,
    )
    .await?;

    transaction.commit().await?;

    // 清除用户缓存（包含active_bans字段）并清理锁键
    crate::database::models::User::clear_caches_with_locks(
        &[(ban.user_id, Some(target_user.username.clone()))],
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// 解除封禁
///
/// DELETE /v3/bans/{ban_id}
pub async fn revoke_ban(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<RevokeBanRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    let ban_id = parse_ban_id(&info.0)?;
    let admin_user_id: UserId = user.id.into();

    let ban = UserBan::get(ban_id, &pool, &redis)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    if !ban.is_active {
        return Err(ApiError::InvalidInput("此封禁已被解除".to_string()));
    }

    // 获取被封禁用户信息（用于清除缓存）
    let target_user =
        crate::database::models::User::get_id(ban.user_id, &**pool, &redis)
            .await?
            .ok_or_else(|| ApiError::NotFound)?;

    // 权限层级检查：版主无法解除管理员或其他版主的封禁
    let target_role = Role::from_string(&target_user.role);
    if user.role == Role::Moderator {
        // 检查被封禁用户的角色
        if matches!(target_role, Role::Admin | Role::Moderator) {
            return Err(ApiError::CustomAuthentication(
                "版主无法解除针对管理员或其他版主的封禁".to_string(),
            ));
        }
        // 检查封禁创建者的角色
        let banned_by_user = crate::database::models::User::get_id(
            ban.banned_by,
            &**pool,
            &redis,
        )
        .await?;
        if let Some(banned_by) = banned_by_user {
            let banned_by_role = Role::from_string(&banned_by.role);
            if matches!(banned_by_role, Role::Admin) {
                return Err(ApiError::CustomAuthentication(
                    "版主无法解除由管理员创建的封禁".to_string(),
                ));
            }
        }
    }

    let mut transaction = pool.begin().await?;

    // 解除封禁
    UserBan::deactivate(ban_id, &mut transaction).await?;

    // 记录历史
    let history_id = generate_ban_history_id(&mut transaction).await?;
    BanHistory::insert(
        history_id,
        BanHistoryBuilder {
            ban_id,
            action: "revoked".to_string(),
            operator_id: admin_user_id,
            old_data: Some(serde_json::json!({
                "is_active": true,
            })),
            new_data: serde_json::json!({
                "is_active": false,
            }),
            reason: body.reason.clone(),
        },
        &mut transaction,
    )
    .await?;

    // 发送通知
    if body.notify_user {
        NotificationBuilder {
            body: NotificationBody::UserUnbanned {
                ban_id: crate::models::v3::bans::UserBanId(ban_id.0 as u64),
                ban_type: ban.ban_type.clone(),
                reason: body.reason.clone(),
            },
        }
        .insert(ban.user_id, &mut transaction, &redis)
        .await?;
    }

    transaction.commit().await?;

    // 清除用户缓存（包含active_bans字段）并清理锁键
    crate::database::models::User::clear_caches_with_locks(
        &[(ban.user_id, Some(target_user.username.clone()))],
        &redis,
    )
    .await?;

    // 记录日志
    info!(
        "用户封禁解除: 管理员 {} 解除了用户 {} 的封禁 {}",
        admin_user_id.0, ban.user_id.0, ban_id.0
    );

    Ok(HttpResponse::NoContent().finish())
}

/// 获取用户的封禁记录
///
/// GET /v3/bans/user/{user_id}
pub async fn get_user_bans(
    req: HttpRequest,
    info: web::Path<(String,)>,
    query: web::Query<BansQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    let target_user_id = parse_user_id(&info.0)?;
    let include_inactive = query.include_inactive.unwrap_or(false);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (query.page.unwrap_or(1) - 1) * limit;

    let bans = UserBan::get_user_all_bans(
        target_user_id,
        include_inactive,
        limit,
        offset,
        &**pool,
    )
    .await?;

    // 批量获取封禁者用户信息，避免 N+1 查询
    let banned_by_ids: Vec<crate::database::models::UserId> = bans
        .iter()
        .map(|b| b.banned_by)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let banned_by_users = crate::database::models::User::get_many_ids(
        &banned_by_ids,
        &**pool,
        &redis,
    )
    .await?;
    let user_map: std::collections::HashMap<_, _> = banned_by_users
        .into_iter()
        .map(|u| (u.id, u.username))
        .collect();

    // 转换为 API 响应格式
    let mut response_bans = Vec::new();
    for ban in bans {
        let banned_by_username = user_map.get(&ban.banned_by).cloned();

        response_bans.push(crate::models::v3::bans::UserBan {
            id: crate::models::v3::bans::UserBanId(ban.id.0 as u64),
            user_id: crate::models::ids::UserId(ban.user_id.0 as u64),
            ban_type: crate::models::v3::bans::BanType::parse(&ban.ban_type)
                .unwrap_or(crate::models::v3::bans::BanType::Global),
            reason: ban.reason,
            internal_reason: ban.internal_reason,
            banned_by: crate::models::ids::UserId(ban.banned_by.0 as u64),
            banned_by_username,
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
            is_active: ban.is_active,
            can_appeal: None,
            appeal: None,
        });
    }

    let total = response_bans.len() as i64;
    Ok(HttpResponse::Ok().json(PaginatedBans {
        bans: response_bans,
        total,
        page: query.page.unwrap_or(1),
        limit,
    }))
}

/// 获取封禁历史
///
/// GET /v3/bans/{ban_id}/history
pub async fn get_ban_history(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    let ban_id = parse_ban_id(&info.0)?;

    // 验证封禁存在
    UserBan::get(ban_id, &pool, &redis)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let history = BanHistory::get_by_ban_id(ban_id, &**pool).await?;

    // 转换为 API 响应格式
    let mut response_history = Vec::new();
    for entry in history {
        let operator = crate::database::models::User::get_id(
            entry.operator_id,
            &**pool,
            &redis,
        )
        .await?;

        response_history.push(crate::models::v3::bans::BanHistoryEntry {
            id: crate::models::v3::bans::BanHistoryId(entry.id.0 as u64),
            ban_id: crate::models::v3::bans::UserBanId(entry.ban_id.0 as u64),
            action: entry.action,
            operator_id: crate::models::ids::UserId(entry.operator_id.0 as u64),
            operator_username: operator.map(|u| u.username),
            operated_at: entry.operated_at,
            old_data: entry.old_data,
            new_data: entry.new_data,
            reason: entry.reason,
        });
    }

    Ok(HttpResponse::Ok().json(response_history))
}

/// 获取所有封禁列表
///
/// GET /v3/bans
pub async fn list_bans(
    req: HttpRequest,
    query: web::Query<BansQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    let limit = query.limit.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1);
    let offset = (page - 1) * limit;
    let is_active = query.is_active.unwrap_or(true);

    // 使用窗口函数合并查询，避免双次数据库查询
    let bans_with_count = sqlx::query_as!(
        BanRowWithCount,
        r#"
        SELECT id, user_id, ban_type, reason, internal_reason, banned_by,
               banned_at, expires_at, is_active, metadata,
               COUNT(*) OVER() as total_count
        FROM user_bans
        WHERE is_active = $1
        ORDER BY banned_at DESC
        LIMIT $2 OFFSET $3
        "#,
        is_active,
        limit,
        offset
    )
    .fetch_all(&**pool)
    .await?;

    // 从第一条记录获取总数，如果没有记录则为0
    let total = bans_with_count
        .first()
        .and_then(|b| b.total_count)
        .unwrap_or(0);

    // 转换为不带 total_count 的结构
    let bans: Vec<BanRow> = bans_with_count
        .into_iter()
        .map(|b| BanRow {
            id: b.id,
            user_id: b.user_id,
            ban_type: b.ban_type,
            reason: b.reason,
            internal_reason: b.internal_reason,
            banned_by: b.banned_by,
            banned_at: b.banned_at,
            expires_at: b.expires_at,
            is_active: b.is_active,
            metadata: b.metadata,
        })
        .collect();

    // 批量获取所有 banned_by 用户信息，避免 N+1 查询
    let banned_by_ids: Vec<UserId> = bans
        .iter()
        .map(|b| UserId(b.banned_by))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let banned_by_users = crate::database::models::User::get_many_ids(
        &banned_by_ids,
        &**pool,
        &redis,
    )
    .await?;

    // 构建用户名映射
    let banned_by_usernames: std::collections::HashMap<i64, String> =
        banned_by_users
            .into_iter()
            .map(|u| (u.id.0, u.username))
            .collect();

    // 只有 Admin 才能看到 internal_reason（信息安全控制）
    let is_admin = user.role == Role::Admin;

    // 转换为 API 响应格式
    let response_bans: Vec<crate::models::v3::bans::UserBan> = bans
        .into_iter()
        .map(|ban| crate::models::v3::bans::UserBan {
            id: crate::models::v3::bans::UserBanId(ban.id as u64),
            user_id: crate::models::ids::UserId(ban.user_id as u64),
            ban_type: crate::models::v3::bans::BanType::parse(&ban.ban_type)
                .unwrap_or(crate::models::v3::bans::BanType::Global),
            reason: ban.reason,
            internal_reason: if is_admin { ban.internal_reason } else { None },
            banned_by: crate::models::ids::UserId(ban.banned_by as u64),
            banned_by_username: banned_by_usernames
                .get(&ban.banned_by)
                .cloned(),
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
            is_active: ban.is_active,
            can_appeal: None,
            appeal: None,
        })
        .collect();

    let response = PaginatedBans {
        bans: response_bans,
        total,
        page,
        limit,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// 获取所有申诉列表
///
/// GET /v3/bans/appeals
pub async fn list_appeals(
    req: HttpRequest,
    query: web::Query<AppealsQueryParams>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (query.page.unwrap_or(1) - 1) * limit;
    let status = query.status.as_ref().map(|s| s.as_str());

    // 使用窗口函数合并查询，避免双次数据库查询
    // 按创建时间降序排列，最新的申诉在最上面
    let appeals_with_count = if let Some(status_str) = status {
        sqlx::query_as!(
            AppealRowWithCount,
            r#"
            SELECT id, ban_id, user_id, reason, status, created_at,
                   reviewed_by, reviewed_at, review_notes, thread_id,
                   COUNT(*) OVER() as total_count
            FROM user_ban_appeals
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            status_str,
            limit,
            offset
        )
        .fetch_all(&**pool)
        .await?
    } else {
        sqlx::query_as!(
            AppealRowWithCount,
            r#"
            SELECT id, ban_id, user_id, reason, status, created_at,
                   reviewed_by, reviewed_at, review_notes, thread_id,
                   COUNT(*) OVER() as total_count
            FROM user_ban_appeals
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&**pool)
        .await?
    };

    // 从第一条记录获取总数，如果没有记录则为0
    let total = appeals_with_count
        .first()
        .and_then(|a| a.total_count)
        .unwrap_or(0);

    // 转换为不带 total_count 的结构
    let appeals: Vec<AppealRow> = appeals_with_count
        .into_iter()
        .map(|a| AppealRow {
            id: a.id,
            ban_id: a.ban_id,
            user_id: a.user_id,
            reason: a.reason,
            status: a.status,
            created_at: a.created_at,
            reviewed_by: a.reviewed_by,
            reviewed_at: a.reviewed_at,
            review_notes: a.review_notes,
            thread_id: a.thread_id,
        })
        .collect();

    // 批量获取所有 reviewer 用户信息，避免 N+1 查询
    let reviewer_ids: Vec<UserId> = appeals
        .iter()
        .filter_map(|a| a.reviewed_by.map(UserId))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let reviewer_users = if !reviewer_ids.is_empty() {
        crate::database::models::User::get_many_ids(
            &reviewer_ids,
            &**pool,
            &redis,
        )
        .await?
    } else {
        Vec::new()
    };

    // 构建审核人用户名映射
    let reviewer_usernames: std::collections::HashMap<i64, String> =
        reviewer_users
            .into_iter()
            .map(|u| (u.id.0, u.username))
            .collect();

    // 转换为 API 响应格式
    let response_appeals: Vec<crate::models::v3::bans::BanAppeal> = appeals
        .into_iter()
        .map(|appeal| crate::models::v3::bans::BanAppeal {
            id: crate::models::v3::bans::BanAppealId(appeal.id as u64),
            ban_id: crate::models::v3::bans::UserBanId(appeal.ban_id as u64),
            user_id: crate::models::ids::UserId(appeal.user_id as u64),
            reason: appeal.reason,
            status: crate::models::v3::bans::AppealStatus::parse(
                &appeal.status,
            )
            .unwrap_or(crate::models::v3::bans::AppealStatus::Pending),
            created_at: appeal.created_at,
            reviewed_by: appeal
                .reviewed_by
                .map(|id| crate::models::ids::UserId(id as u64)),
            reviewed_by_username: appeal
                .reviewed_by
                .and_then(|id| reviewer_usernames.get(&id).cloned()),
            reviewed_at: appeal.reviewed_at,
            review_notes: appeal.review_notes,
            thread_id: appeal
                .thread_id
                .map(|t| crate::models::threads::ThreadId(t as u64)),
        })
        .collect();

    Ok(HttpResponse::Ok().json(PaginatedAppeals {
        appeals: response_appeals,
        total,
        page: query.page.unwrap_or(1),
        limit,
    }))
}

/// 审核申诉
///
/// PATCH /v3/bans/appeals/{appeal_id}
pub async fn review_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<ReviewAppealRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    let appeal_id = parse_appeal_id(&info.0)?;
    let admin_user_id: UserId = user.id.into();

    let appeal = BanAppeal::get(appeal_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    if appeal.status != "pending" {
        return Err(ApiError::InvalidInput("此申诉已被处理".to_string()));
    }

    let mut transaction = pool.begin().await?;

    // 更新申诉状态
    let status = convert_appeal_status(&body.status);
    BanAppeal::review(
        appeal_id,
        status.clone(),
        admin_user_id,
        body.review_notes.clone(),
        &mut transaction,
    )
    .await?;

    // 如果批准申诉，解除封禁
    let cache_to_clear = if matches!(
        body.status,
        crate::models::v3::bans::AppealStatus::Approved
    ) {
        let ban = UserBan::get(appeal.ban_id, &pool, &redis)
            .await?
            .ok_or_else(|| ApiError::NotFound)?;

        if ban.is_active {
            // 获取被封禁用户信息（用于清除缓存）
            let target_user = crate::database::models::User::get_id(
                ban.user_id,
                &**pool,
                &redis,
            )
            .await?
            .ok_or_else(|| ApiError::NotFound)?;

            UserBan::deactivate(appeal.ban_id, &mut transaction).await?;

            // 记录历史
            let history_id = generate_ban_history_id(&mut transaction).await?;
            BanHistory::insert(
                history_id,
                BanHistoryBuilder {
                    ban_id: appeal.ban_id,
                    action: "revoked_by_appeal".to_string(),
                    operator_id: admin_user_id,
                    old_data: Some(serde_json::json!({
                        "is_active": true,
                    })),
                    new_data: serde_json::json!({
                        "is_active": false,
                        "appeal_id": appeal_id.0,
                    }),
                    reason: format!(
                        "申诉通过：{}",
                        body.review_notes.clone().unwrap_or_default()
                    ),
                },
                &mut transaction,
            )
            .await?;

            Some((ban.user_id, appeal.ban_id, target_user.username))
        } else {
            None
        }
    } else {
        None
    };

    // 发送通知
    if body.notify_user {
        NotificationBuilder {
            body: NotificationBody::AppealReviewed {
                appeal_id: crate::models::v3::bans::BanAppealId(
                    appeal_id.0 as u64,
                ),
                ban_id: crate::models::v3::bans::UserBanId(
                    appeal.ban_id.0 as u64,
                ),
                status: body.status.as_str().to_string(),
                review_notes: body.review_notes.clone(),
            },
        }
        .insert(appeal.user_id, &mut transaction, &redis)
        .await?;
    }

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 事务提交后清除缓存
    // 如果批准申诉，使用 clear_caches_with_locks 防止锁超时
    // 如果拒绝申诉，只需清除数据缓存即可
    if let Some((user_id, _ban_id, username)) = cache_to_clear {
        // 申诉批准，封禁已解除
        crate::database::models::User::clear_caches_with_locks(
            &[(user_id, Some(username))],
            &redis,
        )
        .await?;
    } else {
        // 申诉被拒绝或其他情况，只清除数据缓存
        // 获取被申诉用户信息
        let target_user = crate::database::models::User::get_id(
            appeal.user_id,
            &**pool,
            &redis,
        )
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

        crate::database::models::User::clear_caches(
            &[(appeal.user_id, Some(target_user.username))],
            &redis,
        )
        .await?;
    }

    // 记录日志
    info!(
        "申诉审核完成: 管理员 {} 审核了申诉 {}, 结果: {:?}",
        admin_user_id.0, appeal_id.0, body.status
    );

    Ok(HttpResponse::NoContent().finish())
}

// ==================== 用户路由 ====================

/// 获取自己的封禁状态
///
/// GET /v3/user/bans
pub async fn get_my_bans(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    let user_id: UserId = user.id.into();
    let bans = UserBan::get_user_active_bans(user_id, &pool, &redis).await?;

    let mut active_bans = Vec::new();
    for ban in bans {
        let appeal = BanAppeal::get_by_ban_id(ban.id, &**pool).await?;

        active_bans.push(crate::models::v3::bans::UserBanPublic {
            id: crate::models::v3::bans::UserBanId(ban.id.0 as u64),
            ban_type: crate::models::v3::bans::BanType::parse(&ban.ban_type)
                .unwrap_or(crate::models::v3::bans::BanType::Global),
            reason: ban.reason,
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
            is_active: ban.is_active,
            can_appeal: appeal.is_none(),
            appeal_id: appeal
                .map(|a| crate::models::v3::bans::BanAppealId(a.id.0 as u64)),
        });
    }

    Ok(HttpResponse::Ok().json(UserActiveBans { active_bans }))
}

/// 创建申诉
///
/// POST /v3/user/bans/{ban_id}/appeal
pub async fn create_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    body: web::Json<CreateAppealRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    // 输入验证
    if body.reason.is_empty() || body.reason.len() > 2000 {
        return Err(ApiError::InvalidInput(
            "申诉原因长度必须在 1-2000 字符之间".to_string(),
        ));
    }

    let ban_id = parse_ban_id(&info.0)?;
    let user_id: UserId = user.id.into();

    // 检查申诉频率限制（24小时内最多3次）
    let recent_appeals_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM user_ban_appeals
         WHERE user_id = $1 AND created_at > NOW() - INTERVAL '24 hours'",
        user_id.0
    )
    .fetch_one(&**pool)
    .await?
    .unwrap_or(0);

    if recent_appeals_count >= 3 {
        return Err(ApiError::InvalidInput(
            "您在24小时内已提交过多申诉，请稍后再试".to_string(),
        ));
    }

    // 获取封禁
    let ban = UserBan::get(ban_id, &pool, &redis)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    // 验证封禁属于当前用户
    if ban.user_id != user_id {
        return Err(ApiError::NotFound);
    }

    // 验证封禁仍然有效
    if !ban.is_active {
        return Err(ApiError::InvalidInput("此封禁已被解除".to_string()));
    }

    // 检查是否已经申诉过
    let existing_appeal = BanAppeal::get_by_ban_id(ban_id, &**pool).await?;
    if existing_appeal.is_some() {
        return Err(ApiError::InvalidInput(
            "您已对此封禁提交过申诉".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;

    // 创建申诉
    let appeal_id = generate_ban_appeal_id(&mut transaction).await?;
    let appeal = BanAppeal::insert(
        appeal_id,
        BanAppealBuilder {
            ban_id,
            user_id,
            reason: body.reason.clone(),
        },
        &mut transaction,
    )
    .await?;

    // 创建申诉交流线程
    let thread_id = crate::database::models::thread_item::ThreadBuilder {
        type_: crate::models::threads::ThreadType::BanAppeal,
        members: vec![user_id], // 申诉人加入线程
        project_id: None,
        report_id: None,
        ban_appeal_id: Some(appeal_id),
        creator_application_id: None,
    }
    .insert(&mut transaction)
    .await?;

    // 将申诉原因作为第一条消息添加到线程中
    crate::database::models::thread_item::ThreadMessageBuilder {
        author_id: Some(user_id),
        body: crate::models::threads::MessageBody::Text {
            body: body.reason.clone(),
            private: false,
            replying_to: None,
            associated_images: vec![],
        },
        thread_id,
        hide_identity: false,
    }
    .insert(&mut transaction)
    .await?;

    // 更新申诉记录关联线程
    BanAppeal::set_thread_id(appeal_id, thread_id, &mut transaction).await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除用户缓存，以便前端能立即获取到最新的申诉状态
    crate::database::models::User::clear_caches(
        &[(user_id, Some(user.username.clone()))],
        &redis,
    )
    .await?;

    let response = crate::models::v3::bans::BanAppeal {
        id: crate::models::v3::bans::BanAppealId(appeal_id.0 as u64),
        ban_id: crate::models::v3::bans::UserBanId(ban_id.0 as u64),
        user_id: crate::models::ids::UserId(user_id.0 as u64),
        reason: appeal.reason,
        status: crate::models::v3::bans::AppealStatus::Pending,
        created_at: appeal.created_at,
        reviewed_by: None,
        reviewed_by_username: None,
        reviewed_at: None,
        review_notes: None,
        thread_id: Some(crate::models::threads::ThreadId(thread_id.0 as u64)),
    };

    Ok(HttpResponse::Created().json(response))
}

/// 获取自己的申诉详情
///
/// GET /v3/user/appeals/{appeal_id}
pub async fn get_my_appeal(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    let appeal_id = parse_appeal_id(&info.0)?;
    let user_id: UserId = user.id.into();

    let appeal = BanAppeal::get(appeal_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    // 验证申诉属于当前用户
    if appeal.user_id != user_id {
        return Err(ApiError::NotFound);
    }

    let response = crate::models::v3::bans::BanAppeal {
        id: crate::models::v3::bans::BanAppealId(appeal_id.0 as u64),
        ban_id: crate::models::v3::bans::UserBanId(appeal.ban_id.0 as u64),
        user_id: crate::models::ids::UserId(appeal.user_id.0 as u64),
        reason: appeal.reason,
        status: crate::models::v3::bans::AppealStatus::parse(&appeal.status)
            .unwrap_or(crate::models::v3::bans::AppealStatus::Pending),
        created_at: appeal.created_at,
        reviewed_by: appeal
            .reviewed_by
            .map(|id| crate::models::ids::UserId(id.0 as u64)),
        reviewed_by_username: None,
        reviewed_at: appeal.reviewed_at,
        review_notes: appeal.review_notes,
        thread_id: appeal
            .thread_id
            .map(|t| crate::models::threads::ThreadId(t.0 as u64)),
    };

    Ok(HttpResponse::Ok().json(response))
}

// ==================== 辅助结构 ====================

/// 封禁数据库行
#[derive(sqlx::FromRow)]
struct BanRow {
    id: i64,
    user_id: i64,
    ban_type: String,
    reason: String,
    internal_reason: Option<String>,
    banned_by: i64,
    banned_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
    is_active: bool,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

/// 封禁数据库行（带总数，用于窗口函数查询）
#[derive(sqlx::FromRow)]
struct BanRowWithCount {
    id: i64,
    user_id: i64,
    ban_type: String,
    reason: String,
    internal_reason: Option<String>,
    banned_by: i64,
    banned_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
    is_active: bool,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
    total_count: Option<i64>,
}

/// 申诉数据库行
#[derive(sqlx::FromRow)]
struct AppealRow {
    id: i64,
    ban_id: i64,
    user_id: i64,
    reason: String,
    status: String,
    created_at: chrono::DateTime<Utc>,
    reviewed_by: Option<i64>,
    reviewed_at: Option<chrono::DateTime<Utc>>,
    review_notes: Option<String>,
    thread_id: Option<i64>,
}

/// 申诉数据库行（带总数，用于窗口函数查询）
#[derive(sqlx::FromRow)]
struct AppealRowWithCount {
    id: i64,
    ban_id: i64,
    user_id: i64,
    reason: String,
    status: String,
    created_at: chrono::DateTime<Utc>,
    reviewed_by: Option<i64>,
    reviewed_at: Option<chrono::DateTime<Utc>>,
    review_notes: Option<String>,
    thread_id: Option<i64>,
    total_count: Option<i64>,
}

/// 批量获取封禁详情
///
/// GET /v3/bans/batch?ids=["id1","id2",...]
///
/// 用于前端批量获取封禁详情，避免 N+1 请求问题
pub async fn get_bans_batch(
    req: HttpRequest,
    query: web::Query<BatchBansQuery>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (_, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !is_admin_or_moderator(&user.role) {
        return Err(ApiError::CustomAuthentication(
            "您没有权限查看此信息".to_string(),
        ));
    }

    // 解析 JSON 格式的 ID 列表
    let ban_ids: Vec<String> =
        serde_json::from_str(&query.ids).map_err(|_| {
            ApiError::InvalidInput(
                "无效的 ids 参数格式，需要 JSON 数组".to_string(),
            )
        })?;

    // 限制批量查询数量
    if ban_ids.len() > 100 {
        return Err(ApiError::InvalidInput(
            "批量查询数量不能超过 100".to_string(),
        ));
    }

    if ban_ids.is_empty() {
        return Ok(HttpResponse::Ok()
            .json(Vec::<crate::models::v3::bans::UserBan>::new()));
    }

    // 转换 ID
    let db_ban_ids: Vec<i64> = ban_ids
        .iter()
        .filter_map(|id| parse_base62(id).ok().map(|v| v as i64))
        .collect();

    if db_ban_ids.is_empty() {
        return Ok(HttpResponse::Ok()
            .json(Vec::<crate::models::v3::bans::UserBan>::new()));
    }

    // 批量查询封禁
    let bans = sqlx::query_as!(
        BanRow,
        r#"
        SELECT id, user_id, ban_type, reason, internal_reason, banned_by,
               banned_at, expires_at, is_active, metadata
        FROM user_bans
        WHERE id = ANY($1)
        "#,
        &db_ban_ids
    )
    .fetch_all(&**pool)
    .await?;

    // 批量获取所有 banned_by 用户信息
    let banned_by_ids: Vec<UserId> = bans
        .iter()
        .map(|b| UserId(b.banned_by))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let banned_by_users = crate::database::models::User::get_many_ids(
        &banned_by_ids,
        &**pool,
        &redis,
    )
    .await?;

    let banned_by_usernames: std::collections::HashMap<i64, String> =
        banned_by_users
            .into_iter()
            .map(|u| (u.id.0, u.username))
            .collect();

    // 只有 Admin 才能看到 internal_reason
    let is_admin = user.role == Role::Admin;

    // 转换为 API 响应格式
    let response_bans: Vec<crate::models::v3::bans::UserBan> = bans
        .into_iter()
        .map(|ban| crate::models::v3::bans::UserBan {
            id: crate::models::v3::bans::UserBanId(ban.id as u64),
            user_id: crate::models::ids::UserId(ban.user_id as u64),
            ban_type: crate::models::v3::bans::BanType::parse(&ban.ban_type)
                .unwrap_or(crate::models::v3::bans::BanType::Global),
            reason: ban.reason,
            internal_reason: if is_admin { ban.internal_reason } else { None },
            banned_by: crate::models::ids::UserId(ban.banned_by as u64),
            banned_by_username: banned_by_usernames
                .get(&ban.banned_by)
                .cloned(),
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
            is_active: ban.is_active,
            can_appeal: None,
            appeal: None,
        })
        .collect();

    Ok(HttpResponse::Ok().json(response_bans))
}
