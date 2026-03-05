use super::ApiError;
use crate::database;
use crate::database::redis::RedisPool;
use crate::models::ids::random_base62;
use crate::models::projects::ProjectStatus;
use crate::queue::moderation::{ApprovalType, IdentifiedFile, MissingMetadata};
use crate::queue::session::AuthQueue;
use crate::{auth::check_is_moderator_from_headers, models::pats::Scopes};
use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;
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
}

pub(crate) const PENDING_COUNTS_NAMESPACE: &str = "moderation_pending_counts";
const PENDING_COUNTS_CACHE_KEY: &str = "all";
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
    check_is_moderator_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PROJECT_READ]),
    )
    .await?;

    // 查 Redis 缓存
    let mut redis_conn = redis.connect().await?;
    if let Some(cached) = redis_conn
        .get_deserialized_from_json::<ModerationPendingCounts>(
            PENDING_COUNTS_NAMESPACE,
            PENDING_COUNTS_CACHE_KEY,
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

    let result = ModerationPendingCounts {
        projects: counts.projects,
        reports: counts.reports,
        appeals: counts.appeals,
        profile_reviews: counts.profile_reviews,
        image_reviews: counts.image_reviews,
        creator_applications: counts.creator_applications,
    };

    redis_conn
        .set_serialized_to_json(
            PENDING_COUNTS_NAMESPACE,
            PENDING_COUNTS_CACHE_KEY,
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
            .delete(PENDING_COUNTS_NAMESPACE, PENDING_COUNTS_CACHE_KEY)
            .await
    }
    .await
    {
        log::warn!("清除待处理计数缓存失败: {}", e);
    }
}
