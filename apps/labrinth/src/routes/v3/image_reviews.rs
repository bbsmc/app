use actix_web::{HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use super::ApiError;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::{self as db_models};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::ids::random_base62;
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::util::img::delete_old_images;

#[derive(Deserialize)]
pub struct ImageReviewListQuery {
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
    &["all", "pending", "approved", "rejected", "auto_deleted"];

#[derive(Serialize)]
pub struct ImageReviewItem {
    pub id: i64,
    pub image_url: String,
    pub raw_image_url: Option<String>,
    /// 火山引擎风控缓存的图片 URL（S3 删除后仍可查看）
    pub risk_image_url: Option<String>,
    pub uploader_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub source_type: String,
    pub source_id: Option<i64>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub risk_labels: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub review_notes: Option<String>,
}

/// 管理员获取图片审核列表
/// GET /_internal/moderation/image-reviews
pub async fn list_image_reviews(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    query: web::Query<ImageReviewListQuery>,
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

    if !VALID_STATUSES.contains(&query.status.as_str()) {
        return Err(ApiError::InvalidInput(format!(
            "无效的状态参数: {}",
            query.status
        )));
    }

    let count = query.count.min(500);

    let rows = sqlx::query!(
        r#"
        SELECT r.id, r.image_url, r.raw_image_url, r.risk_image_url,
               r.uploader_id, r.source_type, r.source_id, r.project_id,
               r.risk_labels, r.status, r.created_at,
               r.reviewed_by, r.reviewed_at, r.review_notes,
               u.username, u.avatar_url,
               m.name as "project_name?"
        FROM image_content_reviews r
        JOIN users u ON u.id = r.uploader_id
        LEFT JOIN mods m ON m.id = r.project_id
        WHERE ($1 = 'all' OR r.status = $1)
        ORDER BY r.created_at DESC
        LIMIT $2
        "#,
        &query.status,
        count,
    )
    .fetch_all(&**pool)
    .await?;

    let items: Vec<ImageReviewItem> = rows
        .into_iter()
        .map(|row| ImageReviewItem {
            id: row.id,
            image_url: row.image_url,
            raw_image_url: row.raw_image_url,
            risk_image_url: row.risk_image_url,
            uploader_id: crate::models::ids::UserId::from(
                crate::database::models::ids::UserId(row.uploader_id),
            )
            .to_string(),
            username: row.username,
            avatar_url: row.avatar_url,
            source_type: row.source_type,
            source_id: row.source_id,
            project_id: row.project_id.map(|id| {
                crate::models::ids::ProjectId::from(
                    crate::database::models::ids::ProjectId(id),
                )
                .to_string()
            }),
            project_name: row.project_name,
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

/// 管理员批准图片审核
/// POST /_internal/moderation/image-reviews/{id}/approve
pub async fn approve_image_review(
    req: HttpRequest,
    info: web::Path<(i64,)>,
    body: web::Json<ReviewAction>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
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

    let review_id = info.into_inner().0;

    let mut transaction = pool.begin().await?;

    let _review = sqlx::query!(
        "SELECT id FROM image_content_reviews
         WHERE id = $1 AND status = 'pending'
         FOR UPDATE",
        review_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(ApiError::NotFound)?;

    sqlx::query!(
        "UPDATE image_content_reviews
         SET status = 'approved', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3",
        moderator.id.0 as i64,
        body.notes.as_deref(),
        review_id,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::NoContent().body(""))
}

/// 管理员拒绝图片审核（删除图片）
/// POST /_internal/moderation/image-reviews/{id}/reject
pub async fn reject_image_review(
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

    if let Some(ref notes) = body.notes
        && notes.len() > 500
    {
        return Err(ApiError::InvalidInput(
            "审核备注不能超过500个字符".to_string(),
        ));
    }

    let review_id = info.into_inner().0;

    let mut transaction = pool.begin().await?;

    let review = sqlx::query!(
        "SELECT id, image_url, raw_image_url, uploader_id,
                source_type, source_id, project_id, status
         FROM image_content_reviews
         WHERE id = $1 AND status = 'pending'
         FOR UPDATE",
        review_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(ApiError::NotFound)?;

    // 根据来源类型删除图片记录
    match review.source_type.as_str() {
        "markdown" => {
            if let Some(source_id) = review.source_id {
                sqlx::query!(
                    "DELETE FROM uploaded_images WHERE id = $1",
                    source_id,
                )
                .execute(&mut *transaction)
                .await?;
            }
        }
        "gallery" => {
            if let Some(project_id) = review.project_id {
                sqlx::query!(
                    "DELETE FROM mods_gallery WHERE image_url = $1 AND mod_id = $2",
                    &review.image_url,
                    project_id,
                )
                .execute(&mut *transaction)
                .await?;
            }
        }
        "avatar" => {
            // 拒绝头像审核：清空用户头像（仅当头像未被更换时）
            let result = sqlx::query!(
                "UPDATE users SET avatar_url = NULL, raw_avatar_url = NULL
                 WHERE id = $1 AND avatar_url = $2",
                review.uploader_id,
                &review.image_url,
            )
            .execute(&mut *transaction)
            .await?;
            if result.rows_affected() == 0 {
                log::info!(
                    "拒绝头像审核但用户已更换头像，跳过清空 (user_id={}, review_image={})",
                    review.uploader_id,
                    &review.image_url
                );
            }
            // 清除用户缓存
            if let Err(e) = db_models::User::clear_caches(
                &[(db_models::ids::UserId(review.uploader_id), None)],
                &redis,
            )
            .await
            {
                log::warn!("拒绝头像审核后清除用户缓存失败: {}", e);
            }
        }
        _ => {}
    }

    // 更新审核记录
    sqlx::query!(
        "UPDATE image_content_reviews
         SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3",
        moderator.id.0 as i64,
        body.notes.as_deref(),
        review_id,
    )
    .execute(&mut *transaction)
    .await?;

    // 发送通知给上传者
    let notification = NotificationBuilder {
        body: NotificationBody::ImageReviewResult {
            review_id,
            source_type: review.source_type.clone(),
            status: "rejected".to_string(),
            review_notes: body.notes.clone(),
        },
    };
    notification
        .insert(
            crate::database::models::ids::UserId(review.uploader_id),
            &mut transaction,
            &redis,
        )
        .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 清除项目缓存（gallery 类型，尽力而为，事务已提交）
    if review.source_type == "gallery"
        && let Some(project_id) = review.project_id
        && let Err(e) =
            clear_project_cache_by_id(project_id, &pool, &redis).await
    {
        log::warn!(
            "拒绝图片审核后清除项目缓存失败 (review_id={}, project_id={}): {}",
            review_id,
            project_id,
            e
        );
    }

    // 删除 S3 文件（尽力而为）
    if let Err(e) = delete_old_images(
        Some(review.image_url.clone()),
        review.raw_image_url.clone(),
        &***file_host,
    )
    .await
    {
        log::warn!(
            "拒绝图片审核时删除 S3 图片失败 (review_id={}): {}",
            review_id,
            e
        );
    }

    // 发送飞书通知
    if let Err(e) = send_feishu_image_review_notification(
        &review.source_type,
        "rejected",
        &review.image_url,
        body.notes.as_deref(),
    )
    .await
    {
        log::warn!("发送飞书图片审核通知失败: {}", e);
    }

    Ok(HttpResponse::NoContent().body(""))
}

/// 清除项目缓存（尽力而为）
async fn clear_project_cache_by_id(
    project_id: i64,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    let project = db_models::Project::get_id(
        db_models::ids::ProjectId(project_id),
        pool,
        redis,
    )
    .await?;
    if let Some(project) = project {
        db_models::Project::clear_cache(
            project.inner.id,
            project.inner.slug,
            None,
            redis,
        )
        .await?;
    }
    Ok(())
}

// ============================================================
// 共享的图片风控处理函数（同步调用，在返回响应前执行）
// 供 images.rs / projects.rs / project_creation.rs 调用
// ============================================================

/// 涉政图片处理：删除 S3 文件 + 创建审计记录
///
/// 在风控检查发现涉政内容后调用。上传的图片不应保存到数据库，
/// 需要删除已上传的 S3 文件并创建审计记录（使用 Volcengine 的 frame URL）。
#[allow(clippy::too_many_arguments)]
pub async fn handle_political_image_delete(
    s3_image_url: &str,
    s3_raw_image_url: Option<&str>,
    risk_image_url: Option<&str>,
    username: &str,
    uploader_id: i64,
    labels: &str,
    source_type: &str,
    source_id: Option<i64>,
    project_id: Option<i64>,
    pool: &PgPool,
    file_host: &(dyn FileHost + Send + Sync),
) {
    log::error!(
        "[POLITICAL_IMAGE_DELETED] source={}, url={}, user={}, labels={}",
        source_type,
        s3_image_url,
        username,
        labels
    );

    // 删除 S3 文件
    if let Err(e) = delete_old_images(
        Some(s3_image_url.to_string()),
        s3_raw_image_url.map(|s| s.to_string()),
        file_host,
    )
    .await
    {
        log::error!("涉政图片删除S3文件失败 (url={}): {}", s3_image_url, e);
    }

    // 创建审计记录（status = auto_deleted），使用 risk_image_url 作为可查看的图片
    let review_id = random_base62(8) as i64;
    if let Err(e) = sqlx::query!(
        "INSERT INTO image_content_reviews
         (id, image_url, raw_image_url, risk_image_url, uploader_id, source_type, source_id, project_id, risk_labels, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'auto_deleted')",
        review_id,
        s3_image_url,
        s3_raw_image_url,
        risk_image_url,
        uploader_id,
        source_type,
        source_id,
        project_id,
        labels,
    )
    .execute(pool)
    .await
    {
        log::error!("创建涉政图片审计记录失败: {}", e);
    }
}

/// 创建普通风控审核记录（非涉政，图片保留，等待管理员审核）
#[allow(clippy::too_many_arguments)]
pub async fn create_review_record(
    image_url: &str,
    raw_image_url: Option<&str>,
    risk_image_url: Option<&str>,
    uploader_id: i64,
    labels: &str,
    source_type: &str,
    source_id: Option<i64>,
    project_id: Option<i64>,
    pool: &PgPool,
    redis: &crate::database::redis::RedisPool,
) {
    let review_id = random_base62(8) as i64;
    if let Err(e) = sqlx::query!(
        "INSERT INTO image_content_reviews
         (id, image_url, raw_image_url, risk_image_url, uploader_id, source_type, source_id, project_id, risk_labels)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        review_id,
        image_url,
        raw_image_url,
        risk_image_url,
        uploader_id,
        source_type,
        source_id,
        project_id,
        labels,
    )
    .execute(pool)
    .await
    {
        log::error!("创建图片审核记录失败: {}", e);
    } else {
        crate::routes::internal::moderation::clear_pending_counts_cache(redis).await;
    }
}

/// 发送飞书图片审核通知
async fn send_feishu_image_review_notification(
    source_type: &str,
    status: &str,
    image_url: &str,
    notes: Option<&str>,
) -> Result<(), ApiError> {
    let feishu_bot_webhook = match dotenvy::var("FEISHU_BOT_WEBHOOK") {
        Ok(url) => url,
        Err(_) => return Ok(()),
    };
    let client = reqwest::Client::builder().build()?;

    let type_display = match source_type {
        "markdown" => "Markdown图片",
        "gallery" => "项目渲染图",
        "avatar" => "用户头像",
        _ => "图片",
    };
    let status_display = match status {
        "approved" => "已通过",
        "rejected" => "已拒绝（图片已删除）",
        _ => status,
    };
    let notes_text = notes.unwrap_or("无");

    let json_str = serde_json::json!({
        "msg_type": "text",
        "content": {
            "text": format!(
                "[图片审核] {}审核结果：{}，图片：{}，备注：{}",
                type_display, status_display, image_url, notes_text
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
