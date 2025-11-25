use super::ApiError;
use crate::auth::checks::{
    check_resource_ban, filter_visible_versions, is_visible_project,
    is_visible_version,
};
use crate::auth::get_user_from_headers;
use crate::database;
use crate::database::models::loader_fields::{
    self, LoaderField, LoaderFieldEnumValue, VersionField,
};
use crate::database::models::version_item::{
    DependencyBuilder, LoaderVersion, QueryDisk, VersionLinkBuilder,
};
use crate::database::models::{Organization, image_item};
use crate::database::redis::RedisPool;
use crate::models;
use crate::models::analytics::Download;
use crate::models::ids::base62_impl::parse_base62;
use crate::models::ids::{ProjectId, VersionId};
use crate::models::images::ImageContext;
use crate::models::pats::Scopes;
use crate::models::projects::{
    Dependency, FileType, VersionLink, VersionStatus, VersionType,
};
use crate::models::projects::{Loader, skip_nulls};
use crate::models::teams::ProjectPermissions;
use crate::queue::analytics::AnalyticsQueue;
use crate::queue::session::AuthQueue;
use crate::search::SearchConfig;
use crate::search::indexing::remove_documents;
use crate::util::date::get_current_tenths_of_ms;
use crate::util::img;
use crate::util::validate::validation_errors_to_string;
use actix_web::{HttpRequest, HttpResponse, web};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use validator::Validate;

pub mod version_link_thread;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "version",
        web::post().to(super::version_creation::version_create),
    );
    cfg.route("versions", web::get().to(versions_get));

    cfg.service(
        web::scope("version")
            .route("{id}", web::get().to(version_get))
            .route("{id}", web::patch().to(version_edit))
            .route("{id}/download", web::patch().to(version_download))
            .route("{id}", web::delete().to(version_delete))
            .route(
                "{version_id}/file",
                web::post().to(super::version_creation::upload_file_to_version),
            )
            .route(
                "{version_id}/link/{target_version_id}/approve",
                web::post().to(approve_version_link),
            )
            .route(
                "{version_id}/link/{target_version_id}/reject",
                web::post().to(reject_version_link),
            )
            .route(
                "{version_id}/link/{target_version_id}/revoke",
                web::post().to(revoke_version_link),
            )
            .route(
                "{version_id}/link/{target_version_id}/resubmit",
                web::post().to(resubmit_version_link),
            )
            .route(
                "{version_id}/link/{target_version_id}/thread",
                web::post().to(version_link_thread::send_version_link_message),
            ),
    );
}

// 给定一个项目ID/slug和一个版本slug
pub async fn version_project_get(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let info = info.into_inner();
    version_project_get_helper(req, info, pool, redis, session_queue).await
}
pub async fn version_project_get_helper(
    req: HttpRequest,
    id: (String, String),
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let result = database::models::Project::get(&id.0, &**pool, &redis).await?;

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ, Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if let Some(project) = result {
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }

        let versions = database::models::Version::get_many(
            &project.versions,
            &**pool,
            &redis,
        )
        .await?;

        let id_opt = parse_base62(&id.1).ok();
        let version = versions.into_iter().find(|x| {
            Some(x.inner.id.0 as u64) == id_opt
                || x.inner.version_number == id.1
        });

        if let Some(version) = version
            && is_visible_version(&version.inner, &user_option, &pool, &redis)
                .await?
        {
            let version_response = models::projects::Version::from(version);
            return Ok(HttpResponse::Ok().json(version_response));
        }
    }

    Err(ApiError::NotFound)
}

#[derive(Serialize, Deserialize)]
pub struct VersionIds {
    pub ids: String,
}

pub async fn versions_get(
    req: HttpRequest,
    web::Query(ids): web::Query<VersionIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let version_ids =
        serde_json::from_str::<Vec<models::ids::VersionId>>(&ids.ids)?
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<database::models::VersionId>>();
    let versions_data =
        database::models::Version::get_many(&version_ids, &**pool, &redis)
            .await?;

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    let versions =
        filter_visible_versions(versions_data, &user_option, &pool, &redis)
            .await?;

    Ok(HttpResponse::Ok().json(versions))
}

pub async fn version_get(
    req: HttpRequest,
    info: web::Path<(models::ids::VersionId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let id = info.into_inner().0;
    version_get_helper(req, id, pool, redis, session_queue).await
}

pub async fn version_get_helper(
    req: HttpRequest,
    id: models::ids::VersionId,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let db_version_id: database::models::ids::VersionId = id.into();
    let version_data =
        database::models::Version::get(db_version_id, &**pool, &redis).await?;

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if let Some(data) = version_data
        && is_visible_version(&data.inner, &user_option, &pool, &redis).await?
    {
        let version = models::projects::Version::from(data);
        return Ok(HttpResponse::Ok().json(version));
    }

    Err(ApiError::NotFound)
}

#[derive(Serialize, Deserialize, Validate, Default, Debug)]
pub struct EditVersion {
    #[validate(
        length(min = 1, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    pub name: Option<String>,
    #[validate(
        length(min = 1, max = 32),
        regex = "crate::util::validate::RE_URL_SAFE"
    )]
    pub version_number: Option<String>,
    #[validate(length(max = 65536))]
    pub changelog: Option<String>,
    pub version_type: Option<models::projects::VersionType>,
    #[validate(
        length(min = 0, max = 4096),
        custom(function = "crate::util::validate::validate_deps")
    )]
    pub dependencies: Option<Vec<Dependency>>,
    #[validate(length(min = 0, max = 256))]
    pub version_links: Option<Vec<VersionLink>>,
    pub loaders: Option<Vec<Loader>>,
    pub featured: Option<bool>,
    pub downloads: Option<u32>,
    pub status: Option<VersionStatus>,
    pub file_types: Option<Vec<EditVersionFileType>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    pub ordering: Option<Option<i32>>,

    // 扁平化加载器字段
    // 所有其他字段都是加载器特定的 VersionFields
    // 这些在序列化期间被扁平化
    #[serde(deserialize_with = "skip_nulls")]
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,

    pub disk_only: Option<bool>,

    pub disk_urls: Option<Vec<QueryDisk>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EditVersionFileType {
    pub algorithm: String,
    pub hash: String,
    pub file_type: Option<FileType>,
}

pub async fn version_download(
    req: HttpRequest,
    info: web::Path<(VersionId,)>,
    pool: web::Data<PgPool>,
    analytics_queue: web::Data<Arc<AnalyticsQueue>>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let version_id = info.0;
    let id = version_id.into();
    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();
    let user_id = if let Some(user_option) = user_option {
        user_option.id.0
    } else {
        0
    };
    let result = database::models::Version::get(id, &**pool, &redis).await?;

    if let Some(version_item) = result {
        if !is_visible_version(&version_item.inner, &None, &pool, &redis)
            .await?
        {
            return Err(ApiError::NotFound);
        }
        let headers = req
            .headers()
            .into_iter()
            .map(|(key, val)| {
                (
                    key.to_string().to_lowercase(),
                    val.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect::<HashMap<String, String>>();

        // 如果req.peer_addr()有值则用这个的string 没有的话则 用 127.0.0.1
        let ip = if let Some(peer_addr) = headers.get("x-real-ip") {
            peer_addr.to_string()
        } else {
            "127.0.0.1".to_string()
        };

        let ip = crate::util::ip::convert_to_ip_v6(&ip)
            .unwrap_or_else(|_| Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped());
        let id: ProjectId = version_item.inner.project_id.into();

        // if version_item.disks.is_empty() {
        //     return Err(ApiError::NotFound);
        // }
        if version_item.disks.is_empty() {
            let url = version_item.files.first().unwrap().url.clone();
            let url = url::Url::parse(&url).map_err(|_| {
                ApiError::InvalidInput("无效的下载URL!".to_string())
            })?;
            analytics_queue.add_download(Download {
                recorded: get_current_tenths_of_ms(),
                domain: url.host_str().unwrap_or_default().to_string(),
                site_path: url.path().to_string(),
                user_id,
                project_id: id.0,
                version_id: version_id.0,
                ip,
                country: "".to_string(),
                user_agent: headers
                    .get("user-agent")
                    .cloned()
                    .unwrap_or_default(),
                headers: Vec::new(),
            });
        } else {
            let url = version_item.disks.first().unwrap().url.clone();

            let url = url::Url::parse(&url).map_err(|_| {
                ApiError::InvalidInput("无效的下载URL!".to_string())
            })?;

            analytics_queue.add_download(Download {
                recorded: get_current_tenths_of_ms(),
                domain: url.host_str().unwrap_or_default().to_string(),
                site_path: url.path().to_string(),
                user_id,
                project_id: id.0,
                version_id: version_id.0,
                ip,
                country: "".to_string(),
                user_agent: headers
                    .get("user-agent")
                    .cloned()
                    .unwrap_or_default(),
                headers: Vec::new(),
            });
        }
        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn version_edit(
    req: HttpRequest,
    info: web::Path<(VersionId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    new_version: web::Json<serde_json::Value>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let new_version: EditVersion =
        serde_json::from_value(new_version.into_inner())?;
    version_edit_helper(
        req,
        info.into_inner(),
        pool,
        redis,
        new_version,
        session_queue,
    )
    .await
}
pub async fn version_edit_helper(
    req: HttpRequest,
    info: (VersionId,),
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    new_version: EditVersion,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    // 检查用户是否被资源类封禁
    check_resource_ban(&user, &pool).await?;

    new_version.validate().map_err(|err| {
        ApiError::Validation(validation_errors_to_string(err, None))
    })?;

    let version_id = info.0;
    let id = version_id.into();

    let result = database::models::Version::get(id, &**pool, &redis).await?;

    if let Some(version_item) = result {
        let team_member =
            database::models::TeamMember::get_from_user_id_project(
                version_item.inner.project_id,
                user.id.into(),
                false,
                &**pool,
            )
            .await?;

        let organization =
            Organization::get_associated_organization_project_id(
                version_item.inner.project_id,
                &**pool,
            )
            .await?;

        let organization_team_member = if let Some(organization) = &organization
        {
            database::models::TeamMember::get_from_user_id(
                organization.team_id,
                user.id.into(),
                &**pool,
            )
            .await?
        } else {
            None
        };

        let permissions = ProjectPermissions::get_permissions_by_role(
            &user.role,
            &team_member,
            &organization_team_member,
        );

        if let Some(perms) = permissions {
            if !perms.contains(ProjectPermissions::UPLOAD_VERSION) {
                return Err(ApiError::CustomAuthentication(
                    "您没有权限编辑此版本！".to_string(),
                ));
            }

            let mut transaction = pool.begin().await?;

            if let Some(name) = &new_version.name {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET name = $1
                    WHERE (id = $2)
                    ",
                    name.trim(),
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(number) = &new_version.version_number {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET version_number = $1
                    WHERE (id = $2)
                    ",
                    number,
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(disk_only) = &new_version.disk_only {
                if *disk_only {
                    let urls = new_version.disk_urls.unwrap();
                    // for u in urls {
                    //     if u.is_empty(){
                    //         return Err(ApiError::InvalidInput(
                    //             "未填写网盘链接".to_string(),
                    //         ));
                    //     }
                    // }

                    sqlx::query!(
                        "
                        DELETE FROM disk_urls WHERE version_id = $1;
                        ",
                        id as database::models::ids::VersionId,
                    )
                    .execute(&mut *transaction)
                    .await?;

                    for u in urls {
                        sqlx::query!(
                            "
                            INSERT INTO disk_urls (version_id, url,platform)
                            VALUES ($1, $2, $3)
                            ",
                            id as database::models::ids::VersionId,
                            u.url,
                            u.platform,
                        )
                        .execute(&mut *transaction)
                        .await?;
                    }
                } else {
                    sqlx::query!(
                        "
                        DELETE FROM disk_urls WHERE version_id = $1;
                        ",
                        id as database::models::ids::VersionId,
                    )
                    .execute(&mut *transaction)
                    .await?;
                }
            }

            if let Some(version_type) = &new_version.version_type {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET version_type = $1
                    WHERE (id = $2)
                    ",
                    version_type.as_str(),
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(dependencies) = &new_version.dependencies {
                sqlx::query!(
                    "
                    DELETE FROM dependencies WHERE dependent_id = $1
                    ",
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;

                let builders = dependencies
                    .iter()
                    .map(|x| database::models::version_item::DependencyBuilder {
                        project_id: x.project_id.map(|x| x.into()),
                        version_id: x.version_id.map(|x| x.into()),
                        file_name: x.file_name.clone(),
                        dependency_type: x.dependency_type.to_string(),
                    })
                    .collect::<Vec<database::models::version_item::DependencyBuilder>>();

                DependencyBuilder::insert_many(
                    builders,
                    version_item.inner.id,
                    &mut transaction,
                )
                .await?;
            }

            if let Some(version_links) = &new_version.version_links {
                // 先获取旧的版本链接及其审批状态，以便比较和清除缓存
                let old_version_links = sqlx::query!(
                    "
                    SELECT joining_version_id, link_type, language_code, description, approval_status 
                    FROM version_link_version 
                    WHERE version_id = $1
                    ",
                    id as database::models::ids::VersionId,
                )
                .fetch_all(&mut *transaction)
                .await?;

                // 创建一个映射来快速查找旧的链接信息
                let mut old_links_map = std::collections::HashMap::new();
                for old_link in &old_version_links {
                    // 直接使用字段值，它们应该都是 NOT NULL
                    let key = (
                        old_link.joining_version_id,
                        old_link.link_type.clone(),
                        old_link.language_code.clone(),
                    );
                    old_links_map.insert(key, old_link.approval_status.clone());
                }

                // 收集所有需要清除缓存的版本ID（旧的和新的）
                let mut versions_to_clear_cache =
                    std::collections::HashSet::new();

                // 添加旧的目标版本
                for link in &old_version_links {
                    versions_to_clear_cache.insert(link.joining_version_id);
                }

                // 添加新的目标版本
                for link in version_links {
                    let joining_id: database::models::ids::VersionId =
                        link.joining_version_id.into();
                    versions_to_clear_cache.insert(joining_id.0);
                }

                // 删除现有的版本链接
                sqlx::query!(
                    "
                    DELETE FROM version_link_version WHERE version_id = $1
                    ",
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;

                // 处理每个版本链接，判断是否需要重新审核
                let mut builders = Vec::new();

                for link in version_links {
                    let target_version_id: database::models::ids::VersionId =
                        link.joining_version_id.into();

                    // 检查这个链接是否已存在且未改变
                    let key = (
                        target_version_id.0,
                        link.link_type.clone(),
                        link.language_code.clone(),
                    );
                    let existing_status = old_links_map.get(&key);

                    // 如果链接已存在且目标版本未改变，保留原有的审批状态
                    let approval_status = if let Some(status) = existing_status
                    {
                        // existing_status 是 &String（HashMap 存储的是 String）
                        log::info!(
                            "Version link for version {} targeting version {} unchanged, keeping approval status: {}",
                            version_id,
                            target_version_id.0,
                            status
                        );
                        status.clone()
                    } else {
                        // 这是新增或修改的链接，需要进行权限检查
                        log::info!(
                            "Version link for version {} targeting version {} is new or modified, checking permissions",
                            version_id,
                            target_version_id.0
                        );

                        let target_version = database::models::Version::get(
                            target_version_id,
                            &mut *transaction,
                            &redis,
                        )
                        .await?;

                        let mut auto_approve = false;

                        if let Some(target_version) = target_version {
                            // 获取目标项目的团队成员信息
                            let target_project_id =
                                target_version.inner.project_id;
                            let target_team = database::models::TeamMember::get_from_user_id_project(
                                target_project_id,
                                user.id.into(),
                                false,  // allow_pending
                                &mut *transaction,
                            ).await?;

                            // 获取目标项目的组织团队成员信息（如果有）
                            let target_project =
                                database::models::Project::get_id(
                                    target_project_id,
                                    &mut *transaction,
                                    &redis,
                                )
                                .await?;
                            let target_org_team = if let Some(target_project) =
                                target_project
                            {
                                if let Some(org_id) =
                                    target_project.inner.organization_id
                                {
                                    database::models::TeamMember::get_from_user_id_organization(
                                        org_id,
                                        user.id.into(),
                                        false,  // allow_pending
                                        &mut *transaction,
                                    ).await?
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // 判断是否需要自动审核通过
                            if user.role.is_admin() {
                                log::info!(
                                    "Version link auto-approved for version {} edit targeting version {} in project {}: User {} is an ADMIN",
                                    version_id,
                                    target_version_id.0,
                                    target_project_id.0,
                                    user.username
                                );
                                auto_approve = true;
                            } else if user.role.is_mod() {
                                log::info!(
                                    "Version link auto-approved for version {} edit targeting version {} in project {}: User {} is a MODERATOR",
                                    version_id,
                                    target_version_id.0,
                                    target_project_id.0,
                                    user.username
                                );
                                auto_approve = true;
                            } else if target_team.as_ref().is_some_and(|m| {
                                m.accepted
                                    && m.permissions.contains(
                                        ProjectPermissions::UPLOAD_VERSION,
                                    )
                            }) {
                                log::info!(
                                    "Version link auto-approved for version {} edit targeting version {} in project {}: User {} is a TARGET PROJECT TEAM MEMBER with UPLOAD_VERSION permission",
                                    version_id,
                                    target_version_id.0,
                                    target_project_id.0,
                                    user.username
                                );
                                auto_approve = true;
                            } else if target_org_team.as_ref().is_some_and(
                                |m| {
                                    m.accepted
                                        && m.permissions.contains(
                                            ProjectPermissions::UPLOAD_VERSION,
                                        )
                                },
                            ) {
                                log::info!(
                                    "Version link auto-approved for version {} edit targeting version {} in project {}: User {} is a TARGET ORGANIZATION TEAM MEMBER with UPLOAD_VERSION permission",
                                    version_id,
                                    target_version_id.0,
                                    target_project_id.0,
                                    user.username
                                );
                                auto_approve = true;
                            } else {
                                log::info!(
                                    "Version link requires approval for version {} edit targeting version {} in project {}: User {} does not have auto-approval permissions in the target project",
                                    version_id,
                                    target_version_id.0,
                                    target_project_id.0,
                                    user.username
                                );
                            }
                        } else {
                            log::warn!(
                                "Target version {} not found for version link from version {} during edit",
                                target_version_id.0,
                                version_id
                            );
                        }

                        if auto_approve {
                            "approved".to_string()
                        } else {
                            "pending".to_string()
                        }
                    };

                    builders.push(VersionLinkBuilder {
                        joining_version_id: link.joining_version_id.into(),
                        link_type: link.link_type.clone(),
                        language_code: link.language_code.clone(),
                        description: link.description.clone(),
                        approval_status,
                    });
                }

                VersionLinkBuilder::insert_many(
                    builders,
                    version_item.inner.id,
                    &mut transaction,
                )
                .await?;

                // 清除所有受影响的目标版本缓存
                for version_id in versions_to_clear_cache {
                    if let Some(target_version) =
                        database::models::Version::get(
                            database::models::ids::VersionId(version_id),
                            &mut *transaction,
                            &redis,
                        )
                        .await?
                    {
                        database::models::Version::clear_cache(
                            &target_version,
                            &redis,
                        )
                        .await?;
                    }
                }
            }

            if !new_version.fields.is_empty() {
                let version_fields_names = new_version
                    .fields
                    .keys()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>();

                let all_loaders =
                    loader_fields::Loader::list(&mut *transaction, &redis)
                        .await?;
                let loader_ids = version_item
                    .loaders
                    .iter()
                    .filter_map(|x| {
                        all_loaders
                            .iter()
                            .find(|y| &y.loader == x)
                            .map(|y| y.id)
                    })
                    .collect_vec();

                let loader_fields = LoaderField::get_fields(
                    &loader_ids,
                    &mut *transaction,
                    &redis,
                )
                .await?
                .into_iter()
                .filter(|lf| version_fields_names.contains(&lf.field))
                .collect::<Vec<LoaderField>>();

                let loader_field_ids = loader_fields
                    .iter()
                    .map(|lf| lf.id.0)
                    .collect::<Vec<i32>>();
                sqlx::query!(
                    "
                    DELETE FROM version_fields
                    WHERE version_id = $1
                    AND field_id = ANY($2)
                    ",
                    id as database::models::ids::VersionId,
                    &loader_field_ids
                )
                .execute(&mut *transaction)
                .await?;

                let mut loader_field_enum_values =
                    LoaderFieldEnumValue::list_many_loader_fields(
                        &loader_fields,
                        &mut *transaction,
                        &redis,
                    )
                    .await?;

                let mut version_fields = Vec::new();
                for (vf_name, vf_value) in new_version.fields {
                    let loader_field = loader_fields
                        .iter()
                        .find(|lf| lf.field == vf_name)
                        .ok_or_else(|| {
                            ApiError::InvalidInput(format!("加载器字段 '{vf_name}' 不存在于提供的任何加载器中。"))
                        })?;
                    let enum_variants = loader_field_enum_values
                        .remove(&loader_field.id)
                        .unwrap_or_default();
                    let vf: VersionField = VersionField::check_parse(
                        version_id.into(),
                        loader_field.clone(),
                        vf_value.clone(),
                        enum_variants,
                    )
                    .map_err(ApiError::InvalidInput)?;
                    version_fields.push(vf);
                }
                VersionField::insert_many(version_fields, &mut transaction)
                    .await?;
            }

            if let Some(loaders) = &new_version.loaders {
                sqlx::query!(
                    "
                    DELETE FROM loaders_versions WHERE version_id = $1
                    ",
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;

                let mut loader_versions = Vec::new();
                for loader in loaders {
                    let loader_id =
                        database::models::loader_fields::Loader::get_id(
                            &loader.0,
                            &mut *transaction,
                            &redis,
                        )
                        .await?
                        .ok_or_else(|| {
                            ApiError::InvalidInput(
                                "没有提供的加载器的数据库条目。".to_string(),
                            )
                        })?;
                    loader_versions.push(LoaderVersion::new(loader_id, id));
                }
                LoaderVersion::insert_many(loader_versions, &mut transaction)
                    .await?;

                crate::database::models::Project::clear_cache(
                    version_item.inner.project_id,
                    None,
                    None,
                    &redis,
                )
                .await?;
            }

            if let Some(featured) = &new_version.featured {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET featured = $1
                    WHERE (id = $2)
                    ",
                    featured,
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(body) = &new_version.changelog {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET changelog = $1
                    WHERE (id = $2)
                    ",
                    body,
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(downloads) = &new_version.downloads {
                if !user.role.is_mod() {
                    return Err(ApiError::CustomAuthentication(
                        "您没有权限设置此模组的下载次数！".to_string(),
                    ));
                }

                sqlx::query!(
                    "
                    UPDATE versions
                    SET downloads = $1
                    WHERE (id = $2)
                    ",
                    *downloads as i32,
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;

                let diff = *downloads - (version_item.inner.downloads as u32);

                sqlx::query!(
                    "
                    UPDATE mods
                    SET downloads = downloads + $1
                    WHERE (id = $2)
                    ",
                    diff as i32,
                    version_item.inner.project_id
                        as database::models::ids::ProjectId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            if let Some(status) = &new_version.status {
                if !status.can_be_requested() {
                    return Err(ApiError::InvalidInput(
                        "请求的状态无法设置!".to_string(),
                    ));
                }

                sqlx::query!(
                    "
                    UPDATE versions
                    SET status = $1
                    WHERE (id = $2)
                    ",
                    status.as_str(),
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;

                // 如果状态发生变化，且这个版本是汉化包，清除所有目标版本的缓存
                let version_links = sqlx::query!(
                    "
                    SELECT joining_version_id FROM version_link_version WHERE version_id = $1
                    ",
                    id as database::models::ids::VersionId,
                )
                .fetch_all(&mut *transaction)
                .await?;

                for link in version_links {
                    let target_version_id = database::models::ids::VersionId(
                        link.joining_version_id,
                    );
                    if let Some(target_version) =
                        database::models::Version::get(
                            target_version_id,
                            &mut *transaction,
                            &redis,
                        )
                        .await?
                    {
                        database::models::Version::clear_cache(
                            &target_version,
                            &redis,
                        )
                        .await?;
                    }
                }
            }

            if let Some(file_types) = &new_version.file_types {
                for file_type in file_types {
                    let result = sqlx::query!(
                        "
                        SELECT f.id id FROM hashes h
                        INNER JOIN files f ON h.file_id = f.id
                        WHERE h.algorithm = $2 AND h.hash = $1
                        ",
                        file_type.hash.as_bytes(),
                        file_type.algorithm
                    )
                    .fetch_optional(&**pool)
                    .await?
                    .ok_or_else(|| {
                        ApiError::InvalidInput(format!(
                            "Specified file with hash {} does not exist.",
                            file_type.algorithm.clone()
                        ))
                    })?;

                    sqlx::query!(
                        "
                        UPDATE files
                        SET file_type = $2
                        WHERE (id = $1)
                        ",
                        result.id,
                        file_type.file_type.as_ref().map(|x| x.as_str()),
                    )
                    .execute(&mut *transaction)
                    .await?;
                }
            }

            if let Some(ordering) = &new_version.ordering {
                sqlx::query!(
                    "
                    UPDATE versions
                    SET ordering = $1
                    WHERE (id = $2)
                    ",
                    ordering.to_owned() as Option<i32>,
                    id as database::models::ids::VersionId,
                )
                .execute(&mut *transaction)
                .await?;
            }

            // delete any images no longer in the changelog
            let checkable_strings: Vec<&str> = vec![&new_version.changelog]
                .into_iter()
                .filter_map(|x| x.as_ref().map(|y| y.as_str()))
                .collect();
            let context = ImageContext::Version {
                version_id: Some(version_item.inner.id.into()),
            };

            img::delete_unused_images(
                context,
                checkable_strings,
                &mut transaction,
                &redis,
            )
            .await?;

            transaction.commit().await?;
            database::models::Version::clear_cache(&version_item, &redis)
                .await?;
            database::models::Project::clear_cache(
                version_item.inner.project_id,
                None,
                Some(true),
                &redis,
            )
            .await?;
            Ok(HttpResponse::NoContent().body(""))
        } else {
            Err(ApiError::CustomAuthentication(
                "您没有权限编辑此版本！".to_string(),
            ))
        }
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Serialize, Deserialize)]
pub struct VersionListFilters {
    pub loaders: Option<String>,
    pub featured: Option<bool>,
    pub version_type: Option<VersionType>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    /*
        要过滤的加载器字段:
        "game_versions": ["1.16.5", "1.17"]

        返回如果它匹配任何值
    */
    pub loader_fields: Option<String>,
}

pub async fn version_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    web::Query(filters): web::Query<VersionListFilters>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let string = info.into_inner().0;

    let result =
        database::models::Project::get(&string, &**pool, &redis).await?;

    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ, Scopes::VERSION_READ]),
    )
    .await
    .map(|x| x.1)
    .ok();

    if let Some(project) = result {
        if !is_visible_project(&project.inner, &user_option, &pool, false)
            .await?
        {
            return Err(ApiError::NotFound);
        }

        let loader_field_filters = filters.loader_fields.as_ref().map(|x| {
            serde_json::from_str::<HashMap<String, Vec<serde_json::Value>>>(x)
                .unwrap_or_default()
        });
        let loader_filters = filters.loaders.as_ref().map(|x| {
            serde_json::from_str::<Vec<String>>(x).unwrap_or_default()
        });
        let mut versions = database::models::Version::get_many(
            &project.versions,
            &**pool,
            &redis,
        )
        .await?
        .into_iter()
        .skip(filters.offset.unwrap_or(0))
        .take(filters.limit.unwrap_or(usize::MAX))
        .filter(|x| {
            let mut bool = true;

            if let Some(version_type) = filters.version_type {
                bool &= &*x.inner.version_type == version_type.as_str();
            }
            if let Some(loaders) = &loader_filters {
                bool &= x.loaders.iter().any(|y| loaders.contains(y));
            }
            if let Some(loader_fields) = &loader_field_filters {
                for (key, values) in loader_fields {
                    bool &= if let Some(x_vf) =
                        x.version_fields.iter().find(|y| y.field_name == *key)
                    {
                        values.iter().any(|v| x_vf.value.contains_json_value(v))
                    } else {
                        true
                    };
                }
            }
            bool
        })
        .collect::<Vec<_>>();

        let mut response = versions
            .iter()
            .filter(|version| {
                filters
                    .featured
                    .map(|featured| featured == version.inner.featured)
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();

        versions.sort_by(|a, b| {
            b.inner.date_published.cmp(&a.inner.date_published)
        });

        // 尝试用 "自动推荐" 版本填充 versions
        if response.is_empty()
            && !versions.is_empty()
            && filters.featured.unwrap_or(false)
        {
            // TODO: 这是一个临时代码，用于检测自动推荐的版本。
            // 在将来，不是所有的版本都会有 'game_versions' 字段，所以这需要改变。
            let (loaders, game_versions) = futures::future::try_join(
                database::models::loader_fields::Loader::list(&**pool, &redis),
                database::models::legacy_loader_fields::MinecraftGameVersion::list(
                    None,
                    Some(true),
                    &**pool,
                    &redis,
                ),
            )
            .await?;

            let mut joined_filters = Vec::new();
            for game_version in &game_versions {
                for loader in &loaders {
                    joined_filters.push((game_version, loader))
                }
            }

            joined_filters.into_iter().for_each(|filter| {
                versions
                    .iter()
                    .find(|version| {
                        // TODO: 这是一个临时代码，用于检测自动推荐的版本。
                        let game_versions = version
                            .version_fields
                            .iter()
                            .find(|vf| vf.field_name == "game_versions")
                            .map(|vf| vf.value.clone())
                            .map(|v| v.as_strings())
                            .unwrap_or_default();
                        game_versions.contains(&filter.0.version)
                            && version.loaders.contains(&filter.1.loader)
                    })
                    .map(|version| response.push(version.clone()))
                    .unwrap_or(());
            });

            if response.is_empty() {
                versions
                    .into_iter()
                    .for_each(|version| response.push(version));
            }
        }

        response.sort_by(|a, b| {
            b.inner.date_published.cmp(&a.inner.date_published)
        });
        response.dedup_by(|a, b| a.inner.id == b.inner.id);

        let response =
            filter_visible_versions(response, &user_option, &pool, &redis)
                .await?;

        Ok(HttpResponse::Ok().json(response))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn version_delete(
    req: HttpRequest,
    info: web::Path<(VersionId,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    search_config: web::Data<SearchConfig>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_DELETE]),
    )
    .await?
    .1;

    // 检查用户是否被资源类封禁
    check_resource_ban(&user, &pool).await?;

    let id = info.into_inner().0;

    let version = database::models::Version::get(id.into(), &**pool, &redis)
        .await?
        .ok_or_else(|| {
            ApiError::InvalidInput("指定的版本不存在！".to_string())
        })?;

    if !user.role.is_admin() {
        let team_member =
            database::models::TeamMember::get_from_user_id_project(
                version.inner.project_id,
                user.id.into(),
                false,
                &**pool,
            )
            .await
            .map_err(ApiError::Database)?;

        let organization =
            Organization::get_associated_organization_project_id(
                version.inner.project_id,
                &**pool,
            )
            .await?;

        let organization_team_member = if let Some(organization) = &organization
        {
            database::models::TeamMember::get_from_user_id(
                organization.team_id,
                user.id.into(),
                &**pool,
            )
            .await?
        } else {
            None
        };
        let permissions = ProjectPermissions::get_permissions_by_role(
            &user.role,
            &team_member,
            &organization_team_member,
        )
        .unwrap_or_default();

        if !permissions.contains(ProjectPermissions::DELETE_VERSION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限删除此团队中的版本".to_string(),
            ));
        }
    }

    let mut transaction = pool.begin().await?;
    let context = ImageContext::Version {
        version_id: Some(version.inner.id.into()),
    };
    let uploaded_images =
        database::models::Image::get_many_contexted(context, &mut transaction)
            .await?;
    for image in uploaded_images {
        image_item::Image::remove(image.id, &mut transaction, &redis).await?;
    }

    let result = database::models::Version::remove_full(
        version.inner.id,
        &redis,
        &mut transaction,
    )
    .await?;
    transaction.commit().await?;
    remove_documents(&[version.inner.id.into()], &search_config).await?;
    database::models::Project::clear_cache(
        version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;

    if result.is_some() {
        Ok(HttpResponse::NoContent().body(""))
    } else {
        Err(ApiError::NotFound)
    }
}

// 批准版本链接
pub async fn approve_version_link(
    req: HttpRequest,
    info: web::Path<(VersionId, VersionId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (translation_version_id, target_version_id) = info.into_inner();

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    // 获取目标版本（被翻译的版本）
    let target_version = database::models::Version::get(
        target_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound)?;

    // 检查用户是否有权限管理目标项目的版本
    let target_project_id = target_version.inner.project_id;
    let team_member = database::models::TeamMember::get_from_user_id_project(
        target_project_id,
        user.id.into(),
        false,
        &**pool,
    )
    .await?;

    let organization =
        database::models::Organization::get_associated_organization_project_id(
            target_project_id,
            &**pool,
        )
        .await?;

    let organization_team_member = if let Some(organization) = &organization {
        database::models::TeamMember::get_from_user_id(
            organization.team_id,
            user.id.into(),
            &**pool,
        )
        .await?
    } else {
        None
    };

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    if let Some(perms) = permissions {
        if !perms.contains(ProjectPermissions::UPLOAD_VERSION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限管理此项目的翻译链接".to_string(),
            ));
        }
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限管理此项目的翻译链接".to_string(),
        ));
    }

    // 获取翻译版本
    let translation_version = database::models::Version::get(
        translation_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 更新链接状态为已批准
    let mut transaction = pool.begin().await?;

    // 先获取当前链接信息，看是否有thread_id
    let link_info = sqlx::query!(
        "SELECT thread_id FROM version_link_version WHERE version_id = $1 AND joining_version_id = $2",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    sqlx::query!(
        "
        UPDATE version_link_version
        SET approval_status = 'approved'
        WHERE version_id = $1 AND joining_version_id = $2
        ",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    // 创建或获取thread，然后添加批准消息
    if let Some(link) = link_info {
        use crate::database::models::thread_item::{
            ThreadBuilder, ThreadMessageBuilder,
        };
        use crate::models::threads::{MessageBody, ThreadType};

        let thread_id = if let Some(existing_thread_id) = link.thread_id {
            database::models::ids::ThreadId(existing_thread_id)
        } else {
            // 如果没有thread，先创建一个
            let new_thread_id = ThreadBuilder {
                type_: ThreadType::VersionLink,
                members: vec![],
                project_id: None,
                report_id: None,
                ban_appeal_id: None,
            }
            .insert(&mut transaction)
            .await?;

            // 更新version_link_version表中的thread_id
            sqlx::query!(
                "UPDATE version_link_version SET thread_id = $1 WHERE version_id = $2 AND joining_version_id = $3",
                new_thread_id.0,
                translation_version_id.0 as i64,
                target_version_id.0 as i64,
            )
            .execute(&mut *transaction)
            .await?;

            new_thread_id
        };

        // 创建系统消息表示链接已批准
        ThreadMessageBuilder {
            author_id: Some(user.id.into()),
            body: MessageBody::Text {
                body: "✅ 翻译链接已批准".to_string(),
                replying_to: None,
                private: false,
                associated_images: vec![],
            },
            thread_id,
            hide_identity: false,
        }
        .insert(&mut transaction)
        .await?;
    }

    transaction.commit().await?;

    // 清除两个版本的缓存，确保 translated_by 和 version_links 都更新
    database::models::Version::clear_cache(&target_version, &redis).await?;
    database::models::Version::clear_cache(&translation_version, &redis)
        .await?;

    // 清除两个项目的缓存，因为项目可能缓存了版本列表
    database::models::Project::clear_cache(
        target_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;
    database::models::Project::clear_cache(
        translation_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}

// 拒绝版本链接
pub async fn reject_version_link(
    req: HttpRequest,
    info: web::Path<(VersionId, VersionId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (translation_version_id, target_version_id) = info.into_inner();

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    // 获取目标版本（被翻译的版本）
    let target_version = database::models::Version::get(
        target_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound)?;

    // 检查用户是否有权限管理目标项目的版本
    let target_project_id = target_version.inner.project_id;
    let team_member = database::models::TeamMember::get_from_user_id_project(
        target_project_id,
        user.id.into(),
        false,
        &**pool,
    )
    .await?;

    let organization =
        database::models::Organization::get_associated_organization_project_id(
            target_project_id,
            &**pool,
        )
        .await?;

    let organization_team_member = if let Some(organization) = &organization {
        database::models::TeamMember::get_from_user_id(
            organization.team_id,
            user.id.into(),
            &**pool,
        )
        .await?
    } else {
        None
    };

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    if let Some(perms) = permissions {
        if !perms.contains(ProjectPermissions::UPLOAD_VERSION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限管理此项目的翻译链接".to_string(),
            ));
        }
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限管理此项目的翻译链接".to_string(),
        ));
    }

    // 获取翻译版本
    let translation_version = database::models::Version::get(
        translation_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 更新链接状态为已拒绝（不删除，保留thread）
    let mut transaction = pool.begin().await?;

    // 先获取当前链接信息，看是否有thread_id
    let link_info = sqlx::query!(
        "SELECT thread_id FROM version_link_version WHERE version_id = $1 AND joining_version_id = $2",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    sqlx::query!(
        "
        UPDATE version_link_version
        SET approval_status = 'rejected'
        WHERE version_id = $1 AND joining_version_id = $2
        ",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    // 创建或获取thread，然后添加拒绝消息
    if let Some(link) = link_info {
        use crate::database::models::thread_item::{
            ThreadBuilder, ThreadMessageBuilder,
        };
        use crate::models::threads::{MessageBody, ThreadType};

        let thread_id = if let Some(existing_thread_id) = link.thread_id {
            database::models::ids::ThreadId(existing_thread_id)
        } else {
            // 如果没有thread，先创建一个
            let new_thread_id = ThreadBuilder {
                type_: ThreadType::VersionLink,
                members: vec![],
                project_id: None,
                report_id: None,
                ban_appeal_id: None,
            }
            .insert(&mut transaction)
            .await?;

            // 更新version_link_version表中的thread_id
            sqlx::query!(
                "UPDATE version_link_version SET thread_id = $1 WHERE version_id = $2 AND joining_version_id = $3",
                new_thread_id.0,
                translation_version_id.0 as i64,
                target_version_id.0 as i64,
            )
            .execute(&mut *transaction)
            .await?;

            new_thread_id
        };

        // 创建系统消息表示链接已拒绝
        ThreadMessageBuilder {
            author_id: Some(user.id.into()),
            body: MessageBody::Text {
                body: "❌ 翻译链接已拒绝".to_string(),
                replying_to: None,
                private: false,
                associated_images: vec![],
            },
            thread_id,
            hide_identity: false,
        }
        .insert(&mut transaction)
        .await?;
    }

    transaction.commit().await?;

    // 清除两个版本的缓存
    database::models::Version::clear_cache(&target_version, &redis).await?;
    database::models::Version::clear_cache(&translation_version, &redis)
        .await?;

    // 清除两个项目的缓存
    database::models::Project::clear_cache(
        target_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;
    database::models::Project::clear_cache(
        translation_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}

// 撤销已批准的版本链接
pub async fn revoke_version_link(
    req: HttpRequest,
    info: web::Path<(VersionId, VersionId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let (translation_version_id, target_version_id) = info.into_inner();

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    // 获取目标版本（被翻译的版本）
    let target_version = database::models::Version::get(
        target_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound)?;

    // 检查用户是否有权限管理目标项目的版本
    let target_project_id = target_version.inner.project_id;
    let team_member = database::models::TeamMember::get_from_user_id_project(
        target_project_id,
        user.id.into(),
        false,
        &**pool,
    )
    .await?;

    let organization =
        database::models::Organization::get_associated_organization_project_id(
            target_project_id,
            &**pool,
        )
        .await?;

    let organization_team_member = if let Some(organization) = &organization {
        database::models::TeamMember::get_from_user_id(
            organization.team_id,
            user.id.into(),
            &**pool,
        )
        .await?
    } else {
        None
    };

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    if let Some(perms) = permissions {
        if !perms.contains(ProjectPermissions::UPLOAD_VERSION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限管理此项目的翻译链接".to_string(),
            ));
        }
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限管理此项目的翻译链接".to_string(),
        ));
    }

    // 获取翻译版本
    let translation_version = database::models::Version::get(
        translation_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 删除链接（撤销）
    let mut transaction = pool.begin().await?;

    sqlx::query!(
        "
        DELETE FROM version_link_version
        WHERE version_id = $1 AND joining_version_id = $2
        ",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    // 清除两个版本的缓存
    database::models::Version::clear_cache(&target_version, &redis).await?;
    database::models::Version::clear_cache(&translation_version, &redis)
        .await?;

    // 清除两个项目的缓存
    database::models::Project::clear_cache(
        target_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;
    database::models::Project::clear_cache(
        translation_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}

// 重新提交被拒绝的版本链接
pub async fn resubmit_version_link(
    req: HttpRequest,
    info: web::Path<(VersionId, VersionId)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let (translation_version_id, target_version_id) = info.into_inner();

    // 从body中获取重新提交的原因
    let reason =
        body.get("reason").and_then(|v| v.as_str()).ok_or_else(|| {
            ApiError::InvalidInput("需要提供重新提交的原因".to_string())
        })?;

    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    // 获取翻译版本
    let translation_version = database::models::Version::get(
        translation_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 获取目标版本
    let target_version = database::models::Version::get(
        target_version_id.into(),
        &**pool,
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    // 检查用户是否是翻译项目的成员
    let translation_project_id = translation_version.inner.project_id;
    let team_member = database::models::TeamMember::get_from_user_id_project(
        translation_project_id,
        user.id.into(),
        false,
        &**pool,
    )
    .await?;

    let organization =
        database::models::Organization::get_associated_organization_project_id(
            translation_project_id,
            &**pool,
        )
        .await?;

    let organization_team_member = if let Some(organization) = &organization {
        database::models::TeamMember::get_from_user_id(
            organization.team_id,
            user.id.into(),
            &**pool,
        )
        .await?
    } else {
        None
    };

    let permissions = ProjectPermissions::get_permissions_by_role(
        &user.role,
        &team_member,
        &organization_team_member,
    );

    if let Some(perms) = permissions {
        if !perms.contains(ProjectPermissions::UPLOAD_VERSION) {
            return Err(ApiError::CustomAuthentication(
                "您没有权限重新提交此版本链接".to_string(),
            ));
        }
    } else {
        return Err(ApiError::CustomAuthentication(
            "您没有权限重新提交此版本链接".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;

    // 检查链接是否存在且状态为rejected
    let link_info = sqlx::query!(
        "SELECT thread_id, approval_status FROM version_link_version WHERE version_id = $1 AND joining_version_id = $2",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .fetch_optional(&mut *transaction)
    .await?;

    let link = link_info
        .ok_or_else(|| ApiError::InvalidInput("版本链接不存在".to_string()))?;

    // 只有被拒绝的链接才能重新提交
    if link.approval_status != "rejected" {
        return Err(ApiError::InvalidInput(
            "只有被拒绝的链接才能重新提交审核".to_string(),
        ));
    }

    // 更新链接状态为pending
    sqlx::query!(
        "
        UPDATE version_link_version
        SET approval_status = 'pending'
        WHERE version_id = $1 AND joining_version_id = $2
        ",
        translation_version_id.0 as i64,
        target_version_id.0 as i64,
    )
    .execute(&mut *transaction)
    .await?;

    // 如果有thread，添加重新提交的消息
    if let Some(thread_id) = link.thread_id {
        use crate::database::models::thread_item::ThreadMessageBuilder;
        use crate::models::threads::MessageBody;

        let thread_id = database::models::ids::ThreadId(thread_id);

        // 创建系统消息表示链接已重新提交
        ThreadMessageBuilder {
            author_id: Some(user.id.into()),
            body: MessageBody::Text {
                body: format!("📝 重新提交审核\n\n重新提交原因：\n{}", reason),
                replying_to: None,
                private: false,
                associated_images: vec![],
            },
            thread_id,
            hide_identity: false,
        }
        .insert(&mut transaction)
        .await?;
    } else {
        // 如果没有thread，创建一个并添加消息
        use crate::database::models::thread_item::{
            ThreadBuilder, ThreadMessageBuilder,
        };
        use crate::models::threads::{MessageBody, ThreadType};

        let new_thread_id = ThreadBuilder {
            type_: ThreadType::VersionLink,
            members: vec![],
            project_id: None,
            report_id: None,
            ban_appeal_id: None,
        }
        .insert(&mut transaction)
        .await?;

        // 更新version_link_version表中的thread_id
        sqlx::query!(
            "UPDATE version_link_version SET thread_id = $1 WHERE version_id = $2 AND joining_version_id = $3",
            new_thread_id.0,
            translation_version_id.0 as i64,
            target_version_id.0 as i64,
        )
        .execute(&mut *transaction)
        .await?;

        // 创建重新提交消息
        ThreadMessageBuilder {
            author_id: Some(user.id.into()),
            body: MessageBody::Text {
                body: format!("📝 重新提交审核\n\n重新提交原因：\n{}", reason),
                replying_to: None,
                private: false,
                associated_images: vec![],
            },
            thread_id: new_thread_id,
            hide_identity: false,
        }
        .insert(&mut transaction)
        .await?;
    }

    transaction.commit().await?;

    // 清除两个版本的缓存
    database::models::Version::clear_cache(&target_version, &redis).await?;
    database::models::Version::clear_cache(&translation_version, &redis)
        .await?;

    // 清除两个项目的缓存
    database::models::Project::clear_cache(
        target_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;
    database::models::Project::clear_cache(
        translation_version.inner.project_id,
        None,
        Some(true),
        &redis,
    )
    .await?;

    Ok(HttpResponse::NoContent().body(""))
}
