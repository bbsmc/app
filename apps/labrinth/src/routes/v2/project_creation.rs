use crate::database::models::version_item;
use crate::database::redis::RedisPool;
use crate::file_hosting::{FileHost, S3PrivateHost};
use crate::models;
use crate::models::ids::ImageId;
use crate::models::projects::{Loader, Project, ProjectStatus};
use crate::models::v2::projects::{
    DonationLink, LegacyProject, LegacySideType,
};
use crate::queue::session::AuthQueue;
use crate::routes::v3::project_creation::default_project_type;
use crate::routes::v3::project_creation::{CreateError, NewGalleryItem};
use crate::routes::{v2_reroute, v3};
use actix_multipart::Multipart;
use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse, post};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPool;

use std::collections::HashMap;
use std::sync::Arc;
use validator::Validate;

use super::version_creation::InitialVersionData;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(project_create);
}

pub fn default_requested_status() -> ProjectStatus {
    ProjectStatus::Approved
}

#[derive(Serialize, Deserialize, Validate, Clone)]
struct ProjectCreateData {
    #[validate(
        length(min = 3, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    #[serde(alias = "mod_name")]
    /// 项目的标题或名称。
    pub title: String,
    #[validate(length(min = 1, max = 64))]
    #[serde(default = "default_project_type")]
    /// 这个模组的类型
    pub project_type: String,
    #[validate(
        length(min = 3, max = 64),
        regex(path = *crate::util::validate::RE_URL_SAFE)
    )]
    #[serde(alias = "mod_slug")]
    /// 项目的 slug，用于 vanity URLs
    pub slug: String,
    #[validate(length(min = 3, max = 255))]
    #[serde(alias = "mod_description")]
    /// 一个简短的描述。
    pub description: String,
    #[validate(length(max = 65536))]
    #[serde(alias = "mod_body")]
    /// 一个长描述，以 markdown 格式。
    pub body: String,

    /// 客户端支持范围
    pub client_side: LegacySideType,
    /// 服务器支持范围
    pub server_side: LegacySideType,

    #[validate(length(max = 32))]
    #[validate(nested)]
    /// 要上传的初始版本列表
    pub initial_versions: Vec<InitialVersionData>,
    #[validate(length(max = 3))]
    /// 项目所属的类别列表
    pub categories: Vec<String>,
    #[validate(length(max = 256))]
    #[serde(default = "Vec::new")]
    /// 项目所属的附加类别列表
    pub additional_categories: Vec<String>,

    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    /// 一个可选的链接，用于提交模组的问题或错误。
    pub issues_url: Option<String>,
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    /// 一个可选的链接，用于提交模组源代码。
    pub source_url: Option<String>,
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    /// 一个可选的链接，用于提交模组 wiki 页面或其他相关信息。
    pub wiki_url: Option<String>,
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    /// 一个可选的链接，用于提交模组许可证页面。
    pub license_url: Option<String>,
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    /// 一个可选的链接，用于提交模组 discord。
    pub discord_url: Option<String>,
    /// 一个可选的列表，用于提交模组的所有捐赠链接。
    #[validate(nested)]
    pub donation_urls: Option<Vec<DonationLink>>,

    /// 一个可选的布尔值。如果为 true，则项目将被创建为草稿。
    pub is_draft: Option<bool>,

    /// 项目遵循的许可证 id
    pub license_id: String,

    #[validate(length(max = 64))]
    #[validate(nested)]
    /// 要上传的画廊项目名称列表
    pub gallery_items: Option<Vec<NewGalleryItem>>,
    #[serde(default = "default_requested_status")]
    /// 项目一旦被批准，将被设置的状态
    pub requested_status: ProjectStatus,

    // Associations to uploaded images in body/description
    #[validate(length(max = 10))]
    #[serde(default)]
    pub uploaded_images: Vec<ImageId>,

    /// 要创建项目的组织 id
    pub organization_id: Option<models::ids::OrganizationId>,
}

#[post("project")]
pub async fn project_create(
    req: HttpRequest,
    payload: Multipart,
    client: Data<PgPool>,
    redis: Data<RedisPool>,
    file_host: Data<Arc<dyn FileHost + Send + Sync>>,
    private_file_host: Data<Option<Arc<S3PrivateHost>>>,
    session_queue: Data<AuthQueue>,
) -> Result<HttpResponse, CreateError> {
    // 将 V2 的 multipart 负载转换为 V3 的 multipart 负载
    let payload = v2_reroute::alter_actix_multipart(
        payload,
        req.headers().clone(),
        |legacy_create: ProjectCreateData, _| async move {
            // 每个版本将应用 side 类型
            let client_side = legacy_create.client_side;
            let server_side = legacy_create.server_side;

            let project_type = legacy_create.project_type;

            let initial_versions = legacy_create
                .initial_versions
                .into_iter()
                .map(|v| {
                    let mut fields = HashMap::new();
                    fields.extend(v2_reroute::convert_side_types_v3(
                        client_side,
                        server_side,
                    ));
                    fields.insert(
                        "game_versions".to_string(),
                        json!(v.game_versions),
                    );

                    // 现在 modpacks 使用 "mrpack" loader，并且 loaders 被转换为 loader 字段。
                    // 直接设置 'project_type' 被移除，现在它是基于 loader 的。
                    if project_type == "modpack" {
                        fields.insert(
                            "mrpack_loaders".to_string(),
                            json!(v.loaders),
                        );
                    }

                    let loaders = if project_type == "modpack" {
                        vec![Loader("mrpack".to_string())]
                    } else {
                        v.loaders
                    };

                    v3::version_creation::InitialVersionData {
                        project_id: v.project_id,
                        file_parts: v.file_parts,
                        version_number: v.version_number,
                        version_title: v.version_title,
                        version_body: v.version_body,
                        dependencies: v.dependencies,
                        version_links: None,
                        release_channel: v.release_channel,
                        loaders,
                        featured: v.featured,
                        primary_file: v.primary_file,
                        status: v.status,
                        file_types: v.file_types,
                        uploaded_images: v.uploaded_images,
                        ordering: v.ordering,
                        fields,
                        disk_only: v.disk_only,
                        disk_urls: v.disk_urls,
                    }
                })
                .collect();

            let mut link_urls = HashMap::new();
            if let Some(issue_url) = legacy_create.issues_url {
                link_urls.insert("issues".to_string(), issue_url);
            }
            if let Some(source_url) = legacy_create.source_url {
                link_urls.insert("source".to_string(), source_url);
            }
            if let Some(wiki_url) = legacy_create.wiki_url {
                link_urls.insert("wiki".to_string(), wiki_url);
            }
            if let Some(discord_url) = legacy_create.discord_url {
                link_urls.insert("discord".to_string(), discord_url);
            }
            if let Some(donation_urls) = legacy_create.donation_urls {
                for donation_url in donation_urls {
                    link_urls.insert(donation_url.platform, donation_url.url);
                }
            }

            Ok(v3::project_creation::ProjectCreateData {
                name: legacy_create.title,
                slug: legacy_create.slug,
                summary: legacy_create.description, // 描述变为摘要
                description: legacy_create.body,    // 正文变为描述
                initial_versions,
                categories: legacy_create.categories,
                additional_categories: legacy_create.additional_categories,
                license_url: legacy_create.license_url,
                link_urls,
                is_draft: legacy_create.is_draft,
                license_id: legacy_create.license_id,
                gallery_items: legacy_create.gallery_items,
                requested_status: legacy_create.requested_status,
                uploaded_images: legacy_create.uploaded_images,
                organization_id: legacy_create.organization_id,
                // V2 API 不支持付费资源功能
                is_paid: false,
                price: None,
                validity_days: None,
            })
        },
    )
    .await?;

    // 调用 V3 项目创建
    let response = v3::project_creation::project_create(
        req,
        payload,
        client.clone(),
        redis.clone(),
        file_host,
        private_file_host,
        session_queue,
    )
    .await?;

    // 将响应转换为 V2 格式
    match v2_reroute::extract_ok_json::<Project>(response).await {
        Ok(project) => {
            let version_item = match project.versions.first() {
                Some(vid) => {
                    version_item::Version::get((*vid).into(), &**client, &redis)
                        .await?
                }
                None => None,
            };
            let project = LegacyProject::from(project, version_item);
            Ok(HttpResponse::Ok().json(project))
        }
        Err(response) => Ok(response),
    }
}
