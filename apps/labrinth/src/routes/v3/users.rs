use std::{collections::HashMap, sync::Arc};

use actix_web::{HttpRequest, HttpResponse, web};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

use super::{ApiError, oauth_clients::get_user_clients};
use crate::util::img::delete_old_images;
use crate::{
    auth::{
        checks::is_visible_organization, filter_visible_projects,
        get_user_from_headers,
    },
    database::{
        models::User, models::notification_item::NotificationBuilder,
        redis::RedisPool,
    },
    file_hosting::FileHost,
    models::{
        collections::{Collection, CollectionStatus},
        ids::{UserId, random_base62},
        notifications::{Notification, NotificationBody},
        pats::Scopes,
        projects::Project,
        users::{Badges, Role},
    },
    queue::session::AuthQueue,
    util::{routes::read_from_payload, validate::validation_errors_to_string},
};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("user", web::get().to(user_auth_get));
    cfg.route("users", web::get().to(users_get));

    cfg.service(
        web::scope("user")
            .route("{user_id}/projects", web::get().to(projects_list))
            .route("{id}", web::get().to(user_get))
            .route("{user_id}/collections", web::get().to(collections_list))
            .route("{user_id}/organizations", web::get().to(orgs_list))
            .route("{user_id}/forum", web::get().to(user_forum_content))
            .route("{id}", web::patch().to(user_edit))
            .route("{id}/icon", web::patch().to(user_icon_edit))
            .route("{id}", web::delete().to(user_delete))
            .route("{id}/follows", web::get().to(user_follows))
            .route("{id}/notifications", web::get().to(user_notifications))
            .route("{id}/oauth_apps", web::get().to(get_user_clients))
            // 用户封禁相关路由
            .route("bans", web::get().to(super::bans::get_my_bans))
            .route(
                "bans/{ban_id}/appeal",
                web::post().to(super::bans::create_appeal),
            )
            .route(
                "appeals/{appeal_id}",
                web::get().to(super::bans::get_my_appeal),
            )
            .route(
                "{user_id}/profile_reviews/{review_id}/cancel",
                web::post().to(super::profile_reviews::cancel_review),
            ),
    );
}

pub async fn projects_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        let project_data = User::get_projects(id, &**pool, &redis).await?;

        let projects: Vec<_> = crate::database::Project::get_many_ids(
            &project_data,
            &**pool,
            &redis,
        )
        .await?;
        let projects =
            filter_visible_projects(projects, &user, &pool, true).await?;
        Ok(HttpResponse::Ok().json(projects))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn user_auth_get(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (scopes, mut user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    if !scopes.contains(Scopes::USER_READ_EMAIL) {
        user.email = None;
    }

    if !scopes.contains(Scopes::PAYOUTS_READ) {
        user.payout_data = None;
    }

    Ok(HttpResponse::Ok().json(user))
}

#[derive(Serialize, Deserialize)]
pub struct UserIds {
    pub ids: String,
}

pub async fn users_get(
    web::Query(ids): web::Query<UserIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user_ids = serde_json::from_str::<Vec<String>>(&ids.ids)?;

    let users_data = User::get_many(&user_ids, &**pool, &redis).await?;

    let mut users: Vec<crate::models::users::User> =
        users_data.into_iter().map(From::from).collect();

    // 公开接口，清除审核数据
    for user in &mut users {
        user.pending_profile_reviews = None;
    }

    Ok(HttpResponse::Ok().json(users))
}

pub async fn user_get(
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user_data = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(data) = user_data {
        // active_bans 已经在 User::get_many 中自动填充
        let mut response: crate::models::users::User = data.into();
        // 公开接口，清除审核数据
        response.pending_profile_reviews = None;
        Ok(HttpResponse::Ok().json(response))
    } else {
        Err(ApiError::NotFound)
    }
}
pub async fn user_get_(
    userid: crate::database::models::UserId,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<Option<crate::models::users::User>, ApiError> {
    let user_data = User::get_id(userid, &**pool, &redis).await?;

    if let Some(data) = user_data {
        let mut response: crate::models::users::User = data.into();
        response.pending_profile_reviews = None;
        Ok(Some(response))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn collections_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::COLLECTION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        let user_id: UserId = id.into();

        let can_view_private = user
            .map(|y| y.role.is_mod() || y.id == user_id)
            .unwrap_or(false);

        let project_data = User::get_collections(id, &**pool).await?;

        let response: Vec<_> = crate::database::models::Collection::get_many(
            &project_data,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .filter(|x| {
            can_view_private || matches!(x.status, CollectionStatus::Listed)
        })
        .map(Collection::from)
        .collect();

        Ok(HttpResponse::Ok().json(response))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn orgs_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        let org_data = User::get_organizations(id, &**pool).await?;

        let organizations_data =
            crate::database::models::organization_item::Organization::get_many_ids(
                &org_data, &**pool, &redis,
            )
            .await?;

        let team_ids = organizations_data
            .iter()
            .map(|x| x.team_id)
            .collect::<Vec<_>>();

        let teams_data =
            crate::database::models::TeamMember::get_from_team_full_many(
                &team_ids, &**pool, &redis,
            )
            .await?;
        let users = User::get_many_ids(
            &teams_data.iter().map(|x| x.user_id).collect::<Vec<_>>(),
            &**pool,
            &redis,
        )
        .await?;

        let mut organizations = vec![];
        let mut team_groups = HashMap::new();
        for item in teams_data {
            team_groups.entry(item.team_id).or_insert(vec![]).push(item);
        }

        // 来源: Modrinth 上游提交 6f7618db1 - hide hidden orgs from user profiles (#4452)
        for data in organizations_data {
            // 过滤不可见的组织
            if !is_visible_organization(&data, &user, &pool, &redis).await? {
                continue;
            }

            let members_data =
                team_groups.remove(&data.team_id).unwrap_or(vec![]);
            let logged_in = user
                .as_ref()
                .and_then(|user| {
                    members_data
                        .iter()
                        .find(|x| x.user_id == user.id.into() && x.accepted)
                })
                .is_some();

            let team_members: Vec<_> = members_data
                .into_iter()
                .filter(|x| logged_in || x.accepted || id == x.user_id)
                .flat_map(|data| {
                    users.iter().find(|x| x.id == data.user_id).map(|user| {
                        crate::models::teams::TeamMember::from(
                            data,
                            user.clone(),
                            !logged_in,
                        )
                    })
                })
                .collect();

            let organization = crate::models::organizations::Organization::from(
                data,
                team_members,
            );
            organizations.push(organization);
        }

        Ok(HttpResponse::Ok().json(organizations))
    } else {
        Err(ApiError::NotFound)
    }
}

lazy_static! {
    static ref RE_URL_SAFE: Regex =
        Regex::new(r#"^[\p{L}\p{N}!@$()`.+,_"-]*$"#).unwrap();
}

#[derive(Serialize, Deserialize, Validate)]
pub struct EditUser {
    #[validate(length(min = 1, max = 39), regex(path = *RE_URL_SAFE))]
    pub username: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(length(max = 160))]
    pub bio: Option<Option<String>>,
    pub role: Option<Role>,
    pub badges: Option<Badges>,
    #[validate(length(max = 160))]
    pub venmo_handle: Option<String>,
}

pub async fn user_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    new_user: web::Json<EditUser>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (scopes, user) = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    new_user.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(actual_user) = id_option {
        let id = actual_user.id;
        let user_id: UserId = id.into();

        if user.id == user_id || user.role.is_mod() {
            // 检查用户是否被全局封禁（管理员可以绕过）
            if !user.role.is_mod() {
                crate::util::ban_check::check_global_ban(
                    user.id.into(),
                    &pool,
                    &redis,
                )
                .await?;
            }
            let mut transaction = pool.begin().await?;
            let mut pending_fields = Vec::new();

            if let Some(username) = &new_user.username {
                let existing_user_id_option =
                    User::get(username, &**pool, &redis).await?;

                if existing_user_id_option
                    .map(|x| UserId::from(x.id))
                    .map(|id| id == user.id)
                    .unwrap_or(true)
                {
                    let (passed, risk_labels) =
                        crate::util::risk::check_text_risk_with_labels(
                            username,
                            &user.username,
                            &format!("/user/{}", user.username),
                            "修改新的用户名",
                            &redis,
                        )
                        .await?;

                    if passed {
                        // 风控通过，正常保存
                        sqlx::query!(
                            "UPDATE users SET username = $1 WHERE id = $2",
                            username,
                            id as crate::database::models::ids::UserId,
                        )
                        .execute(&mut *transaction)
                        .await?;
                    } else {
                        // 风控未通过，写入审核记录（ON CONFLICT 处理并发竞态）
                        let review_id = random_base62(8) as i64;
                        let result = sqlx::query!(
                            "INSERT INTO user_profile_reviews (id, user_id, review_type, old_value, new_value, risk_labels)
                             VALUES ($1, $2, 'username', $3, $4, $5)
                             ON CONFLICT (user_id, review_type) WHERE status = 'pending' DO NOTHING",
                            review_id,
                            id as crate::database::models::ids::UserId,
                            &actual_user.username,
                            username,
                            &risk_labels,
                        )
                        .execute(&mut *transaction)
                        .await?;

                        if result.rows_affected() == 0 {
                            return Err(ApiError::InvalidInput(
                                "您已有待审核的用户名修改，请等待审核完成或撤销后再试".to_string(),
                            ));
                        }

                        // 发送通知
                        let notification = NotificationBuilder {
                            body: NotificationBody::ProfileReviewPending {
                                review_id,
                                review_type: "username".to_string(),
                            },
                        };
                        notification
                            .insert(id, &mut transaction, &redis)
                            .await?;

                        pending_fields.push("username");
                    }
                } else {
                    return Err(ApiError::InvalidInput(format!(
                        "用户名 {username} 已被占用!"
                    )));
                }
            }

            if let Some(bio) = &new_user.bio {
                if let Some(bio_text) = bio.as_deref() {
                    let (passed, risk_labels) =
                        crate::util::risk::check_text_risk_with_labels(
                            bio_text,
                            &user.username,
                            &format!("/user/{}", user.username),
                            "个人资料简介",
                            &redis,
                        )
                        .await?;

                    if passed {
                        // 风控通过，正常保存
                        sqlx::query!(
                            "UPDATE users SET bio = $1 WHERE id = $2",
                            bio.as_deref(),
                            id as crate::database::models::ids::UserId,
                        )
                        .execute(&mut *transaction)
                        .await?;
                    } else {
                        // 风控未通过，写入审核记录（ON CONFLICT 处理并发竞态）
                        let review_id = random_base62(8) as i64;
                        let result = sqlx::query!(
                            "INSERT INTO user_profile_reviews (id, user_id, review_type, old_value, new_value, risk_labels)
                             VALUES ($1, $2, 'bio', $3, $4, $5)
                             ON CONFLICT (user_id, review_type) WHERE status = 'pending' DO NOTHING",
                            review_id,
                            id as crate::database::models::ids::UserId,
                            actual_user.bio.as_deref(),
                            bio_text,
                            &risk_labels,
                        )
                        .execute(&mut *transaction)
                        .await?;

                        if result.rows_affected() == 0 {
                            return Err(ApiError::InvalidInput(
                                "您已有待审核的简介修改，请等待审核完成或撤销后再试".to_string(),
                            ));
                        }

                        // 发送通知
                        let notification = NotificationBuilder {
                            body: NotificationBody::ProfileReviewPending {
                                review_id,
                                review_type: "bio".to_string(),
                            },
                        };
                        notification
                            .insert(id, &mut transaction, &redis)
                            .await?;

                        pending_fields.push("bio");
                    }
                } else {
                    // bio 被设为 null，直接保存
                    sqlx::query!(
                        "UPDATE users SET bio = NULL WHERE id = $1",
                        id as crate::database::models::ids::UserId,
                    )
                    .execute(&mut *transaction)
                    .await?;
                }
            }

            if let Some(role) = &new_user.role {
                if !user.role.is_admin() {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此用户的角色!".to_string(),
                    ));
                }

                let role = role.to_string();

                sqlx::query!(
                    "
                    UPDATE users
                    SET role = $1
                    WHERE (id = $2)
                    ",
                    role,
                    id as crate::database::models::ids::UserId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(badges) = &new_user.badges {
                if !user.role.is_admin() {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此用户的徽章!".to_string(),
                    ));
                }

                sqlx::query!(
                    "
                    UPDATE users
                    SET badges = $1
                    WHERE (id = $2)
                    ",
                    badges.bits() as i64,
                    id as crate::database::models::ids::UserId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(venmo_handle) = &new_user.venmo_handle {
                if !scopes.contains(Scopes::PAYOUTS_WRITE) {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限编辑此用户的Venmo handle!".to_string(),
                    ));
                }

                sqlx::query!(
                    "
                    UPDATE users
                    SET venmo_handle = $1
                    WHERE (id = $2)
                    ",
                    venmo_handle,
                    id as crate::database::models::ids::UserId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            transaction.commit().await?;
            User::clear_caches(&[(id, Some(actual_user.username))], &redis)
                .await?;

            if !pending_fields.is_empty() {
                crate::routes::internal::moderation::clear_pending_counts_cache(&redis).await;
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "pending_review": true,
                    "fields": pending_fields,
                })))
            } else {
                Ok(HttpResponse::NoContent().body(""))
            }
        } else {
            Err(ApiError::CustomAuthentication(
                "您没有权限编辑此用户!".to_string(),
            ))
        }
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Extension {
    pub ext: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn user_icon_edit(
    web::Query(ext): web::Query<Extension>,
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    mut payload: web::Payload,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?
    .1;
    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(actual_user) = id_option {
        if user.id != actual_user.id.into() && !user.role.is_mod() {
            return Err(ApiError::CustomAuthentication(
                "您没有权限编辑此用户的头像!".to_string(),
            ));
        }

        // 检查用户是否被全局封禁（管理员可以绕过）
        if !user.role.is_mod() {
            crate::util::ban_check::check_global_ban(
                user.id.into(),
                &pool,
                &redis,
            )
            .await?;
        }

        let bytes =
            read_from_payload(&mut payload, 262144, "头像必须小于256KiB")
                .await?;

        let user_id: UserId = actual_user.id.into();

        // 先上传图片到 S3，跳过内置风控（我们手动检查）
        let upload_result = crate::util::img::upload_image_optimized(
            &format!("data/{}", user_id),
            bytes.freeze(),
            &ext.ext,
            Some(96),
            Some(1.0),
            &***file_host,
            crate::util::img::UploadImagePos {
                pos: "用户头像".to_string(),
                url: format!("/user/{}", actual_user.username),
                username: actual_user.username.clone(),
            },
            &redis,
            true, // 跳过内置风控，手动检查
        )
        .await?;

        // 手动进行图片风控检测
        let risk_result = crate::util::risk::check_image_risk_with_labels(
            &upload_result.url,
            &format!("/user/{}", actual_user.username),
            &actual_user.username,
            "用户头像",
            &redis,
        )
        .await?;
        let passed = risk_result.passed;
        let risk_labels = risk_result.labels;

        if passed {
            // 风控通过：先更新数据库，再删除旧头像
            let old_avatar_url = actual_user.avatar_url.clone();
            let old_raw_avatar_url = actual_user.raw_avatar_url.clone();

            let mut tx = pool.begin().await?;
            sqlx::query!(
                "UPDATE users SET avatar_url = $1, raw_avatar_url = $2 WHERE id = $3",
                upload_result.url,
                upload_result.raw_url,
                actual_user.id as crate::database::models::ids::UserId,
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            User::clear_caches(
                &[(actual_user.id, Some(actual_user.username))],
                &redis,
            )
            .await?;

            // 事务成功后删除旧头像（尽力而为）
            if let Err(e) = delete_old_images(
                old_avatar_url,
                old_raw_avatar_url,
                &***file_host,
            )
            .await
            {
                log::warn!("删除旧头像失败: {}", e);
            }

            Ok(HttpResponse::NoContent().body(""))
        } else {
            // 风控未通过，写入审核记录（ON CONFLICT 处理并发竞态）
            let review_id = random_base62(8) as i64;
            let old_value = serde_json::json!({
                "avatar_url": actual_user.avatar_url,
                "raw_avatar_url": actual_user.raw_avatar_url,
            })
            .to_string();
            let new_value = serde_json::json!({
                "avatar_url": upload_result.url,
                "raw_avatar_url": upload_result.raw_url,
            })
            .to_string();

            let mut transaction = pool.begin().await?;

            let result = sqlx::query!(
                "INSERT INTO user_profile_reviews (id, user_id, review_type, old_value, new_value, risk_labels)
                 VALUES ($1, $2, 'avatar', $3, $4, $5)
                 ON CONFLICT (user_id, review_type) WHERE status = 'pending' DO NOTHING",
                review_id,
                actual_user.id as crate::database::models::ids::UserId,
                &old_value,
                &new_value,
                &risk_labels,
            )
            .execute(&mut *transaction)
            .await?;

            if result.rows_affected() == 0 {
                transaction.rollback().await?;
                // 已有 pending 审核，删除刚上传的图片
                if let Err(e) = delete_old_images(
                    Some(upload_result.url),
                    Some(upload_result.raw_url),
                    &***file_host,
                )
                .await
                {
                    log::warn!("清理重复审核头像失败: {}", e);
                }
                return Err(ApiError::InvalidInput(
                    "您已有待审核的头像修改，请等待审核完成或撤销后再试"
                        .to_string(),
                ));
            }

            // 发送通知
            let notification = NotificationBuilder {
                body: NotificationBody::ProfileReviewPending {
                    review_id,
                    review_type: "avatar".to_string(),
                },
            };
            notification
                .insert(actual_user.id, &mut transaction, &redis)
                .await?;

            transaction.commit().await?;

            crate::routes::internal::moderation::clear_pending_counts_cache(
                &redis,
            )
            .await;

            // 清除用户缓存
            User::clear_caches(
                &[(actual_user.id, Some(actual_user.username))],
                &redis,
            )
            .await?;

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "pending_review": true,
                "review_type": "avatar",
            })))
        }
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn user_delete(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_DELETE]),
    )
    .await?
    .1;
    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        if !user.role.is_admin() && user.id != id.into() {
            return Err(ApiError::CustomAuthentication(
                "您没有权限删除此用户!".to_string(),
            ));
        }

        let mut transaction = pool.begin().await?;

        let result = User::remove(id, &mut transaction, &redis).await?;

        transaction.commit().await?;

        if result.is_some() {
            Ok(HttpResponse::NoContent().body(""))
        } else {
            Err(ApiError::NotFound)
        }
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn user_follows(
    req: HttpRequest,
    info: web::Path<(String,)>,
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
    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        if !user.role.is_admin() && user.id != id.into() {
            return Err(ApiError::CustomAuthentication(
                "您没有权限查看此用户关注的内容!".to_string(),
            ));
        }

        let project_ids = User::get_follows(id, &**pool).await?;
        let projects: Vec<_> = crate::database::Project::get_many_ids(
            &project_ids,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .map(Project::from)
        .collect();

        Ok(HttpResponse::Ok().json(projects))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn user_notifications(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::NOTIFICATION_READ]),
    )
    .await?
    .1;
    let id_option = User::get(&info.into_inner().0, &**pool, &redis).await?;

    if let Some(id) = id_option.map(|x| x.id) {
        if !user.role.is_admin() && user.id != id.into() {
            return Err(ApiError::CustomAuthentication(
                "您没有权限查看此用户的通知!".to_string(),
            ));
        }

        let mut notifications: Vec<Notification> =
            crate::database::models::notification_item::Notification::get_many_user(
                id, &**pool, &redis,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

        notifications.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(HttpResponse::Ok().json(notifications))
    } else {
        Err(ApiError::NotFound)
    }
}

// ==================== 用户论坛内容 ====================

/// 用户论坛内容缓存命名空间
pub const USER_FORUM_NAMESPACE: &str = "user_forum";

/// 清除用户论坛内容缓存
///
/// 在以下操作时调用：
/// - 用户发帖
/// - 用户回复帖子
/// - 用户删除帖子
/// - 用户删除回复
/// - 管理员删除用户的帖子/回复
pub async fn clear_user_forum_cache(
    user_id: i64,
    redis: &RedisPool,
) -> Result<(), crate::database::models::DatabaseError> {
    let mut redis_conn = redis.connect().await?;
    // 使用模式匹配删除该用户的所有论坛缓存
    // 缓存键格式: user_forum:{user_id}:{type}:{page}:{limit}
    let pattern = format!("{}:{}:*", USER_FORUM_NAMESPACE, user_id);
    redis_conn.delete_many_pattern(&pattern).await?;
    Ok(())
}

/// 用户论坛内容查询参数
#[derive(Serialize, Deserialize)]
pub struct UserForumQuery {
    /// 内容类型: discussions, posts, all
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    /// 页码，默认 1
    pub page: Option<i64>,
    /// 每页数量，默认 20，最大 100
    pub limit: Option<i64>,
}

/// 用户论坛内容响应
#[derive(Serialize, Deserialize, Clone)]
pub struct UserForumResponse {
    /// 用户发表的帖子
    pub discussions: Vec<ForumDiscussionSummary>,
    /// 用户的回复
    pub posts: Vec<ForumPostSummary>,
    /// 帖子总数
    pub total_discussions: i64,
    /// 回复总数
    pub total_posts: i64,
    /// 当前页码
    pub page: i64,
    /// 每页数量
    pub limit: i64,
}

/// 帖子摘要
#[derive(Serialize, Deserialize, Clone)]
pub struct ForumDiscussionSummary {
    /// 帖子 ID (Base62)
    pub id: String,
    /// 帖子标题
    pub title: String,
    /// 帖子分类
    pub category: String,
    /// 帖子状态 (open/closed)
    pub state: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 回复数量
    pub reply_count: i64,
    /// 最后回复内容（截断）
    pub last_reply_content: Option<String>,
    /// 最后回复时间
    pub last_reply_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 关联的项目 ID (Base62)，如果是资源帖子则有值
    pub project_id: Option<String>,
}

/// 回复摘要
#[derive(Serialize, Deserialize, Clone)]
pub struct ForumPostSummary {
    /// 回复 ID (Base62)
    pub id: String,
    /// 所属帖子 ID (Base62)
    pub discussion_id: String,
    /// 所属帖子标题
    pub discussion_title: String,
    /// 回复内容预览（截断）
    pub content: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 关联的项目 slug，如果是资源帖子则有值
    pub project_slug: Option<String>,
    /// 楼号（在该帖子中的顺序）
    pub floor_number: i64,
}

/// 截断内容到指定长度
fn truncate_content(content: &str, max_len: usize) -> String {
    // 使用迭代器避免分配额外的 Vec<char>
    let char_count = content.chars().count();
    if char_count <= max_len {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

/// 获取用户论坛内容
///
/// GET /v3/user/{user_id}/forum
pub async fn user_forum_content(
    info: web::Path<(String,)>,
    query: web::Query<UserForumQuery>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    // 1. 获取用户
    let user_data = User::get(&info.into_inner().0, &**pool, &redis).await?;
    let user = user_data.ok_or(ApiError::NotFound)?;

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let content_type = query.content_type.as_deref().unwrap_or("all");

    // 2. 尝试从缓存获取
    let cache_key =
        format!("{}:{}:{}:{}", user.id.0, content_type, page, limit);
    let mut redis_conn = redis.connect().await?;

    if let Ok(Some(cached)) = redis_conn
        .get_deserialized_from_json::<UserForumResponse>(
            USER_FORUM_NAMESPACE,
            &cache_key,
        )
        .await
    {
        return Ok(HttpResponse::Ok().json(cached));
    }

    let offset = (page - 1) * limit;
    let mut discussions = vec![];
    let mut posts = vec![];
    let mut total_discussions = 0i64;
    let mut total_posts = 0i64;

    // 3. 查询用户发表的帖子（使用 LEFT JOIN LATERAL 优化子查询性能）
    if content_type == "all" || content_type == "discussions" {
        let disc_rows = sqlx::query!(
            r#"
            SELECT d.id, d.title, d.category, d.state, d.created_at,
                   COALESCE(reply_stats.reply_count, 0) as "reply_count!",
                   last_reply.content as "last_reply_content?",
                   last_reply.created_at as "last_reply_time?",
                   m.id as "project_id?",
                   COUNT(*) OVER() as "total_count!"
            FROM discussions d
            LEFT JOIN LATERAL (
                SELECT COUNT(*) as reply_count
                FROM posts p
                WHERE p.discussion_id = d.id AND p.deleted = false
            ) reply_stats ON true
            LEFT JOIN LATERAL (
                SELECT p.content, p.created_at
                FROM posts p
                WHERE p.discussion_id = d.id AND p.deleted = false
                ORDER BY p.created_at DESC
                LIMIT 1
            ) last_reply ON true
            LEFT JOIN mods m ON m.forum = d.id
            WHERE d.user_id = $1 AND d.deleted = false
            ORDER BY d.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user.id.0,
            limit,
            offset
        )
        .fetch_all(&**pool)
        .await?;

        if let Some(first) = disc_rows.first() {
            total_discussions = first.total_count;
        }

        discussions = disc_rows
            .into_iter()
            .map(|r| ForumDiscussionSummary {
                id: crate::models::ids::base62_impl::to_base62(r.id as u64),
                title: r.title,
                category: r.category,
                state: r.state,
                created_at: r.created_at,
                reply_count: r.reply_count,
                last_reply_content: r
                    .last_reply_content
                    .map(|c| truncate_content(&c, 100)),
                last_reply_time: r.last_reply_time,
                project_id: r.project_id.map(|id| {
                    crate::models::ids::base62_impl::to_base62(id as u64)
                }),
            })
            .collect();
    }

    // 4. 查询用户的回复
    if content_type == "all" || content_type == "posts" {
        let post_rows = sqlx::query!(
            r#"
            SELECT p.id, p.discussion_id, p.content, p.created_at,
                   d.title as discussion_title,
                   (SELECT m.slug FROM mods m WHERE m.forum = d.id LIMIT 1) as "project_slug",
                   (SELECT COUNT(*) + 1 FROM posts p2 WHERE p2.discussion_id = p.discussion_id AND p2.created_at < p.created_at) as "floor_number!",
                   COUNT(*) OVER() as "total_count!"
            FROM posts p
            JOIN discussions d ON d.id = p.discussion_id
            WHERE p.user_id = $1 AND p.deleted = false AND d.deleted = false
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user.id.0,
            limit,
            offset
        )
        .fetch_all(&**pool)
        .await?;

        if let Some(first) = post_rows.first() {
            total_posts = first.total_count;
        }

        posts = post_rows
            .into_iter()
            .map(|r| ForumPostSummary {
                id: crate::models::ids::base62_impl::to_base62(r.id as u64),
                discussion_id: crate::models::ids::base62_impl::to_base62(
                    r.discussion_id as u64,
                ),
                discussion_title: r.discussion_title,
                content: truncate_content(&r.content, 200),
                created_at: r.created_at,
                project_slug: r.project_slug,
                floor_number: r.floor_number,
            })
            .collect();
    }

    let response = UserForumResponse {
        discussions,
        posts,
        total_discussions,
        total_posts,
        page,
        limit,
    };

    // 5. 缓存结果（5分钟过期）
    let _ = redis_conn
        .set(
            USER_FORUM_NAMESPACE,
            &cache_key,
            &serde_json::to_string(&response)?,
            Some(300),
        )
        .await;

    Ok(HttpResponse::Ok().json(response))
}
