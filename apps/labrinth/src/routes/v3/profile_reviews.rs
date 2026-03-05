use actix_web::{HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::ApiError;
use crate::auth::get_user_from_headers;
use crate::database::models::User;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::util::img::delete_old_images;
use std::sync::Arc;

/// 用户撤销资料审核
/// POST /v3/user/{user_id}/profile_reviews/{review_id}/cancel
pub async fn cancel_review(
    req: HttpRequest,
    info: web::Path<(String, i64)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
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

    let (user_id_str, review_id) = info.into_inner();
    let target_user = User::get(&user_id_str, &**pool, &redis)
        .await?
        .ok_or(ApiError::NotFound)?;

    let target_user_id: crate::models::ids::UserId = target_user.id.into();

    // 只有用户本人可以撤销
    if user.id != target_user_id {
        return Err(ApiError::CustomAuthentication(
            "您没有权限撤销此审核!".to_string(),
        ));
    }

    // 使用事务 + FOR UPDATE 防止竞态条件
    let mut transaction = pool.begin().await?;

    let review = sqlx::query!(
        "SELECT id, user_id, review_type, new_value, status
         FROM user_profile_reviews
         WHERE id = $1 AND user_id = $2 AND status = 'pending'
         FOR UPDATE",
        review_id,
        target_user.id.0,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(ApiError::NotFound)?;

    // 先更新数据库状态
    sqlx::query!(
        "UPDATE user_profile_reviews SET status = 'cancelled' WHERE id = $1",
        review_id,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除用户缓存
    User::clear_caches(&[(target_user.id, Some(target_user.username))], &redis)
        .await?;

    // 事务提交后再删除 S3 图片（尽力而为，失败不影响数据一致性）
    if review.review_type == "avatar" {
        if let Ok(avatar_json) =
            serde_json::from_str::<serde_json::Value>(&review.new_value)
        {
            let avatar_url = avatar_json
                .get("avatar_url")
                .and_then(|v| v.as_str())
                .map(String::from);
            let raw_avatar_url = avatar_json
                .get("raw_avatar_url")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Err(e) =
                delete_old_images(avatar_url, raw_avatar_url, &***file_host)
                    .await
            {
                log::warn!(
                    "撤销审核时删除 S3 图片失败 (review_id={}): {}",
                    review_id,
                    e
                );
            }
        } else {
            log::warn!(
                "撤销审核时解析 avatar JSON 失败 (review_id={})",
                review_id
            );
        }
    }

    Ok(HttpResponse::NoContent().body(""))
}

// ==================== 管理员审核路由 ====================

#[derive(Deserialize)]
pub struct ReviewListQuery {
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_count")]
    pub count: i64,
}

fn default_status() -> String {
    "pending".to_string()
}

fn default_count() -> i64 {
    50
}

const VALID_STATUSES: &[&str] =
    &["all", "pending", "approved", "rejected", "cancelled"];

#[derive(Serialize)]
pub struct ProfileReviewItem {
    pub id: i64,
    pub user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub review_type: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub risk_labels: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub review_notes: Option<String>,
}

/// 管理员获取审核列表
/// GET /_internal/moderation/profile-reviews
pub async fn list_reviews(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    query: web::Query<ReviewListQuery>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    crate::auth::check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    // 校验 status 参数
    if !VALID_STATUSES.contains(&query.status.as_str()) {
        return Err(ApiError::InvalidInput(format!(
            "无效的状态参数: {}",
            query.status
        )));
    }

    // 限制 count 上限
    let count = query.count.min(500);

    let rows = sqlx::query!(
        r#"
        SELECT pr.id, pr.user_id, pr.review_type, pr.old_value, pr.new_value,
               pr.risk_labels, pr.status, pr.created_at,
               pr.reviewed_by, pr.reviewed_at, pr.review_notes,
               u.username, u.avatar_url
        FROM user_profile_reviews pr
        JOIN users u ON u.id = pr.user_id
        WHERE ($1 = 'all' OR pr.status = $1)
        ORDER BY pr.created_at DESC
        LIMIT $2
        "#,
        &query.status,
        count,
    )
    .fetch_all(&**pool)
    .await?;

    let items: Vec<ProfileReviewItem> = rows
        .into_iter()
        .map(|row| ProfileReviewItem {
            id: row.id,
            user_id: crate::models::ids::UserId::from(
                crate::database::models::ids::UserId(row.user_id),
            )
            .to_string(),
            username: row.username,
            avatar_url: row.avatar_url,
            review_type: row.review_type,
            old_value: row.old_value,
            new_value: row.new_value,
            risk_labels: row.risk_labels,
            status: row.status,
            created_at: row.created_at,
            reviewed_by: row.reviewed_by.map(|id| {
                crate::models::ids::UserId::from(
                    crate::database::models::ids::UserId(id),
                )
                .to_string()
            }),
            reviewed_at: row.reviewed_at,
            review_notes: row.review_notes,
        })
        .collect();

    Ok(HttpResponse::Ok().json(items))
}

#[derive(Deserialize)]
pub struct ReviewAction {
    pub notes: Option<String>,
}

/// 管理员批准审核
/// POST /_internal/moderation/profile-reviews/{id}/approve
pub async fn approve_review(
    req: HttpRequest,
    info: web::Path<(i64,)>,
    body: web::Json<ReviewAction>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let moderator = crate::auth::check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    // 限制备注长度
    if let Some(ref notes) = body.notes
        && notes.len() > 500
    {
        return Err(ApiError::InvalidInput(
            "审核备注不能超过500个字符".to_string(),
        ));
    }

    let review_id = info.into_inner().0;

    // SELECT 在事务内 + FOR UPDATE 防止竞态
    let mut transaction = pool.begin().await?;

    let review = sqlx::query!(
        "SELECT id, user_id, review_type, old_value, new_value, status
         FROM user_profile_reviews
         WHERE id = $1 AND status = 'pending'
         FOR UPDATE",
        review_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(ApiError::NotFound)?;

    let target_user = User::get_id(
        crate::database::models::ids::UserId(review.user_id),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 保存旧的头像 URL，事务提交后用于清理 S3
    let old_avatar_url = target_user.avatar_url.clone();
    let old_raw_avatar_url = target_user.raw_avatar_url.clone();

    match review.review_type.as_str() {
        "username" => {
            // 检查新用户名在审核期间是否被他人占用
            let existing = sqlx::query_scalar!(
                "SELECT id FROM users WHERE LOWER(username) = LOWER($1) AND id != $2",
                &review.new_value,
                review.user_id,
            )
            .fetch_optional(&mut *transaction)
            .await?;

            if existing.is_some() {
                // 用户名已被占用，自动标记为拒绝
                sqlx::query!(
                    "UPDATE user_profile_reviews
                     SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW(),
                         review_notes = '用户名在审核期间已被其他用户占用'
                     WHERE id = $2",
                    moderator.id.0 as i64,
                    review_id,
                )
                .execute(&mut *transaction)
                .await?;

                let notification = NotificationBuilder {
                    body: NotificationBody::ProfileReviewResult {
                        review_id,
                        review_type: "username".to_string(),
                        status: "rejected".to_string(),
                        review_notes: Some(
                            "用户名在审核期间已被其他用户占用".to_string(),
                        ),
                    },
                };
                notification
                    .insert(
                        crate::database::models::ids::UserId(review.user_id),
                        &mut transaction,
                        &redis,
                    )
                    .await?;

                transaction.commit().await?;

                User::clear_caches(
                    &[(target_user.id, Some(target_user.username))],
                    &redis,
                )
                .await?;

                return Err(ApiError::InvalidInput(format!(
                    "用户名 '{}' 在审核期间已被其他用户占用",
                    review.new_value
                )));
            }

            sqlx::query!(
                "UPDATE users SET username = $1 WHERE id = $2",
                &review.new_value,
                review.user_id,
            )
            .execute(&mut *transaction)
            .await?;
        }
        "bio" => {
            sqlx::query!(
                "UPDATE users SET bio = $1 WHERE id = $2",
                &review.new_value,
                review.user_id,
            )
            .execute(&mut *transaction)
            .await?;
        }
        "avatar" => {
            let new_avatar: serde_json::Value =
                serde_json::from_str(&review.new_value).map_err(|_| {
                    ApiError::InvalidInput("无效的头像数据".to_string())
                })?;

            let new_avatar_url = new_avatar
                .get("avatar_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApiError::InvalidInput("缺少 avatar_url".to_string())
                })?;
            let new_raw_avatar_url = new_avatar
                .get("raw_avatar_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApiError::InvalidInput("缺少 raw_avatar_url".to_string())
                })?;

            // 不在事务内删除 S3 图片，先完成数据库操作
            sqlx::query!(
                "UPDATE users SET avatar_url = $1, raw_avatar_url = $2 WHERE id = $3",
                new_avatar_url,
                new_raw_avatar_url,
                review.user_id,
            )
            .execute(&mut *transaction)
            .await?;
        }
        _ => {
            return Err(ApiError::InvalidInput(format!(
                "未知的审核类型: {}",
                review.review_type
            )));
        }
    }

    // 更新审核记录
    sqlx::query!(
        "UPDATE user_profile_reviews
         SET status = 'approved', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3",
        moderator.id.0 as i64,
        body.notes.as_deref(),
        review_id,
    )
    .execute(&mut *transaction)
    .await?;

    // 发送通知
    let notification = NotificationBuilder {
        body: NotificationBody::ProfileReviewResult {
            review_id,
            review_type: review.review_type.clone(),
            status: "approved".to_string(),
            review_notes: body.notes.clone(),
        },
    };
    notification
        .insert(
            crate::database::models::ids::UserId(review.user_id),
            &mut transaction,
            &redis,
        )
        .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除用户缓存（旧用户名 + 新用户名如果是用户名变更）
    User::clear_caches(
        &[(target_user.id, Some(target_user.username.clone()))],
        &redis,
    )
    .await?;
    if review.review_type == "username" {
        // 同时清除新用户名的缓存键
        User::clear_caches(
            &[(target_user.id, Some(review.new_value.clone()))],
            &redis,
        )
        .await?;
    }

    // 事务提交成功后再删除旧的 S3 头像（尽力而为）
    // 仅当新旧头像不同时才删除旧图片，避免误删当前正在使用的头像
    if review.review_type == "avatar" {
        let new_avatar: Option<serde_json::Value> =
            serde_json::from_str(&review.new_value).ok();
        let new_url = new_avatar
            .as_ref()
            .and_then(|v| v.get("avatar_url"))
            .and_then(|v| v.as_str());

        let same_avatar = match (&old_avatar_url, new_url) {
            (Some(old), Some(new)) => old == new,
            (None, None) => true,
            _ => false,
        };

        if !same_avatar
            && let Err(e) = delete_old_images(
                old_avatar_url,
                old_raw_avatar_url,
                &***file_host,
            )
            .await
        {
            log::warn!(
                "审核通过后删除旧头像失败 (review_id={}): {}",
                review_id,
                e
            );
        }
    }

    Ok(HttpResponse::NoContent().body(""))
}

/// 管理员拒绝审核
/// POST /_internal/moderation/profile-reviews/{id}/reject
pub async fn reject_review(
    req: HttpRequest,
    info: web::Path<(i64,)>,
    body: web::Json<ReviewAction>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let moderator = crate::auth::check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    // 限制备注长度
    if let Some(ref notes) = body.notes
        && notes.len() > 500
    {
        return Err(ApiError::InvalidInput(
            "审核备注不能超过500个字符".to_string(),
        ));
    }

    let review_id = info.into_inner().0;

    // SELECT 在事务内 + FOR UPDATE 防止竞态
    let mut transaction = pool.begin().await?;

    let review = sqlx::query!(
        "SELECT id, user_id, review_type, new_value, status
         FROM user_profile_reviews
         WHERE id = $1 AND status = 'pending'
         FOR UPDATE",
        review_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(ApiError::NotFound)?;

    let target_user = User::get_id(
        crate::database::models::ids::UserId(review.user_id),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 更新审核记录
    sqlx::query!(
        "UPDATE user_profile_reviews
         SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3",
        moderator.id.0 as i64,
        body.notes.as_deref(),
        review_id,
    )
    .execute(&mut *transaction)
    .await?;

    // 发送通知
    let notification = NotificationBuilder {
        body: NotificationBody::ProfileReviewResult {
            review_id,
            review_type: review.review_type.clone(),
            status: "rejected".to_string(),
            review_notes: body.notes.clone(),
        },
    };
    notification
        .insert(
            crate::database::models::ids::UserId(review.user_id),
            &mut transaction,
            &redis,
        )
        .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除用户缓存
    User::clear_caches(
        &[(target_user.id, Some(target_user.username.clone()))],
        &redis,
    )
    .await?;

    // 事务提交后再删除 S3 新图片（尽力而为，不阻塞审核流程）
    // 仅当新旧头像不同时才删除，避免误删当前正在使用的头像
    if review.review_type == "avatar" {
        if let Ok(avatar_json) =
            serde_json::from_str::<serde_json::Value>(&review.new_value)
        {
            let avatar_url = avatar_json
                .get("avatar_url")
                .and_then(|v| v.as_str())
                .map(String::from);
            let raw_avatar_url = avatar_json
                .get("raw_avatar_url")
                .and_then(|v| v.as_str())
                .map(String::from);

            // 如果新头像和用户当前头像相同，跳过删除
            let same_avatar = match (&target_user.avatar_url, &avatar_url) {
                (Some(current), Some(new)) => current == new,
                (None, None) => true,
                _ => false,
            };

            if !same_avatar
                && let Err(e) =
                    delete_old_images(avatar_url, raw_avatar_url, &***file_host)
                        .await
            {
                log::warn!(
                    "拒绝审核时删除 S3 图片失败 (review_id={}): {}",
                    review_id,
                    e
                );
            }
        } else {
            log::warn!(
                "拒绝审核时解析 avatar JSON 失败 (review_id={}, raw={})",
                review_id,
                &review.new_value
            );
        }
    }

    // 发送飞书通知
    if let Err(e) = send_feishu_review_notification(
        &review.review_type,
        "rejected",
        &target_user.username,
        body.notes.as_deref(),
    )
    .await
    {
        log::warn!("发送飞书审核通知失败: {}", e);
    }

    Ok(HttpResponse::NoContent().body(""))
}

/// 管理员一键审批通过所有待审核
/// POST /_internal/moderation/profile-reviews/approve-all
pub async fn approve_all_pending(
    req: HttpRequest,
    body: web::Json<ReviewAction>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let moderator = crate::auth::check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    if let Some(ref notes) = body.notes
        && notes.len() > 500
    {
        return Err(ApiError::InvalidInput(
            "审核备注不能超过500个字符".to_string(),
        ));
    }

    // 单事务：JOIN 预取用户数据 + FOR UPDATE 锁定审核记录
    let mut transaction = pool.begin().await?;

    let pending_reviews = sqlx::query!(
        r#"
        SELECT pr.id, pr.user_id, pr.review_type, pr.new_value,
               u.username, u.avatar_url, u.raw_avatar_url
        FROM user_profile_reviews pr
        JOIN users u ON u.id = pr.user_id
        WHERE pr.status = 'pending'
        ORDER BY pr.created_at ASC
        FOR UPDATE OF pr
        "#,
    )
    .fetch_all(&mut *transaction)
    .await?;

    if pending_reviews.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "approved": 0,
            "failed": 0,
        })));
    }

    let mut approved_ids: Vec<i64> = Vec::new();
    let mut failed_count = 0i64;
    // 收集需要清除缓存的用户信息 (db_user_id, username)
    let mut cache_entries: Vec<(
        crate::database::models::ids::UserId,
        Option<String>,
    )> = Vec::new();
    // 收集需要删除的旧头像
    let mut old_avatars_to_delete: Vec<(Option<String>, Option<String>)> =
        Vec::new();

    for review in &pending_reviews {
        let db_user_id = crate::database::models::ids::UserId(review.user_id);

        match review.review_type.as_str() {
            "username" => {
                // 检查用户名冲突
                let conflict = sqlx::query_scalar!(
                    "SELECT id FROM users WHERE LOWER(username) = LOWER($1) AND id != $2",
                    &review.new_value,
                    review.user_id,
                )
                .fetch_optional(&mut *transaction)
                .await?;

                if conflict.is_some() {
                    // 冲突 → 标记为拒绝
                    sqlx::query!(
                        "UPDATE user_profile_reviews
                         SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW(),
                             review_notes = '批量审批：用户名在审核期间已被其他用户占用'
                         WHERE id = $2",
                        moderator.id.0 as i64,
                        review.id,
                    )
                    .execute(&mut *transaction)
                    .await?;

                    NotificationBuilder {
                        body: NotificationBody::ProfileReviewResult {
                            review_id: review.id,
                            review_type: "username".to_string(),
                            status: "rejected".to_string(),
                            review_notes: Some(
                                "批量审批：用户名在审核期间已被其他用户占用"
                                    .to_string(),
                            ),
                        },
                    }
                    .insert(db_user_id, &mut transaction, &redis)
                    .await?;

                    cache_entries
                        .push((db_user_id, Some(review.username.clone())));
                    failed_count += 1;
                    continue;
                }

                sqlx::query!(
                    "UPDATE users SET username = $1 WHERE id = $2",
                    &review.new_value,
                    review.user_id,
                )
                .execute(&mut *transaction)
                .await?;

                // 清除新旧两个用户名的缓存
                cache_entries.push((db_user_id, Some(review.username.clone())));
                cache_entries
                    .push((db_user_id, Some(review.new_value.clone())));
            }
            "bio" => {
                sqlx::query!(
                    "UPDATE users SET bio = $1 WHERE id = $2",
                    &review.new_value,
                    review.user_id,
                )
                .execute(&mut *transaction)
                .await?;

                cache_entries.push((db_user_id, Some(review.username.clone())));
            }
            "avatar" => {
                let new_avatar: serde_json::Value =
                    match serde_json::from_str(&review.new_value) {
                        Ok(v) => v,
                        Err(_) => {
                            log::warn!(
                                "批量审批: 无效的头像数据 (review_id={})",
                                review.id
                            );
                            failed_count += 1;
                            continue;
                        }
                    };

                let new_avatar_url =
                    new_avatar.get("avatar_url").and_then(|v| v.as_str());
                let new_raw_avatar_url =
                    new_avatar.get("raw_avatar_url").and_then(|v| v.as_str());

                if let (Some(avatar_url), Some(raw_url)) =
                    (new_avatar_url, new_raw_avatar_url)
                {
                    // 记录旧头像，事务提交后删除
                    let same = match (&review.avatar_url, new_avatar_url) {
                        (Some(old), Some(new)) => old == new,
                        (None, None) => true,
                        _ => false,
                    };
                    if !same {
                        old_avatars_to_delete.push((
                            review.avatar_url.clone(),
                            review.raw_avatar_url.clone(),
                        ));
                    }

                    sqlx::query!(
                        "UPDATE users SET avatar_url = $1, raw_avatar_url = $2 WHERE id = $3",
                        avatar_url,
                        raw_url,
                        review.user_id,
                    )
                    .execute(&mut *transaction)
                    .await?;

                    cache_entries
                        .push((db_user_id, Some(review.username.clone())));
                } else {
                    log::warn!(
                        "批量审批: 缺少头像字段 (review_id={})",
                        review.id
                    );
                    failed_count += 1;
                    continue;
                }
            }
            _ => {
                log::warn!(
                    "批量审批: 未知审核类型 {} (review_id={})",
                    review.review_type,
                    review.id
                );
                failed_count += 1;
                continue;
            }
        }

        approved_ids.push(review.id);
    }

    // 批量更新所有通过的审核记录
    if !approved_ids.is_empty() {
        sqlx::query!(
            "UPDATE user_profile_reviews
             SET status = 'approved', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
             WHERE id = ANY($3)",
            moderator.id.0 as i64,
            body.notes.as_deref(),
            &approved_ids,
        )
        .execute(&mut *transaction)
        .await?;

        // 批量插入通知
        for review in pending_reviews
            .iter()
            .filter(|r| approved_ids.contains(&r.id))
        {
            NotificationBuilder {
                body: NotificationBody::ProfileReviewResult {
                    review_id: review.id,
                    review_type: review.review_type.clone(),
                    status: "approved".to_string(),
                    review_notes: body.notes.clone(),
                },
            }
            .insert(
                crate::database::models::ids::UserId(review.user_id),
                &mut transaction,
                &redis,
            )
            .await?;
        }
    }

    let approved_count = approved_ids.len() as i64;

    transaction.commit().await?;

    // 事务提交后：批量清除缓存
    if !cache_entries.is_empty() {
        let _ = User::clear_caches(&cache_entries, &redis).await;
    }
    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 事务提交后：尽力清理旧头像
    for (old_url, old_raw) in old_avatars_to_delete {
        if let Err(e) = delete_old_images(old_url, old_raw, &***file_host).await
        {
            log::warn!("批量审批删除旧头像失败: {}", e);
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "approved": approved_count,
        "failed": failed_count,
    })))
}

/// 发送飞书审核通知
async fn send_feishu_review_notification(
    review_type: &str,
    status: &str,
    username: &str,
    notes: Option<&str>,
) -> Result<(), ApiError> {
    let feishu_bot_webhook = match dotenvy::var("FEISHU_BOT_WEBHOOK") {
        Ok(url) => url,
        Err(_) => return Ok(()), // 未配置则跳过
    };
    let client = reqwest::Client::builder().build()?;

    let type_display = match review_type {
        "avatar" => "头像",
        "username" => "用户名",
        "bio" => "简介",
        _ => "资料",
    };
    let status_display = match status {
        "approved" => "已通过",
        "rejected" => "已拒绝",
        _ => status,
    };
    let notes_text = notes.unwrap_or("无");

    let json_str = serde_json::json!({
        "msg_type": "text",
        "content": {
            "text": format!(
                "[资料审核] 用户 {} 的{}修改审核结果：{}，备注：{}",
                username, type_display, status_display, notes_text
            )
        }
    });

    let _ = client
        .post(feishu_bot_webhook)
        .header("Content-Type", "application/json")
        .json(&json_str)
        .send()
        .await;

    Ok(())
}
