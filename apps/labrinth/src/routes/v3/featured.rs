use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::auth::check_is_admin_from_headers;
use crate::database::models::featured_item::{DBFeaturedItem, NewFeaturedItem};
use crate::database::models::{self as db_models, DatabaseError};
use crate::database::redis::RedisPool;
use crate::file_hosting::FileHost;
use crate::models::featured::{
    FEATURED_SECTIONS, FeaturedItem, FeaturedItemAdmin, is_valid_section,
    section_keys_joined,
};
use crate::models::pats::Scopes;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::util::ext::get_image_content_type;
use crate::util::img::{UploadImagePos, upload_image_optimized};
use crate::util::routes::read_from_payload;

const FEATURED_NAMESPACE: &str = "featured";
const FEATURED_CACHE_KEY: &str = "all";
const FEATURED_CACHE_TTL_SECS: i64 = 30 * 60;

const FEATURED_IMAGE_FOLDER: &str = "data/featured";
const FEATURED_IMAGE_MAX_BYTES: usize = 4 * 1024 * 1024;
const FEATURED_IMAGE_TARGET_WIDTH: u32 = 1600;

const TITLE_MAX_CHARS: usize = 128;
const DESCRIPTION_MAX_CHARS: usize = 512;
const BADGE_MAX_CHARS: usize = 32;
const URL_MAX_CHARS: usize = 2048;

type FeaturedMap = HashMap<String, Vec<FeaturedItem>>;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("featured", web::get().to(get_featured));
}

pub fn admin_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("featured")
            .route("sections", web::get().to(admin_list_sections))
            .route("items", web::get().to(admin_list_items))
            .route("items", web::post().to(admin_create_item))
            .route("items/reorder", web::post().to(admin_reorder_items))
            .route("items/{id}", web::patch().to(admin_update_item))
            .route("items/{id}", web::delete().to(admin_delete_item))
            .route("image", web::post().to(admin_upload_image)),
    );
}

#[derive(Deserialize)]
pub struct FeaturedQuery {
    pub sections: Option<String>,
}

pub async fn get_featured(
    web::Query(query): web::Query<FeaturedQuery>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let requested: Option<Vec<String>> = match query.sections {
        Some(raw) => {
            let mut keys: Vec<String> = Vec::new();
            for key in raw.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                if !is_valid_section(key) {
                    return Err(ApiError::InvalidInput(format!(
                        "未知的推荐位板块: {key}，可选值: {}",
                        section_keys_joined()
                    )));
                }
                if !keys.iter().any(|x| x == key) {
                    keys.push(key.to_string());
                }
            }
            if keys.is_empty() {
                return Err(ApiError::InvalidInput(
                    "sections 不能为空".to_string(),
                ));
            }
            Some(keys)
        }
        None => None,
    };

    let all = load_enabled_cached(&pool, &redis).await?;

    let body: FeaturedMap = match requested {
        Some(keys) => keys
            .into_iter()
            .map(|key| {
                let items = all.get(&key).cloned().unwrap_or_default();
                (key, items)
            })
            .collect(),
        None => all,
    };

    Ok(HttpResponse::Ok().json(body))
}

async fn load_enabled_cached(
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<FeaturedMap, DatabaseError> {
    redis
        .get_cached_key_singleflight_with_expiry(
            FEATURED_NAMESPACE,
            FEATURED_CACHE_KEY,
            FEATURED_CACHE_TTL_SECS,
            || async {
                let rows = DBFeaturedItem::list_enabled(pool).await?;
                let mut map: FeaturedMap = HashMap::new();
                for row in rows {
                    map.entry(row.section.clone())
                        .or_default()
                        .push(FeaturedItem::from(row));
                }
                Ok(map)
            },
        )
        .await
}

async fn invalidate_cache(redis: &RedisPool) -> Result<(), DatabaseError> {
    let mut conn = redis.connect().await?;
    conn.delete(FEATURED_NAMESPACE, FEATURED_CACHE_KEY).await
}

#[derive(Deserialize)]
pub struct AdminItemsQuery {
    pub section: String,
}

#[derive(Deserialize)]
pub struct CreateFeaturedItem {
    pub section: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub badge: String,
    pub image_url: String,
    pub link: String,
    pub project_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct EditFeaturedItem {
    pub title: Option<String>,
    pub description: Option<String>,
    pub badge: Option<String>,
    pub image_url: Option<String>,
    pub link: Option<String>,
    pub project_id: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct ReorderFeaturedItems {
    pub section: String,
    pub ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct ImageExtension {
    pub ext: String,
}

pub async fn admin_list_sections(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    Ok(HttpResponse::Ok().json(FEATURED_SECTIONS))
}

pub async fn admin_list_items(
    req: HttpRequest,
    web::Query(query): web::Query<AdminItemsQuery>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_READ]),
    )
    .await?;

    let section = validate_section(&query.section)?;
    let items = DBFeaturedItem::list_section(section, &**pool)
        .await?
        .into_iter()
        .map(FeaturedItemAdmin::from)
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(items))
}

pub async fn admin_create_item(
    req: HttpRequest,
    body: web::Json<CreateFeaturedItem>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let body = body.into_inner();
    let section = validate_section(&body.section)?.to_string();
    let title = validate_text(&body.title, "标题", 1, TITLE_MAX_CHARS)?;
    let description =
        validate_text(&body.description, "描述", 0, DESCRIPTION_MAX_CHARS)?;
    let badge = validate_text(&body.badge, "角标", 0, BADGE_MAX_CHARS)?;
    let image_url = validate_link(&body.image_url, "图片地址", false)?;
    let link = validate_link(&body.link, "跳转链接", true)?;
    let project_id =
        resolve_optional_project(body.project_id.as_deref(), &pool, &redis)
            .await?;

    let item = DBFeaturedItem::insert(
        NewFeaturedItem {
            section,
            title,
            description,
            badge,
            image_url,
            link,
            project_id,
            enabled: body.enabled,
            created_by: Some(db_models::UserId::from(user.id).0),
        },
        &**pool,
    )
    .await?;

    invalidate_cache(&redis).await?;

    Ok(HttpResponse::Ok().json(FeaturedItemAdmin::from(item)))
}

pub async fn admin_update_item(
    req: HttpRequest,
    path: web::Path<(i64,)>,
    body: web::Json<EditFeaturedItem>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let (id,) = path.into_inner();
    let body = body.into_inner();

    let mut item = DBFeaturedItem::get(id, &**pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    if let Some(title) = &body.title {
        item.title = validate_text(title, "标题", 1, TITLE_MAX_CHARS)?;
    }
    if let Some(description) = &body.description {
        item.description =
            validate_text(description, "描述", 0, DESCRIPTION_MAX_CHARS)?;
    }
    if let Some(badge) = &body.badge {
        item.badge = validate_text(badge, "角标", 0, BADGE_MAX_CHARS)?;
    }
    if let Some(image_url) = &body.image_url {
        item.image_url = validate_link(image_url, "图片地址", false)?;
    }
    if let Some(link) = &body.link {
        item.link = validate_link(link, "跳转链接", true)?;
    }
    if body.project_id.is_some() {
        item.project_id =
            resolve_optional_project(body.project_id.as_deref(), &pool, &redis)
                .await?;
    }
    if let Some(enabled) = body.enabled {
        item.enabled = enabled;
    }

    let updated = item.update(&**pool).await?.ok_or(ApiError::NotFound)?;

    invalidate_cache(&redis).await?;

    Ok(HttpResponse::Ok().json(FeaturedItemAdmin::from(updated)))
}

pub async fn admin_delete_item(
    req: HttpRequest,
    path: web::Path<(i64,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let (id,) = path.into_inner();
    if !DBFeaturedItem::delete(id, &**pool).await? {
        return Err(ApiError::NotFound);
    }

    invalidate_cache(&redis).await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn admin_reorder_items(
    req: HttpRequest,
    body: web::Json<ReorderFeaturedItems>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let body = body.into_inner();
    let section = validate_section(&body.section)?;

    let mut current = DBFeaturedItem::section_ids(section, &**pool).await?;
    let mut requested = body.ids.clone();
    current.sort_unstable();
    requested.sort_unstable();
    if current != requested {
        return Err(ApiError::InvalidInput(
            "排序列表与板块当前内容不一致，请刷新后重试".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    for (index, id) in body.ids.iter().enumerate() {
        DBFeaturedItem::set_sort_order(
            *id,
            section,
            index as i32,
            &mut *transaction,
        )
        .await?;
    }
    transaction.commit().await?;

    invalidate_cache(&redis).await?;

    let items = DBFeaturedItem::list_section(section, &**pool)
        .await?
        .into_iter()
        .map(FeaturedItemAdmin::from)
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(items))
}

pub async fn admin_upload_image(
    req: HttpRequest,
    web::Query(query): web::Query<ImageExtension>,
    file_host: web::Data<Arc<dyn FileHost + Send + Sync>>,
    mut payload: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::USER_WRITE]),
    )
    .await?;

    let ext = query
        .ext
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if get_image_content_type(&ext).is_none() {
        return Err(ApiError::InvalidInput(format!(
            "不支持的图片格式: {ext}，仅接受 png / jpeg / gif / webp / bmp"
        )));
    }

    let bytes = read_from_payload(
        &mut payload,
        FEATURED_IMAGE_MAX_BYTES,
        "图片大小必须小于 4MB",
    )
    .await?;

    let upload = upload_image_optimized(
        FEATURED_IMAGE_FOLDER,
        bytes.freeze(),
        &ext,
        Some(FEATURED_IMAGE_TARGET_WIDTH),
        None,
        &***file_host,
        UploadImagePos {
            pos: "推荐位图片".to_string(),
            url: "/moderation/featured".to_string(),
            username: user.username.clone(),
        },
        &redis,
        true,
    )
    .await?;

    Ok(HttpResponse::Ok().json(json!({
        "url": upload.url,
        "raw_url": upload.raw_url,
    })))
}

fn validate_section(raw: &str) -> Result<&str, ApiError> {
    let key = raw.trim();
    if is_valid_section(key) {
        Ok(key)
    } else {
        Err(ApiError::InvalidInput(format!(
            "未知的推荐位板块: {key}，可选值: {}",
            section_keys_joined()
        )))
    }
}

fn validate_text(
    raw: &str,
    field: &str,
    min_chars: usize,
    max_chars: usize,
) -> Result<String, ApiError> {
    let value = raw.trim();
    let len = value.chars().count();
    if len < min_chars {
        return Err(ApiError::InvalidInput(format!("{field}不能为空")));
    }
    if len > max_chars {
        return Err(ApiError::InvalidInput(format!(
            "{field}不能超过 {max_chars} 个字符"
        )));
    }
    Ok(value.to_string())
}

fn validate_link(
    raw: &str,
    field: &str,
    allow_http: bool,
) -> Result<String, ApiError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ApiError::InvalidInput(format!("{field}不能为空")));
    }
    if value.chars().count() > URL_MAX_CHARS {
        return Err(ApiError::InvalidInput(format!(
            "{field}不能超过 {URL_MAX_CHARS} 个字符"
        )));
    }
    if value.chars().any(|c| c.is_whitespace() || c == '\\') {
        return Err(ApiError::InvalidInput(format!(
            "{field}不能包含空白字符或反斜杠"
        )));
    }
    let is_site_path = value.starts_with('/') && !value.starts_with("//");
    let is_absolute = value.starts_with("https://")
        || (allow_http && value.starts_with("http://"));
    if !is_site_path && !is_absolute {
        return Err(ApiError::InvalidInput(format!(
            "{field}必须是以 / 开头的站内路径，或以 https:// 开头的完整地址"
        )));
    }
    Ok(value.to_string())
}

async fn resolve_optional_project(
    raw: Option<&str>,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Option<i64>, ApiError> {
    let Some(raw) = raw.map(str::trim).filter(|x| !x.is_empty()) else {
        return Ok(None);
    };
    let project = db_models::Project::get(raw, pool, redis)
        .await?
        .ok_or_else(|| ApiError::InvalidInput(format!("项目不存在: {raw}")))?;
    Ok(Some(project.inner.id.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_accepts_site_path_and_https() {
        assert_eq!(
            validate_link("/modpack/snk", "链接", true).unwrap(),
            "/modpack/snk"
        );
        assert_eq!(
            validate_link(" https://example.com/a?b=1 ", "链接", true).unwrap(),
            "https://example.com/a?b=1"
        );
        assert!(validate_link("http://example.com/a", "链接", true).is_ok());
        assert!(
            validate_link("http://example.com/a.jpg", "图片", false).is_err()
        );
    }

    #[test]
    fn link_rejects_bad_forms() {
        assert!(validate_link("", "链接", true).is_err());
        assert!(validate_link("modpack/snk", "链接", true).is_err());
        assert!(validate_link("//evil.com", "链接", true).is_err());
        assert!(validate_link("/\\evil.com", "链接", true).is_err());
        assert!(validate_link("javascript:alert(1)", "链接", true).is_err());
        assert!(validate_link("/a b", "链接", true).is_err());
    }

    #[test]
    fn text_limits_count_chars_not_bytes() {
        let title = "汉".repeat(TITLE_MAX_CHARS);
        assert!(validate_text(&title, "标题", 1, TITLE_MAX_CHARS).is_ok());
        let too_long = "汉".repeat(TITLE_MAX_CHARS + 1);
        assert!(validate_text(&too_long, "标题", 1, TITLE_MAX_CHARS).is_err());
        assert!(validate_text("   ", "标题", 1, TITLE_MAX_CHARS).is_err());
        assert_eq!(validate_text("   ", "描述", 0, 10).unwrap(), "");
    }

    #[test]
    fn section_validation() {
        assert_eq!(validate_section(" home ").unwrap(), "home");
        assert!(validate_section("search_modpack").is_ok());
        assert!(validate_section("unknown").is_err());
    }
}
