use super::ApiError;
use crate::database;
use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::to_base62;
use crate::models::ids::random_base62;
use crate::models::projects::ProjectStatus;
use crate::queue::moderation::{ApprovalType, IdentifiedFile, MissingMetadata};
use crate::queue::session::AuthQueue;
use crate::{auth::check_is_moderator_from_headers, models::pats::Scopes};
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("moderation/projects", web::get().to(get_projects));
    cfg.route("moderation/project/{id}", web::get().to(get_project_meta));
    cfg.route("moderation/project", web::post().to(set_project_meta));
    cfg.route(
        "moderation/pending-counts",
        web::get().to(get_pending_counts),
    );
    cfg.route(
        "moderation/translation-tracking-status",
        web::get().to(get_translation_tracking_status),
    );
    // 用户资料审核路由
    cfg.route(
        "moderation/profile-reviews",
        web::get().to(crate::routes::v3::profile_reviews::list_reviews),
    );
    cfg.route(
        "moderation/profile-reviews/approve-all",
        web::post().to(crate::routes::v3::profile_reviews::approve_all_pending),
    );
    cfg.route(
        "moderation/profile-reviews/{id}/approve",
        web::post().to(crate::routes::v3::profile_reviews::approve_review),
    );
    cfg.route(
        "moderation/profile-reviews/{id}/reject",
        web::post().to(crate::routes::v3::profile_reviews::reject_review),
    );
    // 图片内容审核路由
    cfg.route(
        "moderation/image-reviews",
        web::get().to(crate::routes::v3::image_reviews::list_image_reviews),
    );
    cfg.route(
        "moderation/image-reviews/{id}/approve",
        web::post().to(crate::routes::v3::image_reviews::approve_image_review),
    );
    cfg.route(
        "moderation/image-reviews/{id}/reject",
        web::post().to(crate::routes::v3::image_reviews::reject_image_review),
    );
    // 数据分析路由
    cfg.route(
        "moderation/analytics/registrations",
        web::get().to(get_analytics_registrations),
    );
    cfg.route(
        "moderation/analytics/downloads",
        web::get().to(get_analytics_downloads),
    );
    cfg.route(
        "moderation/analytics/top-projects",
        web::get().to(get_analytics_top_projects),
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

pub async fn get_projects(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    count: web::Query<ResultCount>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    use futures::stream::TryStreamExt;

    let project_ids = sqlx::query!(
        "
        SELECT id FROM mods
        WHERE status = $1
        ORDER BY queued ASC
        LIMIT $2;
        ",
        ProjectStatus::Processing.as_str(),
        count.count as i64
    )
    .fetch(&**pool)
    .map_ok(|m| database::models::ProjectId(m.id))
    .try_collect::<Vec<database::models::ProjectId>>()
    .await?;

    let projects: Vec<_> =
        database::Project::get_many_ids(&project_ids, &**pool, &redis)
            .await?
            .into_iter()
            .map(crate::models::projects::Project::from)
            .collect();

    Ok(HttpResponse::Ok().json(projects))
}

pub async fn get_project_meta(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    info: web::Path<(String,)>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let project_id = info.into_inner().0;
    let project =
        database::models::Project::get(&project_id, &**pool, &redis).await?;

    if let Some(project) = project {
        let rows = sqlx::query!(
            "
            SELECT
            f.metadata, v.id version_id
            FROM versions v
            INNER JOIN files f ON f.version_id = v.id
            WHERE v.mod_id = $1
            ",
            project.inner.id.0
        )
        .fetch_all(&**pool)
        .await?;

        let mut merged = MissingMetadata {
            identified: HashMap::new(),
            flame_files: HashMap::new(),
            unknown_files: HashMap::new(),
        };

        let mut check_hashes = Vec::new();
        let mut check_flames = Vec::new();

        for row in rows {
            if let Some(metadata) = row
                .metadata
                .and_then(|x| serde_json::from_value::<MissingMetadata>(x).ok())
            {
                merged.identified.extend(metadata.identified);
                merged.flame_files.extend(metadata.flame_files);
                merged.unknown_files.extend(metadata.unknown_files);

                check_hashes.extend(merged.flame_files.keys().cloned());
                check_hashes.extend(merged.unknown_files.keys().cloned());
                check_flames
                    .extend(merged.flame_files.values().map(|x| x.id as i32));
            }
        }

        let rows = sqlx::query!(
            "
            SELECT encode(mef.sha1, 'escape') sha1, mel.status status
            FROM moderation_external_files mef
            INNER JOIN moderation_external_licenses mel ON mef.external_license_id = mel.id
            WHERE mef.sha1 = ANY($1)
            ",
            &check_hashes
                .iter()
                .map(|x| x.as_bytes().to_vec())
                .collect::<Vec<_>>()
        )
        .fetch_all(&**pool)
        .await?;

        for row in rows {
            if let Some(sha1) = row.sha1 {
                if let Some(val) = merged.flame_files.remove(&sha1) {
                    merged.identified.insert(
                        sha1,
                        IdentifiedFile {
                            file_name: val.file_name,
                            status: ApprovalType::from_string(&row.status)
                                .unwrap_or(ApprovalType::Unidentified),
                        },
                    );
                } else if let Some(val) = merged.unknown_files.remove(&sha1) {
                    merged.identified.insert(
                        sha1,
                        IdentifiedFile {
                            file_name: val,
                            status: ApprovalType::from_string(&row.status)
                                .unwrap_or(ApprovalType::Unidentified),
                        },
                    );
                }
            }
        }

        let rows = sqlx::query!(
            "
            SELECT mel.id, mel.flame_project_id, mel.status status
            FROM moderation_external_licenses mel
            WHERE mel.flame_project_id = ANY($1)
            ",
            &check_flames,
        )
        .fetch_all(&**pool)
        .await?;

        for row in rows {
            if let Some(sha1) = merged
                .flame_files
                .iter()
                .find(|x| Some(x.1.id as i32) == row.flame_project_id)
                .map(|x| x.0.clone())
                && let Some(val) = merged.flame_files.remove(&sha1)
            {
                merged.identified.insert(
                    sha1,
                    IdentifiedFile {
                        file_name: val.file_name.clone(),
                        status: ApprovalType::from_string(&row.status)
                            .unwrap_or(ApprovalType::Unidentified),
                    },
                );
            }
        }

        Ok(HttpResponse::Ok().json(merged))
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Judgement {
    Flame {
        id: i32,
        status: ApprovalType,
        link: String,
        title: String,
    },
    Unknown {
        status: ApprovalType,
        proof: Option<String>,
        link: Option<String>,
        title: Option<String>,
    },
}

pub async fn set_project_meta(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    judgements: web::Json<HashMap<String, Judgement>>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let mut transaction = pool.begin().await?;

    let mut ids = Vec::new();
    let mut titles = Vec::new();
    let mut statuses = Vec::new();
    let mut links = Vec::new();
    let mut proofs = Vec::new();
    let mut flame_ids = Vec::new();

    let mut file_hashes = Vec::new();

    for (hash, judgement) in judgements.0 {
        let id = random_base62(8);

        let (title, status, link, proof, flame_id) = match judgement {
            Judgement::Flame {
                id,
                status,
                link,
                title,
            } => (
                Some(title),
                status,
                Some(link),
                Some("See Flame page/license for permission".to_string()),
                Some(id),
            ),
            Judgement::Unknown {
                status,
                proof,
                link,
                title,
            } => (title, status, link, proof, None),
        };

        ids.push(id as i64);
        titles.push(title);
        statuses.push(status.as_str());
        links.push(link);
        proofs.push(proof);
        flame_ids.push(flame_id);
        file_hashes.push(hash);
    }

    sqlx::query(
    "
        INSERT INTO moderation_external_licenses (id, title, status, link, proof, flame_project_id)
        SELECT * FROM UNNEST ($1::bigint[], $2::varchar[], $3::varchar[], $4::varchar[], $5::varchar[], $6::integer[])
        "
    )
        .bind(&ids[..])
        .bind(&titles[..])
        .bind(&statuses[..])
        .bind(&links[..])
        .bind(&proofs[..])
        .bind(&flame_ids[..])
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        "
            INSERT INTO moderation_external_files (sha1, external_license_id)
            SELECT * FROM UNNEST ($1::bytea[], $2::bigint[])
            ON CONFLICT (sha1)
            DO NOTHING
            ",
    )
    .bind(&file_hashes[..])
    .bind(&ids[..])
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

/// 汉化追踪状态项
#[derive(serde::Serialize)]
pub struct TranslationTrackingItem {
    /// 项目 ID
    pub project_id: String,
    /// 项目 slug
    pub project_slug: Option<String>,
    /// 项目名称
    pub project_name: String,
    /// 项目图标
    pub project_icon: Option<String>,
    /// 汉化包 slug
    pub translation_pack_slug: Option<String>,
    /// 最新版本 ID
    pub latest_version_id: Option<String>,
    /// 最新版本号
    pub latest_version_number: Option<String>,
    /// 最新版本发布时间
    pub latest_version_published: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否有已批准的汉化绑定
    pub has_approved_translation: bool,
    /// 已批准的汉化版本 ID
    pub approved_translation_version_id: Option<String>,
    /// 已批准的汉化版本号
    pub approved_translation_version_number: Option<String>,
    /// 版本发布后经过的秒数
    pub seconds_since_published: Option<i64>,
}

/// 汉化追踪状态响应
#[derive(serde::Serialize)]
pub struct TranslationTrackingStatusResponse {
    /// 追踪项目列表
    pub items: Vec<TranslationTrackingItem>,
    /// 总数
    pub total: i64,
    /// 查询时间
    pub queried_at: chrono::DateTime<chrono::Utc>,
}

/// 获取所有开启汉化追踪的项目的状态
pub async fn get_translation_tracking_status(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    use chrono::Utc;
    use futures::stream::TryStreamExt;

    // 查询所有开启了汉化追踪的项目及其最新版本和汉化状态
    let rows = sqlx::query!(
        r#"
        WITH tracked_projects AS (
            -- 获取所有开启汉化追踪的项目
            SELECT
                m.id,
                m.slug,
                m.name,
                m.icon_url,
                m.translation_tracker
            FROM mods m
            WHERE m.translation_tracking = true
            AND m.status = 'approved'
        ),
        latest_versions AS (
            -- 获取每个项目的最新版本
            SELECT DISTINCT ON (v.mod_id)
                v.mod_id,
                v.id as version_id,
                v.version_number,
                v.date_published
            FROM versions v
            WHERE v.mod_id IN (SELECT id FROM tracked_projects)
            AND v.status = 'listed'
            ORDER BY v.mod_id, v.date_published DESC
        ),
        approved_translations AS (
            -- 获取每个版本的已批准汉化绑定
            SELECT DISTINCT ON (vlv.joining_version_id)
                vlv.joining_version_id as original_version_id,
                vlv.version_id as translation_version_id,
                tv.version_number as translation_version_number
            FROM version_link_version vlv
            INNER JOIN versions tv ON tv.id = vlv.version_id
            WHERE vlv.approval_status = 'approved'
            AND vlv.link_type = 'translation'
            ORDER BY vlv.joining_version_id, vlv.created_at DESC
        )
        SELECT
            tp.id as project_id,
            tp.slug as project_slug,
            tp.name as project_name,
            tp.icon_url as project_icon,
            tp.translation_tracker,
            lv.version_id as "latest_version_id?",
            lv.version_number as "latest_version_number?",
            lv.date_published as "latest_version_published?",
            at.translation_version_id as "translation_version_id?",
            at.translation_version_number as "translation_version_number?"
        FROM tracked_projects tp
        LEFT JOIN latest_versions lv ON lv.mod_id = tp.id
        LEFT JOIN approved_translations at ON at.original_version_id = lv.version_id
        ORDER BY lv.date_published DESC NULLS LAST
        "#
    )
    .fetch(&**pool)
    .try_collect::<Vec<_>>()
    .await?;

    let now = Utc::now();
    let items: Vec<TranslationTrackingItem> = rows
        .into_iter()
        .map(|row| {
            let seconds_since_published = row
                .latest_version_published
                .map(|pub_time| (now - pub_time).num_seconds());

            TranslationTrackingItem {
                project_id: crate::models::ids::ProjectId::from(
                    database::models::ids::ProjectId(row.project_id),
                )
                .to_string(),
                project_slug: row.project_slug,
                project_name: row.project_name,
                project_icon: row.project_icon,
                translation_pack_slug: row.translation_tracker,
                latest_version_id: row.latest_version_id.map(|id| {
                    crate::models::ids::VersionId::from(
                        database::models::ids::VersionId(id),
                    )
                    .to_string()
                }),
                latest_version_number: row.latest_version_number,
                latest_version_published: row.latest_version_published,
                has_approved_translation: row.translation_version_id.is_some(),
                approved_translation_version_id: row
                    .translation_version_id
                    .map(|id| {
                        crate::models::ids::VersionId::from(
                            database::models::ids::VersionId(id),
                        )
                        .to_string()
                    }),
                approved_translation_version_number: row
                    .translation_version_number,
                seconds_since_published,
            }
        })
        .collect();

    let total = items.len() as i64;

    Ok(HttpResponse::Ok().json(TranslationTrackingStatusResponse {
        items,
        total,
        queried_at: now,
    }))
}

// ==================== 待处理数量统计 ====================

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModerationPendingCounts {
    pub projects: i64,
    pub reports: i64,
    pub appeals: i64,
    pub profile_reviews: i64,
    pub image_reviews: i64,
    pub creator_applications: i64,
    #[serde(default)]
    pub incentive_applications: i64,
    #[serde(default)]
    pub payout_transfers: i64,
}

pub(crate) const PENDING_COUNTS_NAMESPACE: &str = "moderation_pending_counts";
const PENDING_COUNTS_ADMIN_CACHE_KEY: &str = "admin";
const PENDING_COUNTS_MODERATOR_CACHE_KEY: &str = "moderator";
const PENDING_COUNTS_TTL: i64 = 180; // 3 分钟

/// 获取各审核类别的待处理数量
///
/// GET /_internal/moderation/pending-counts
pub async fn get_pending_counts(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    let user = check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let cache_key = if user.role.is_admin() {
        PENDING_COUNTS_ADMIN_CACHE_KEY
    } else {
        PENDING_COUNTS_MODERATOR_CACHE_KEY
    };

    // 查 Redis 缓存
    let mut redis_conn = redis.connect().await?;
    if let Some(cached) = redis_conn
        .get_deserialized_from_json::<ModerationPendingCounts>(
            PENDING_COUNTS_NAMESPACE,
            cache_key,
        )
        .await?
    {
        return Ok(HttpResponse::Ok().json(cached));
    }

    let counts = sqlx::query!(
        r#"
        SELECT
            (SELECT COUNT(*) FROM mods WHERE status = 'processing') as "projects!",
            (SELECT COUNT(*) FROM reports WHERE closed = FALSE) as "reports!",
            (SELECT COUNT(*) FROM user_ban_appeals WHERE status = 'pending') as "appeals!",
            (SELECT COUNT(*) FROM user_profile_reviews WHERE status = 'pending') as "profile_reviews!",
            (SELECT COUNT(*) FROM image_content_reviews WHERE status = 'pending') as "image_reviews!",
            (SELECT COUNT(*) FROM creator_applications WHERE status = 'pending') as "creator_applications!"
        "#,
    )
    .fetch_one(&**pool)
    .await?;

    let incentive_applications: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM incentive_applications WHERE status = 'pending'"
    )
    .fetch_one(&**pool)
    .await?
    .unwrap_or(0);

    let payout_transfers = if user.role.is_admin() {
        sqlx::query_scalar!(
            "
            SELECT COUNT(*)
            FROM payouts
            WHERE status = 'in-transit'
              AND method = 'yunzhanghu_alipay'
              AND platform_id IS NULL
              AND yunzhanghu_submit_started_at IS NULL
            "
        )
        .fetch_one(&**pool)
        .await?
        .unwrap_or(0)
    } else {
        0
    };

    let result = ModerationPendingCounts {
        projects: counts.projects,
        reports: counts.reports,
        appeals: counts.appeals,
        profile_reviews: counts.profile_reviews,
        image_reviews: counts.image_reviews,
        creator_applications: counts.creator_applications,
        incentive_applications,
        payout_transfers,
    };

    redis_conn
        .set_serialized_to_json(
            PENDING_COUNTS_NAMESPACE,
            cache_key,
            &result,
            Some(PENDING_COUNTS_TTL),
        )
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

/// 清除待处理数量缓存（尽力而为，失败仅记录日志）
pub async fn clear_pending_counts_cache(redis: &RedisPool) {
    if let Err(e) = async {
        let mut redis_conn = redis.connect().await?;
        redis_conn
            .delete(PENDING_COUNTS_NAMESPACE, PENDING_COUNTS_ADMIN_CACHE_KEY)
            .await?;
        redis_conn
            .delete(
                PENDING_COUNTS_NAMESPACE,
                PENDING_COUNTS_MODERATOR_CACHE_KEY,
            )
            .await
    }
    .await
    {
        log::warn!("清除待处理计数缓存失败: {}", e);
    }
}

// ==================== 数据分析 ====================

const ANALYTICS_NAMESPACE: &str = "moderation_analytics";
const ANALYTICS_CACHE_TTL: i64 = 300; // 5 分钟

#[derive(Deserialize)]
pub struct RegistrationsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default = "default_registrations_resolution")]
    pub resolution: String,
}

fn default_registrations_resolution() -> String {
    "day".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistrationsPoint {
    pub time: i64,
    pub total: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistrationsResponse {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub resolution: String,
    pub total: i64,
    pub points: Vec<RegistrationsPoint>,
}

/// 注册趋势
///
/// GET /_internal/moderation/analytics/registrations
pub async fn get_analytics_registrations(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    query: web::Query<RegistrationsQuery>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let resolution = match query.resolution.as_str() {
        "hour" => "hour",
        _ => "day",
    };
    let default_span = if resolution == "hour" {
        Duration::days(2)
    } else {
        Duration::days(30)
    };
    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let start_date = query.start_date.unwrap_or(end_date - default_span);

    if end_date < start_date {
        return Err(ApiError::InvalidInput(
            "end_date 必须晚于 start_date".to_string(),
        ));
    }

    let cache_key = format!(
        "registrations:{resolution}:{}:{}",
        start_date.timestamp(),
        end_date.timestamp()
    );
    let mut redis_conn = redis.connect().await?;
    if let Some(cached) = redis_conn
        .get_deserialized_from_json::<RegistrationsResponse>(
            ANALYTICS_NAMESPACE,
            &cache_key,
        )
        .await?
    {
        return Ok(HttpResponse::Ok().json(cached));
    }

    let rows = sqlx::query!(
        r#"
        SELECT
            DATE_TRUNC($1, created) as "bucket!",
            COUNT(*) as "total!"
        FROM users
        WHERE created BETWEEN $2 AND $3
        GROUP BY DATE_TRUNC($1, created)
        ORDER BY DATE_TRUNC($1, created)
        "#,
        resolution,
        start_date,
        end_date,
    )
    .fetch_all(&**pool)
    .await?;

    let points: Vec<RegistrationsPoint> = rows
        .into_iter()
        .map(|r| RegistrationsPoint {
            time: r.bucket.timestamp(),
            total: r.total,
        })
        .collect();

    let total: i64 = points.iter().map(|p| p.total).sum();

    let response = RegistrationsResponse {
        start_date,
        end_date,
        resolution: resolution.to_string(),
        total,
        points,
    };

    redis_conn
        .set_serialized_to_json(
            ANALYTICS_NAMESPACE,
            &cache_key,
            &response,
            Some(ANALYTICS_CACHE_TTL),
        )
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Deserialize)]
pub struct DownloadsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub resolution_minutes: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadsPoint {
    pub time: i64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadsResponse {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub resolution_minutes: u32,
    pub total: u64,
    pub points: Vec<DownloadsPoint>,
}

/// 全站下载趋势
///
/// GET /_internal/moderation/analytics/downloads
pub async fn get_analytics_downloads(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    clickhouse: web::Data<clickhouse::Client>,
    query: web::Query<DownloadsQuery>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let start_date = query.start_date.unwrap_or(end_date - Duration::days(14));
    let resolution_minutes = query.resolution_minutes.unwrap_or(60 * 24);

    if end_date < start_date {
        return Err(ApiError::InvalidInput(
            "end_date 必须晚于 start_date".to_string(),
        ));
    }
    if resolution_minutes == 0 {
        return Err(ApiError::InvalidInput(
            "resolution_minutes 必须大于 0".to_string(),
        ));
    }

    let cache_key = format!(
        "downloads:{}:{}:{}",
        start_date.timestamp(),
        end_date.timestamp(),
        resolution_minutes
    );
    let mut redis_conn = redis.connect().await?;
    if let Some(cached) = redis_conn
        .get_deserialized_from_json::<DownloadsResponse>(
            ANALYTICS_NAMESPACE,
            &cache_key,
        )
        .await?
    {
        return Ok(HttpResponse::Ok().json(cached));
    }

    let intervals = crate::clickhouse::fetch_global_downloads_timeseries(
        start_date,
        end_date,
        resolution_minutes,
        clickhouse.into_inner(),
    )
    .await?;

    let points: Vec<DownloadsPoint> = intervals
        .into_iter()
        .map(|i| DownloadsPoint {
            time: i.time as i64,
            total: i.total,
        })
        .collect();

    let total: u64 = points.iter().map(|p| p.total).sum();

    let response = DownloadsResponse {
        start_date,
        end_date,
        resolution_minutes,
        total,
        points,
    };

    redis_conn
        .set_serialized_to_json(
            ANALYTICS_NAMESPACE,
            &cache_key,
            &response,
            Some(ANALYTICS_CACHE_TTL),
        )
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Deserialize)]
pub struct TopProjectsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default = "default_top_projects_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    pub search: Option<String>,
}

fn default_top_projects_limit() -> u32 {
    50
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TopProjectItem {
    pub rank: u32,
    pub project_id: String,
    pub slug: Option<String>,
    pub name: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TopProjectsResponse {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_downloads: u64,
    pub total_count: u32,
    pub items: Vec<TopProjectItem>,
}

/// 资源下载排行
///
/// GET /_internal/moderation/analytics/top-projects
pub async fn get_analytics_top_projects(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    clickhouse: web::Data<clickhouse::Client>,
    query: web::Query<TopProjectsQuery>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let start_date = query.start_date.unwrap_or(end_date - Duration::days(7));
    let limit = query.limit.clamp(1, 200);
    let offset = query.offset;
    let search = query
        .search
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if end_date < start_date {
        return Err(ApiError::InvalidInput(
            "end_date 必须晚于 start_date".to_string(),
        ));
    }

    // 仅缓存默认请求（无搜索、offset=0、limit=50）
    let is_cacheable = search.is_none()
        && offset == 0
        && limit == default_top_projects_limit();
    let cache_key = format!(
        "top-projects:{}:{}:{}",
        start_date.timestamp(),
        end_date.timestamp(),
        limit
    );
    let mut redis_conn = redis.connect().await?;
    if is_cacheable
        && let Some(cached) = redis_conn
            .get_deserialized_from_json::<TopProjectsResponse>(
                ANALYTICS_NAMESPACE,
                &cache_key,
            )
            .await?
    {
        return Ok(HttpResponse::Ok().json(cached));
    }

    // ClickHouse 拿排行（按搜索过滤前先多取一些以便后续过滤）
    let fetch_limit = if search.is_some() {
        // 搜索时取更大窗口再用 PG 过滤
        (limit + offset).clamp(500, 2000)
    } else {
        limit + offset
    };

    let totals = crate::clickhouse::fetch_top_projects_downloads(
        start_date,
        end_date,
        fetch_limit,
        clickhouse.into_inner(),
    )
    .await?;

    if totals.is_empty() {
        return Ok(HttpResponse::Ok().json(TopProjectsResponse {
            start_date,
            end_date,
            total_downloads: 0,
            total_count: 0,
            items: vec![],
        }));
    }

    let project_ids: Vec<i64> = totals.iter().map(|t| t.id as i64).collect();

    let project_rows = sqlx::query!(
        r#"
        SELECT id, name, slug, icon_url
        FROM mods
        WHERE id = ANY($1)
        "#,
        &project_ids,
    )
    .fetch_all(&**pool)
    .await?;

    let project_map: HashMap<i64, (String, Option<String>, Option<String>)> =
        project_rows
            .into_iter()
            .map(|r| (r.id, (r.name, r.slug, r.icon_url)))
            .collect();

    let total_downloads: u64 = totals.iter().map(|t| t.total).sum();

    struct EnrichedRow {
        rank: u32,
        id: u64,
        total: u64,
        name: String,
        slug: Option<String>,
        icon_url: Option<String>,
    }

    let needle = search.as_ref().map(|s| s.to_lowercase());

    // 全站排名按 ClickHouse 返回顺序（已按下载量降序），过滤后保留原始排名
    let mut items: Vec<TopProjectItem> = totals
        .into_iter()
        .enumerate()
        .filter_map(|(idx, t)| {
            project_map.get(&(t.id as i64)).map(|(name, slug, icon)| {
                EnrichedRow {
                    rank: (idx + 1) as u32,
                    id: t.id,
                    total: t.total,
                    name: name.clone(),
                    slug: slug.clone(),
                    icon_url: icon.clone(),
                }
            })
        })
        .filter(|row| {
            if let Some(needle) = &needle {
                row.name.to_lowercase().contains(needle)
                    || row
                        .slug
                        .as_ref()
                        .map(|sl| sl.to_lowercase().contains(needle))
                        .unwrap_or(false)
            } else {
                true
            }
        })
        .map(|row| TopProjectItem {
            rank: row.rank,
            project_id: to_base62(row.id),
            slug: row.slug,
            name: row.name,
            icon_url: row.icon_url,
            downloads: row.total,
        })
        .collect();

    let total_count = items.len() as u32;

    let start = (offset as usize).min(items.len());
    let end = (start + limit as usize).min(items.len());
    items = items[start..end].to_vec();

    let response = TopProjectsResponse {
        start_date,
        end_date,
        total_downloads,
        total_count,
        items,
    };

    if is_cacheable {
        redis_conn
            .set_serialized_to_json(
                ANALYTICS_NAMESPACE,
                &cache_key,
                &response,
                Some(ANALYTICS_CACHE_TTL),
            )
            .await?;
    }

    Ok(HttpResponse::Ok().json(response))
}
