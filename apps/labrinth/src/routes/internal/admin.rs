use crate::auth::check_is_admin_from_headers;
use crate::auth::validate::get_user_record_from_bearer_token;
use crate::database::models::{
    self as db_models, notification_item::NotificationBuilder,
};
use crate::database::redis::RedisPool;
use crate::models::analytics::Download;
use crate::models::ids::ProjectId;
use crate::models::ids::base62_impl::{parse_base62, to_base62};
use crate::models::notifications::NotificationBody;
use crate::models::pats::Scopes;
use crate::models::teams::ProjectPermissions;
use crate::queue::analytics::AnalyticsQueue;
use crate::queue::incentive::IncentiveQueue;
use crate::queue::session::AuthQueue;
use crate::routes::ApiError;
use crate::search::SearchConfig;
use crate::util::date::get_current_tenths_of_ms;
use crate::util::guards::admin_key_guard;
use actix_web::{HttpRequest, HttpResponse, get, patch, post, web};
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
            .service(fix_modpack_loaders)
            .service(toggle_project_incentive)
            .service(list_incentive_projects)
            .service(incentive_stats)
            .service(list_incentive_applications)
            .service(review_incentive_application),
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
    incentive_queue: web::Data<Arc<IncentiveQueue>>,
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

    let user_id = user
        .and_then(|(scopes, x)| {
            if scopes.contains(Scopes::PERFORM_ANALYTICS) {
                Some(x.id.0 as u64)
            } else {
                None
            }
        })
        .unwrap_or(0);

    analytics_queue.add_download(Download {
        recorded: get_current_tenths_of_ms(),
        domain: url.host_str().unwrap_or_default().to_string(),
        site_path: url.path().to_string(),
        user_id,
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
    incentive_queue.add(
        project_id as u64,
        user_id,
        ip,
        chrono::Utc::now().timestamp(),
    );

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

// ==================== 项目激励准入开关 ====================

#[derive(Deserialize)]
pub struct ToggleIncentiveBody {
    pub enable: bool,
    pub notes: Option<String>,
    /// 关闭时是否一并把所有 pending 事件转为 voided（不再结算）
    #[serde(default)]
    pub void_pending: bool,
}

#[derive(Serialize)]
pub struct IncentiveProjectInfo {
    pub project_id: String,
    pub title: Option<String>,
    pub slug: Option<String>,
    /// 是否正式开通激励（incentive_enabled_projects 表是否有记录）
    pub enabled: bool,
    pub enabled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled_by: Option<String>,
    pub notes: Option<String>,
    pub lifetime_eff_downloads: i64,
    pub pending_amount: rust_decimal::Decimal,
    pub settled_amount: rust_decimal::Decimal,
    pub voided_amount: rust_decimal::Decimal,
    pub last_event_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[patch("projects/{id}/incentive")]
pub async fn toggle_project_incentive(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<ToggleIncentiveBody>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        None,
    )
    .await?;

    let project_id = parse_base62(&info.0)
        .map_err(|_| ApiError::InvalidInput("无效的项目 ID".to_string()))?
        as i64;

    let project_item = db_models::Project::get_id(
        crate::database::models::ids::ProjectId(project_id),
        pool.as_ref(),
        &redis,
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    let was_enabled = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM incentive_enabled_projects WHERE project_id = $1) AS "exists!""#,
        project_id,
    )
    .fetch_one(pool.as_ref())
    .await?;

    if body.enable {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "
            INSERT INTO incentive_enabled_projects (project_id, enabled_by, notes)
            VALUES ($1, $2, $3)
            ON CONFLICT (project_id) DO UPDATE SET
                enabled_by = EXCLUDED.enabled_by,
                notes = EXCLUDED.notes
            ",
            project_id,
            user.id.0 as i64,
            body.notes.as_deref(),
        )
        .execute(&mut *tx)
        .await?;

        if !was_enabled {
            let text = match body.notes.as_deref() {
                Some(notes) if !notes.trim().is_empty() => format!(
                    "管理员已为该资源开通创作者激励。备注：{}",
                    notes.trim()
                ),
                _ => "管理员已为该资源开通创作者激励，激励将开始累计。"
                    .to_string(),
            };
            insert_project_incentive_notification(
                &mut tx,
                &redis,
                project_id,
                "incentive_project_enabled",
                "[创作者激励] 已开通：",
                text,
            )
            .await?;
        }

        tx.commit().await?;

        db_models::Project::clear_cache(
            project_item.inner.id,
            project_item.inner.slug.clone(),
            None,
            &redis,
        )
        .await?;

        let _ = crate::queue::incentive::audit_log(
            pool.as_ref(),
            Some(user.id.0 as i64),
            "enable_project",
            "project",
            project_id,
            body.notes.as_ref().map(|n| serde_json::json!({"notes": n})),
        )
        .await;
    } else {
        let mut tx = pool.begin().await?;
        let mut voided_count: i64 = 0;

        let delete_result = sqlx::query!(
            "DELETE FROM incentive_enabled_projects WHERE project_id = $1",
            project_id,
        )
        .execute(&mut *tx)
        .await?;

        if delete_result.rows_affected() > 0 {
            if body.void_pending {
                voided_count =
                    void_project_pending_in_tx(&mut tx, project_id).await?;
            }

            if let Some(thread) = sqlx::query!(
                r#"
                SELECT thread_id AS "thread_id!"
                FROM incentive_applications
                WHERE project_id = $1 AND thread_id IS NOT NULL
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                project_id,
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                let body_text = match body.notes.as_deref() {
                    Some(notes) if !notes.trim().is_empty() => format!(
                        "管理员关闭了该资源的创作者激励。关闭理由：{}\n\n作者可以根据要求调整后重新提交申请。",
                        notes.trim()
                    ),
                    _ => {
                        "管理员关闭了该资源的创作者激励。作者可以根据要求调整后重新提交申请。"
                            .to_string()
                    }
                };

                crate::database::models::thread_item::ThreadMessageBuilder {
                    author_id: None,
                    body: crate::models::threads::MessageBody::Text {
                        body: body_text,
                        private: false,
                        replying_to: None,
                        associated_images: vec![],
                    },
                    thread_id: crate::database::models::ids::ThreadId(
                        thread.thread_id,
                    ),
                    hide_identity: false,
                }
                .insert(&mut tx)
                .await?;
            }

            let text = match body.notes.as_deref() {
                Some(notes) if !notes.trim().is_empty() => format!(
                    "管理员已关闭该资源的创作者激励。关闭理由：{}{}",
                    notes.trim(),
                    if body.void_pending {
                        if voided_count > 0 {
                            " 本次关闭同时作废当前待结算激励。"
                        } else {
                            " 当前没有待结算激励需要作废。"
                        }
                    } else {
                        ""
                    }
                ),
                _ => format!(
                    "管理员已关闭该资源的创作者激励。{}",
                    if body.void_pending {
                        if voided_count > 0 {
                            "本次关闭同时作废当前待结算激励。"
                        } else {
                            "当前没有待结算激励需要作废。"
                        }
                    } else {
                        "作者可以根据要求调整后重新提交申请。"
                    }
                ),
            };
            insert_project_incentive_notification(
                &mut tx,
                &redis,
                project_id,
                "incentive_project_disabled",
                "[创作者激励] 已关闭：",
                text,
            )
            .await?;
        }

        tx.commit().await?;

        db_models::Project::clear_cache(
            project_item.inner.id,
            project_item.inner.slug.clone(),
            None,
            &redis,
        )
        .await?;

        let _ = crate::queue::incentive::audit_log(
            pool.as_ref(),
            Some(user.id.0 as i64),
            "disable_project",
            "project",
            project_id,
            Some(serde_json::json!({
                "void_pending": body.void_pending,
                "voided_event_count": voided_count,
                "notes": body.notes,
            })),
        )
        .await;
    }

    Ok(HttpResponse::NoContent().finish())
}

async fn insert_project_incentive_notification(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    redis: &RedisPool,
    project_id: i64,
    notification_type: &str,
    name_prefix: &str,
    text: String,
) -> Result<(), ApiError> {
    let project = sqlx::query!(
        "
        SELECT name, team_id
        FROM mods
        WHERE id = $1
        ",
        project_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    let view_payouts = ProjectPermissions::VIEW_PAYOUTS.bits() as i64;
    let edit_details = ProjectPermissions::EDIT_DETAILS.bits() as i64;
    let recipients = sqlx::query!(
        "
        SELECT DISTINCT user_id
        FROM team_members
        WHERE team_id = $1
          AND accepted = TRUE
          AND (
              payouts_split > 0
              OR is_owner = TRUE
              OR (permissions & $2) <> 0
              OR (permissions & $3) <> 0
          )
        ",
        project.team_id,
        view_payouts,
        edit_details,
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| crate::database::models::ids::UserId(row.user_id))
    .collect::<Vec<_>>();

    if recipients.is_empty() {
        return Ok(());
    }

    let project_b62 = to_base62(project_id as u64);
    NotificationBuilder {
        body: NotificationBody::LegacyMarkdown {
            notification_type: Some(notification_type.to_string()),
            name: format!("{name_prefix}{}", project.name),
            text,
            link: format!("/project/{project_b62}/settings/incentive"),
            actions: vec![],
        },
    }
    .insert_many(recipients, tx, redis)
    .await?;

    Ok(())
}

async fn void_project_pending_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: i64,
) -> Result<i64, sqlx::Error> {
    let voided_amount = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(payout_amount), 0)::numeric AS "voided_amount!"
        FROM incentive_download_events
        WHERE project_id = $1 AND status = 'pending'
        "#,
        project_id,
    )
    .fetch_one(&mut **tx)
    .await?;

    let res = sqlx::query!(
        "
        UPDATE incentive_download_events
        SET status = 'voided', settled_at = NOW()
        WHERE project_id = $1 AND status = 'pending'
        ",
        project_id,
    )
    .execute(&mut **tx)
    .await?;

    if res.rows_affected() > 0 {
        sqlx::query!(
            "
            UPDATE incentive_project_counters
            SET pending_amount = 0,
                voided_amount = voided_amount + $2,
                updated_at = NOW()
            WHERE project_id = $1
            ",
            project_id,
            voided_amount,
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(res.rows_affected() as i64)
}

#[get("incentive/projects")]
pub async fn list_incentive_projects(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(&req, &**pool, &redis, &session_queue, None)
        .await?;

    // 显示所有有数据的项目：要么正式开通（enabled），要么有累计有效下载（counter 有记录）
    // 排序：待结算金额降序优先，其次累计下载降序
    let rows = sqlx::query!(
        r#"
        SELECT
            COALESCE(e.project_id, c.project_id) AS "project_id!",
            m.name AS "title?",
            m.slug AS "slug?",
            (e.project_id IS NOT NULL) AS "enabled!",
            e.enabled_at AS "enabled_at?",
            e.enabled_by AS "enabled_by?",
            e.notes AS "notes?",
            c.lifetime_eff_downloads AS "lifetime_eff_downloads?",
            c.pending_amount AS "pending_amount?",
            c.settled_amount AS "settled_amount?",
            c.voided_amount AS "voided_amount?",
            c.last_event_at AS "last_event_at?"
        FROM incentive_project_counters c
        FULL OUTER JOIN incentive_enabled_projects e ON e.project_id = c.project_id
        LEFT JOIN mods m ON m.id = COALESCE(e.project_id, c.project_id)
        WHERE COALESCE(c.lifetime_eff_downloads, 0) > 0 OR e.project_id IS NOT NULL
        ORDER BY COALESCE(c.pending_amount, 0) DESC,
                 COALESCE(c.lifetime_eff_downloads, 0) DESC,
                 e.enabled_at DESC NULLS LAST
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let result: Vec<IncentiveProjectInfo> = rows
        .into_iter()
        .map(|r| IncentiveProjectInfo {
            project_id: to_base62(r.project_id as u64),
            title: r.title,
            slug: r.slug,
            enabled: r.enabled,
            enabled_at: r.enabled_at,
            enabled_by: r.enabled_by.map(|v| to_base62(v as u64)),
            notes: r.notes,
            lifetime_eff_downloads: r.lifetime_eff_downloads.unwrap_or(0),
            pending_amount: r.pending_amount.unwrap_or_default(),
            settled_amount: r.settled_amount.unwrap_or_default(),
            voided_amount: r.voided_amount.unwrap_or_default(),
            last_event_at: r.last_event_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(result))
}

// ==================== 激励申请审核（版主侧） ====================

#[derive(Serialize)]
pub struct AdminApplicationItem {
    pub id: i64,
    pub project_id: String,
    pub project_title: Option<String>,
    pub project_slug: Option<String>,
    pub applicant_user_id: String,
    pub applicant_username: Option<String>,
    pub reason: Option<String>,
    pub status: String,
    pub review_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub thread_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ListApplicationsQuery {
    /// 筛选：pending / approved / rejected / withdrawn / all
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[get("incentive/applications")]
pub async fn list_incentive_applications(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    query: web::Query<ListApplicationsQuery>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(&req, &**pool, &redis, &session_queue, None)
        .await?;

    let status_filter = query.status.as_deref().unwrap_or("pending");
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let status_param = if status_filter == "all" {
        None
    } else {
        Some(status_filter)
    };

    let rows = sqlx::query!(
        r#"
        SELECT a.id, a.project_id,
               m.name AS "project_title?",
               m.slug AS "project_slug?",
               a.applicant_user_id,
               u.username AS "applicant_username?",
               a.reason, a.status, a.review_notes,
               a.reviewed_by, a.reviewed_at, a.created_at, a.thread_id
        FROM incentive_applications a
        LEFT JOIN mods m ON m.id = a.project_id
        LEFT JOIN users u ON u.id = a.applicant_user_id
        WHERE ($3::text IS NULL OR a.status = $3)
        ORDER BY a.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset,
        status_param,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let result: Vec<AdminApplicationItem> = rows
        .into_iter()
        .map(|r| AdminApplicationItem {
            id: r.id,
            project_id: to_base62(r.project_id as u64),
            project_title: r.project_title,
            project_slug: r.project_slug,
            applicant_user_id: to_base62(r.applicant_user_id as u64),
            applicant_username: r.applicant_username,
            reason: r.reason,
            status: r.status,
            review_notes: r.review_notes,
            reviewed_by: r.reviewed_by.map(|v| to_base62(v as u64)),
            reviewed_at: r.reviewed_at,
            created_at: r.created_at,
            thread_id: r.thread_id.map(|v| to_base62(v as u64)),
        })
        .collect();

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Deserialize)]
pub struct ReviewApplicationBody {
    /// approved / rejected
    pub status: String,
    pub review_notes: Option<String>,
}

#[patch("incentive/applications/{id}")]
pub async fn review_incentive_application(
    req: HttpRequest,
    info: web::Path<(i64,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<ReviewApplicationBody>,
) -> Result<HttpResponse, ApiError> {
    let mod_user = check_is_admin_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        None,
    )
    .await?;

    let target_status = match body.status.as_str() {
        "approved" => "approved",
        "rejected" => "rejected",
        _ => {
            return Err(ApiError::InvalidInput(
                "status 必须是 approved 或 rejected".to_string(),
            ));
        }
    };

    let appl_id = info.0;
    let mut tx = pool.begin().await?;

    let pending = sqlx::query!(
        "
        SELECT project_id, applicant_user_id, thread_id
        FROM incentive_applications
        WHERE id = $1 AND status = 'pending'
        FOR UPDATE
        ",
        appl_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::InvalidInput("申请不存在或已审核".to_string()))?;

    let project_item = if target_status == "approved" {
        Some(
            db_models::Project::get_id(
                crate::database::models::ids::ProjectId(pending.project_id),
                pool.as_ref(),
                &redis,
            )
            .await?
            .ok_or(ApiError::NotFound)?,
        )
    } else {
        None
    };

    sqlx::query!(
        "
        UPDATE incentive_applications
        SET status = $2, review_notes = $3, reviewed_by = $4, reviewed_at = NOW()
        WHERE id = $1
        ",
        appl_id,
        target_status,
        body.review_notes.as_deref(),
        mod_user.id.0 as i64,
    )
    .execute(&mut *tx)
    .await?;

    if target_status == "approved" {
        sqlx::query!(
            "
            INSERT INTO incentive_enabled_projects (project_id, enabled_by, notes)
            VALUES ($1, $2, $3)
            ON CONFLICT (project_id) DO UPDATE SET
                enabled_by = EXCLUDED.enabled_by,
                notes = EXCLUDED.notes,
                enabled_at = NOW()
            ",
            pending.project_id,
            mod_user.id.0 as i64,
            format!("approved appl#{}", appl_id),
        )
        .execute(&mut *tx)
        .await?;
    }

    // thread 写系统消息（author_id = NULL）
    if let Some(tid) = pending.thread_id {
        let sys_msg = if target_status == "approved" {
            match body.review_notes.as_deref() {
                Some(n) if !n.trim().is_empty() => {
                    format!("申请已通过。审核备注：{n}")
                }
                _ => "申请已通过".to_string(),
            }
        } else {
            match body.review_notes.as_deref() {
                Some(n) if !n.trim().is_empty() => {
                    format!("申请被拒绝。原因：{n}")
                }
                _ => "申请被拒绝".to_string(),
            }
        };

        crate::database::models::thread_item::ThreadMessageBuilder {
            author_id: None,
            body: crate::models::threads::MessageBody::Text {
                body: sys_msg,
                private: false,
                replying_to: None,
                associated_images: vec![],
            },
            thread_id: crate::database::models::ids::ThreadId(tid),
            hide_identity: false,
        }
        .insert(&mut tx)
        .await?;
    }

    // 通知申请人
    {
        let project_title = sqlx::query!(
            "SELECT name FROM mods WHERE id = $1",
            pending.project_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|r| r.name);
        let title = project_title.unwrap_or_else(|| "项目".to_string());

        let (name, text) = if target_status == "approved" {
            (
                format!("[激励申请] 已通过：{title}"),
                "你的创作者激励申请已通过，激励将开始累计。".to_string(),
            )
        } else {
            (
                format!("[激励申请] 已拒绝：{title}"),
                body.review_notes
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "你的激励申请未通过审核。".to_string()),
            )
        };

        let project_b62 = crate::models::ids::base62_impl::to_base62(
            pending.project_id as u64,
        );
        crate::database::models::notification_item::NotificationBuilder {
            body:
                crate::models::notifications::NotificationBody::LegacyMarkdown {
                    notification_type: Some(
                        "incentive_application_reviewed".to_string(),
                    ),
                    name,
                    text,
                    link: format!("/project/{project_b62}/settings/incentive"),
                    actions: vec![],
                },
        }
        .insert(
            crate::database::models::ids::UserId(pending.applicant_user_id),
            &mut tx,
            &redis,
        )
        .await?;
    }

    tx.commit().await?;

    if let Some(project_item) = project_item {
        db_models::Project::clear_cache(
            project_item.inner.id,
            project_item.inner.slug,
            None,
            &redis,
        )
        .await?;
    }

    let _ = crate::queue::incentive::audit_log(
        pool.as_ref(),
        Some(mod_user.id.0 as i64),
        if target_status == "approved" {
            "review_approve"
        } else {
            "review_reject"
        },
        "application",
        appl_id,
        Some(serde_json::json!({
            "project_id": pending.project_id,
            "applicant_user_id": pending.applicant_user_id,
            "review_notes": body.review_notes,
        })),
    )
    .await;

    crate::routes::internal::moderation::clear_pending_counts_cache(&redis)
        .await;

    Ok(HttpResponse::NoContent().finish())
}

// ==================== 激励全局统计（admin 后台） ====================

#[derive(Serialize)]
pub struct IncentiveStats {
    pub total_projects: i64,
    pub total_enabled: i64,
    pub total_eff_downloads: i64,
    pub total_pending: rust_decimal::Decimal,
    pub total_settled: rust_decimal::Decimal,
    pub total_withdrawable: rust_decimal::Decimal,
    pub total_voided: rust_decimal::Decimal,
    pub today_eff_downloads: i64,
    pub today_amount: rust_decimal::Decimal,
    pub today_active_projects: i64,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub tier_distribution: Vec<TierBucket>,
    pub top_projects: Vec<TopProjectItem>,
}

#[derive(Serialize)]
pub struct DailyTrendPoint {
    pub date: chrono::NaiveDate,
    pub effective_downloads: i64,
    pub daily_amount: rust_decimal::Decimal,
    pub active_projects: i64,
}

#[derive(Serialize)]
pub struct TierBucket {
    pub tier: String,
    pub project_count: i64,
    pub total_downloads: i64,
}

#[derive(Serialize)]
pub struct TopProjectItem {
    pub project_id: String,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub lifetime_eff_downloads: i64,
    pub pending_amount: rust_decimal::Decimal,
}

#[get("incentive/stats")]
pub async fn incentive_stats(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_admin_from_headers(&req, &**pool, &redis, &session_queue, None)
        .await?;

    // 1. 全局汇总；待提现金额沿用 payout/balance 的 available 口径，
    // 按用户截到分并将负余额归零，避免负余额抵消其他用户的正余额。
    // 同时排除承接注销用户历史收益的 Ghost 账号。
    let totals = sqlx::query!(
        r#"
        WITH earnings AS (
            SELECT
                user_id,
                COALESCE(SUM(amount), 0)::numeric AS earned
            FROM payouts_values
            WHERE date_available <= NOW()
              AND user_id <> $1
            GROUP BY user_id
        ), withdrawn AS (
            SELECT
                user_id,
                COALESCE(SUM(amount), 0)::numeric AS amount,
                COALESCE(SUM(
                    CASE
                        WHEN method = 'yunzhanghu_alipay' THEN 0
                        ELSE COALESCE(fee, 0)
                    END
                ), 0)::numeric AS old_channel_fees
            FROM payouts
            WHERE status IN ('success', 'in-transit')
              AND user_id <> $1
            GROUP BY user_id
        ), withdrawable_users AS (
            SELECT
                e.user_id,
                TRUNC(GREATEST(
                    ROUND(e.earned, 16)
                        - ROUND(COALESCE(w.amount, 0), 16)
                        - ROUND(COALESCE(w.old_channel_fees, 0), 16),
                    0
                ), 2) AS amount
            FROM earnings e
            LEFT JOIN withdrawn w ON w.user_id = e.user_id
        )
        SELECT
            COUNT(*) AS "total_projects!",
            COALESCE(SUM(lifetime_eff_downloads), 0)::bigint AS "total_eff_downloads!",
            COALESCE(SUM(pending_amount), 0)::numeric AS "total_pending!",
            COALESCE(SUM(settled_amount), 0)::numeric AS "total_settled!",
            COALESCE((
                SELECT SUM(amount)
                FROM withdrawable_users
            ), 0)::numeric AS "total_withdrawable!",
            COALESCE(SUM(voided_amount), 0)::numeric AS "total_voided!"
        FROM incentive_project_counters
        "#,
        crate::models::users::DELETED_USER.0 as i64,
    )
    .fetch_one(pool.as_ref())
    .await?;

    let total_enabled: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM incentive_enabled_projects")
            .fetch_one(pool.as_ref())
            .await?
            .unwrap_or(0);

    // 2. 今日数据
    let today = sqlx::query!(
        r#"
        SELECT
            COUNT(*)::bigint AS "eff_downloads!",
            COALESCE(SUM(payout_amount), 0)::numeric AS "amount!",
            COUNT(DISTINCT project_id)::bigint AS "active_projects!"
        FROM incentive_download_events
        WHERE recorded_at >= CURRENT_DATE
          AND recorded_at < CURRENT_DATE + INTERVAL '1 day'
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;

    // 3. 30 天每日趋势（按应用时区分日聚合，PG session 已设为 Asia/Shanghai）
    let daily_rows = sqlx::query!(
        r#"
        SELECT
            DATE(recorded_at) AS "date!",
            COUNT(*)::bigint AS "effective_downloads!",
            COALESCE(SUM(payout_amount), 0)::numeric AS "daily_amount!",
            COUNT(DISTINCT project_id)::bigint AS "active_projects!"
        FROM incentive_download_events
        WHERE recorded_at >= NOW() - INTERVAL '30 days'
        GROUP BY DATE(recorded_at)
        ORDER BY DATE(recorded_at)
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;
    let daily_trend: Vec<DailyTrendPoint> = daily_rows
        .into_iter()
        .map(|r| DailyTrendPoint {
            date: r.date,
            effective_downloads: r.effective_downloads,
            daily_amount: r.daily_amount,
            active_projects: r.active_projects,
        })
        .collect();

    // 4. 档位分布
    let tier_rows = sqlx::query!(
        r#"
        SELECT
            CASE
                WHEN lifetime_eff_downloads < 100 THEN '01_<100'
                WHEN lifetime_eff_downloads < 1000 THEN '02_100-1K'
                WHEN lifetime_eff_downloads < 10000 THEN '03_1K-1W'
                WHEN lifetime_eff_downloads < 100000 THEN '04_1W-10W'
                ELSE '05_>10W'
            END AS "tier!",
            COUNT(*)::bigint AS "project_count!",
            COALESCE(SUM(lifetime_eff_downloads), 0)::bigint AS "total_downloads!"
        FROM incentive_project_counters
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;
    let tier_distribution: Vec<TierBucket> = tier_rows
        .into_iter()
        .map(|r| TierBucket {
            tier: r.tier,
            project_count: r.project_count,
            total_downloads: r.total_downloads,
        })
        .collect();

    // 5. Top 20 项目（按待结算）
    let top_rows = sqlx::query!(
        r#"
        SELECT c.project_id, m.name AS "title?", m.slug AS "slug?",
               c.lifetime_eff_downloads, c.pending_amount
        FROM incentive_project_counters c
        LEFT JOIN mods m ON m.id = c.project_id
        WHERE c.lifetime_eff_downloads > 0
        ORDER BY c.pending_amount DESC, c.lifetime_eff_downloads DESC
        LIMIT 20
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let top_projects: Vec<TopProjectItem> = top_rows
        .into_iter()
        .map(|r| TopProjectItem {
            project_id: to_base62(r.project_id as u64),
            title: r.title,
            slug: r.slug,
            lifetime_eff_downloads: r.lifetime_eff_downloads,
            pending_amount: r.pending_amount,
        })
        .collect();

    Ok(HttpResponse::Ok().json(IncentiveStats {
        total_projects: totals.total_projects,
        total_enabled,
        total_eff_downloads: totals.total_eff_downloads,
        total_pending: totals.total_pending,
        total_settled: totals.total_settled,
        total_withdrawable: totals.total_withdrawable,
        total_voided: totals.total_voided,
        today_eff_downloads: today.eff_downloads,
        today_amount: today.amount,
        today_active_projects: today.active_projects,
        daily_trend,
        tier_distribution,
        top_projects,
    }))
}
