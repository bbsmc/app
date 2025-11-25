use super::ApiError;
use crate::auth::get_user_from_headers;
use crate::models::ids::VersionId;
use crate::models::ids::base62_impl::parse_base62;
use crate::models::pats::Scopes;
use crate::models::projects::Project;
use crate::models::v2::projects::LegacyProject;
use crate::queue::session::AuthQueue;
use crate::routes::internal;
use crate::routes::v3;
use crate::{database::redis::RedisPool, routes::v2_reroute};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("moderation")
            .service(get_projects)
            .service(get_translation_links),
    );
}

#[derive(Deserialize)]
pub struct ResultCount {
    #[serde(default = "default_count")]
    pub count: i16,
}

fn default_count() -> i16 {
    100
}

#[get("projects")]
pub async fn get_projects(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    count: web::Query<ResultCount>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let response = internal::moderation::get_projects(
        req,
        pool.clone(),
        redis.clone(),
        web::Query(internal::moderation::ResultCount { count: count.count }),
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)?;

    // 将响应转换为 V2 项目
    match v2_reroute::extract_ok_json::<Vec<Project>>(response).await {
        Ok(project) => {
            let legacy_projects =
                LegacyProject::from_many(project, &**pool, &redis).await?;
            Ok(HttpResponse::Ok().json(legacy_projects))
        }
        Err(response) => Ok(response),
    }
}

#[derive(Deserialize)]
pub struct TranslationLinksQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct TranslationLinkResponse {
    pub id: String,
    pub translation_version_id: String,
    pub translation_version_number: String,
    pub translation_project_id: String,
    pub translation_project_slug: String,
    pub translation_project_title: String,
    pub translation_project_icon: Option<String>,
    pub target_version_id: String,
    pub target_version_number: String,
    pub target_project_id: String,
    pub target_project_slug: String,
    pub target_project_title: String,
    pub target_project_icon: Option<String>,
    pub language_code: String,
    pub description: Option<String>,
    pub approval_status: String,
    pub submitter_id: String,
    pub submitter_username: String,
    pub submitter_avatar: Option<String>,
    pub created: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct TranslationLinksResponse {
    pub links: Vec<TranslationLinkResponse>,
    pub total: u32,
}

#[get("translation-links")]
pub async fn get_translation_links(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    query: web::Query<TranslationLinksQuery>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证用户是否为管理员
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?
    .1;

    if !user.role.is_admin() && !user.role.is_mod() {
        return Err(ApiError::CustomAuthentication(
            "您没有权限访问此页面".to_string(),
        ));
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = ((page - 1) * limit) as i64;
    let limit_i64 = limit as i64;

    // 构建查询条件
    let status_filter = query.status.as_deref();

    // 获取总数
    let total_count: i64 = if let Some(status) = status_filter {
        if status == "all" {
            sqlx::query_scalar!("SELECT COUNT(*) FROM version_link_version")
                .fetch_one(&**pool)
                .await?
                .unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM version_link_version WHERE approval_status = $1",
                status
            )
            .fetch_one(&**pool)
            .await?
            .unwrap_or(0)
        }
    } else {
        sqlx::query_scalar!("SELECT COUNT(*) FROM version_link_version")
            .fetch_one(&**pool)
            .await?
            .unwrap_or(0)
    };

    // 获取翻译链接列表 - 使用单个查询避免类型不匹配
    let links = sqlx::query!(
        r#"
        SELECT 
            vlv.version_id as translation_version_id,
            vlv.joining_version_id as target_version_id,
            vlv.language_code,
            vlv.description,
            vlv.approval_status,
            v1.version_number as translation_version_number,
            v1.mod_id as translation_project_id,
            v1.author_id as submitter_id,
            v1.date_published as created,
            v2.version_number as target_version_number,
            v2.mod_id as target_project_id,
            p1.slug as translation_project_slug,
            p1.name as translation_project_title,
            p1.icon_url as translation_project_icon,
            p2.slug as target_project_slug,
            p2.name as target_project_title,
            p2.icon_url as target_project_icon,
            u.username as submitter_username,
            u.avatar_url as submitter_avatar
        FROM version_link_version vlv
        INNER JOIN versions v1 ON v1.id = vlv.version_id
        INNER JOIN versions v2 ON v2.id = vlv.joining_version_id
        INNER JOIN mods p1 ON p1.id = v1.mod_id
        INNER JOIN mods p2 ON p2.id = v2.mod_id
        INNER JOIN users u ON u.id = v1.author_id
        WHERE ($1::text IS NULL OR $1 = 'all' OR vlv.approval_status = $1)
        ORDER BY v1.date_published DESC
        LIMIT $2 OFFSET $3
        "#,
        status_filter,
        limit_i64,
        offset
    )
    .fetch_all(&**pool)
    .await?;

    // 转换为响应格式
    let mut response_links = Vec::new();
    for link in links {
        use crate::models::ids::{ProjectId, UserId, VersionId};

        let translation_version_id =
            VersionId(link.translation_version_id as u64);
        let target_version_id = VersionId(link.target_version_id as u64);
        let translation_project_id =
            ProjectId(link.translation_project_id as u64);
        let target_project_id = ProjectId(link.target_project_id as u64);
        let submitter_id = UserId(link.submitter_id as u64);

        response_links.push(TranslationLinkResponse {
            id: format!("{}-{}", translation_version_id, target_version_id),
            translation_version_id: translation_version_id.to_string(),
            translation_version_number: link.translation_version_number,
            translation_project_id: translation_project_id.to_string(),
            translation_project_slug: link
                .translation_project_slug
                .unwrap_or_else(|| translation_project_id.to_string()),
            translation_project_title: link.translation_project_title,
            translation_project_icon: link.translation_project_icon,
            target_version_id: target_version_id.to_string(),
            target_version_number: link.target_version_number,
            target_project_id: target_project_id.to_string(),
            target_project_slug: link
                .target_project_slug
                .unwrap_or_else(|| target_project_id.to_string()),
            target_project_title: link.target_project_title,
            target_project_icon: link.target_project_icon,
            language_code: link.language_code,
            description: link.description,
            approval_status: link.approval_status,
            submitter_id: submitter_id.to_string(),
            submitter_username: link.submitter_username,
            submitter_avatar: link.submitter_avatar,
            created: link.created,
        });
    }

    Ok(HttpResponse::Ok().json(TranslationLinksResponse {
        links: response_links,
        total: total_count as u32,
    }))
}

// 管理员批准翻译链接
#[post("translation-links/{translation_id}/{target_id}/approve")]
pub async fn admin_approve_link(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证管理员权限
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    if !user.role.is_admin() && !user.role.is_mod() {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    // 转换ID并调用v3的approve函数
    let translation_version_id = VersionId(parse_base62(&info.0)?);
    let target_version_id = VersionId(parse_base62(&info.1)?);

    v3::versions::approve_version_link(
        req,
        web::Path::from((translation_version_id, target_version_id)),
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

// 管理员拒绝翻译链接
#[post("translation-links/{translation_id}/{target_id}/reject")]
pub async fn admin_reject_link(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证管理员权限
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    if !user.role.is_admin() && !user.role.is_mod() {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    // 转换ID并调用v3的reject函数
    let translation_version_id = VersionId(parse_base62(&info.0)?);
    let target_version_id = VersionId(parse_base62(&info.1)?);

    v3::versions::reject_version_link(
        req,
        web::Path::from((translation_version_id, target_version_id)),
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}

// 管理员撤销已批准的翻译链接
#[post("translation-links/{translation_id}/{target_id}/revoke")]
pub async fn admin_revoke_link(
    req: HttpRequest,
    info: web::Path<(String, String)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    // 验证管理员权限
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::VERSION_WRITE]),
    )
    .await?
    .1;

    if !user.role.is_admin() && !user.role.is_mod() {
        return Err(ApiError::CustomAuthentication(
            "您没有权限执行此操作".to_string(),
        ));
    }

    // 转换ID并调用v3的revoke函数
    let translation_version_id = VersionId(parse_base62(&info.0)?);
    let target_version_id = VersionId(parse_base62(&info.1)?);

    v3::versions::revoke_version_link(
        req,
        web::Path::from((translation_version_id, target_version_id)),
        pool,
        redis,
        session_queue,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}
