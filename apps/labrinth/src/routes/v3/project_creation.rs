use super::project_pricing::{validate_price, validate_validity_days};
use super::version_creation::{InitialVersionData, try_create_version_fields};
use crate::auth::{
    AuthenticationError, check_resource_ban, get_user_from_headers,
};
use crate::database::models::loader_fields::{
    Loader, LoaderField, LoaderFieldEnumValue,
};
use crate::database::models::thread_item::ThreadBuilder;
use crate::database::models::{self, User, UserId as DBUserId, image_item};
use crate::database::redis::RedisPool;
use crate::file_hosting::{FileHost, FileHostingError, S3PrivateHost};
use crate::models::error::ApiError;
use crate::models::ids::base62_impl::to_base62;
use crate::models::ids::{ImageId, OrganizationId};
use crate::models::images::{Image, ImageContext};
use crate::models::pats::Scopes;
use crate::models::projects::{
    License, Link, MonetizationStatus, ProjectId, ProjectStatus, VersionId,
    VersionStatus,
};
use crate::models::teams::{OrganizationPermissions, ProjectPermissions};
use crate::models::threads::ThreadType;
use crate::models::users::UserId;
use crate::queue::session::AuthQueue;
use crate::search::indexing::IndexingError;
use crate::util::img::upload_image_optimized;
use crate::util::routes::read_from_field;
use crate::util::validate::validation_errors_to_string;
use actix_multipart::{Field, Multipart};
use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{HttpRequest, HttpResponse};
use chrono::Utc;
use futures::stream::StreamExt;
use image::ImageError;
use itertools::Itertools;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use validator::Validate;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.route("project", web::post().to(project_create));
}

#[derive(Error, Debug)]
pub enum CreateError {
    #[error("环境错误")]
    EnvError(#[from] dotenvy::Error),
    #[error("发生未知的数据库错误")]
    SqlxDatabaseError(#[from] sqlx::Error),
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] models::DatabaseError),
    #[error("索引错误: {0}")]
    IndexingError(#[from] IndexingError),
    #[error("解析多部分负载时出错: {0}")]
    MultipartError(#[from] actix_multipart::MultipartError),
    #[error("解析 JSON 时出错: {0}")]
    SerDeError(#[from] serde_json::Error),
    #[error("验证输入时出错: {0}")]
    ValidationError(String),
    #[error("上传文件时出错: {0}")]
    FileHostingError(#[from] FileHostingError),
    #[error("验证上传文件时出错: {0}")]
    FileValidationError(#[from] crate::validate::ValidationError),
    #[error("{0}")]
    MissingValueError(String),
    #[error("图像格式无效: {0}")]
    InvalidIconFormat(String),
    #[error("多部分数据错误: {0}")]
    InvalidInput(String),
    #[error("游戏版本无效: {0}")]
    InvalidGameVersion(String),
    #[error("加载器无效: {0}")]
    InvalidLoader(String),
    #[error("类别无效: {0}")]
    InvalidCategory(String),
    #[error("版本文件类型无效: {0}")]
    InvalidFileType(String),
    #[error("该 Slug 已被占用！")]
    SlugCollision,
    #[error("认证错误: {0}")]
    Unauthorized(#[from] AuthenticationError),
    #[error("认证错误: {0}")]
    CustomAuthenticationError(String),
    #[error("图像解析错误: {0}")]
    ImageError(#[from] ImageError),
    #[error("重定向错误: {0}")]
    RerouteError(#[from] reqwest::Error),
    #[error("您已被封禁：{0}")]
    Banned(String),
}

impl actix_web::ResponseError for CreateError {
    fn status_code(&self) -> StatusCode {
        match self {
            CreateError::EnvError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::SqlxDatabaseError(..) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            CreateError::DatabaseError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::IndexingError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::FileHostingError(..) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            CreateError::SerDeError(..) => StatusCode::BAD_REQUEST,
            CreateError::MultipartError(..) => StatusCode::BAD_REQUEST,
            CreateError::MissingValueError(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidIconFormat(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidInput(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidGameVersion(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidLoader(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidCategory(..) => StatusCode::BAD_REQUEST,
            CreateError::InvalidFileType(..) => StatusCode::BAD_REQUEST,
            CreateError::Unauthorized(..) => StatusCode::UNAUTHORIZED,
            CreateError::CustomAuthenticationError(..) => {
                StatusCode::UNAUTHORIZED
            }
            CreateError::SlugCollision => StatusCode::BAD_REQUEST,
            CreateError::ValidationError(..) => StatusCode::BAD_REQUEST,
            CreateError::FileValidationError(..) => StatusCode::BAD_REQUEST,
            CreateError::ImageError(..) => StatusCode::BAD_REQUEST,
            CreateError::RerouteError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::Banned(..) => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ApiError {
            error: match self {
                CreateError::EnvError(..) => "environment_error",
                CreateError::SqlxDatabaseError(..) => "database_error",
                CreateError::DatabaseError(..) => "database_error",
                CreateError::IndexingError(..) => "indexing_error",
                CreateError::FileHostingError(..) => "file_hosting_error",
                CreateError::SerDeError(..) => "invalid_input",
                CreateError::MultipartError(..) => "invalid_input",
                CreateError::MissingValueError(..) => "invalid_input",
                CreateError::InvalidIconFormat(..) => "invalid_input",
                CreateError::InvalidInput(..) => "invalid_input",
                CreateError::InvalidGameVersion(..) => "invalid_input",
                CreateError::InvalidLoader(..) => "invalid_input",
                CreateError::InvalidCategory(..) => "invalid_input",
                CreateError::InvalidFileType(..) => "invalid_input",
                CreateError::Unauthorized(..) => "unauthorized",
                CreateError::CustomAuthenticationError(..) => "unauthorized",
                CreateError::SlugCollision => "invalid_input",
                CreateError::ValidationError(..) => "invalid_input",
                CreateError::FileValidationError(..) => "invalid_input",
                CreateError::ImageError(..) => "invalid_image",
                CreateError::RerouteError(..) => "reroute_error",
                CreateError::Banned(..) => "user_banned",
            },
            description: self.to_string(),
        })
    }
}

pub fn default_project_type() -> String {
    "mod".to_string()
}

fn default_requested_status() -> ProjectStatus {
    ProjectStatus::Approved
}

#[derive(Serialize, Deserialize, Validate, Clone)]
pub struct ProjectCreateData {
    #[validate(
        length(min = 3, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    #[serde(alias = "mod_name")]
    /// 项目的标题或名称。
    pub name: String,
    #[validate(
        length(min = 3, max = 64),
        regex(path = *crate::util::validate::RE_URL_SAFE)
    )]
    #[serde(alias = "mod_slug")]
    /// 项目的别名，用于 vanity URLs
    pub slug: String,
    #[validate(length(min = 3, max = 255))]
    #[serde(alias = "mod_description")]
    /// 项目的简短描述。
    pub summary: String,
    #[validate(length(max = 65536))]
    #[serde(alias = "mod_body")]
    /// 项目的详细描述，以 markdown 格式。
    pub description: String,

    #[validate(length(max = 32))]
    #[validate(nested)]
    /// 要与创建的项目一起上传的初始版本列表
    pub initial_versions: Vec<InitialVersionData>,
    #[validate(length(max = 3))]
    /// 项目所属的类别列表。
    pub categories: Vec<String>,
    #[validate(length(max = 256))]
    #[serde(default = "Vec::new")]
    /// 项目所属的类别列表。
    pub additional_categories: Vec<String>,

    /// 项目许可证页面的可选链接
    pub license_url: Option<String>,
    /// 项目所有捐赠链接的列表
    #[validate(custom(
        function = "crate::util::validate::validate_url_hashmap_values"
    ))]
    #[serde(default)]
    pub link_urls: HashMap<String, String>,

    /// 一个可选布尔值。如果为 true，则项目将被创建为草稿。
    pub is_draft: Option<bool>,

    /// 项目遵循的许可证 id
    pub license_id: String,

    #[validate(length(max = 64))]
    #[validate(nested)]
    /// 要上传的画廊项目的 multipart 名称
    pub gallery_items: Option<Vec<NewGalleryItem>>,
    #[serde(default = "default_requested_status")]
    /// 一旦批准，项目的状态
    pub requested_status: ProjectStatus,

    // 正文/描述中上传的图像关联
    #[validate(length(max = 10))]
    #[serde(default)]
    pub uploaded_images: Vec<ImageId>,

    /// 要创建项目的组织 id
    pub organization_id: Option<OrganizationId>,

    /// 是否为付费资源（仅高级创作者可设置）
    #[serde(default)]
    pub is_paid: bool,
    /// 付费资源价格（单位：元，整数 1-1000，仅当 is_paid=true 时有效）
    pub price: Option<i32>,
    /// 授权有效期天数（None 表示永久，仅当 is_paid=true 时有效）
    pub validity_days: Option<i32>,
}

#[derive(Serialize, Deserialize, Validate, Clone)]
pub struct NewGalleryItem {
    /// 画廊媒体所在的 multipart 项的名称
    pub item: String,
    /// 画廊项目是否应在搜索中显示
    pub featured: bool,
    #[validate(length(min = 1, max = 2048))]
    /// 画廊项目的标题
    pub name: Option<String>,
    #[validate(length(min = 1, max = 2048))]
    /// 画廊项目的描述
    pub description: Option<String>,
    pub ordering: i64,
}

pub struct UploadedFile {
    pub file_id: String,
    pub file_name: String,
}

pub async fn undo_uploads(
    file_host: &dyn FileHost,
    uploaded_files: &[UploadedFile],
) -> Result<(), CreateError> {
    for file in uploaded_files {
        file_host
            .delete_file_version(&file.file_id, &file.file_name)
            .await?;
    }
    Ok(())
}

/// 用于收集非涉政但触发风控的渲染图信息（事务提交后创建审核记录）
struct GalleryRiskCheckItem {
    image_url: String,
    raw_image_url: Option<String>,
    risk_image_url: Option<String>,
    uploader_id: i64,
    project_id: i64,
    labels: String,
}

pub async fn project_create(
    req: HttpRequest,
    mut payload: Multipart,
    client: Data<PgPool>,
    redis: Data<RedisPool>,
    file_host: Data<Arc<dyn FileHost + Send + Sync>>,
    private_file_host: Data<Option<Arc<S3PrivateHost>>>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, CreateError> {
    let mut transaction = client.begin().await?;
    let mut uploaded_files = Vec::new();
    let mut gallery_risk_items = Vec::new();

    let result = project_create_inner(
        req,
        &mut payload,
        &mut transaction,
        &***file_host,
        private_file_host.as_ref().as_ref().map(|h| h.as_ref()),
        &mut uploaded_files,
        &client,
        &redis,
        &session_queue,
        &mut gallery_risk_items,
    )
    .await;

    if result.is_err() {
        let undo_result = undo_uploads(&***file_host, &uploaded_files).await;
        let rollback_result = transaction.rollback().await;

        undo_result?;
        if let Err(e) = rollback_result {
            return Err(e.into());
        }
    } else {
        transaction.commit().await?;

        crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
            .await;

        // 事务提交后，为非涉政的风控触发图片创建审核记录
        for item in &gallery_risk_items {
            super::image_reviews::create_review_record(
                &item.image_url,
                item.raw_image_url.as_deref(),
                item.risk_image_url.as_deref(),
                item.uploader_id,
                &item.labels,
                "gallery",
                None,
                Some(item.project_id),
                &client,
                &redis,
            )
            .await;
        }
    }

    result
}
/*

项目创建步骤：
获取已登录用户
    必须匹配版本创建中的作者

1. Data
    - 从 multipart 表单中获取 "data" 字段；必须是第一个
    - 验证：字符串长度
    - 创建版本
        - 与版本创建中的一些共享逻辑
        - 创建 VersionBuilders 列表
    - 创建 ProjectBuilder

2. Upload
    - 图标：检查文件格式 & 大小
        - 上传到 backblaze & 记录 URL
    - 项目文件
        - 检查匹配版本
        - 文件大小限制？
        - 检查文件类型
            - 最终，恶意软件扫描
        - 上传到 backblaze & create VersionFileBuilder


3. 创建
    - 数据库操作
    - 将项目信息添加到索引队列
*/

#[allow(clippy::too_many_arguments)]
async fn project_create_inner(
    req: HttpRequest,
    payload: &mut Multipart,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    file_host: &dyn FileHost,
    private_file_host: Option<&S3PrivateHost>,
    uploaded_files: &mut Vec<UploadedFile>,
    pool: &PgPool,
    redis: &RedisPool,
    session_queue: &AuthQueue,
    gallery_risk_items: &mut Vec<GalleryRiskCheckItem>,
) -> Result<HttpResponse, CreateError> {
    // 上传到 CDN 的文件的 base URL
    let cdn_url = dotenvy::var("CDN_URL")?;

    // 当前登录用户
    let current_user = get_user_from_headers(
        &req,
        pool,
        redis,
        session_queue,
        Some(&[Scopes::PROJECT_CREATE]),
    )
    .await?
    .1;

    // 检查用户是否被资源类封禁
    check_resource_ban(&current_user, pool)
        .await
        .map_err(|e| CreateError::Banned(e.to_string()))?;

    let project_id: ProjectId =
        models::generate_project_id(transaction).await?.into();
    let all_loaders =
        models::loader_fields::Loader::list(&mut **transaction, redis).await?;

    let project_create_data: ProjectCreateData;
    let mut versions;
    let mut versions_map = std::collections::HashMap::new();
    let mut gallery_urls = Vec::new();
    {
        // 第一个 multipart 字段必须命名为 "data" 并包含一个 JSON `ProjectCreateData` 对象。

        let mut field = payload
            .next()
            .await
            .map(|m| m.map_err(CreateError::MultipartError))
            .unwrap_or_else(|| {
                Err(CreateError::MissingValueError(String::from(
                    "分段上传中没有“data”字段",
                )))
            })?;

        let content_disposition =
            field.content_disposition().ok_or_else(|| {
                CreateError::MissingValueError(String::from(
                    "缺少 Content-Disposition",
                ))
            })?;
        let name = content_disposition.get_name().ok_or_else(|| {
            CreateError::MissingValueError(String::from("缺少内容名称"))
        })?;

        if name != "data" {
            return Err(CreateError::InvalidInput(String::from(
                "`data` 字段必须位于文件字段之前",
            )));
        }

        let mut data = Vec::new();
        while let Some(chunk) = field.next().await {
            data.extend_from_slice(
                &chunk.map_err(CreateError::MultipartError)?,
            );
        }
        let create_data: ProjectCreateData = serde_json::from_slice(&data)?;

        create_data.validate().map_err(|err| {
            CreateError::InvalidInput(validation_errors_to_string(err, None))
        })?;

        // 付费资源验证：只有高级创作者才能创建付费资源
        if create_data.is_paid {
            // 获取用户完整信息以检查 is_premium_creator
            let db_user_id = DBUserId(current_user.id.0 as i64);
            let db_user =
                models::User::get_id(db_user_id, &mut **transaction, redis)
                    .await?
                    .ok_or_else(|| {
                        CreateError::InvalidInput("用户不存在".to_string())
                    })?;

            if !db_user.is_premium_creator {
                return Err(CreateError::InvalidInput(
                    "只有高级创作者才能创建付费资源".to_string(),
                ));
            }

            // 验证价格必须设置且在 1-1000 范围内
            match create_data.price {
                Some(price) => {
                    validate_price(price).map_err(|e| {
                        CreateError::InvalidInput(e.to_string())
                    })?;
                }
                None => {
                    return Err(CreateError::InvalidInput(
                        "付费资源必须设置价格".to_string(),
                    ));
                }
            }

            // 验证有效期（如果设置）必须大于 0
            validate_validity_days(create_data.validity_days)
                .map_err(|e| CreateError::InvalidInput(e.to_string()))?;
        }

        // Modrinth 上游提交 79c263301: 添加 .to_lowercase() 确保大小写不敏感
        let slug_project_id_option: Option<ProjectId> = serde_json::from_str(
            &format!("\"{}\"", create_data.slug.to_lowercase()),
        )
        .ok();

        if let Some(slug_project_id) = slug_project_id_option {
            let slug_project_id: models::ids::ProjectId =
                slug_project_id.into();
            let results = sqlx::query!(
                "
                SELECT EXISTS(SELECT 1 FROM mods WHERE id=$1)
                ",
                slug_project_id as models::ids::ProjectId
            )
            .fetch_one(&mut **transaction)
            .await
            .map_err(|e| CreateError::DatabaseError(e.into()))?;

            if results.exists.unwrap_or(false) {
                return Err(CreateError::SlugCollision);
            }
        }

        // Modrinth 上游提交 79c263301: 添加 text_id_lower 检查防止与项目 ID 冲突
        {
            let results = sqlx::query!(
                "
                SELECT EXISTS(
                    SELECT 1 FROM mods
                    WHERE
                        slug = LOWER($1)
                        OR text_id_lower = LOWER($1)
                )
                ",
                create_data.slug
            )
            .fetch_one(&mut **transaction)
            .await
            .map_err(|e| CreateError::DatabaseError(e.into()))?;

            if results.exists.unwrap_or(false) {
                return Err(CreateError::SlugCollision);
            }
        }

        // 为 `initial_versions` 中指定的版本创建 VersionBuilders
        versions = Vec::with_capacity(create_data.initial_versions.len());
        for (i, data) in create_data.initial_versions.iter().enumerate() {
            // 创建一个 multipart 字段名称到版本索引的映射
            for name in &data.file_parts {
                if versions_map.insert(name.to_owned(), i).is_some() {
                    // 如果名称已使用
                    return Err(CreateError::InvalidInput(String::from(
                        "重复的 multipart 字段名称",
                    )));
                }
            }
            versions.push(
                create_initial_version(
                    data,
                    project_id,
                    current_user.id,
                    &all_loaders,
                    transaction,
                    redis,
                )
                .await?,
            );
        }

        project_create_data = create_data;
    }

    let mut icon_data = None;

    let mut error = None;
    while let Some(item) = payload.next().await {
        let mut field: Field = item?;

        if error.is_some() {
            continue;
        }

        let result = async {
            let content_disposition =
                field.content_disposition().cloned().ok_or_else(|| {
                    CreateError::MissingValueError(String::from(
                        "缺少 Content-Disposition",
                    ))
                })?;

            let name = content_disposition.get_name().ok_or_else(|| {
                CreateError::MissingValueError(String::from("缺少内容名称"))
            })?;

            let (file_name, file_extension) =
                super::version_creation::get_name_ext(&content_disposition)?;

            if name == "icon" {
                if icon_data.is_some() {
                    return Err(CreateError::InvalidInput(String::from(
                        "只能设置一个资源",
                    )));
                }
                let username = current_user.username.clone();
                // 将图标上传到 CDN
                icon_data = Some(
                    process_icon_upload(
                        uploaded_files,
                        project_id.0,
                        file_extension,
                        file_host,
                        field,
                        redis,
                        username,
                    )
                    .await?,
                );
                return Ok(());
            }
            if let Some(gallery_items) = &project_create_data.gallery_items {
                if gallery_items.iter().filter(|a| a.featured).count() > 1 {
                    return Err(CreateError::InvalidInput(String::from(
                        "只能设置一个渲染图为主要渲染图.",
                    )));
                }
                if let Some(item) =
                    gallery_items.iter().find(|x| x.item == name)
                {
                    let data = read_from_field(
                        &mut field,
                        2 * (1 << 20),
                        "渲染图文件请不要超过2MB",
                    )
                    .await?;

                    let (_, file_extension) =
                        super::version_creation::get_name_ext(
                            &content_disposition,
                        )?;

                    let url = format!("data/{project_id}/images");
                    // 跳过内置风控，事后异步检查
                    let upload_result = upload_image_optimized(
                        &url,
                        data.freeze(),
                        file_extension,
                        Some(350),
                        Some(1.0),
                        file_host,
                        crate::util::img::UploadImagePos {
                            pos: "项目渲染图".to_string(),
                            url: format!("/project/{}", project_id),
                            username: current_user.username.clone(),
                        },
                        redis,
                        true,
                    )
                    .await
                    .map_err(|e| {
                        CreateError::InvalidIconFormat(e.to_string())
                    })?;

                    uploaded_files.push(UploadedFile {
                        file_id: upload_result.raw_url_path.clone(),
                        file_name: upload_result.raw_url_path,
                    });

                    // 同步风控检查
                    let uploader_id =
                        crate::database::models::UserId::from(
                            current_user.id,
                        )
                        .0;
                    let project_db_id =
                        crate::database::models::ProjectId::from(
                            project_id,
                        )
                        .0;
                    let risk_result =
                        crate::util::risk::check_image_risk_with_labels(
                            &upload_result.url,
                            &format!("/project/{}", project_id),
                            &current_user.username,
                            "项目渲染图",
                            redis,
                        )
                        .await;
                    // 涉政内容：创建审计记录并拒绝整个项目创建
                    // （S3 文件会由 project_create 的 undo_uploads 清理）
                    if let Ok(ref result) = risk_result
                        && !result.passed
                        && crate::util::risk::contains_political_labels(
                            &result.labels,
                        )
                    {
                        log::error!(
                            "[POLITICAL_IMAGE_DELETED] source=gallery, url={}, user={}, labels={}",
                            &upload_result.url,
                            &current_user.username,
                            &result.labels
                        );
                        // 创建审计记录（使用风控缓存 URL）
                        super::image_reviews::create_review_record(
                            &upload_result.url,
                            Some(&upload_result.raw_url),
                            result.frame_url.as_deref(),
                            uploader_id,
                            &result.labels,
                            "gallery",
                            None,
                            Some(project_db_id),
                            pool,
                            redis,
                        )
                        .await;
                        // 将审计记录标记为 auto_deleted
                        let _ = sqlx::query!(
                            "UPDATE image_content_reviews SET status = 'auto_deleted' WHERE image_url = $1 AND status = 'pending'",
                            &upload_result.url,
                        )
                        .execute(pool)
                        .await;
                        return Err(CreateError::InvalidInput(
                            "渲染图内容违规，已被拦截".to_string(),
                        ));
                    }
                    // 非涉政风控未通过：收集待审核信息
                    if let Ok(ref result) = risk_result
                        && !result.passed
                    {
                        gallery_risk_items.push(GalleryRiskCheckItem {
                            image_url: upload_result.url.clone(),
                            raw_image_url: Some(
                                upload_result.raw_url.clone(),
                            ),
                            risk_image_url: result.frame_url.clone(),
                            uploader_id,
                            project_id: project_db_id,
                            labels: result.labels.clone(),
                        });
                    }
                    gallery_urls.push(crate::models::projects::GalleryItem {
                        url: upload_result.url,
                        raw_url: upload_result.raw_url,
                        featured: item.featured,
                        name: item.name.clone(),
                        description: item.description.clone(),
                        created: Utc::now(),
                        ordering: item.ordering,
                    });

                    return Ok(());
                }
            }
            let index = if let Some(i) = versions_map.get(name) {
                *i
            } else {
                return Err(CreateError::InvalidInput(format!(
                    "文件 `{file_name}` (字段 {name}) 未在版本数据中指定"
                )));
            };
            // `index` 对于这些列表总是有效的
            let created_version = versions.get_mut(index).unwrap();
            let version_data =
                project_create_data.initial_versions.get(index).unwrap();
            // TODO: 可能冗余，这个计算是否在其他地方完成？

            let existing_file_names = created_version
                .files
                .iter()
                .map(|x| x.filename.clone())
                .collect();
            // 上传新的 jar 文件
            super::version_creation::upload_file(
                &mut field,
                file_host,
                private_file_host,
                version_data.file_parts.len(),
                uploaded_files,
                &mut created_version.files,
                &mut created_version.dependencies,
                &cdn_url,
                &content_disposition,
                project_id,
                created_version.version_id.into(),
                &created_version.version_fields,
                version_data.loaders.clone(),
                version_data.primary_file.is_some(),
                version_data.primary_file.as_deref() == Some(name),
                None,
                existing_file_names,
                transaction,
                redis,
                current_user.username.clone(),
                project_create_data.is_paid,
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

    {
        // 检查是否所有指定的文件都已上传
        for (version_data, builder) in project_create_data
            .initial_versions
            .iter()
            .zip(versions.iter())
        {
            if version_data.file_parts.len() != builder.files.len() {
                return Err(CreateError::InvalidInput(String::from(
                    "文件上传异常，请重新刷新页面选择文件进行上传",
                )));
            }
        }

        // 将类别名称列表转换为实际类别
        let mut categories =
            Vec::with_capacity(project_create_data.categories.len());
        for category in &project_create_data.categories {
            let ids = models::categories::Category::get_ids(
                category,
                &mut **transaction,
            )
            .await?;
            if ids.is_empty() {
                return Err(CreateError::InvalidCategory(category.clone()));
            }

            // TODO: 我们应该过滤掉与任何版本的项目类型不匹配的类别
            // ie: 如果 mod 和 modpack 共享一个名称，则只有 modpack 应该存在，如果它只有 modpack 作为版本
            categories.extend(ids.values());
        }

        let mut additional_categories =
            Vec::with_capacity(project_create_data.additional_categories.len());
        for category in &project_create_data.additional_categories {
            let ids = models::categories::Category::get_ids(
                category,
                &mut **transaction,
            )
            .await?;
            if ids.is_empty() {
                return Err(CreateError::InvalidCategory(category.clone()));
            }
            // TODO: 我们应该过滤掉与任何版本的项目类型不匹配的类别
            // ie: 如果 mod 和 modpack 共享一个名称，则只有 modpack 应该存在，如果它只有 modpack 作为版本
            additional_categories.extend(ids.values());
        }

        let mut members = vec![];

        if let Some(organization_id) = project_create_data.organization_id {
            let org = models::Organization::get_id(
                organization_id.into(),
                pool,
                redis,
            )
            .await?
            .ok_or_else(|| {
                CreateError::InvalidInput("团队ID无效".to_string())
            })?;

            let team_member = models::TeamMember::get_from_user_id(
                org.team_id,
                current_user.id.into(),
                pool,
            )
            .await?;

            let perms = OrganizationPermissions::get_permissions_by_role(
                &current_user.role,
                &team_member,
            );

            if !perms
                .map(|x| x.contains(OrganizationPermissions::ADD_PROJECT))
                .unwrap_or(false)
            {
                return Err(CreateError::CustomAuthenticationError(
                    "您没有权限在此团队中创建项目！".to_string(),
                ));
            }
        } else {
            members.push(models::team_item::TeamMemberBuilder {
                user_id: current_user.id.into(),
                role: crate::models::teams::DEFAULT_ROLE.to_owned(),
                is_owner: true,
                permissions: ProjectPermissions::all(),
                organization_permissions: None,
                accepted: true,
                payouts_split: Decimal::ONE_HUNDRED,
                ordering: 0,
            })
        }
        let team = models::team_item::TeamBuilder { members };

        let team_id = team.insert(&mut *transaction).await?;

        let status;
        if project_create_data.is_draft.unwrap_or(false) {
            status = ProjectStatus::Draft;
        } else {
            status = ProjectStatus::Processing;
            if project_create_data.initial_versions.is_empty() {
                return Err(CreateError::InvalidInput(String::from(
                    "请先上传一个版本文件",
                )));
            }
        }

        let license_id = spdx::Expression::parse(
            &project_create_data.license_id,
        )
        .map_err(|err| {
            CreateError::InvalidInput(format!(
                "填写的URL内SPDX 许可证标识符无效 {err}"
            ))
        })?;

        let mut link_urls = vec![];

        let link_platforms =
            models::categories::LinkPlatform::list(&mut **transaction, redis)
                .await?;
        for (platform, url) in &project_create_data.link_urls {
            let platform_id = models::categories::LinkPlatform::get_id(
                platform,
                &mut **transaction,
            )
            .await?
            .ok_or_else(|| {
                CreateError::InvalidInput(format!(
                    "链接平台{}不存在.",
                    platform.clone()
                ))
            })?;
            let link_platform = link_platforms
                .iter()
                .find(|x| x.id == platform_id)
                .ok_or_else(|| {
                    CreateError::InvalidInput(format!(
                        "链接平台{}不存在.",
                        platform.clone()
                    ))
                })?;
            link_urls.push(models::project_item::LinkUrl {
                platform_id,
                platform_name: link_platform.name.clone(),
                url: url.clone(),
                donation: link_platform.donation,
            })
        }

        let project_builder_actual = models::project_item::ProjectBuilder {
            project_id: project_id.into(),
            team_id,
            organization_id: project_create_data
                .organization_id
                .map(|x| x.into()),
            name: project_create_data.name,
            summary: project_create_data.summary,
            description: project_create_data.description,
            icon_url: icon_data.clone().map(|x| x.0),
            raw_icon_url: icon_data.clone().map(|x| x.1),

            license_url: project_create_data.license_url,
            categories,
            additional_categories,
            initial_versions: versions,
            status,
            requested_status: Some(project_create_data.requested_status),
            license: license_id.to_string(),
            slug: Some(project_create_data.slug),
            link_urls,
            gallery_items: gallery_urls
                .iter()
                .map(|x| models::project_item::GalleryItem {
                    image_url: x.url.clone(),
                    raw_image_url: x.raw_url.clone(),
                    featured: x.featured,
                    name: x.name.clone(),
                    description: x.description.clone(),
                    created: x.created,
                    ordering: x.ordering,
                })
                .collect(),
            color: icon_data.and_then(|x| x.2),
            monetization_status: MonetizationStatus::Monetized,
            is_paid: project_create_data.is_paid,
        };
        let project_builder = project_builder_actual.clone();

        let now = Utc::now();

        let id = project_builder_actual.insert(&mut *transaction).await?;
        User::clear_project_cache(&[current_user.id.into()], redis).await?;

        // 如果是付费资源，插入定价信息
        if project_create_data.is_paid
            && let Some(price) = project_create_data.price
        {
            // 将 i32 转换为 Decimal
            let price_decimal = Decimal::from(price);
            models::ProjectPricing::upsert(
                id,
                price_decimal,
                project_create_data.validity_days,
                &mut *transaction,
            )
            .await?;
        }

        for image_id in project_create_data.uploaded_images {
            if let Some(db_image) = image_item::Image::get(
                image_id.into(),
                &mut **transaction,
                redis,
            )
            .await?
            {
                let image: Image = db_image.into();
                if !matches!(image.context, ImageContext::Project { .. })
                    || image.context.inner_id().is_some()
                {
                    return Err(CreateError::InvalidInput(format!(
                        "图片 {} 没有在资源正文中使用过",
                        image_id
                    )));
                }

                sqlx::query!(
                    "
                    UPDATE uploaded_images
                    SET mod_id = $1
                    WHERE id = $2
                    ",
                    id as models::ids::ProjectId,
                    image_id.0 as i64
                )
                .execute(&mut **transaction)
                .await?;

                image_item::Image::clear_cache(image.id.into(), redis).await?;
            } else {
                return Err(CreateError::InvalidInput(format!(
                    "图片 {} 不存在",
                    image_id
                )));
            }
        }

        let thread_id = ThreadBuilder {
            type_: ThreadType::Project,
            members: vec![],
            project_id: Some(id),
            report_id: None,
            ban_appeal_id: None,
            creator_application_id: None,
        }
        .insert(&mut *transaction)
        .await?;

        let loaders = project_builder
            .initial_versions
            .iter()
            .flat_map(|v| v.loaders.clone())
            .unique()
            .collect::<Vec<_>>();
        let (project_types, games) = Loader::list(&mut **transaction, redis)
            .await?
            .into_iter()
            .fold(
                (Vec::new(), Vec::new()),
                |(mut project_types, mut games), loader| {
                    if loaders.contains(&loader.id) {
                        project_types.extend(loader.supported_project_types);
                        games.extend(loader.supported_games);
                    }
                    (project_types, games)
                },
            );

        let response = crate::models::projects::Project {
            id: project_id,
            slug: project_builder.slug.clone(),
            project_types,
            games,
            team_id: team_id.into(),
            organization: project_create_data.organization_id,
            name: project_builder.name.clone(),
            summary: project_builder.summary.clone(),
            description: project_builder.description.clone(),
            published: now,
            updated: now,
            approved: None,
            queued: None,
            status,
            requested_status: project_builder.requested_status,
            moderator_message: None,
            license: License {
                id: project_create_data.license_id.clone(),
                name: "".to_string(),
                url: project_builder.license_url.clone(),
            },
            downloads: 0,
            followers: 0,
            categories: project_create_data.categories,
            additional_categories: project_create_data.additional_categories,
            loaders: vec![],
            versions: project_builder
                .initial_versions
                .iter()
                .map(|v| v.version_id.into())
                .collect::<Vec<_>>(),
            icon_url: project_builder.icon_url.clone(),
            link_urls: project_builder
                .link_urls
                .clone()
                .into_iter()
                .map(|x| (x.platform_name.clone(), Link::from(x)))
                .collect(),
            gallery: gallery_urls,
            color: project_builder.color,
            thread_id: thread_id.into(),
            monetization_status: MonetizationStatus::Monetized,
            fields: HashMap::new(), // Fields instantiate to empty
            wiki_open: false,
            issues_type: 0,
            forum: None,
            translation_tracking: false,
            translation_tracker: None,
            translation_source: None,
            is_paid: project_create_data.is_paid,
            user_has_purchased: None, // 新创建的项目不返回购买状态
            price: project_create_data.price.map(rust_decimal::Decimal::from),
            validity_days: project_create_data.validity_days,
        };

        Ok(HttpResponse::Ok().json(response))
    }
}

async fn create_initial_version(
    version_data: &InitialVersionData,
    project_id: ProjectId,
    author: UserId,
    all_loaders: &[models::loader_fields::Loader],
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
) -> Result<models::version_item::VersionBuilder, CreateError> {
    if version_data.project_id.is_some() {
        return Err(CreateError::InvalidInput(String::from("该版本已经存在")));
    }

    version_data.validate().map_err(|err| {
        CreateError::ValidationError(validation_errors_to_string(err, None))
    })?;

    // 随机生成一个新 ID 用于版本
    let version_id: VersionId =
        models::generate_version_id(transaction).await?.into();

    let loaders = version_data
        .loaders
        .iter()
        .map(|x| {
            all_loaders
                .iter()
                .find(|y| y.loader == x.0)
                .ok_or_else(|| CreateError::InvalidLoader(x.0.clone()))
                .map(|y| y.id)
        })
        .collect::<Result<Vec<models::LoaderId>, CreateError>>()?;

    let loader_fields =
        LoaderField::get_fields(&loaders, &mut **transaction, redis).await?;
    let mut loader_field_enum_values =
        LoaderFieldEnumValue::list_many_loader_fields(
            &loader_fields,
            &mut **transaction,
            redis,
        )
        .await?;

    let version_fields = try_create_version_fields(
        version_id,
        &version_data.fields,
        &loader_fields,
        &mut loader_field_enum_values,
    )?;

    let dependencies = version_data
        .dependencies
        .iter()
        .map(|d| models::version_item::DependencyBuilder {
            version_id: d.version_id.map(|x| x.into()),
            project_id: d.project_id.map(|x| x.into()),
            dependency_type: d.dependency_type.to_string(),
            file_name: None,
        })
        .collect::<Vec<_>>();

    let version = models::version_item::VersionBuilder {
        version_id: version_id.into(),
        project_id: project_id.into(),
        author_id: author.into(),
        name: version_data.version_title.clone(),
        version_number: version_data.version_number.clone(),
        changelog: version_data.version_body.clone().unwrap_or_default(),
        files: Vec::new(),
        dependencies,
        version_links: Vec::new(),
        loaders,
        version_fields,
        featured: version_data.featured,
        status: VersionStatus::Listed,
        version_type: version_data.release_channel.to_string(),
        requested_status: None,
        ordering: version_data.ordering,
        disk_url: version_data.disk_urls.clone(),
    };

    Ok(version)
}

async fn process_icon_upload(
    uploaded_files: &mut Vec<UploadedFile>,
    id: u64,
    file_extension: &str,
    file_host: &dyn FileHost,
    mut field: Field,
    redis: &RedisPool,
    username: String,
) -> Result<(String, String, Option<u32>), CreateError> {
    let mut cap = 262144;

    if username.to_lowercase() == "bbsmc" {
        cap = 2621440
    }

    let data = read_from_field(&mut field, cap, "图标必须小于 256KB").await?;
    let upload_result = crate::util::img::upload_image_optimized(
        &format!("data/{}", to_base62(id)),
        data.freeze(),
        file_extension,
        Some(96),
        Some(1.0),
        file_host,
        crate::util::img::UploadImagePos {
            pos: "项目图标".to_string(),
            url: format!("/project/{}", id),
            username,
        },
        redis,
        false,
    )
    .await
    .map_err(|e| CreateError::InvalidIconFormat(e.to_string()))?;

    uploaded_files.push(UploadedFile {
        file_id: upload_result.raw_url_path.clone(),
        file_name: upload_result.raw_url_path,
    });

    uploaded_files.push(UploadedFile {
        file_id: upload_result.url_path.clone(),
        file_name: upload_result.url_path,
    });

    Ok((
        upload_result.url,
        upload_result.raw_url,
        upload_result.color,
    ))
}
