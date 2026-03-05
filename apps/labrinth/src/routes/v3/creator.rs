//! 高级创作者申请路由（用户端）
//!
//! 提供用户提交和查看高级创作者申请的功能。
//! 管理员审核路由在 routes/internal/creator.rs 中。

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::LazyLock;
use validator::Validate;

use crate::auth::get_user_from_headers;

/// 身份证号正则表达式：17 位数字 + 1 位数字或 X/x
static ID_CARD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{17}[\dXx]$").expect("Invalid ID card regex")
});
use crate::database::models::UserId as DBUserId;
use crate::database::models::creator_application_item::{
    CreatorApplication, CreatorApplicationBuilder,
};
use crate::database::models::ids::CreatorApplicationId;
use crate::database::models::thread_item::{
    ThreadBuilder, ThreadMessageBuilder,
};
use crate::database::redis::RedisPool;
use crate::models::pats::Scopes;
use crate::models::threads::{MessageBody, ThreadType};
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::validate::validation_errors_to_string;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("user/creator")
            .route("apply", web::post().to(apply_creator))
            .route("application", web::get().to(get_my_application)),
    );
}

// ==================== 请求/响应结构 ====================

/// 高级创作者申请请求
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ApplyCreatorRequest {
    /// 真实姓名
    #[validate(length(
        min = 2,
        max = 100,
        message = "真实姓名长度必须在 2-100 个字符之间"
    ))]
    pub real_name: String,
    /// 联系方式（如 QQ、微信、手机号）
    #[validate(length(
        min = 5,
        max = 100,
        message = "联系方式长度必须在 5-100 个字符之间（QQ/微信/手机号）"
    ))]
    pub contact_info: String,
    /// 身份证号（必填）
    #[validate(length(min = 18, max = 18, message = "身份证号必须是 18 位"))]
    #[validate(regex(
        path = "*ID_CARD_REGEX",
        message = "身份证号格式无效，必须是 17 位数字加 1 位数字或 X"
    ))]
    pub id_card_number: Option<String>,
    /// 作品链接（可选，如 GitHub、之前的作品）
    #[validate(length(
        max = 500,
        message = "作品链接长度不能超过 500 个字符"
    ))]
    pub portfolio_links: Option<String>,
    /// 申请理由（可选）
    #[validate(length(
        max = 2000,
        message = "申请理由长度不能超过 2000 个字符"
    ))]
    pub application_reason: Option<String>,
}

/// 高级创作者申请响应
#[derive(Debug, Clone, Serialize)]
pub struct CreatorApplicationResponse {
    pub id: i64,
    pub status: String,
    pub real_name: String,
    pub contact_info: String,
    pub id_card_number: Option<String>,
    pub portfolio_links: Option<String>,
    pub application_reason: Option<String>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// 关联的对话线程 ID（用于与管理员沟通）
    pub thread_id: Option<String>,
}

impl From<CreatorApplication> for CreatorApplicationResponse {
    fn from(app: CreatorApplication) -> Self {
        Self {
            id: app.id,
            status: app.status.to_string(),
            real_name: app.real_name,
            contact_info: app.contact_info,
            id_card_number: app.id_card_number,
            portfolio_links: app.portfolio_links,
            application_reason: app.application_reason,
            review_note: app.review_note,
            created_at: app.created_at,
            reviewed_at: app.reviewed_at,
            thread_id: app
                .thread_id
                .map(|id| crate::models::ids::ThreadId::from(id).to_string()),
        }
    }
}

// ==================== 用户路由 ====================

/// 提交高级创作者申请
///
/// POST /v3/user/creator/apply
pub async fn apply_creator(
    req: HttpRequest,
    body: web::Json<ApplyCreatorRequest>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    body.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    // 检查用户是否已经是高级创作者
    let db_user =
        crate::database::models::User::get_id(db_user_id, &**pool, &redis)
            .await?
            .ok_or_else(|| ApiError::InvalidInput("用户不存在".to_string()))?;

    if db_user.is_premium_creator {
        return Err(ApiError::InvalidInput("您已经是高级创作者".to_string()));
    }

    // 检查是否有待处理的申请
    if CreatorApplication::has_pending_application(db_user_id, &**pool).await? {
        return Err(ApiError::InvalidInput(
            "您已有待处理的申请，请等待审核".to_string(),
        ));
    }

    // 创建申请
    let mut transaction = pool.begin().await?;

    let mut builder = CreatorApplicationBuilder::new(
        db_user_id,
        body.real_name.clone(),
        body.contact_info.clone(),
    );
    builder.id_card_number = body.id_card_number.clone();
    builder.portfolio_links = body.portfolio_links.clone();
    builder.application_reason = body.application_reason.clone();

    let application_id = builder.insert(&mut transaction).await?;

    // 创建关联的对话线程（用于与管理员沟通）
    let thread_id = ThreadBuilder {
        type_: ThreadType::CreatorApplication,
        members: vec![db_user_id], // 申请人加入线程
        project_id: None,
        report_id: None,
        ban_appeal_id: None,
        creator_application_id: Some(CreatorApplicationId(application_id)),
    }
    .insert(&mut transaction)
    .await?;

    // 将申请理由作为第一条消息添加到线程中
    let initial_message =
        body.application_reason.clone().unwrap_or_else(|| {
            format!(
                "我申请成为高级创作者。\n姓名：{}\n联系方式：{}",
                body.real_name, body.contact_info
            )
        });

    ThreadMessageBuilder {
        author_id: Some(db_user_id),
        body: MessageBody::Text {
            body: initial_message,
            private: false,
            replying_to: None,
            associated_images: vec![],
        },
        thread_id,
        hide_identity: false,
    }
    .insert(&mut transaction)
    .await?;

    // 更新申请记录关联线程
    CreatorApplication::set_thread_id(
        application_id,
        thread_id,
        &mut transaction,
    )
    .await?;

    transaction.commit().await?;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    // 获取新创建的申请
    let application = CreatorApplication::get_by_id(application_id, &**pool)
        .await?
        .ok_or_else(|| ApiError::InvalidInput("申请创建失败".to_string()))?;

    Ok(HttpResponse::Created()
        .json(CreatorApplicationResponse::from(application)))
}

/// 获取当前用户的申请状态
///
/// GET /v3/user/creator/application
pub async fn get_my_application(
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
        Some(&[Scopes::USER_READ]),
    )
    .await?
    .1;

    let db_user_id = DBUserId(user.id.0 as i64);

    let application =
        CreatorApplication::get_by_user(db_user_id, &**pool).await?;

    match application {
        Some(app) => {
            Ok(HttpResponse::Ok().json(CreatorApplicationResponse::from(app)))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "description": "您还没有提交过申请"
        }))),
    }
}
