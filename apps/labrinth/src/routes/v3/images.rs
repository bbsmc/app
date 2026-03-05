use std::sync::Arc;

use super::threads::is_authorized_thread;
use crate::auth::checks::{
    check_forum_ban, check_global_ban, check_resource_ban,
    is_team_member_project, is_team_member_version,
};
use crate::auth::get_user_from_headers;
use crate::database;
use crate::database::models::{
    project_item, report_item, thread_item, version_item,
};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::ids::{ThreadMessageId, VersionId};
use crate::models::images::{Image, ImageContext};
use crate::models::reports::ReportId;
use crate::models::threads::ThreadType;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::img::upload_image_optimized;
use crate::util::routes::read_from_payload;
use actix_web::{HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("image").route("", web::post().to(images_add)), // .route("/can_upload", web::get().to(images_can_upload)),
    );
    // cfg.route("image", web::post().to(images_add));
}

#[derive(Serialize, Deserialize)]
pub struct ImageUpload {
    pub ext: String,

    // Context 必须是允许的上下文
    // 当前：project, version, thread_message, report
    pub context: String,

    // 可选的上下文 ID 以关联
    pub project_id: Option<String>, // 允许 slug 或 id
    pub user_id: Option<String>,    // 允许 slug 或 id
    pub wiki_id: Option<i64>,       // 允许 slug 或 id
    pub version_id: Option<VersionId>,
    pub thread_message_id: Option<ThreadMessageId>,
    pub report_id: Option<ReportId>,
}
// pub async fn images_can_upload(
//     req: HttpRequest,
//     pool: web::Data<PgPool>,
//     redis: web::Data<RedisPool>,
//     session_queue: web::Data<AuthQueue>,
// ) -> Result<HttpResponse, ApiError> {
//     let user =
//         get_user_from_headers(&req, &**pool, &redis, &session_queue, None)
//             .await?
//             .1;

//     let mut transaction = pool.begin().await?;

//     let images = database::models::image_item::Image::get_user_images(
//         database::models::UserId::from(user.id),
//         &mut transaction,
//     )
//     .await?;

//     if images.len() >= 300 {
//         return Err(ApiError::ImageLimit(images.len() as u32, 300));
//     }

//     Ok(HttpResponse::Ok().json(true))
// }

pub async fn images_add(
    req: HttpRequest,
    web::Query(data): web::Query<ImageUpload>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    mut payload: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let mut context = ImageContext::from_str(&data.context, None);

    let scopes = vec![context.relevant_scope()];

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&scopes),
    )
    .await?
    .1;

    // 根据上下文类型检查封禁状态
    // - project/version: 检查资源类封禁
    // - thread_message/report: 检查论坛类封禁（申诉线程除外）
    // - user: 检查全局封禁
    match &context {
        ImageContext::Project { .. } | ImageContext::Version { .. } => {
            check_resource_ban(&user, &pool).await?;
        }
        ImageContext::ThreadMessage { .. } | ImageContext::Report { .. } => {
            // 论坛类封禁检查在后面处理（需要判断是否是申诉线程）
        }
        ImageContext::User { .. } => {
            check_global_ban(&user, &pool).await?;
        }
        ImageContext::Unknown => {}
    }

    // 尝试将提供的 ID 与上下文关联
    // 如果无法找到上下文，或者用户无权上传上下文的图片，则返回错误
    match &mut context {
        ImageContext::Project { project_id } => {
            if let Some(id) = data.project_id {
                let project =
                    project_item::Project::get(&id, &**pool, &redis).await?;
                if let Some(project) = project {
                    if is_team_member_project(
                        &project.inner,
                        &Some(user.clone()),
                        &pool,
                    )
                    .await?
                    {
                        *project_id = Some(project.inner.id.into());
                    } else {
                        return Err(ApiError::CustomAuthentication(
                            "您无权为此项目上传图片".to_string(),
                        ));
                    }
                } else {
                    return Err(ApiError::InvalidInput(
                        "未找到该项目。".to_string(),
                    ));
                }
            }
        }
        ImageContext::Version { version_id } => {
            if let Some(id) = data.version_id {
                let version =
                    version_item::Version::get(id.into(), &**pool, &redis)
                        .await?;
                if let Some(version) = version {
                    if is_team_member_version(
                        &version.inner,
                        &Some(user.clone()),
                        &pool,
                        &redis,
                    )
                    .await?
                    {
                        *version_id = Some(version.inner.id.into());
                    } else {
                        return Err(ApiError::CustomAuthentication(
                            "您无权为此版本上传图片".to_string(),
                        ));
                    }
                } else {
                    return Err(ApiError::InvalidInput(
                        "未找到该版本。".to_string(),
                    ));
                }
            }
        }
        ImageContext::ThreadMessage { thread_message_id } => {
            if let Some(id) = data.thread_message_id {
                let thread_message =
                    thread_item::ThreadMessage::get(id.into(), &**pool)
                        .await?
                        .ok_or_else(|| {
                            ApiError::InvalidInput("未找到该消息。".to_string())
                        })?;
                let thread =
                    thread_item::Thread::get(thread_message.thread_id, &**pool)
                        .await?
                        .ok_or_else(|| {
                            ApiError::InvalidInput(
                                "未找到与该消息关联的线程".to_string(),
                            )
                        })?;

                // 检查论坛封禁（申诉线程除外）
                if thread.type_ != ThreadType::BanAppeal {
                    check_forum_ban(&user, &pool).await?;
                }

                if is_authorized_thread(&thread, &user, &pool).await? {
                    *thread_message_id = Some(thread_message.id.into());
                } else {
                    return Err(ApiError::CustomAuthentication(
                        "您无权为此线程消息上传图片".to_string(),
                    ));
                }
            } else {
                // 没有指定 thread_message_id，检查论坛封禁
                check_forum_ban(&user, &pool).await?;
            }
        }
        ImageContext::Report { report_id } => {
            // 举报图片需要检查论坛封禁
            check_forum_ban(&user, &pool).await?;

            if let Some(id) = data.report_id {
                let report = report_item::Report::get(id.into(), &**pool)
                    .await?
                    .ok_or_else(|| {
                        ApiError::InvalidInput("未找到该举报。".to_string())
                    })?;
                let thread =
                    thread_item::Thread::get(report.thread_id, &**pool)
                        .await?
                        .ok_or_else(|| {
                            ApiError::InvalidInput(
                                "未找到与该举报关联的线程。".to_string(),
                            )
                        })?;
                if is_authorized_thread(&thread, &user, &pool).await? {
                    *report_id = Some(report.id.into());
                } else {
                    return Err(ApiError::CustomAuthentication(
                        "您无权为此举报上传图片".to_string(),
                    ));
                }
            }
        }

        ImageContext::User { user_id } => {
            if data.user_id.is_some() {
                let mut transaction = pool.begin().await?;
                let images =
                    database::models::image_item::Image::get_user_images(
                        database::models::UserId::from(user.id),
                        &mut transaction,
                    )
                    .await?;

                if images.len() >= 300 {
                    // println!("images.len() >= 3");
                    return Err(ApiError::ImageLimit(images.len() as u32, 300));
                }

                *user_id = None;
            } else {
                *user_id = None;
            }

            // transaction.commit().await?;
        }

        ImageContext::Unknown => {
            return Err(ApiError::InvalidInput(
               "Context 必须是以下之一：project, version, thread_message, wiki, report 或 user".to_string(),
            ));
        }
    }

    // 将图片上传到文件主机
    let bytes =
        read_from_payload(&mut payload, 4_194_304, "图片大小必须小于4MB")
            .await?;

    let content_length = bytes.len();
    // 跳过内置风控，手动检查
    let upload_result = upload_image_optimized(
        "data/cached_images",
        bytes.freeze(),
        &data.ext,
        None,
        None,
        &***file_host,
        crate::util::img::UploadImagePos {
            pos: "Markdown图片上传,具体使用地点未知".to_string(),
            url: format!("/user/{}", &user.username),
            username: user.username.clone(),
        },
        &redis,
        true,
    )
    .await?;

    // 同步风控检查（在保存到数据库之前）
    let uploader_id = database::models::UserId::from(user.id).0;
    let risk_result = crate::util::risk::check_image_risk_with_labels(
        &upload_result.url,
        &format!("/user/{}", &user.username),
        &user.username,
        "Markdown图片",
        &redis,
    )
    .await;

    // 涉政内容：直接拦截，删除 S3，不保存到数据库
    if let Ok(ref result) = risk_result
        && !result.passed
        && crate::util::risk::contains_political_labels(&result.labels)
    {
        super::image_reviews::handle_political_image_delete(
            &upload_result.url,
            Some(&upload_result.raw_url),
            result.frame_url.as_deref(),
            &user.username,
            uploader_id,
            &result.labels,
            "markdown",
            None, // source_id: 涉政不保存到 uploaded_images
            None,
            &pool,
            &***file_host,
        )
        .await;
        return Err(ApiError::InvalidInput(
            "图片内容违规，已被拦截".to_string(),
        ));
    }

    // 非涉政或风控通过：保存到数据库
    let mut transaction = pool.begin().await?;

    let db_image: database::models::Image = database::models::Image {
        id: database::models::generate_image_id(&mut transaction).await?,
        url: upload_result.url.clone(),
        raw_url: upload_result.raw_url.clone(),
        size: content_length as u64,
        created: chrono::Utc::now(),
        owner_id: database::models::UserId::from(user.id),
        context: context.context_as_str().to_string(),
        project_id: if let ImageContext::Project {
            project_id: Some(id),
        } = context
        {
            Some(crate::database::models::ProjectId::from(id))
        } else {
            None
        },
        version_id: if let ImageContext::Version {
            version_id: Some(id),
        } = context
        {
            Some(database::models::VersionId::from(id))
        } else {
            None
        },
        thread_message_id: if let ImageContext::ThreadMessage {
            thread_message_id: Some(id),
        } = context
        {
            Some(database::models::ThreadMessageId::from(id))
        } else {
            None
        },
        report_id: if let ImageContext::Report {
            report_id: Some(id),
        } = context
        {
            Some(database::models::ReportId::from(id))
        } else {
            None
        },
        user_id: if let ImageContext::User { user_id: Some(id) } = context {
            Some(database::models::UserId::from(id))
        } else {
            None
        },
    };

    db_image.insert(&mut transaction).await?;

    let image = Image {
        id: db_image.id.into(),
        url: db_image.url.clone(),
        size: db_image.size,
        created: db_image.created,
        owner_id: db_image.owner_id.into(),
        context,
    };

    transaction.commit().await?;

    // 非涉政风控未通过：创建审核记录（图片已保存，等待管理员审核）
    if let Ok(ref result) = risk_result
        && !result.passed
    {
        super::image_reviews::create_review_record(
            &upload_result.url,
            Some(&upload_result.raw_url),
            result.frame_url.as_deref(),
            uploader_id,
            &result.labels,
            "markdown",
            Some(db_image.id.0),
            None,
            &pool,
            &redis,
        )
        .await;
    }

    Ok(HttpResponse::Ok().json(image))
}
