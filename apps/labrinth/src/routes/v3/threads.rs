use std::sync::Arc;

use crate::auth::{check_forum_ban, get_user_from_headers};
use crate::database;
use crate::database::models::image_item;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::thread_item::ThreadMessageBuilder;
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::ids::ThreadMessageId;
use crate::models::images::{Image, ImageContext};
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::models::projects::ProjectStatus;
use crate::models::threads::{MessageBody, Thread, ThreadId, ThreadType};
use crate::models::users::User;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use actix_web::{HttpRequest, HttpResponse, web};
use futures::TryStreamExt;
use serde::Deserialize;
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("thread")
            .route("{id}", web::get().to(thread_get))
            .route("{id}", web::post().to(thread_send_message)),
    );
    cfg.service(
        web::scope("message").route("{id}", web::delete().to(message_delete)),
    );
    cfg.route("threads", web::get().to(threads_get));
}

pub async fn is_authorized_thread(
    thread: &database::models::Thread,
    user: &User,
    pool: &PgPool,
) -> Result<bool, ApiError> {
    if user.role.is_mod() {
        return Ok(true);
    }

    let user_id: database::models::UserId = user.id.into();
    Ok(match thread.type_ {
        ThreadType::Report => {
            if let Some(report_id) = thread.report_id {
                let report_exists = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM reports WHERE id = $1 AND reporter = $2)",
                    report_id as database::models::ids::ReportId,
                    user_id as database::models::ids::UserId,
                )
                .fetch_one(pool)
                .await?
                .exists;

                report_exists.unwrap_or(false)
            } else {
                false
            }
        }
        ThreadType::Project => {
            if let Some(project_id) = thread.project_id {
                let project_exists = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM mods m INNER JOIN team_members tm ON tm.team_id = m.team_id AND tm.user_id = $2 WHERE m.id = $1)",
                    project_id as database::models::ids::ProjectId,
                    user_id as database::models::ids::UserId,
                )
                    .fetch_one(pool)
                    .await?
                    .exists;

                if !project_exists.unwrap_or(false) {
                    let org_exists = sqlx::query!(
                        "SELECT EXISTS(SELECT 1 FROM mods m INNER JOIN organizations o ON m.organization_id = o.id INNER JOIN team_members tm ON tm.team_id = o.team_id AND tm.user_id = $2 WHERE m.id = $1)",
                        project_id as database::models::ids::ProjectId,
                        user_id as database::models::ids::UserId,
                    )
                        .fetch_one(pool)
                        .await?
                        .exists;

                    org_exists.unwrap_or(false)
                } else {
                    true
                }
            } else {
                false
            }
        }
        ThreadType::DirectMessage => thread.members.contains(&user_id),
        ThreadType::VersionLink => {
            // 获取版本链接信息
            let link_info = sqlx::query!(
                r#"
                SELECT vlv.version_id, vlv.joining_version_id, v1.mod_id as translation_project_id, v2.mod_id as original_project_id
                FROM version_link_version vlv
                INNER JOIN versions v1 ON v1.id = vlv.version_id
                INNER JOIN versions v2 ON v2.id = vlv.joining_version_id
                WHERE vlv.thread_id = $1
                "#,
                thread.id.0
            )
            .fetch_optional(pool)
            .await?;

            if let Some(link) = link_info {
                // 检查用户是否是翻译项目的成员
                let translation_member = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM mods m INNER JOIN team_members tm ON tm.team_id = m.team_id AND tm.user_id = $2 WHERE m.id = $1)",
                    link.translation_project_id,
                    user_id as database::models::ids::UserId,
                )
                .fetch_one(pool)
                .await?
                .exists
                .unwrap_or(false);

                if translation_member {
                    return Ok(true);
                }

                // 检查用户是否是原项目的成员

                sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM mods m INNER JOIN team_members tm ON tm.team_id = m.team_id AND tm.user_id = $2 WHERE m.id = $1)",
                    link.original_project_id,
                    user_id as database::models::ids::UserId,
                )
                .fetch_one(pool)
                .await?
                .exists
                .unwrap_or(false)
            } else {
                false
            }
        }
        ThreadType::BanAppeal => {
            // 封禁申诉线程：申诉人可以访问
            if let Some(ban_appeal_id) = thread.ban_appeal_id {
                let appeal_exists = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM user_ban_appeals WHERE id = $1 AND user_id = $2)",
                    ban_appeal_id as database::models::ids::BanAppealId,
                    user_id as database::models::ids::UserId,
                )
                .fetch_one(pool)
                .await?
                .exists;

                appeal_exists.unwrap_or(false)
            } else {
                false
            }
        }
        ThreadType::CreatorApplication => {
            // 创作者申请线程：申请人可以访问
            if let Some(creator_application_id) = thread.creator_application_id
            {
                let application_exists = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM creator_applications WHERE id = $1 AND user_id = $2)",
                    creator_application_id as database::models::ids::CreatorApplicationId,
                    user_id as database::models::ids::UserId,
                )
                .fetch_one(pool)
                .await?
                .exists;

                application_exists.unwrap_or(false)
            } else {
                false
            }
        }
    })
}

pub async fn filter_authorized_threads(
    threads: Vec<database::models::Thread>,
    user: &User,
    pool: &web::Data<PgPool>,
    redis: &RedisPool,
) -> Result<Vec<Thread>, ApiError> {
    let user_id: database::models::UserId = user.id.into();

    let mut return_threads = Vec::new();
    let mut check_threads = Vec::new();

    for thread in threads {
        if user.role.is_mod()
            || (thread.type_ == ThreadType::DirectMessage
                && thread.members.contains(&user_id))
        {
            return_threads.push(thread);
        } else {
            check_threads.push(thread);
        }
    }

    if !check_threads.is_empty() {
        let project_thread_ids = check_threads
            .iter()
            .filter(|x| x.type_ == ThreadType::Project)
            .flat_map(|x| x.project_id.map(|x| x.0))
            .collect::<Vec<_>>();

        if !project_thread_ids.is_empty() {
            sqlx::query!(
                "
                SELECT m.id FROM mods m
                INNER JOIN team_members tm ON tm.team_id = m.team_id AND user_id = $2
                WHERE m.id = ANY($1)
                ",
                &*project_thread_ids,
                user_id as database::models::ids::UserId,
            )
            .fetch(&***pool)
            .map_ok(|row| {
                check_threads.retain(|x| {
                    let bool = x.project_id.map(|x| x.0) == Some(row.id);

                    if bool {
                        return_threads.push(x.clone());
                    }

                    !bool
                });
            })
            .try_collect::<Vec<()>>()
            .await?;
        }

        let org_project_thread_ids = check_threads
            .iter()
            .filter(|x| x.type_ == ThreadType::Project)
            .flat_map(|x| x.project_id.map(|x| x.0))
            .collect::<Vec<_>>();

        if !org_project_thread_ids.is_empty() {
            sqlx::query!(
                "
                SELECT m.id FROM mods m
                INNER JOIN organizations o ON o.id = m.organization_id
                INNER JOIN team_members tm ON tm.team_id = o.team_id AND user_id = $2
                WHERE m.id = ANY($1)
                ",
                &*project_thread_ids,
                user_id as database::models::ids::UserId,
            )
            .fetch(&***pool)
            .map_ok(|row| {
                check_threads.retain(|x| {
                    let bool = x.project_id.map(|x| x.0) == Some(row.id);

                    if bool {
                        return_threads.push(x.clone());
                    }

                    !bool
                });
            })
            .try_collect::<Vec<()>>()
            .await?;
        }

        let report_thread_ids = check_threads
            .iter()
            .filter(|x| x.type_ == ThreadType::Report)
            .flat_map(|x| x.report_id.map(|x| x.0))
            .collect::<Vec<_>>();

        if !report_thread_ids.is_empty() {
            sqlx::query!(
                "
                SELECT id FROM reports
                WHERE id = ANY($1) AND reporter = $2
                ",
                &*report_thread_ids,
                user_id as database::models::ids::UserId,
            )
            .fetch(&***pool)
            .map_ok(|row| {
                check_threads.retain(|x| {
                    let bool = x.report_id.map(|x| x.0) == Some(row.id);

                    if bool {
                        return_threads.push(x.clone());
                    }

                    !bool
                });
            })
            .try_collect::<Vec<()>>()
            .await?;
        }

        // 处理 BanAppeal 类型的线程
        let ban_appeal_thread_ids = check_threads
            .iter()
            .filter(|x| x.type_ == ThreadType::BanAppeal)
            .flat_map(|x| x.ban_appeal_id.map(|x| x.0))
            .collect::<Vec<_>>();

        if !ban_appeal_thread_ids.is_empty() {
            sqlx::query!(
                "
                SELECT id FROM user_ban_appeals
                WHERE id = ANY($1) AND user_id = $2
                ",
                &*ban_appeal_thread_ids,
                user_id as database::models::ids::UserId,
            )
            .fetch(&***pool)
            .map_ok(|row| {
                check_threads.retain(|x| {
                    let bool = x.ban_appeal_id.map(|x| x.0) == Some(row.id);

                    if bool {
                        return_threads.push(x.clone());
                    }

                    !bool
                });
            })
            .try_collect::<Vec<()>>()
            .await?;
        }

        // 处理 CreatorApplication 类型的线程
        let creator_application_thread_ids = check_threads
            .iter()
            .filter(|x| x.type_ == ThreadType::CreatorApplication)
            .flat_map(|x| x.creator_application_id.map(|x| x.0))
            .collect::<Vec<_>>();

        if !creator_application_thread_ids.is_empty() {
            sqlx::query!(
                "
                SELECT id FROM creator_applications
                WHERE id = ANY($1) AND user_id = $2
                ",
                &*creator_application_thread_ids,
                user_id as database::models::ids::UserId,
            )
            .fetch(&***pool)
            .map_ok(|row| {
                check_threads.retain(|x| {
                    let bool =
                        x.creator_application_id.map(|x| x.0) == Some(row.id);

                    if bool {
                        return_threads.push(x.clone());
                    }

                    !bool
                });
            })
            .try_collect::<Vec<()>>()
            .await?;
        }
    }

    let mut user_ids = return_threads
        .iter()
        .flat_map(|x| x.members.clone())
        .collect::<Vec<database::models::UserId>>();
    user_ids.append(
        &mut return_threads
            .iter()
            .flat_map(|x| {
                x.messages
                    .iter()
                    .filter_map(|x| x.author_id)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<database::models::UserId>>(),
    );

    let users: Vec<User> =
        database::models::User::get_many_ids(&user_ids, &***pool, redis)
            .await?
            .into_iter()
            .map(From::from)
            .collect();

    let mut final_threads = Vec::new();

    for thread in return_threads {
        let mut authors = thread.members.clone();

        authors.append(
            &mut thread
                .messages
                .iter()
                .filter_map(|x| {
                    if x.hide_identity && !user.role.is_mod() {
                        None
                    } else {
                        x.author_id
                    }
                })
                .collect::<Vec<_>>(),
        );

        final_threads.push(Thread::from(
            thread,
            users
                .iter()
                .filter(|x| authors.contains(&x.id.into()))
                .cloned()
                .collect(),
            user,
        ));
    }

    Ok(final_threads)
}

pub async fn thread_get(
    req: HttpRequest,
    info: web::Path<(ThreadId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0.into();

    let thread_data = database::models::Thread::get(string, &**pool).await?;

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::THREAD_READ]),
    )
    .await?
    .1;

    if let Some(mut data) = thread_data
        && is_authorized_thread(&data, &user, &pool).await?
    {
        let authors = &mut data.members;

        authors.append(
            &mut data
                .messages
                .iter()
                .filter_map(|x| {
                    if x.hide_identity && !user.role.is_mod() {
                        None
                    } else {
                        x.author_id
                    }
                })
                .collect::<Vec<_>>(),
        );

        let users: Vec<User> =
            database::models::User::get_many_ids(authors, &**pool, &redis)
                .await?
                .into_iter()
                .map(From::from)
                .collect();

        return Ok(HttpResponse::Ok().json(Thread::from(data, users, &user)));
    }
    Err(ApiError::NotFound)
}

#[derive(Deserialize)]
pub struct ThreadIds {
    pub ids: String,
}

pub async fn threads_get(
    req: HttpRequest,
    web::Query(ids): web::Query<ThreadIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::THREAD_READ]),
    )
    .await?
    .1;

    let thread_ids: Vec<database::models::ids::ThreadId> =
        serde_json::from_str::<Vec<ThreadId>>(&ids.ids)?
            .into_iter()
            .map(|x| x.into())
            .collect();

    let threads_data =
        database::models::Thread::get_many(&thread_ids, &**pool).await?;

    let threads =
        filter_authorized_threads(threads_data, &user, &pool, &redis).await?;

    Ok(HttpResponse::Ok().json(threads))
}

#[derive(Deserialize)]
pub struct NewThreadMessage {
    pub body: MessageBody,
}

pub async fn thread_send_message(
    req: HttpRequest,
    info: web::Path<(ThreadId,)>,
    pool: web::Data<PgPool>,
    new_message: web::Json<NewThreadMessage>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::THREAD_WRITE]),
    )
    .await?
    .1;

    let string: database::models::ThreadId = info.into_inner().0.into();

    // 先获取线程，检查是否是申诉线程
    let thread_for_ban_check =
        database::models::Thread::get(string, &**pool).await?;

    // 如果不是申诉线程或创作者申请线程，检查用户是否被论坛类封禁
    // 申诉线程例外：被封禁的用户仍需能够与管理员沟通申诉
    // 创作者申请线程例外：被封禁用户仍需能与管理员沟通申请事宜
    if let Some(ref thread) = thread_for_ban_check {
        let bypass_ban_check = matches!(
            thread.type_,
            crate::models::threads::ThreadType::BanAppeal
                | crate::models::threads::ThreadType::CreatorApplication
        );
        if !bypass_ban_check {
            check_forum_ban(&user, &pool).await?;
        }
    } else {
        // 线程不存在，稍后会返回 NotFound
        check_forum_ban(&user, &pool).await?;
    }

    if let MessageBody::Text {
        body,
        replying_to,
        private,
        ..
    } = &new_message.body
    {
        if body.len() > 65536 {
            return Err(ApiError::InvalidInput("输入内容过长!".to_string()));
        }

        if *private && !user.role.is_mod() {
            return Err(ApiError::InvalidInput(
                "您不允许发送私密消息!".to_string(),
            ));
        }

        if let Some(replying_to) = replying_to {
            let thread_message = database::models::ThreadMessage::get(
                (*replying_to).into(),
                &**pool,
            )
            .await?;

            if let Some(thread_message) = thread_message {
                if thread_message.thread_id != string {
                    return Err(ApiError::InvalidInput(
                        "回复的消息来自另一个线程!".to_string(),
                    ));
                }
            } else {
                return Err(ApiError::InvalidInput(
                    "回复的消息不存在!".to_string(),
                ));
            }
        }
    } else {
        return Err(ApiError::InvalidInput(
            "您只能通过此路由发送文本消息!".to_string(),
        ));
    }

    // 使用之前获取的线程，避免重复查询
    if let Some(thread) = thread_for_ban_check {
        if !is_authorized_thread(&thread, &user, &pool).await? {
            return Err(ApiError::NotFound);
        }

        let mut transaction = pool.begin().await?;

        let id = ThreadMessageBuilder {
            author_id: Some(user.id.into()),
            body: new_message.body.clone(),
            thread_id: thread.id,
            hide_identity: user.role.is_mod(),
        }
        .insert(&mut transaction)
        .await?;

        if let Some(project_id) = thread.project_id {
            let project =
                database::models::Project::get_id(project_id, &**pool, &redis)
                    .await?;

            if let Some(project) = project
                && project.inner.status != ProjectStatus::Processing
                && user.role.is_mod()
            {
                let members = database::models::TeamMember::get_from_team_full(
                    project.inner.team_id,
                    &**pool,
                    &redis,
                )
                .await?;

                NotificationBuilder {
                    body: NotificationBody::ModeratorMessage {
                        thread_id: thread.id.into(),
                        message_id: id.into(),
                        project_id: Some(project.inner.id.into()),
                        report_id: None,
                    },
                }
                .insert_many(
                    members.into_iter().map(|x| x.user_id).collect(),
                    &mut transaction,
                    &redis,
                )
                .await?;
            }
        } else if let Some(report_id) = thread.report_id {
            let report =
                database::models::report_item::Report::get(report_id, &**pool)
                    .await?;

            if let Some(report) = report {
                if report.closed && !user.role.is_mod() {
                    return Err(ApiError::InvalidInput(
                        "您不能回复已关闭的举报".to_string(),
                    ));
                }

                if user.id != report.reporter.into() {
                    NotificationBuilder {
                        body: NotificationBody::ModeratorMessage {
                            thread_id: thread.id.into(),
                            message_id: id.into(),
                            project_id: None,
                            report_id: Some(report.id.into()),
                        },
                    }
                    .insert(report.reporter, &mut transaction, &redis)
                    .await?;
                }
            }
        } else if let Some(ban_appeal_id) = thread.ban_appeal_id {
            // 申诉线程：管理员回复时通知用户
            if user.role.is_mod() {
                // 获取申诉信息，找到申诉用户
                let appeal = sqlx::query!(
                    "SELECT user_id FROM user_ban_appeals WHERE id = $1",
                    ban_appeal_id.0
                )
                .fetch_optional(&**pool)
                .await?;

                if let Some(appeal) = appeal {
                    let appeal_user_id =
                        database::models::ids::UserId(appeal.user_id);
                    // 只有当发送者不是申诉用户本人时才发送通知
                    if user.id != appeal_user_id.into() {
                        NotificationBuilder {
                            body: NotificationBody::BanAppealMessage {
                                appeal_id: crate::models::v3::bans::BanAppealId(
                                    ban_appeal_id.0 as u64,
                                ),
                                thread_id: thread.id.into(),
                                message_id: id.into(),
                            },
                        }
                        .insert(appeal_user_id, &mut transaction, &redis)
                        .await?;
                    }
                }
            }
        } else if let Some(creator_application_id) =
            thread.creator_application_id
        {
            // 创作者申请线程
            let application = sqlx::query!(
                "SELECT user_id FROM creator_applications WHERE id = $1",
                creator_application_id.0
            )
            .fetch_optional(&**pool)
            .await?;

            if let Some(app) = application {
                let app_user_id = database::models::ids::UserId(app.user_id);

                if user.role.is_mod() {
                    // 管理员回复时：通知申请用户
                    if user.id != app_user_id.into() {
                        NotificationBuilder {
                            body: NotificationBody::CreatorApplicationMessage {
                                application_id: creator_application_id.0,
                                thread_id: thread.id.into(),
                                message_id: id.into(),
                            },
                        }
                        .insert(app_user_id, &mut transaction, &redis)
                        .await?;
                    }
                } else {
                    // 用户回复时：通知所有管理员（限制 100 人，避免性能问题）
                    let mod_ids = sqlx::query!(
                        "SELECT id FROM users WHERE role IN ('admin', 'moderator') LIMIT 100"
                    )
                    .fetch_all(&**pool)
                    .await?
                    .into_iter()
                    .map(|row| database::models::ids::UserId(row.id))
                    // 排除申请用户自己（如果用户也是管理员，不需要给自己发通知）
                    .filter(|mod_id| *mod_id != app_user_id)
                    .collect::<Vec<_>>();

                    if !mod_ids.is_empty() {
                        NotificationBuilder {
                            body: NotificationBody::CreatorApplicationMessage {
                                application_id: creator_application_id.0,
                                thread_id: thread.id.into(),
                                message_id: id.into(),
                            },
                        }
                        .insert_many(mod_ids, &mut transaction, &redis)
                        .await?;
                    }
                }
            }
        }

        if let MessageBody::Text {
            associated_images, ..
        } = &new_message.body
        {
            for image_id in associated_images {
                if let Some(db_image) = image_item::Image::get(
                    (*image_id).into(),
                    &mut *transaction,
                    &redis,
                )
                .await?
                {
                    let image: Image = db_image.into();
                    if !matches!(
                        image.context,
                        ImageContext::ThreadMessage { .. }
                    ) || image.context.inner_id().is_some()
                    {
                        return Err(ApiError::InvalidInput(format!(
                            "图片 {} 已被使用或不属于 'thread_message' 上下文",
                            image_id
                        )));
                    }

                    sqlx::query!(
                        "
                        UPDATE uploaded_images
                        SET thread_message_id = $1
                        WHERE id = $2
                        ",
                        thread.id.0,
                        image_id.0 as i64
                    )
                    .execute(&mut *transaction)
                    .await?;

                    image_item::Image::clear_cache(image.id.into(), &redis)
                        .await?;
                } else {
                    return Err(ApiError::InvalidInput(format!(
                        "图片 {} 不存在",
                        image_id
                    )));
                }
            }
        }

        transaction.commit().await?;

        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn message_delete(
    req: HttpRequest,
    info: web::Path<(ThreadMessageId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::THREAD_WRITE]),
    )
    .await?
    .1;

    // 检查用户是否被论坛类封禁
    check_forum_ban(&user, &pool).await?;

    let result = database::models::ThreadMessage::get(
        info.into_inner().0.into(),
        &**pool,
    )
    .await?;

    if let Some(thread) = result {
        if !user.role.is_mod() && thread.author_id != Some(user.id.into()) {
            return Err(ApiError::CustomAuthentication(
                "您不能删除此消息!".to_string(),
            ));
        }

        let mut transaction = pool.begin().await?;

        let context = ImageContext::ThreadMessage {
            thread_message_id: Some(thread.id.into()),
        };
        let images =
            database::Image::get_many_contexted(context, &mut transaction)
                .await?;
        let cdn_url = dotenvy::var("CDN_URL")?;
        for image in images {
            let name = image.url.split(&format!("{cdn_url}/")).nth(1);
            if let Some(icon_path) = name {
                file_host.delete_file_version("", icon_path).await?;
            }
            database::Image::remove(image.id, &mut transaction, &redis).await?;
        }

        let private = if let MessageBody::Text { private, .. } = thread.body {
            private
        } else if let MessageBody::Deleted { private, .. } = thread.body {
            private
        } else {
            false
        };

        database::models::ThreadMessage::remove_full(
            thread.id,
            private,
            &mut transaction,
        )
        .await?;
        transaction.commit().await?;

        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}
