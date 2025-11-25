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
    auth::{filter_visible_projects, get_user_from_headers},
    database::{models::User, redis::RedisPool},
    file_hosting::FileHost,
    models::{
        collections::{Collection, CollectionStatus},
        ids::UserId,
        notifications::Notification,
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

    let users: Vec<crate::models::users::User> =
        users_data.into_iter().map(From::from).collect();

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
        let response: crate::models::users::User = data.into();
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
        let response: crate::models::users::User = data.into();
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

        for data in organizations_data {
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
    #[validate(length(min = 1, max = 39), regex = "RE_URL_SAFE")]
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
                    &**pool,
                    &redis,
                )
                .await?;
            }
            let mut transaction = pool.begin().await?;

            if let Some(username) = &new_user.username {
                let existing_user_id_option =
                    User::get(username, &**pool, &redis).await?;

                if existing_user_id_option
                    .map(|x| UserId::from(x.id))
                    .map(|id| id == user.id)
                    .unwrap_or(true)
                {
                    let risk = crate::util::risk::check_text_risk(
                        username,
                        &user.username,
                        &format!("/user/{}", user.username),
                        "修改新的用户名",
                        &redis,
                    )
                    .await?;
                    if !risk {
                        return Err(ApiError::InvalidInput(
                            "新的用户名包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的修改新的用户名".to_string(),
                        ));
                    }

                    sqlx::query!(
                        "
                        UPDATE users
                        SET username = $1
                        WHERE (id = $2)
                        ",
                        username,
                        id as crate::database::models::ids::UserId,
                    )
                    .execute(&mut *transaction)
                    .await?;
                } else {
                    return Err(ApiError::InvalidInput(format!(
                        "用户名 {username} 已被占用!"
                    )));
                }
            }

            if let Some(bio) = &new_user.bio {
                if bio.as_deref().is_some() {
                    // 应用场景URL

                    let risk = crate::util::risk::check_text_risk(
                        bio.as_deref().unwrap(),
                        &user.username,
                        &format!("/user/{}", user.username),
                        "个人资料简介",
                        &redis,
                    )
                    .await?;
                    if !risk {
                        return Err(ApiError::InvalidInput(
                            "简介包含敏感词，已被记录该次提交，请勿在本网站使用涉及敏感词的简介".to_string(),
                        ));
                    }
                }

                sqlx::query!(
                    "
                    UPDATE users
                    SET bio = $1
                    WHERE (id = $2)
                    ",
                    bio.as_deref(),
                    id as crate::database::models::ids::UserId,
                )
                .execute(&mut *transaction)
                .await?;
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
            Ok(HttpResponse::NoContent().body(""))
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
                &**pool,
                &redis,
            )
            .await?;
        }

        delete_old_images(
            actual_user.avatar_url,
            actual_user.raw_avatar_url,
            &***file_host,
        )
        .await?;

        let bytes =
            read_from_payload(&mut payload, 262144, "头像必须小于256KiB")
                .await?;

        let user_id: UserId = actual_user.id.into();
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
        )
        .await?;

        sqlx::query!(
            "
            UPDATE users
            SET avatar_url = $1, raw_avatar_url = $2
            WHERE (id = $3)
            ",
            upload_result.url,
            upload_result.raw_url,
            actual_user.id as crate::database::models::ids::UserId,
        )
        .execute(&**pool)
        .await?;
        User::clear_caches(&[(actual_user.id, None)], &redis).await?;

        Ok(HttpResponse::NoContent().body(""))
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
