//! 首页聚合接口：前端一次请求拿到首页需要的全部列表数据
//!
//! `GET /v3/homepage?types=hot_projects,hot_translations,latest_translations,latest_plugins`
//!
//! 返回 `{ "<type>": [v2 搜索结果条目...] }`，条目格式与 `GET /v2/search` 的 `hits` 完全一致。
//! 每个 type 的结果单独缓存在 Redis，同一 type 并发未命中时只回源搜索一次。
//! 数据不区分用户，登录与未登录拿到的是同一份缓存。

use actix_web::{HttpResponse, web};
use serde::Deserialize;
use std::collections::HashMap;

use crate::database::models::DatabaseError;
use crate::database::redis::RedisPool;
use crate::models::projects::SearchRequest;
use crate::models::v2::search::{
    LegacyResultSearchProject, LegacySearchResults,
};
use crate::routes::ApiError;
use crate::search::{SearchConfig, search_for_project};

const HOMEPAGE_SECTION_NAMESPACE: &str = "homepage_section";
/// 每个 type 的缓存时间
const HOMEPAGE_SECTION_TTL_SECS: i64 = 5 * 60;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("homepage", web::get().to(get_homepage));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HomepageSection {
    /// 热门资源
    HotProjects,
    /// 热门汉化包
    HotTranslations,
    /// 最近更新的汉化包（侧栏推荐）
    LatestTranslations,
    /// 最近更新的服务器插件
    LatestPlugins,
}

impl HomepageSection {
    const ALL: [Self; 4] = [
        Self::HotProjects,
        Self::HotTranslations,
        Self::LatestTranslations,
        Self::LatestPlugins,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::HotProjects => "hot_projects",
            Self::HotTranslations => "hot_translations",
            Self::LatestTranslations => "latest_translations",
            Self::LatestPlugins => "latest_plugins",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|x| x.as_str() == value)
    }

    /// 与前端原先请求的 `/v2/search` 参数一致（facet 已换成 v3 写法 project_types）
    fn search_request(self) -> SearchRequest {
        let (index, limit, facets) = match self {
            Self::HotProjects => ("relevance", 6, None),
            Self::HotTranslations => {
                ("relevance", 6, Some(r#"[["project_types:language"]]"#))
            }
            Self::LatestTranslations => {
                ("updated", 5, Some(r#"[["project_types:language"]]"#))
            }
            Self::LatestPlugins => {
                ("updated", 5, Some(r#"[["project_types:plugin"]]"#))
            }
        };

        SearchRequest {
            query: None,
            offset: None,
            index: Some(index.to_string()),
            limit: Some(limit.to_string()),
            new_filters: None,
            facets: facets.map(str::to_string),
            filters: None,
            version: None,
        }
    }

    async fn load(
        self,
        search_config: &SearchConfig,
    ) -> Result<Vec<LegacyResultSearchProject>, DatabaseError> {
        // 缓存工具要求 DatabaseError，这里把搜索错误包一层
        let results = search_for_project(&self.search_request(), search_config)
            .await
            .map_err(|e| {
                DatabaseError::Database2(format!(
                    "首页数据 {} 搜索失败: {e}",
                    self.as_str()
                ))
            })?;
        Ok(LegacySearchResults::from(results).hits)
    }
}

#[derive(Deserialize)]
pub struct HomepageQuery {
    /// 逗号分隔的 type 列表
    pub types: String,
}

/// 首页聚合数据（公开接口，无需登录）
pub async fn get_homepage(
    web::Query(query): web::Query<HomepageQuery>,
    redis: web::Data<RedisPool>,
    search_config: web::Data<SearchConfig>,
) -> Result<HttpResponse, ApiError> {
    let mut sections = Vec::new();
    for raw in query
        .types
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
    {
        let section = HomepageSection::parse(raw).ok_or_else(|| {
            ApiError::InvalidInput(format!(
                "未知的首页数据类型: {raw}，可选值: {}",
                HomepageSection::ALL.map(HomepageSection::as_str).join(", ")
            ))
        })?;
        if !sections.contains(&section) {
            sections.push(section);
        }
    }
    if sections.is_empty() {
        return Err(ApiError::InvalidInput("types 不能为空".to_string()));
    }

    let redis = &**redis;
    let search_config = &**search_config;
    let results = futures::future::try_join_all(sections.into_iter().map(
        |section| async move {
            let hits = redis
                .get_cached_key_singleflight_with_expiry(
                    HOMEPAGE_SECTION_NAMESPACE,
                    section.as_str(),
                    HOMEPAGE_SECTION_TTL_SECS,
                    || section.load(search_config),
                )
                .await?;
            Ok::<_, DatabaseError>((section.as_str(), hits))
        },
    ))
    .await?;

    let body: HashMap<&str, Vec<LegacyResultSearchProject>> =
        results.into_iter().collect();
    Ok(HttpResponse::Ok().json(body))
}
