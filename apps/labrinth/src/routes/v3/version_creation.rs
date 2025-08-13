use super::project_creation::{CreateError, UploadedFile};
use crate::auth::get_user_from_headers;
use crate::database::models::loader_fields::{
    LoaderField, LoaderFieldEnumValue, VersionField,
};
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::version_item::{
    DependencyBuilder, QueryDisk, VersionBuilder, VersionFileBuilder,
};
use crate::database::models::{self, image_item, Organization};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::images::{Image, ImageContext, ImageId};
use crate::models::notifications::NotificationBody;
use crate::models::pack::PackFileHash;
use crate::models::pats::Scopes;
use crate::models::projects::{skip_nulls, DependencyType, ProjectStatus};
use crate::models::projects::{
    Dependency, FileType, Loader, ProjectId, Version, VersionFile, VersionId,
    VersionLink, VersionStatus, VersionType,
};
use crate::models::teams::ProjectPermissions;
use crate::queue::moderation::AutomatedModerationQueue;
use crate::queue::session::AuthQueue;
use crate::util::routes::read_from_field;
use crate::util::validate::validation_errors_to_string;
use crate::validate::{validate_file, ValidationResult};
use actix_multipart::{Field, Multipart};
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use futures::stream::StreamExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use validator::Validate;

fn default_requested_status() -> VersionStatus {
    VersionStatus::Listed
}

#[derive(Serialize, Deserialize, Validate, Clone)]
pub struct InitialVersionData {
    #[serde(alias = "mod_id")]
    pub project_id: Option<ProjectId>,
    #[validate(length(min = 0, max = 256))]
    pub file_parts: Vec<String>,
    #[validate(
        length(min = 1, max = 32),
        regex = "crate::util::validate::RE_URL_SAFE"
    )]
    pub version_number: String,
    #[validate(
        length(min = 1, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    #[serde(alias = "name")]
    pub version_title: String,
    #[validate(length(max = 65536))]
    #[serde(alias = "changelog")]
    pub version_body: Option<String>,
    #[validate(
        length(min = 0, max = 4096),
        custom(function = "crate::util::validate::validate_deps")
    )]
    pub dependencies: Vec<Dependency>,
    #[validate(length(min = 0, max = 256))]
    pub version_links: Option<Vec<VersionLink>>,
    #[serde(alias = "version_type")]
    pub release_channel: VersionType,
    #[validate(length(min = 1))]
    pub loaders: Vec<Loader>,
    pub featured: bool,
    pub primary_file: Option<String>,
    #[serde(default = "default_requested_status")]
    pub status: VersionStatus,
    #[serde(default = "HashMap::new")]
    pub file_types: HashMap<String, Option<FileType>>,
    // 关联到 changelog 中的上传图像
    #[validate(length(max = 10))]
    #[serde(default)]
    pub uploaded_images: Vec<ImageId>,
    // 相对于其他版本的顺序
    pub ordering: Option<i32>,

    // Flattened loader fields
    // 所有其他字段都是特定于加载器的 VersionFields
    // 这些在序列化期间被展平
    #[serde(deserialize_with = "skip_nulls")]
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,

    pub disk_only: bool,
    pub disk_urls: Option<Vec<QueryDisk>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct InitialFileData {
    #[serde(default = "HashMap::new")]
    pub file_types: HashMap<String, Option<FileType>>,
}

// under `/api/v1/version`
pub async fn version_create(
    req: HttpRequest,
    mut payload: Multipart,
    client: Data<PgPool>,
    redis: Data<RedisPool>,
    file_host: Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: Data<AuthQueue>,
    moderation_queue: web::Data<AutomatedModerationQueue>,
) -> Result<HttpResponse, CreateError> {
    let mut transaction = client.begin().await?;
    let mut uploaded_files = Vec::new();

    let result = version_create_inner(
        req,
        &mut payload,
        &mut transaction,
        &redis,
        &***file_host,
        &mut uploaded_files,
        &client,
        &session_queue,
        &moderation_queue,
    )
    .await;

    if result.is_err() {
        let undo_result = super::project_creation::undo_uploads(
            &***file_host,
            &uploaded_files,
        )
        .await;
        let rollback_result = transaction.rollback().await;

        undo_result?;
        if let Err(e) = rollback_result {
            return Err(e.into());
        }
    } else {
        transaction.commit().await?;
    }

    result
}

#[allow(clippy::too_many_arguments)]
async fn version_create_inner(
    req: HttpRequest,
    payload: &mut Multipart,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
    file_host: &dyn FileHost,
    uploaded_files: &mut Vec<UploadedFile>,
    pool: &PgPool,
    session_queue: &AuthQueue,
    moderation_queue: &AutomatedModerationQueue,
) -> Result<HttpResponse, CreateError> {
    let cdn_url = dotenvy::var("CDN_URL")?;

    let mut initial_version_data = None;
    let mut version_builder = None;
    let mut selected_loaders = None;

    let user = get_user_from_headers(
        &req,
        pool,
        redis,
        session_queue,
        Some(&[Scopes::VERSION_CREATE]),
    )
    .await?
    .1;

    let mut error = None;
    while let Some(item) = payload.next().await {
        let mut field: Field = item?;

        if error.is_some() {
            continue;
        }

        let result = async {
            let content_disposition = field.content_disposition().clone();
            let name = content_disposition.get_name().ok_or_else(|| {
                CreateError::MissingValueError("缺少内容名称".to_string())
            })?;

            if name == "data" {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    data.extend_from_slice(&chunk?);
                }

                let version_create_data: InitialVersionData = serde_json::from_slice(&data)?;
                initial_version_data = Some(version_create_data);
                let version_create_data = initial_version_data.as_ref().unwrap();
                if version_create_data.project_id.is_none() {
                    return Err(CreateError::MissingValueError(
                        "缺少项目id".to_string(),
                    ));
                }

                version_create_data.validate().map_err(|err| {
                    CreateError::ValidationError(validation_errors_to_string(err, None))
                })?;

                if !version_create_data.status.can_be_requested() {
                    return Err(CreateError::InvalidInput(
                        "指定的状态不能被请求".to_string(),
                    ));
                }
                if version_create_data.disk_only && version_create_data.disk_urls.is_some() && version_create_data.disk_urls.clone().unwrap().len() > 3{
                    return Err(CreateError::InvalidInput(
                        "最多提供三个网盘".to_string(),
                    ));
                }

                let project_id: models::ProjectId = version_create_data.project_id.unwrap().into();

                // 确保项目存在
                if models::Project::get_id(project_id, &mut **transaction, redis)
                    .await?
                    .is_none()
                {
                    return Err(CreateError::InvalidInput(
                        "提供的项目id无效".to_string(),
                    ));
                }

                // 检查创建此版本的用户是否是项目团队成员
                // 项目版本正在添加。
                let team_member = models::TeamMember::get_from_user_id_project(
                    project_id,
                    user.id.into(),
                    false,
                    &mut **transaction,
                )
                .await?;

                // 获取附加的组织，如果存在，并获取成员项目权限
                let organization = models::Organization::get_associated_organization_project_id(
                    project_id,
                    &mut **transaction,
                )
                .await?;

                let organization_team_member = if let Some(organization) = &organization {
                    models::TeamMember::get_from_user_id(
                        organization.team_id,
                        user.id.into(),
                        &mut **transaction,
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

                if !permissions.contains(ProjectPermissions::UPLOAD_VERSION) {
                    return Err(CreateError::CustomAuthenticationError(
                        "您没有权限上传此版本!".to_string(),
                    ));
                }

                let version_id: VersionId = models::generate_version_id(transaction).await?.into();

                let all_loaders =
                    models::loader_fields::Loader::list(&mut **transaction, redis).await?;
                let loaders = version_create_data
                    .loaders
                    .iter()
                    .map(|x| {
                        all_loaders
                            .iter()
                            .find(|y| y.loader == x.0)
                            .cloned()
                            .ok_or_else(|| CreateError::InvalidLoader(x.0.clone()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                selected_loaders = Some(loaders.clone());
                let loader_ids: Vec<models::LoaderId> = loaders.iter().map(|y| y.id).collect_vec();

                let loader_fields =
                    LoaderField::get_fields(&loader_ids, &mut **transaction, redis).await?;
                let mut loader_field_enum_values = LoaderFieldEnumValue::list_many_loader_fields(
                    &loader_fields,
                    &mut **transaction,
                    redis,
                )
                .await?;
                let version_fields = try_create_version_fields(
                    version_id,
                    &version_create_data.fields,
                    &loader_fields,
                    &mut loader_field_enum_values,
                )?;

                let dependencies = version_create_data
                    .dependencies
                    .iter()
                    .map(|d| models::version_item::DependencyBuilder {
                        version_id: d.version_id.map(|x| x.into()),
                        project_id: d.project_id.map(|x| x.into()),
                        dependency_type: d.dependency_type.to_string(),
                        file_name: None,
                    })
                    .collect::<Vec<_>>();

                // 处理版本链接
                let mut version_links = Vec::new();
                
                if let Some(links) = &version_create_data.version_links {
                    for link in links {
                        // 获取被翻译版本的信息
                        let target_version_id: models::VersionId = link.joining_version_id.into();
                        let target_version = models::Version::get(target_version_id, &mut **transaction, redis).await?;
                        
                        let mut auto_approve = false;
                        
                        if let Some(target_version) = target_version {
                            // 获取目标项目的团队成员信息
                            let target_project_id = target_version.inner.project_id;
                            let target_team = models::TeamMember::get_from_user_id_project(
                                target_project_id,
                                user.id.into(),
                                false,  // allow_pending
                                &mut **transaction,
                            ).await?;
                            
                            // 获取目标项目的组织团队成员信息（如果有）
                            let target_project = models::Project::get_id(target_project_id, &mut **transaction, redis).await?;
                            let target_org_team = if let Some(target_project) = target_project {
                                if let Some(org_id) = target_project.inner.organization_id {
                                    models::TeamMember::get_from_user_id_organization(
                                        org_id,
                                        user.id.into(),
                                        false,  // allow_pending
                                        &mut **transaction,
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
                                    "Version link auto-approved for translation version {} targeting version {} in project {}: User {} is an ADMIN",
                                    version_id, target_version_id.0, target_project_id.0, user.username
                                );
                                auto_approve = true;
                            } else if user.role.is_mod() {
                                log::info!(
                                    "Version link auto-approved for translation version {} targeting version {} in project {}: User {} is a MODERATOR",
                                    version_id, target_version_id.0, target_project_id.0, user.username
                                );
                                auto_approve = true;
                            } else if target_team.as_ref().map_or(false, |m| m.accepted && m.permissions.contains(ProjectPermissions::UPLOAD_VERSION)) {
                                log::info!(
                                    "Version link auto-approved for translation version {} targeting version {} in project {}: User {} is a TARGET PROJECT TEAM MEMBER with UPLOAD_VERSION permission",
                                    version_id, target_version_id.0, target_project_id.0, user.username
                                );
                                auto_approve = true;
                            } else if target_org_team.as_ref().map_or(false, |m| m.accepted && m.permissions.contains(ProjectPermissions::UPLOAD_VERSION)) {
                                log::info!(
                                    "Version link auto-approved for translation version {} targeting version {} in project {}: User {} is a TARGET ORGANIZATION TEAM MEMBER with UPLOAD_VERSION permission",
                                    version_id, target_version_id.0, target_project_id.0, user.username
                                );
                                auto_approve = true;
                            } else {
                                log::info!(
                                    "Version link requires approval for translation version {} targeting version {} in project {}: User {} does not have auto-approval permissions in the target project",
                                    version_id, target_version_id.0, target_project_id.0, user.username
                                );
                            }
                        } else {
                            log::warn!(
                                "Target version {} not found for version link from version {}",
                                target_version_id.0, version_id
                            );
                        }
                        
                        version_links.push(models::version_item::VersionLinkBuilder {
                            joining_version_id: link.joining_version_id.into(),
                            link_type: link.link_type.clone(),
                            language_code: link.language_code.clone(),
                            description: link.description.clone(),
                            approval_status: if auto_approve { 
                                "approved".to_string() 
                            } else { 
                                "pending".to_string() 
                            },
                        });
                    }
                }

                // let disk_url = None;
                // if version_create_data.disk_only && version_create_data.disk_urls.is_some(){
                //     disk_url = version_create_data.disk_urls.clone().unwrap();
                // }
                version_builder = Some(VersionBuilder {
                    version_id: version_id.into(),
                    project_id,
                    author_id: user.id.into(),
                    name: version_create_data.version_title.clone(),
                    version_number: version_create_data.version_number.clone(),
                    changelog: version_create_data.version_body.clone().unwrap_or_default(),
                    files: Vec::new(),
                    dependencies,
                    version_links,
                    loaders: loader_ids,
                    version_fields,
                    version_type: version_create_data.release_channel.to_string(),
                    featured: version_create_data.featured,
                    status: version_create_data.status,
                    requested_status: None,
                    ordering: version_create_data.ordering,
                    disk_url: version_create_data.disk_urls.clone(),
                });

                return Ok(());
            }

            let version = version_builder.as_mut().ok_or_else(|| {
                CreateError::InvalidInput("`data` field 必须在文件字段之前".to_string())
            })?;



            let loaders = selected_loaders.as_ref().ok_or_else(|| {
                CreateError::InvalidInput("`data` field 必须在文件字段之前".to_string())
            })?;
            let loaders = loaders
                .iter()
                .map(|x| Loader(x.loader.clone()))
                .collect::<Vec<_>>();

            let version_data = initial_version_data
                .clone()
                .ok_or_else(|| CreateError::InvalidInput("`data` field 是必需的".to_string()))?;

            let existing_file_names = version.files.iter().map(|x| x.filename.clone()).collect();

            if version_data.file_parts.is_empty() && version_data.disk_only {
                return Ok(())
            }

            if !version_data.disk_only && version_data.file_parts.is_empty() {
                return Err(CreateError::InvalidInput(
                    "未上传文件".to_string(),
                ));
            }

            upload_file(
                &mut field,
                file_host,
                version_data.file_parts.len(),
                uploaded_files,
                &mut version.files,
                &mut version.dependencies,
                &cdn_url,
                &content_disposition,
                version.project_id.into(),
                version.version_id.into(),
                &version.version_fields,
                loaders,
                version_data.primary_file.is_some(),
                version_data.primary_file.as_deref() == Some(name),
                version_data.file_types.get(name).copied().flatten(),
                existing_file_names,
                transaction,
                redis,
                user.username.clone()
            )
            .await?;

            Ok(())
        }
        .await;

        if result.is_err() {
            error = result.err();
        }
    }

    if let Some(error) = error {
        return Err(error);
    }

    let version_data = initial_version_data.ok_or_else(|| {
        CreateError::InvalidInput("`data` field 是必需的".to_string())
    })?;
    let builder = version_builder.ok_or_else(|| {
        CreateError::InvalidInput("`data` field 是必需的".to_string())
    })?;

    if builder.files.is_empty() && !version_data.disk_only {
        return Err(CreateError::InvalidInput(
            "必须上传一个文件！".to_string(),
        ));
    }

    if version_data.disk_only && version_data.disk_urls.is_none() {
        return Err(CreateError::InvalidInput("未填写网盘地址".to_string()));
    }
    use futures::stream::TryStreamExt;

    let users = sqlx::query!(
        "
        SELECT follower_id FROM mod_follows
        WHERE mod_id = $1
        ",
        builder.project_id as crate::database::models::ids::ProjectId
    )
    .fetch(&mut **transaction)
    .map_ok(|m| models::ids::UserId(m.follower_id))
    .try_collect::<Vec<models::ids::UserId>>()
    .await?;

    let project_id: ProjectId = builder.project_id.into();
    let version_id: VersionId = builder.version_id.into();

    NotificationBuilder {
        body: NotificationBody::ProjectUpdate {
            project_id,
            version_id,
        },
    }
    .insert_many(users, &mut *transaction, redis)
    .await?;

    let loader_structs = selected_loaders.unwrap_or_default();
    let (all_project_types, all_games): (Vec<String>, Vec<String>) =
        loader_structs.iter().fold((vec![], vec![]), |mut acc, x| {
            acc.0.extend_from_slice(&x.supported_project_types);
            acc.1.extend(x.supported_games.clone());
            acc
        });
    let mut files = builder
        .files
        .iter()
        .map(|file| VersionFile {
            hashes: file
                .hashes
                .iter()
                .map(|hash| {
                    (
                        hash.algorithm.clone(),
                        // 这是一个 hack，因为哈希目前存储为 ASCII
                        // 在数据库中，但在这里表示为 Vec<u8>。 在某个时候，我们需要将哈希更改为数据库中的真实字节
                        // 并在此处添加更多处理。
                        String::from_utf8(hash.hash.clone()).unwrap(),
                    )
                })
                .collect(),
            url: file.url.clone(),
            filename: file.filename.clone(),
            primary: file.primary,
            size: file.size,
            file_type: file.file_type,
        })
        .collect::<Vec<_>>();
    let mut disk_url = None;
    if version_data.disk_only && version_data.disk_urls.is_some() {
        disk_url = version_data.disk_urls.clone();
    }
    let mut disk_urls: Vec<QueryDisk> = vec![];
    if version_data.disk_urls.is_some() {
        disk_urls = version_data.disk_urls.unwrap();
    }
    if version_data.disk_only {
        files.push(VersionFile {
            hashes: HashMap::new(),
            url: disk_url.unwrap().first().unwrap().url.clone(),
            filename: "".to_string(),
            primary: false,
            size: 0,
            file_type: None,
        });
    }
    
    // 在移动 version_links 之前先收集需要清除缓存的目标版本ID
    let target_version_ids_to_clear: Vec<models::ids::VersionId> = version_data.version_links
        .as_ref()
        .map(|links| links.iter().map(|link| link.joining_version_id.into()).collect())
        .unwrap_or_default();
    
    let response = Version {
        id: builder.version_id.into(),
        project_id: builder.project_id.into(),
        author_id: user.id,
        featured: builder.featured,
        name: builder.name.clone(),
        version_number: builder.version_number.clone(),
        project_types: all_project_types,
        games: all_games,
        changelog: builder.changelog.clone(),
        date_published: Utc::now(),
        downloads: 0,
        version_type: version_data.release_channel,
        status: builder.status,
        requested_status: builder.requested_status,
        ordering: builder.ordering,
        files,
        dependencies: version_data.dependencies,
        version_links: version_data.version_links.unwrap_or_default(),
        translated_by: Vec::new(), // 新创建的版本没有翻译
        loaders: version_data.loaders,
        fields: version_data.fields,
        disk_urls,
        disk_only: version_data.disk_only,
    };

    let project_id = builder.project_id;
    builder.insert(transaction).await?;
    
    // 清除版本链接目标版本的缓存（新建版本时）
    for target_version_id in target_version_ids_to_clear {
        if let Some(target_version) = models::Version::get(
            target_version_id,
            &mut **transaction,
            redis,
        ).await? {
            models::Version::clear_cache(&target_version, redis).await?;
        }
    }

    for image_id in version_data.uploaded_images {
        if let Some(db_image) =
            image_item::Image::get(image_id.into(), &mut **transaction, redis)
                .await?
        {
            let image: Image = db_image.into();
            if !matches!(image.context, ImageContext::Report { .. })
                || image.context.inner_id().is_some()
            {
                return Err(CreateError::InvalidInput(format!(
                    "Image {} 不是未使用的，并且处于 'version' 上下文",
                    image_id
                )));
            }

            sqlx::query!(
                "
                UPDATE uploaded_images
                SET version_id = $1
                WHERE id = $2
                ",
                version_id.0 as i64,
                image_id.0 as i64
            )
            .execute(&mut **transaction)
            .await?;

            image_item::Image::clear_cache(image.id.into(), redis).await?;
        } else {
            return Err(CreateError::InvalidInput(format!(
                "Image {} 不存在",
                image_id
            )));
        }
    }

    models::Project::clear_cache(project_id, None, Some(true), redis).await?;

    let project_status = sqlx::query!(
        "SELECT status FROM mods WHERE id = $1",
        project_id as models::ProjectId,
    )
    .fetch_optional(pool)
    .await?;

    if let Some(project_status) = project_status {
        if project_status.status == ProjectStatus::Processing.as_str() {
            moderation_queue.projects.insert(project_id.into());
        }
    }

    Ok(HttpResponse::Ok().json(response))
}

pub async fn upload_file_to_version(
    req: HttpRequest,
    url_data: web::Path<(VersionId,)>,
    mut payload: Multipart,
    client: Data<PgPool>,
    redis: Data<RedisPool>,
    file_host: Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, CreateError> {
    let mut transaction = client.begin().await?;
    let mut uploaded_files = Vec::new();

    let version_id = models::VersionId::from(url_data.into_inner().0);

    let result = upload_file_to_version_inner(
        req,
        &mut payload,
        client,
        &mut transaction,
        redis,
        &***file_host,
        &mut uploaded_files,
        version_id,
        &session_queue,
    )
    .await;

    if result.is_err() {
        let undo_result = super::project_creation::undo_uploads(
            &***file_host,
            &uploaded_files,
        )
        .await;
        let rollback_result = transaction.rollback().await;

        undo_result?;
        if let Err(e) = rollback_result {
            return Err(e.into());
        }
    } else {
        transaction.commit().await?;
    }

    result
}

#[allow(clippy::too_many_arguments)]
async fn upload_file_to_version_inner(
    req: HttpRequest,
    payload: &mut Multipart,
    client: Data<PgPool>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: Data<RedisPool>,
    file_host: &dyn FileHost,
    uploaded_files: &mut Vec<UploadedFile>,
    version_id: models::VersionId,
    session_queue: &AuthQueue,
) -> Result<HttpResponse, CreateError> {
    let cdn_url = dotenvy::var("CDN_URL")?;

    let mut initial_file_data: Option<InitialFileData> = None;
    let mut file_builders: Vec<VersionFileBuilder> = Vec::new();

    let user = get_user_from_headers(
        &req,
        &**client,
        &redis,
        session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    let result = models::Version::get(version_id, &**client, &redis).await?;

    let version = match result {
        Some(v) => v,
        None => {
            return Err(CreateError::InvalidInput(
                "提供的版本id无效".to_string(),
            ));
        }
    };

    let all_loaders =
        models::loader_fields::Loader::list(&mut **transaction, &redis).await?;
    let selected_loaders = version
        .loaders
        .iter()
        .map(|x| {
            all_loaders
                .iter()
                .find(|y| &y.loader == x)
                .cloned()
                .ok_or_else(|| CreateError::InvalidLoader(x.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if models::Project::get_id(
        version.inner.project_id,
        &mut **transaction,
        &redis,
    )
    .await?
    .is_none()
    {
        return Err(CreateError::InvalidInput("提供的项目id无效".to_string()));
    }

    if !user.role.is_admin() {
        let team_member = models::TeamMember::get_from_user_id_project(
            version.inner.project_id,
            user.id.into(),
            false,
            &mut **transaction,
        )
        .await?;

        let organization =
            Organization::get_associated_organization_project_id(
                version.inner.project_id,
                &**client,
            )
            .await?;

        let organization_team_member = if let Some(organization) = &organization
        {
            models::TeamMember::get_from_user_id(
                organization.team_id,
                user.id.into(),
                &mut **transaction,
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

        if !permissions.contains(ProjectPermissions::UPLOAD_VERSION) {
            return Err(CreateError::CustomAuthenticationError(
                "您没有权限上传文件到此版本!".to_string(),
            ));
        }
    }

    let project_id = ProjectId(version.inner.project_id.0 as u64);
    let mut error = None;
    while let Some(item) = payload.next().await {
        let mut field: Field = item?;

        if error.is_some() {
            continue;
        }

        let result = async {
            let content_disposition = field.content_disposition().clone();
            let name = content_disposition.get_name().ok_or_else(|| {
                CreateError::MissingValueError("缺少内容名称".to_string())
            })?;

            if name == "data" {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    data.extend_from_slice(&chunk?);
                }
                let file_data: InitialFileData = serde_json::from_slice(&data)?;

                initial_file_data = Some(file_data);
                return Ok(());
            }

            let file_data = initial_file_data.as_ref().ok_or_else(|| {
                CreateError::InvalidInput(String::from(
                    "`data` field 必须在文件字段之前",
                ))
            })?;

            let loaders = selected_loaders
                .iter()
                .map(|x| Loader(x.loader.clone()))
                .collect::<Vec<_>>();

            let mut dependencies = version
                .dependencies
                .iter()
                .map(|x| DependencyBuilder {
                    project_id: x.project_id,
                    version_id: x.version_id,
                    file_name: x.file_name.clone(),
                    dependency_type: x.dependency_type.clone(),
                })
                .collect();

            upload_file(
                &mut field,
                file_host,
                0,
                uploaded_files,
                &mut file_builders,
                &mut dependencies,
                &cdn_url,
                &content_disposition,
                project_id,
                version_id.into(),
                &version.version_fields,
                loaders,
                true,
                false,
                file_data.file_types.get(name).copied().flatten(),
                version.files.iter().map(|x| x.filename.clone()).collect(),
                transaction,
                &redis,
                user.username.clone(),
            )
            .await?;

            Ok(())
        }
        .await;

        if result.is_err() {
            error = result.err();
        }
    }

    if let Some(error) = error {
        return Err(error);
    }

    if file_builders.is_empty() {
        return Err(CreateError::InvalidInput(
            "至少需要上传一个文件".to_string(),
        ));
    } else {
        for file in file_builders {
            file.insert(version_id, &mut *transaction).await?;
        }
    }

    // 清除版本缓存
    models::Version::clear_cache(&version, &redis).await?;

    Ok(HttpResponse::NoContent().body(""))
}
// 此函数用于添加文件到版本，上传版本的初始文件，以及上传项目的初始版本文件

#[allow(clippy::too_many_arguments)]
pub async fn upload_file(
    field: &mut Field,
    file_host: &dyn FileHost,
    total_files_len: usize,
    uploaded_files: &mut Vec<UploadedFile>,
    version_files: &mut Vec<VersionFileBuilder>,
    dependencies: &mut Vec<DependencyBuilder>,
    cdn_url: &str,
    content_disposition: &actix_web::http::header::ContentDisposition,
    project_id: ProjectId,
    version_id: VersionId,
    version_fields: &[VersionField],
    loaders: Vec<Loader>,
    ignore_primary: bool,
    force_primary: bool,
    file_type: Option<FileType>,
    other_file_names: Vec<String>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
    username: String,
) -> Result<(), CreateError> {
    let (file_name, file_extension) = get_name_ext(content_disposition)?;

    println!("Uploading file {file_name}.{file_extension}");
    if other_file_names.contains(&format!("{}.{}", file_name, file_extension)) {
        return Err(CreateError::InvalidInput(
            "此文件在这之前已经被上传到BBSMC过,无法重复上传!1".to_string(),
        ));
    }

    if file_name.contains('/') {
        return Err(CreateError::InvalidInput(
            "文件名不能包含斜杠！".to_string(),
        ));
    }

    let content_type = crate::util::ext::project_file_type(file_extension)
        .ok_or_else(|| {
            CreateError::InvalidFileType(file_extension.to_string())
        })?;

    let data = read_from_field(
        field, 1024 * (1 << 20),
        "项目文件超出了 1GB 的上限。请联系版主或管理员以请求上传更大文件的权限。"
    ).await?;

    let hash = sha1::Sha1::from(&data).hexdigest();
    let exists = sqlx::query!(
        "
        SELECT EXISTS(SELECT 1 FROM hashes h
        INNER JOIN files f ON f.id = h.file_id
        INNER JOIN versions v ON v.id = f.version_id
        WHERE h.algorithm = $2 AND h.hash = $1 AND v.mod_id != $3)
        ",
        hash.as_bytes(),
        "sha1",
        project_id.0 as i64
    )
    .fetch_one(&mut **transaction)
    .await?
    .exists
    .unwrap_or(false);

    if exists && username.to_lowercase() != "bbsmc" {
        println!("已存在 {}", hash);
        return Err(CreateError::InvalidInput(
            "此文件在这之前已经被上传到BBSMC过,无法重复上传2".to_string(),
        ));
    }

    let validation_result = validate_file(
        data.clone().into(),
        file_extension.to_string(),
        loaders.clone(),
        file_type,
        version_fields.to_vec(),
        &mut *transaction,
        redis,
    )
    .await?;

    if let ValidationResult::PassWithPackDataAndFiles {
        ref format,
        ref files,
    } = validation_result
    {
        if dependencies.is_empty() {
            let hashes: Vec<Vec<u8>> = format
                .files
                .iter()
                .filter_map(|x| x.hashes.get(&PackFileHash::Sha1))
                .map(|x| x.as_bytes().to_vec())
                .collect();

            let res = sqlx::query!(
                "
                    SELECT v.id version_id, v.mod_id project_id, h.hash hash FROM hashes h
                    INNER JOIN files f on h.file_id = f.id
                    INNER JOIN versions v on f.version_id = v.id
                    WHERE h.algorithm = 'sha1' AND h.hash = ANY($1)
                    ",
                &*hashes
            )
            .fetch_all(&mut **transaction)
            .await?;

            for file in &format.files {
                if let Some(dep) = res.iter().find(|x| {
                    Some(&*x.hash)
                        == file
                            .hashes
                            .get(&PackFileHash::Sha1)
                            .map(|x| x.as_bytes())
                }) {
                    dependencies.push(DependencyBuilder {
                        project_id: Some(models::ProjectId(dep.project_id)),
                        version_id: Some(models::VersionId(dep.version_id)),
                        file_name: None,
                        dependency_type: DependencyType::Embedded.to_string(),
                    });
                } else if let Some(first_download) = file.downloads.first() {
                    dependencies.push(DependencyBuilder {
                        project_id: None,
                        version_id: None,
                        file_name: Some(
                            first_download
                                .rsplit('/')
                                .next()
                                .unwrap_or(first_download)
                                .to_string(),
                        ),
                        dependency_type: DependencyType::Embedded.to_string(),
                    });
                }
            }

            for file in files {
                if !file.is_empty() {
                    dependencies.push(DependencyBuilder {
                        project_id: None,
                        version_id: None,
                        file_name: Some(file.to_string()),
                        dependency_type: DependencyType::Embedded.to_string(),
                    });
                }
            }
        }
    }

    let data = data.freeze();
    let primary = (validation_result.is_passed()
        && version_files.iter().all(|x| !x.primary)
        && !ignore_primary)
        || force_primary
        || total_files_len == 1;

    let file_path_encode = format!(
        "data/{}/versions/{}/{}",
        project_id,
        version_id,
        urlencoding::encode(file_name)
    );
    let file_path =
        format!("data/{}/versions/{}/{}", project_id, version_id, &file_name);

    let upload_data = file_host
        .upload_file(content_type, &file_path, data)
        .await?;

    uploaded_files.push(UploadedFile {
        file_id: upload_data.file_id,
        file_name: file_path,
    });

    let sha1_bytes = upload_data.content_sha1.into_bytes();
    let sha512_bytes = upload_data.content_sha512.into_bytes();

    if version_files.iter().any(|x| {
        x.hashes
            .iter()
            .any(|y| y.hash == sha1_bytes || y.hash == sha512_bytes)
    }) {
        return Err(CreateError::InvalidInput(
            "此文件在这之前已经被上传到BBSMC过,无法重复上传3".to_string(),
        ));
    }

    if let ValidationResult::Warning(msg) = validation_result {
        if primary {
            return Err(CreateError::InvalidInput(msg.to_string()));
        }
    }

    version_files.push(VersionFileBuilder {
        filename: file_name.to_string(),
        url: format!("{cdn_url}/{file_path_encode}"),
        hashes: vec![
            models::version_item::HashBuilder {
                algorithm: "sha1".to_string(),
                // 这是一个无效的转换 - 数据库期望哈希的字节，但这是字符串版本。
                hash: sha1_bytes,
            },
            models::version_item::HashBuilder {
                algorithm: "sha512".to_string(),
                // 这是一个无效的转换 - 数据库期望哈希的字节，但这是字符串版本。
                hash: sha512_bytes,
            },
        ],
        primary,
        size: upload_data.content_length,
        file_type,
    });

    Ok(())
}

pub fn get_name_ext(
    content_disposition: &actix_web::http::header::ContentDisposition,
) -> Result<(&str, &str), CreateError> {
    let file_name = content_disposition.get_filename().ok_or_else(|| {
        CreateError::MissingValueError("Missing content file name".to_string())
    })?;
    let file_extension = if let Some(last_period) = file_name.rfind('.') {
        file_name.get((last_period + 1)..).unwrap_or("")
    } else {
        return Err(CreateError::MissingValueError(
            "Missing content file extension".to_string(),
        ));
    };
    Ok((file_name, file_extension))
}

// 在 project_creation 和 version_creation 之间重用的功能
// 从获取的数据创建 VersionFields 列表，并检查所有必需的字段是否存在
pub fn try_create_version_fields(
    version_id: VersionId,
    submitted_fields: &HashMap<String, serde_json::Value>,
    loader_fields: &[LoaderField],
    loader_field_enum_values: &mut HashMap<
        models::LoaderFieldId,
        Vec<LoaderFieldEnumValue>,
    >,
) -> Result<Vec<VersionField>, CreateError> {
    // info!(submitted_fields);
    let mut version_fields = vec![];
    let mut remaining_mandatory_loader_fields = loader_fields
        .iter()
        .filter(|lf| !lf.optional)
        .map(|lf| lf.field.clone())
        .collect::<HashSet<_>>();

    for (key, value) in submitted_fields.iter() {
        let loader_field = loader_fields
            .iter()
            .find(|lf| &lf.field == key)
            .ok_or_else(|| {
                CreateError::InvalidInput(format!(
                    "加载器字段 '{key}' 对于任何提供的加载器都不存在,"
                ))
            })?;
        remaining_mandatory_loader_fields.remove(&loader_field.field);
        let enum_variants = loader_field_enum_values
            .remove(&loader_field.id)
            .unwrap_or_default();

        let vf: VersionField = VersionField::check_parse(
            version_id.into(),
            loader_field.clone(),
            value.clone(),
            enum_variants,
        )
        .map_err(CreateError::InvalidInput)?;
        version_fields.push(vf);
    }

    if !remaining_mandatory_loader_fields.is_empty() {
        return Err(CreateError::InvalidInput(format!(
            "缺少必需的加载器字段: {}",
            remaining_mandatory_loader_fields.iter().join(", ")
        )));
    }
    Ok(version_fields)
}
