use crate::database::models::categories::LinkPlatform;
use crate::database::models::{project_item, version_item};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::projects::{
    Link, MonetizationStatus, Project, ProjectStatus, SearchRequest, Version,
};
use crate::models::v2::projects::{
    DonationLink, LegacyProject, LegacySideType, LegacyVersion,
};
use crate::models::v2::search::LegacySearchResults;
use crate::queue::moderation::AutomatedModerationQueue;
use crate::queue::session::AuthQueue;
use crate::routes::v3::projects::ProjectIds;
use crate::routes::{ApiError, v2_reroute, v3};
use crate::search::{SearchConfig, SearchError, search_for_project};
use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use validator::Validate;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(project_search);
    cfg.service(projects_get);
    cfg.service(projects_edit);
    cfg.service(random_projects_get);

    cfg.service(
        web::scope("project")
            .service(project_get)
            .service(project_get_check)
            .service(project_delete)
            .service(project_edit)
            .service(project_icon_edit)
            .service(delete_project_icon)
            .service(add_gallery_item)
            .service(edit_gallery_item)
            .service(delete_gallery_item)
            .service(project_follow)
            .service(project_unfollow)
            .service(super::teams::team_members_get_project)
            .service(
                web::scope("{id}")
                    .service(super::versions::version_list)
                    .service(super::versions::version_project_get)
                    .service(dependency_list)
                    .service(get_translation_links),
            ),
    );
}

#[get("search")]
pub async fn project_search(
    web::Query(info): web::Query<SearchRequest>,
    config: web::Data<SearchConfig>,
) -> Result<HttpResponse, SearchError> {
    // 搜索现在使用 loader_fields 而不是显式的 'client_side' 和 'server_side' 字段
    // 尽管后端发生了变化，但这对 API 调用影响不大
    // 除了 'versions:x' 现在变成了 'game_versions:x'
    let facets: Option<Vec<Vec<String>>> = if let Some(facets) = info.facets {
        let facets = serde_json::from_str::<Vec<Vec<String>>>(&facets)?;

        // 这些加载器以前是与 'mod' 结合在一起，作为插件，但现在
        // 它们是自己的加载器类型。我们将 'mod' 转换为 'mod' OR 'plugin'
        // 因为它基本上与以前相同。
        let facets = v2_reroute::convert_plugin_loader_facets_v3(facets);

        Some(
            facets
                .into_iter()
                .map(|facet| {
                    facet
                        .into_iter()
                        .map(|facet| {
                            if let Some((key, operator, val)) =
                                parse_facet(&facet)
                            {
                                format!(
                                    "{}{}{}",
                                    match key.as_str() {
                                        "versions" => "game_versions",
                                        "project_type" => "project_types",
                                        "title" => "name",
                                        x => x,
                                    },
                                    operator,
                                    val
                                )
                            } else {
                                facet.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        )
    } else {
        None
    };

    let info = SearchRequest {
        facets: facets.and_then(|x| serde_json::to_string(&x).ok()),
        ..info
    };

    let results = search_for_project(&info, &config).await?;

    let results = LegacySearchResults::from(results);

    Ok(HttpResponse::Ok().json(results))
}

/// 解析一个 facet 为 key, operator, 和 value
fn parse_facet(facet: &str) -> Option<(String, String, String)> {
    let mut key = String::new();
    let mut operator = String::new();
    let mut val = String::new();

    let mut iterator = facet.chars();
    while let Some(char) = iterator.next() {
        match char {
            ':' | '=' => {
                operator.push(char);
                val = iterator.collect::<String>();
                return Some((key, operator, val));
            }
            '<' | '>' => {
                operator.push(char);
                if let Some(next_char) = iterator.next() {
                    if next_char == '=' {
                        operator.push(next_char);
                    } else {
                        val.push(next_char);
                    }
                }
                val.push_str(&iterator.collect::<String>());
                return Some((key, operator, val));
            }
            ' ' => continue,
            _ => key.push(char),
        }
    }

    None
}

#[derive(Deserialize, Validate)]
pub struct RandomProjects {
    #[validate(range(min = 1, max = 100))]
    pub count: u32,
}

#[get("projects_random")]
pub async fn random_projects_get(
    web::Query(count): web::Query<RandomProjects>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let count = v3::projects::RandomProjects { count: count.count };

    let response = v3::projects::random_projects_get(
        web::Query(count),
        pool.clone(),
        redis.clone(),
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
    .or_else(v2_reroute::flatten_404_error)?;
    // 将响应转换为 V2 格式
    match v2_reroute::extract_ok_json::<Vec<Project>>(response).await {
        Ok(project) => {
            let legacy_projects =
                LegacyProject::from_many(project, &**pool, &redis).await?;
            Ok(HttpResponse::Ok().json(legacy_projects))
        }
        Err(response) => Ok(response),
    }
}

#[get("projects")]
pub async fn projects_get(
    req: HttpRequest,
    web::Query(ids): web::Query<ProjectIds>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 调用 V3 项目创建
    let response = v3::projects::projects_get(
        req,
        web::Query(ids),
        pool.clone(),
        redis.clone(),
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
    .or_else(v2_reroute::flatten_404_error)?;

    // 将响应转换为 V2 格式
    match v2_reroute::extract_ok_json::<Vec<Project>>(response).await {
        Ok(project) => {
            let legacy_projects =
                LegacyProject::from_many(project, &**pool, &redis).await?;
            Ok(HttpResponse::Ok().json(legacy_projects))
        }
        Err(response) => Ok(response),
    }
}

#[get("{id}")]
pub async fn project_get(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 将 V2 数据转换为 V3 数据
    // 调用 V3 项目创建
    let response = v3::projects::project_get(
        req,
        info,
        pool.clone(),
        redis.clone(),
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)?;

    // 将响应转换为 V2 格式
    match v2_reroute::extract_ok_json::<Project>(response).await {
        Ok(project) => {
            let version_item = match project.versions.first() {
                Some(vid) => {
                    version_item::Version::get((*vid).into(), &**pool, &redis)
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

// 检查项目 ID 或 slug 的有效性
#[get("{id}/check")]
pub async fn project_get_check(
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    // 返回一个 ID，不需要转换
    v3::projects::project_get_check(info, pool, redis)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

#[derive(Serialize)]
struct DependencyInfo {
    pub projects: Vec<LegacyProject>,
    pub versions: Vec<LegacyVersion>,
}

#[get("dependencies")]
pub async fn dependency_list(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // TODO: 测试，可能
    let response = v3::projects::dependency_list(
        req,
        info,
        pool.clone(),
        redis.clone(),
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)?;

    match v2_reroute::extract_ok_json::<
        crate::routes::v3::projects::DependencyInfo,
    >(response)
    .await
    {
        Ok(dependency_info) => {
            let converted_projects = LegacyProject::from_many(
                dependency_info.projects,
                &**pool,
                &redis,
            )
            .await?;
            let converted_versions = dependency_info
                .versions
                .into_iter()
                .map(LegacyVersion::from)
                .collect();

            Ok(HttpResponse::Ok().json(DependencyInfo {
                projects: converted_projects,
                versions: converted_versions,
            }))
        }
        Err(response) => Ok(response),
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct EditProject {
    #[validate(
        length(min = 3, max = 64),
        custom(function = "crate::util::validate::validate_name")
    )]
    pub title: Option<String>,
    #[validate(length(min = 3, max = 256))]
    pub description: Option<String>,
    #[validate(length(max = 65536))]
    pub body: Option<String>,
    #[validate(length(max = 3))]
    pub categories: Option<Vec<String>>,
    #[validate(length(max = 256))]
    pub additional_categories: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub issues_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub source_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub wiki_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub license_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub discord_url: Option<Option<String>>,
    #[validate(nested)]
    pub donation_urls: Option<Vec<DonationLink>>,
    pub license_id: Option<String>,
    pub client_side: Option<LegacySideType>,
    pub server_side: Option<LegacySideType>,
    #[validate(
        length(min = 3, max = 64),
        regex(path = *crate::util::validate::RE_URL_SAFE)
    )]
    pub slug: Option<String>,
    pub status: Option<ProjectStatus>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    pub requested_status: Option<Option<ProjectStatus>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(length(max = 2000))]
    pub moderation_message: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(length(max = 65536))]
    pub moderation_message_body: Option<Option<String>>,
    pub monetization_status: Option<MonetizationStatus>,
    pub wiki_open: Option<bool>,
    #[validate(range(min = 0, max = 3))]
    pub issues_type: Option<i32>,
    /// 汉化追踪标记，仅管理员可设置
    pub translation_tracking: Option<bool>,
}

#[patch("{id}")]
#[allow(clippy::too_many_arguments)]
pub async fn project_edit(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    search_config: web::Data<SearchConfig>,
    new_project: web::Json<EditProject>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    moderation_queue: web::Data<AutomatedModerationQueue>,
) -> Result<HttpResponse, ApiError> {
    let v2_new_project = new_project.into_inner();
    let client_side = v2_new_project.client_side;
    let server_side = v2_new_project.server_side;
    let new_slug = v2_new_project.slug.clone();

    // 检查项目类型是否正确
    // 我们期望上传的版本是 modpack 类型，但可能无法在此处检查。
    // 毕竟，理论上，他们可以创建一个真正的 'fabric' mod，而 modpack 不再携带是否是 mod 或 modpack 的信息，
    // 因为那些信息在版本中。

    // 理想情况下，如果项目 '应该' 是 modpack：
    // - 将 loaders 更改为 mrpack 仅
    // - 为项目添加类别以对应 loaders

    let mut new_links = HashMap::new();
    if let Some(issues_url) = v2_new_project.issues_url {
        if let Some(issues_url) = issues_url {
            new_links.insert("issues".to_string(), Some(issues_url));
        } else {
            new_links.insert("issues".to_string(), None);
        }
    }

    if let Some(source_url) = v2_new_project.source_url {
        if let Some(source_url) = source_url {
            new_links.insert("source".to_string(), Some(source_url));
        } else {
            new_links.insert("source".to_string(), None);
        }
    }

    if let Some(wiki_url) = v2_new_project.wiki_url {
        if let Some(wiki_url) = wiki_url {
            new_links.insert("wiki".to_string(), Some(wiki_url));
        } else {
            new_links.insert("wiki".to_string(), None);
        }
    }

    if let Some(discord_url) = v2_new_project.discord_url {
        if let Some(discord_url) = discord_url {
            new_links.insert("discord".to_string(), Some(discord_url));
        } else {
            new_links.insert("discord".to_string(), None);
        }
    }

    // 在 v2 中，设置捐赠链接会重置所有其他捐赠链接
    // （重置为新链接）
    if let Some(donation_urls) = v2_new_project.donation_urls {
        // 从项目中获取当前捐赠链接，以便我们知道要删除什么
        let fetched_example_project =
            project_item::Project::get(&info.0, &**pool, &redis).await?;
        let donation_links = fetched_example_project
            .map(|x| {
                x.urls
                    .into_iter()
                    .filter_map(|l| {
                        if l.donation {
                            Some(Link::from(l)) // TODO: tests
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // 将现有捐赠链接设置为 None
        for old_link in donation_links {
            new_links.insert(old_link.platform, None);
        }

        // 添加新捐赠链接
        for donation_url in donation_urls {
            new_links.insert(donation_url.id, Some(donation_url.url));
        }
    }

    let new_project = v3::projects::EditProject {
        name: v2_new_project.title,
        summary: v2_new_project.description, // 描述变为摘要
        description: v2_new_project.body,    // 正文变为描述
        categories: v2_new_project.categories,
        additional_categories: v2_new_project.additional_categories,
        license_url: v2_new_project.license_url,
        link_urls: Some(new_links),
        license_id: v2_new_project.license_id,
        slug: v2_new_project.slug,
        status: v2_new_project.status,
        requested_status: v2_new_project.requested_status,
        moderation_message: v2_new_project.moderation_message,
        moderation_message_body: v2_new_project.moderation_message_body,
        monetization_status: v2_new_project.monetization_status,
        wiki_open: v2_new_project.wiki_open,
        issues_type: v2_new_project.issues_type,
        translation_tracking: v2_new_project.translation_tracking,
    };

    // 这返回 204 或失败，所以我们不需要对其做任何处理
    let project_id = info.clone().0;
    let mut response = v3::projects::project_edit(
        req.clone(),
        info,
        pool.clone(),
        search_config,
        web::Json(new_project),
        redis.clone(),
        session_queue.clone(),
        moderation_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)?;

    // 如果客户端和服务器端被设置，我们将为每个版本调用版本设置路由，以设置每个版本的服务器端类型。
    if response.status().is_success()
        && (client_side.is_some() || server_side.is_some())
    {
        let project_item = project_item::Project::get(
            &new_slug.unwrap_or(project_id),
            &**pool,
            &redis,
        )
        .await?;
        let version_ids = project_item.map(|x| x.versions).unwrap_or_default();
        let versions =
            version_item::Version::get_many(&version_ids, &**pool, &redis)
                .await?;
        for version in versions {
            let version = Version::from(version);
            let mut fields = version.fields;
            let (current_client_side, current_server_side) =
                v2_reroute::convert_side_types_v2(&fields, None);
            let client_side = client_side.unwrap_or(current_client_side);
            let server_side = server_side.unwrap_or(current_server_side);
            fields.extend(v2_reroute::convert_side_types_v3(
                client_side,
                server_side,
            ));

            response = v3::versions::version_edit_helper(
                req.clone(),
                (version.id,),
                pool.clone(),
                redis.clone(),
                v3::versions::EditVersion {
                    fields,
                    ..Default::default()
                },
                session_queue.clone(),
            )
            .await?;
        }
    }
    Ok(response)
}

#[derive(Deserialize, Validate)]
pub struct BulkEditProject {
    #[validate(length(max = 3))]
    pub categories: Option<Vec<String>>,
    #[validate(length(max = 3))]
    pub add_categories: Option<Vec<String>>,
    pub remove_categories: Option<Vec<String>>,

    #[validate(length(max = 256))]
    pub additional_categories: Option<Vec<String>>,
    #[validate(length(max = 3))]
    pub add_additional_categories: Option<Vec<String>>,
    pub remove_additional_categories: Option<Vec<String>>,

    #[validate(nested)]
    pub donation_urls: Option<Vec<DonationLink>>,
    #[validate(nested)]
    pub add_donation_urls: Option<Vec<DonationLink>>,
    #[validate(nested)]
    pub remove_donation_urls: Option<Vec<DonationLink>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub issues_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub source_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub wiki_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(
        custom(function = "crate::util::validate::validate_url"),
        length(max = 2048)
    )]
    pub discord_url: Option<Option<String>>,
}

#[patch("projects")]
pub async fn projects_edit(
    req: HttpRequest,
    web::Query(ids): web::Query<ProjectIds>,
    pool: web::Data<PgPool>,
    bulk_edit_project: web::Json<BulkEditProject>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let bulk_edit_project = bulk_edit_project.into_inner();

    let mut link_urls = HashMap::new();

    // 如果我们正在设置捐赠链接，我们将设置每个可能的捐赠链接为 None，
    // 因为设置将删除所有链接，然后重新添加我们想要保留的链接
    if let Some(donation_url) = bulk_edit_project.donation_urls {
        let link_platforms = LinkPlatform::list(&**pool, &redis).await?;
        for link in link_platforms {
            if link.donation {
                link_urls.insert(link.name, None);
            }
        }
        // 添加新捐赠链接
        for donation_url in donation_url {
            link_urls.insert(donation_url.id, Some(donation_url.url));
        }
    }

    // 对于每个删除，我们将链接设置为 None
    if let Some(donation_url) = bulk_edit_project.remove_donation_urls {
        for donation_url in donation_url {
            link_urls.insert(donation_url.id, None);
        }
    }

    // 对于每个添加，我们将链接设置为新链接
    if let Some(donation_url) = bulk_edit_project.add_donation_urls {
        for donation_url in donation_url {
            link_urls.insert(donation_url.id, Some(donation_url.url));
        }
    }

    if let Some(issue_url) = bulk_edit_project.issues_url {
        if let Some(issue_url) = issue_url {
            link_urls.insert("issues".to_string(), Some(issue_url));
        } else {
            link_urls.insert("issues".to_string(), None);
        }
    }

    if let Some(source_url) = bulk_edit_project.source_url {
        if let Some(source_url) = source_url {
            link_urls.insert("source".to_string(), Some(source_url));
        } else {
            link_urls.insert("source".to_string(), None);
        }
    }

    if let Some(wiki_url) = bulk_edit_project.wiki_url {
        if let Some(wiki_url) = wiki_url {
            link_urls.insert("wiki".to_string(), Some(wiki_url));
        } else {
            link_urls.insert("wiki".to_string(), None);
        }
    }

    if let Some(discord_url) = bulk_edit_project.discord_url {
        if let Some(discord_url) = discord_url {
            link_urls.insert("discord".to_string(), Some(discord_url));
        } else {
            link_urls.insert("discord".to_string(), None);
        }
    }

    // 这返回 NoContent 或失败，所以我们不需要对其做任何处理
    v3::projects::projects_edit(
        req,
        web::Query(ids),
        pool.clone(),
        web::Json(v3::projects::BulkEditProject {
            categories: bulk_edit_project.categories,
            add_categories: bulk_edit_project.add_categories,
            remove_categories: bulk_edit_project.remove_categories,
            additional_categories: bulk_edit_project.additional_categories,
            add_additional_categories: bulk_edit_project
                .add_additional_categories,
            remove_additional_categories: bulk_edit_project
                .remove_additional_categories,
            link_urls: Some(link_urls),
        }),
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[derive(Serialize, Deserialize)]
pub struct Extension {
    pub ext: String,
}

#[patch("{id}/icon")]
#[allow(clippy::too_many_arguments)]
pub async fn project_icon_edit(
    web::Query(ext): web::Query<Extension>,
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    payload: web::Payload,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::project_icon_edit(
        web::Query(v3::projects::Extension { ext: ext.ext }),
        req,
        info,
        pool,
        redis,
        file_host,
        payload,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[delete("{id}/icon")]
pub async fn delete_project_icon(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::delete_project_icon(
        req,
        info,
        pool,
        redis,
        file_host,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[derive(Serialize, Deserialize, Validate)]
pub struct GalleryCreateQuery {
    pub featured: bool,
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    #[validate(length(min = 1, max = 2048))]
    pub description: Option<String>,
    pub ordering: Option<i64>,
}

#[post("{id}/gallery")]
#[allow(clippy::too_many_arguments)]
pub async fn add_gallery_item(
    web::Query(ext): web::Query<Extension>,
    req: HttpRequest,
    web::Query(item): web::Query<GalleryCreateQuery>,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    payload: web::Payload,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::add_gallery_item(
        web::Query(v3::projects::Extension { ext: ext.ext }),
        req,
        web::Query(v3::projects::GalleryCreateQuery {
            featured: item.featured,
            name: item.title,
            description: item.description,
            ordering: item.ordering,
        }),
        info,
        pool,
        redis,
        file_host,
        payload,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[derive(Serialize, Deserialize, Validate)]
pub struct GalleryEditQuery {
    /// 要编辑的画廊项目的 URL
    pub url: String,
    pub featured: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(length(min = 1, max = 255))]
    pub title: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::double_option"
    )]
    #[validate(length(min = 1, max = 2048))]
    pub description: Option<Option<String>>,
    pub ordering: Option<i64>,
}

#[patch("{id}/gallery")]
pub async fn edit_gallery_item(
    req: HttpRequest,
    web::Query(item): web::Query<GalleryEditQuery>,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::edit_gallery_item(
        req,
        web::Query(v3::projects::GalleryEditQuery {
            url: item.url,
            featured: item.featured,
            name: item.title,
            description: item.description,
            ordering: item.ordering,
        }),
        info,
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[derive(Serialize, Deserialize)]
pub struct GalleryDeleteQuery {
    pub url: String,
}

#[delete("{id}/gallery")]
pub async fn delete_gallery_item(
    req: HttpRequest,
    web::Query(item): web::Query<GalleryDeleteQuery>,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::delete_gallery_item(
        req,
        web::Query(v3::projects::GalleryDeleteQuery { url: item.url }),
        info,
        pool,
        redis,
        file_host,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[delete("{id}")]
pub async fn project_delete(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    search_config: web::Data<SearchConfig>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::project_delete(
        req,
        info,
        pool,
        redis,
        search_config,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

#[post("{id}/follow")]
pub async fn project_follow(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::project_follow(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

#[delete("{id}/follow")]
pub async fn project_unfollow(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 返回 NoContent，所以不需要转换
    v3::projects::project_unfollow(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}

#[get("translation_links")]
pub async fn get_translation_links(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 直接调用 v3 的函数，返回相同的数据结构
    v3::projects::get_translation_links(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}
