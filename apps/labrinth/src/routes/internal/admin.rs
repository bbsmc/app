use crate::auth::validate::get_user_record_from_bearer_token;
use crate::database::redis::RedisPool;
use crate::models::analytics::Download;
use crate::models::ids::ProjectId;
use crate::models::ids::base62_impl::{parse_base62, to_base62};
use crate::models::pats::Scopes;
use crate::queue::analytics::AnalyticsQueue;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::search::SearchConfig;
use crate::util::date::get_current_tenths_of_ms;
use crate::util::guards::admin_key_guard;
use actix_web::{HttpRequest, HttpResponse, patch, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("admin")
            .service(count_download)
            .service(force_reindex)
            .service(fix_modpack_loaders),
    );
}

#[derive(Deserialize)]
pub struct DownloadBody {
    pub url: String,
    pub project_id: ProjectId,
    pub version_name: String,

    pub ip: String,
    pub headers: HashMap<String, String>,
}

// This is an internal route, cannot be used without key
#[patch("/_count-download", guard = "admin_key_guard")]
#[allow(clippy::too_many_arguments)]
pub async fn count_download(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    analytics_queue: web::Data<Arc<AnalyticsQueue>>,
    session_queue: web::Data<AuthQueue>,
    download_body: web::Json<DownloadBody>,
) -> Result<HttpResponse, ApiError> {
    let token = download_body
        .headers
        .iter()
        .find(|x| x.0.to_lowercase() == "authorization")
        .map(|x| &**x.1);

    let user = get_user_record_from_bearer_token(
        &req,
        token,
        &**pool,
        &redis,
        &session_queue,
    )
    .await
    .ok()
    .flatten();

    let project_id: crate::database::models::ids::ProjectId =
        download_body.project_id.into();

    let id_option = crate::models::ids::base62_impl::parse_base62(
        &download_body.version_name,
    )
    .ok()
    .map(|x| x as i64);

    let (version_id, project_id) = if let Some(version) = sqlx::query!(
        "
            SELECT v.id id, v.mod_id mod_id FROM files f
            INNER JOIN versions v ON v.id = f.version_id
            WHERE f.url = $1
            ",
        download_body.url,
    )
    .fetch_optional(pool.as_ref())
    .await?
    {
        (version.id, version.mod_id)
    } else if let Some(version) = sqlx::query!(
        "
        SELECT id, mod_id FROM versions
        WHERE ((version_number = $1 OR id = $3) AND mod_id = $2)
        ",
        download_body.version_name,
        project_id as crate::database::models::ids::ProjectId,
        id_option
    )
    .fetch_optional(pool.as_ref())
    .await?
    {
        (version.id, version.mod_id)
    } else {
        return Err(ApiError::InvalidInput("指定的版本不存在！".to_string()));
    };

    let url = url::Url::parse(&download_body.url).map_err(|_| {
        ApiError::InvalidInput("指定的下载链接无效！".to_string())
    })?;

    let ip = crate::util::ip::convert_to_ip_v6(&download_body.ip)
        .unwrap_or_else(|_| Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped());

    analytics_queue.add_download(Download {
        recorded: get_current_tenths_of_ms(),
        domain: url.host_str().unwrap_or_default().to_string(),
        site_path: url.path().to_string(),
        user_id: user
            .and_then(|(scopes, x)| {
                if scopes.contains(Scopes::PERFORM_ANALYTICS) {
                    Some(x.id.0 as u64)
                } else {
                    None
                }
            })
            .unwrap_or(0),
        project_id: project_id as u64,
        version_id: version_id as u64,
        ip,
        country: String::new(), // MaxMind 功能已移除
        user_agent: download_body
            .headers
            .get("user-agent")
            .cloned()
            .unwrap_or_default(),
        headers: download_body
            .headers
            .clone()
            .into_iter()
            .filter(|x| {
                !crate::routes::analytics::FILTERED_HEADERS
                    .contains(&&*x.0.to_lowercase())
            })
            .collect(),
    });

    Ok(HttpResponse::NoContent().body(""))
}

#[post("/_force_reindex", guard = "admin_key_guard")]
pub async fn force_reindex(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<SearchConfig>,
) -> Result<HttpResponse, ApiError> {
    use crate::search::indexing::index_projects;
    let redis = redis.get_ref();
    index_projects(pool.as_ref().clone(), redis.clone(), &config).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ==================== 修复 Modpack Loader 设置 ====================

/// 请求体：修复 modpack 版本的 loader 设置
#[derive(Deserialize)]
pub struct FixModpackLoadersBody {
    /// 项目 ID（修复整个项目的所有版本）
    pub project_id: Option<String>,
    /// 版本 ID 列表（修复指定版本）
    pub version_ids: Option<Vec<String>>,
    /// 预览模式，不执行实际修改
    #[serde(default)]
    pub dry_run: bool,
    /// 修复后是否自动重新索引搜索
    #[serde(default)]
    pub reindex: bool,
}

/// 响应体：修复结果
#[derive(Serialize)]
pub struct FixModpackLoadersResult {
    /// 修复的版本数量
    pub fixed_count: usize,
    /// 跳过的版本数量（已经是正确的 loader 或不需要修复）
    pub skipped_count: usize,
    /// 详细修复信息
    pub details: Vec<FixedVersionDetail>,
    /// 是否已重新索引
    pub reindexed: bool,
}

/// 单个版本的修复详情
#[derive(Serialize)]
pub struct FixedVersionDetail {
    /// 版本 ID（Base62 编码）
    pub version_id: String,
    /// 版本名称
    pub version_name: String,
    /// 原来的 loaders
    pub old_loaders: Vec<String>,
    /// 修复后的 loaders
    pub new_loaders: Vec<String>,
    /// 添加的 mrpack_loaders 字段值
    pub mrpack_loaders_added: Vec<String>,
}

/// 修复 modpack 项目版本的 loader 设置
///
/// 将错误使用 forge/fabric/neoforge/quilt 的 modpack 版本修正为使用 mrpack loader，
/// 并将原来的 loader 保存到 mrpack_loaders 字段中。
///
/// 这是一个内部管理接口，需要 admin key 认证。
#[post("/_fix_modpack_loaders", guard = "admin_key_guard")]
pub async fn fix_modpack_loaders(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    search_config: web::Data<SearchConfig>,
    body: web::Json<FixModpackLoadersBody>,
) -> Result<HttpResponse, ApiError> {
    use crate::database::models::ids as db_ids;

    let mut transaction = pool.begin().await?;

    // 1. 获取需要处理的版本列表
    let version_ids: Vec<i64> = if let Some(project_id_str) = &body.project_id {
        // 按项目获取所有版本
        let project_id = parse_base62(project_id_str)
            .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?;

        // 检查项目是否存在
        let versions = sqlx::query!(
            "SELECT id FROM versions WHERE mod_id = $1",
            project_id as i64
        )
        .fetch_all(&mut *transaction)
        .await?;

        if versions.is_empty() {
            return Err(ApiError::InvalidInput(
                "项目不存在或没有版本".to_string(),
            ));
        }

        versions.into_iter().map(|v| v.id).collect()
    } else if let Some(version_ids_str) = &body.version_ids {
        // 按指定版本 ID 列表
        version_ids_str
            .iter()
            .map(|id| parse_base62(id).map(|x| x as i64))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ApiError::InvalidInput("无效的版本 ID".to_string()))?
    } else {
        return Err(ApiError::InvalidInput(
            "必须提供 project_id 或 version_ids".to_string(),
        ));
    };

    if version_ids.is_empty() {
        return Ok(HttpResponse::Ok().json(FixModpackLoadersResult {
            fixed_count: 0,
            skipped_count: 0,
            details: vec![],
            reindexed: false,
        }));
    }

    // 2. 获取 mrpack loader 的 ID
    let mrpack_loader =
        sqlx::query!("SELECT id FROM loaders WHERE loader = 'mrpack'")
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or_else(|| {
                ApiError::InvalidInput("mrpack loader 不存在".to_string())
            })?;
    let mrpack_loader_id = mrpack_loader.id;

    // 3. 获取 mrpack_loaders 字段的 ID 和枚举类型
    let mrpack_loaders_field = sqlx::query!(
        "SELECT id, enum_type FROM loader_fields WHERE field = 'mrpack_loaders'"
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        ApiError::InvalidInput("mrpack_loaders 字段不存在".to_string())
    })?;
    let mrpack_loaders_field_id = mrpack_loaders_field.id;
    let mrpack_loaders_enum_type = mrpack_loaders_field.enum_type;

    // 4. 获取枚举值映射 (loader_name -> enum_value_id)
    let enum_values: HashMap<String, i32> =
        if let Some(enum_type) = mrpack_loaders_enum_type {
            sqlx::query!(
            "SELECT id, value FROM loader_field_enum_values WHERE enum_id = $1",
            enum_type
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|v| (v.value, v.id))
        .collect()
        } else {
            HashMap::new()
        };

    // 5. 需要修复的 mod loader 列表
    let mod_loader_names = ["forge", "fabric", "neoforge", "quilt"];

    // 6. 处理每个版本
    let mut result = FixModpackLoadersResult {
        fixed_count: 0,
        skipped_count: 0,
        details: vec![],
        reindexed: false,
    };

    let mut project_ids_to_clear: std::collections::HashSet<i64> =
        std::collections::HashSet::new();

    for version_id in version_ids {
        // 获取版本当前的 loaders
        let current_loaders: Vec<String> = sqlx::query!(
            "SELECT l.loader FROM loaders_versions lv
             INNER JOIN loaders l ON lv.loader_id = l.id
             WHERE lv.version_id = $1",
            version_id
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|l| l.loader)
        .collect();

        // 获取版本信息
        let version_info = sqlx::query!(
            "SELECT name, mod_id FROM versions WHERE id = $1",
            version_id
        )
        .fetch_one(&mut *transaction)
        .await?;

        // 检查是否已经是 mrpack
        if current_loaders.contains(&"mrpack".to_string()) {
            result.skipped_count += 1;
            continue;
        }

        // 检查是否有需要转换的 mod loaders
        let loaders_to_convert: Vec<String> = current_loaders
            .iter()
            .filter(|l| mod_loader_names.contains(&l.as_str()))
            .cloned()
            .collect();

        if loaders_to_convert.is_empty() {
            result.skipped_count += 1;
            continue;
        }

        let detail = FixedVersionDetail {
            version_id: to_base62(version_id as u64),
            version_name: version_info.name.clone(),
            old_loaders: current_loaders.clone(),
            new_loaders: vec!["mrpack".to_string()],
            mrpack_loaders_added: loaders_to_convert.clone(),
        };

        if !body.dry_run {
            // 7. 删除旧的 loaders
            sqlx::query!(
                "DELETE FROM loaders_versions WHERE version_id = $1",
                version_id
            )
            .execute(&mut *transaction)
            .await?;

            // 8. 添加 mrpack loader
            sqlx::query!(
                "INSERT INTO loaders_versions (version_id, loader_id) VALUES ($1, $2)",
                version_id,
                mrpack_loader_id
            )
            .execute(&mut *transaction)
            .await?;

            // 9. 删除旧的 mrpack_loaders 字段（如果存在）
            sqlx::query!(
                "DELETE FROM version_fields WHERE version_id = $1 AND field_id = $2",
                version_id,
                mrpack_loaders_field_id
            )
            .execute(&mut *transaction)
            .await?;

            // 10. 添加 mrpack_loaders 字段
            for loader in &loaders_to_convert {
                if let Some(enum_value_id) = enum_values.get(loader) {
                    sqlx::query!(
                        "INSERT INTO version_fields (version_id, field_id, enum_value)
                         VALUES ($1, $2, $3)",
                        version_id,
                        mrpack_loaders_field_id,
                        enum_value_id
                    )
                    .execute(&mut *transaction)
                    .await?;
                } else {
                    log::warn!(
                        "跳过 loader '{}' 的 mrpack_loaders 设置：枚举值不存在",
                        loader
                    );
                }
            }

            // 11. 清除版本缓存
            redis
                .connect()
                .await?
                .delete(
                    crate::database::models::version_item::VERSIONS_NAMESPACE,
                    version_id,
                )
                .await?;

            // 记录需要清除缓存的项目 ID
            project_ids_to_clear.insert(version_info.mod_id);

            log::info!(
                "已修复版本 {} ({}): {} -> mrpack, mrpack_loaders: {:?}",
                version_info.name,
                to_base62(version_id as u64),
                current_loaders.join(","),
                loaders_to_convert
            );
        }

        result.fixed_count += 1;
        result.details.push(detail);
    }

    if !body.dry_run {
        // 12. 提交事务
        transaction.commit().await?;

        // 13. 清除项目缓存
        for project_id in project_ids_to_clear {
            crate::database::models::Project::clear_cache(
                db_ids::ProjectId(project_id),
                None,
                Some(true),
                &redis,
            )
            .await?;
        }

        // 14. 触发重新索引（如果请求）
        if body.reindex && result.fixed_count > 0 {
            use crate::search::indexing::index_projects;
            let redis_ref = redis.get_ref();
            index_projects(
                pool.as_ref().clone(),
                redis_ref.clone(),
                &search_config,
            )
            .await?;
            result.reindexed = true;
            log::info!("已重新索引搜索");
        }

        log::info!(
            "修复 modpack loader 完成：修复 {} 个版本，跳过 {} 个版本",
            result.fixed_count,
            result.skipped_count
        );
    } else {
        transaction.rollback().await?;
        log::info!(
            "修复 modpack loader 预览：将修复 {} 个版本，跳过 {} 个版本",
            result.fixed_count,
            result.skipped_count
        );
    }

    Ok(HttpResponse::Ok().json(result))
}
