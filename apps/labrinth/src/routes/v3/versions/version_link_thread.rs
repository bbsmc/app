use crate::auth::get_user_from_headers;
use crate::database;
use crate::database::models::thread_item::{
    ThreadBuilder, ThreadMessageBuilder,
};
use crate::database::redis::RedisPool;
use crate::models::ids::VersionId;
use crate::models::pats::Scopes;
use crate::models::threads::{MessageBody, ThreadType};
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct SendVersionLinkMessage {
    pub body: String,
}

// 发送消息到版本链接thread
pub async fn send_version_link_message(
    req: HttpRequest,
    info: web::Path<(VersionId, VersionId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    message_body: web::Json<SendVersionLinkMessage>,
) -> Result<HttpResponse, ApiError> {
    let (translation_version_id, target_version_id) = info.into_inner();

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::THREAD_WRITE]),
    )
    .await?
    .1;

    // 验证消息长度
    if message_body.body.len() > 65536 {
        return Err(ApiError::InvalidInput("消息内容过长!".to_string()));
    }

    let mut transaction = pool.begin().await?;

    // 转换ID类型
    let db_translation_id: database::models::ids::VersionId =
        translation_version_id.into();
    let db_target_id: database::models::ids::VersionId =
        target_version_id.into();

    // 查找版本链接及其thread
    let link_info = sqlx::query!(
        r#"
        SELECT vlv.thread_id, v1.mod_id as translation_project_id, v2.mod_id as original_project_id,
               m1.team_id as translation_team_id, m2.team_id as original_team_id
        FROM version_link_version vlv
        INNER JOIN versions v1 ON v1.id = vlv.version_id
        INNER JOIN versions v2 ON v2.id = vlv.joining_version_id
        INNER JOIN mods m1 ON m1.id = v1.mod_id
        INNER JOIN mods m2 ON m2.id = v2.mod_id
        WHERE vlv.version_id = $1 AND vlv.joining_version_id = $2
        "#,
        db_translation_id.0,
        db_target_id.0
    )
    .fetch_optional(&mut *transaction)
    .await?;

    let link = link_info
        .ok_or_else(|| ApiError::InvalidInput("版本链接不存在".to_string()))?;

    // 权限检查
    let user_id: database::models::ids::UserId = user.id.into();

    let is_translation_member = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        link.translation_team_id,
        user_id.0
    )
    .fetch_one(&mut *transaction)
    .await?
    .exists
    .unwrap_or(false);

    let is_original_member = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        link.original_team_id,
        user_id.0
    )
    .fetch_one(&mut *transaction)
    .await?
    .exists
    .unwrap_or(false);

    if !is_translation_member && !is_original_member && !user.role.is_mod() {
        return Err(ApiError::CustomAuthentication(
            "您无权向此版本链接发送消息".to_string(),
        ));
    }

    // 获取或创建thread
    let thread_id = if let Some(existing_thread_id) = link.thread_id {
        database::models::ids::ThreadId(existing_thread_id)
    } else {
        // 创建新thread
        let new_thread_id = ThreadBuilder {
            type_: ThreadType::VersionLink,
            members: vec![], // Version link threads不需要固定成员
            project_id: None,
            report_id: None,
            ban_appeal_id: None,
            creator_application_id: None,
        }
        .insert(&mut transaction)
        .await?;

        // 更新version_link_version表中的thread_id
        sqlx::query!(
            "UPDATE version_link_version SET thread_id = $1 WHERE version_id = $2 AND joining_version_id = $3",
            new_thread_id.0,
            db_translation_id.0,
            db_target_id.0
        )
        .execute(&mut *transaction)
        .await?;

        new_thread_id
    };

    // 插入消息
    let message_id = ThreadMessageBuilder {
        author_id: Some(user_id),
        body: MessageBody::Text {
            body: message_body.body.clone(),
            replying_to: None,
            private: false, // 版本链接thread中的消息都是公开的
            associated_images: vec![],
        },
        thread_id,
        hide_identity: false, // 版本链接thread中不隐藏身份
    }
    .insert(&mut transaction)
    .await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "thread_id": crate::models::ids::ThreadId::from(thread_id),
        "message_id": crate::models::ids::ThreadMessageId::from(message_id)
    })))
}
