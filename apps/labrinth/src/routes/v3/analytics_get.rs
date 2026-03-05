use super::ApiError;
use crate::database;
use crate::database::redis::RedisPool;
use crate::models::teams::ProjectPermissions;
use crate::{
    auth::get_user_from_headers,
    database::models::user_item,
    models::{
        ids::{ProjectId, VersionId, base62_impl::to_base62},
        pats::Scopes,
    },
    queue::session::AuthQueue,
};
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::types::PgInterval;
use std::collections::HashMap;
use std::convert::TryInto;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("analytics")
            .route("playtime", web::get().to(playtimes_get))
            .route("views", web::get().to(views_get))
            .route("downloads", web::get().to(downloads_get))
            .route("revenue", web::get().to(revenue_get))
            .route(
                "countries/downloads",
                web::get().to(countries_downloads_get),
            )
            .route("countries/views", web::get().to(countries_views_get)),
    );
}

/// 传递给获取分析数据的数据
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
/// start_date 和 end_date 是可选的，默认分别为两周前和当前日期。
/// resolution_minutes 是可选的。这指的是我们正在查看的窗口（每天、每分钟等），默认为 1440（1 天）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetData {
    // 项目 ID 列表或版本 ID 列表，但不能同时使用。
    // 如果未提供，则使用用户有权访问的所有项目
    pub project_ids: Option<String>,

    pub start_date: Option<DateTime<Utc>>, // 默认两周前
    pub end_date: Option<DateTime<Utc>>,   // 默认当前日期

    pub resolution_minutes: Option<u32>, // 默认 1 天。在未聚合到分辨率的路径中忽略（例如：/countries）
}

/// 获取一组项目或版本的游玩时间数据
/// 数据以哈希映射的形式返回，项目/版本 ID 映射到每天的游玩时间数据。
/// 例如:
/// {
///     "4N1tEhnO": {
///         "20230824": 23
///    }
///}
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
#[derive(Serialize, Deserialize, Clone)]
pub struct FetchedPlaytime {
    pub time: u64,
    pub total_seconds: u64,
    pub loader_seconds: HashMap<String, u64>,
    pub game_version_seconds: HashMap<String, u64>,
    pub parent_seconds: HashMap<VersionId, u64>,
}
pub async fn playtimes_get(
    req: HttpRequest,
    clickhouse: web::Data<clickhouse::Client>,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ANALYTICS]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());
    let resolution_minutes = data.resolution_minutes.unwrap_or(60 * 24);

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    // - 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    let project_ids =
        filter_allowed_ids(project_ids, user, &pool, &redis, None).await?;

    // 获取游玩时间
    let playtimes = crate::clickhouse::fetch_playtimes(
        project_ids.unwrap_or_default(),
        start_date,
        end_date,
        resolution_minutes,
        clickhouse.into_inner(),
    )
    .await?;

    let mut hm = HashMap::new();
    for playtime in playtimes {
        let id_string = to_base62(playtime.id);
        if !hm.contains_key(&id_string) {
            hm.insert(id_string.clone(), HashMap::new());
        }
        if let Some(hm) = hm.get_mut(&id_string) {
            hm.insert(playtime.time, playtime.total);
        }
    }

    Ok(HttpResponse::Ok().json(hm))
}

/// 获取一组项目或版本的浏览数据
/// 数据以哈希映射的形式返回，项目/版本 ID 映射到每天的浏览量。
/// 例如:
/// {
///     "4N1tEhnO": {
///         "20230824": 1090
///    }
///}
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
pub async fn views_get(
    req: HttpRequest,
    clickhouse: web::Data<clickhouse::Client>,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ANALYTICS]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());
    let resolution_minutes = data.resolution_minutes.unwrap_or(60 * 24);

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    // - 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    let project_ids =
        filter_allowed_ids(project_ids, user, &pool, &redis, None).await?;

    // 获取浏览量
    let views = crate::clickhouse::fetch_views(
        project_ids.unwrap_or_default(),
        start_date,
        end_date,
        resolution_minutes,
        clickhouse.into_inner(),
    )
    .await?;

    let mut hm = HashMap::new();
    for views in views {
        let id_string = to_base62(views.id);
        if !hm.contains_key(&id_string) {
            hm.insert(id_string.clone(), HashMap::new());
        }
        if let Some(hm) = hm.get_mut(&id_string) {
            hm.insert(views.time, views.total);
        }
    }

    Ok(HttpResponse::Ok().json(hm))
}

/// 获取一组项目或版本的下载数据
/// 数据以哈希映射的形式返回，项目/版本 ID 映射到每天的下载量的哈希映射。
/// 例如:
/// {
///     "4N1tEhnO": {
///         "20230824": 32
///    }
///}
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
pub async fn downloads_get(
    req: HttpRequest,
    clickhouse: web::Data<clickhouse::Client>,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user_option = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ANALYTICS]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());
    let resolution_minutes = data.resolution_minutes.unwrap_or(60 * 24);

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    // - 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    let project_ids =
        filter_allowed_ids(project_ids, user_option, &pool, &redis, None)
            .await?;

    // 获取下载量
    let downloads = crate::clickhouse::fetch_downloads(
        project_ids.unwrap_or_default(),
        start_date,
        end_date,
        resolution_minutes,
        clickhouse.into_inner(),
    )
    .await?;

    let mut hm = HashMap::new();
    for downloads in downloads {
        let id_string = to_base62(downloads.id);
        if !hm.contains_key(&id_string) {
            hm.insert(id_string.clone(), HashMap::new());
        }
        if let Some(hm) = hm.get_mut(&id_string) {
            hm.insert(downloads.time, downloads.total);
        }
    }

    Ok(HttpResponse::Ok().json(hm))
}

/// 获取一组项目的收入数据
/// 数据以哈希映射的形式返回，项目 ID 映射到每天的收入数据。
/// 例如:
/// {
///     "4N1tEhnO": {
///         "20230824": 0.001
///    }
///}
/// 只能使用项目 ID。未经授权的项目将被过滤掉。
pub async fn revenue_get(
    req: HttpRequest,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::PAYOUTS_READ]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());
    let resolution_minutes = data.resolution_minutes.unwrap_or(60 * 24);

    // 将开始日期和结束日期四舍五入到最近的分辨率，因为我们使用的是 pgadmin，没有在 fetch 命令中进行四舍五入
    // 将开始日期四舍五入到最近的分辨率
    let diff = start_date.timestamp() % (resolution_minutes as i64 * 60);
    let start_date = start_date - Duration::seconds(diff);

    // 将结束日期四舍五入到最近的分辨率
    let diff = end_date.timestamp() % (resolution_minutes as i64 * 60);
    let end_date =
        end_date + Duration::seconds((resolution_minutes as i64 * 60) - diff);

    // 将字符串列表转换为项目 ID 列表
    // - 过滤掉未经授权的项目
    // - 如果未提供项目 ID，则默认使用用户有权访问的所有项目
    let project_ids = filter_allowed_ids(
        project_ids,
        user.clone(),
        &pool,
        &redis,
        Some(true),
    )
    .await?;

    let duration: PgInterval = Duration::minutes(resolution_minutes as i64)
        .try_into()
        .map_err(|_| {
            ApiError::InvalidInput("无效的 resolution_minutes 参数".to_string())
        })?;
    // 获取收入数据
    let project_ids = project_ids.unwrap_or_default();

    struct PayoutValue {
        mod_id: Option<i64>,
        amount_sum: Option<rust_decimal::Decimal>,
        interval_start: Option<DateTime<Utc>>,
    }

    let payouts_values = if project_ids.is_empty() {
        sqlx::query!(
            "
            SELECT mod_id, SUM(amount) amount_sum, DATE_BIN($4::interval, created, TIMESTAMP '2001-01-01') AS interval_start
            FROM payouts_values
            WHERE user_id = $1 AND created BETWEEN $2 AND $3
            GROUP by mod_id, interval_start ORDER BY interval_start
            ",
            user.id.0 as i64,
            start_date,
            end_date,
            duration,
        )
            .fetch_all(&**pool)
            .await?.into_iter().map(|x| PayoutValue {
            mod_id: x.mod_id,
            amount_sum: x.amount_sum,
            interval_start: x.interval_start,
        }).collect::<Vec<_>>()
    } else {
        sqlx::query!(
            "
            SELECT mod_id, SUM(amount) amount_sum, DATE_BIN($4::interval, created, TIMESTAMP '2001-01-01') AS interval_start
            FROM payouts_values
            WHERE mod_id = ANY($1) AND created BETWEEN $2 AND $3
            GROUP by mod_id, interval_start ORDER BY interval_start
            ",
            &project_ids.iter().map(|x| x.0 as i64).collect::<Vec<_>>(),
            start_date,
            end_date,
            duration,
        )
            .fetch_all(&**pool)
            .await?.into_iter().map(|x| PayoutValue {
            mod_id: x.mod_id,
            amount_sum: x.amount_sum,
            interval_start: x.interval_start,
        }).collect::<Vec<_>>()
    };

    let mut hm: HashMap<_, _> = project_ids
        .into_iter()
        .map(|x| (x.to_string(), HashMap::new()))
        .collect::<HashMap<_, _>>();
    for value in payouts_values {
        if let Some(mod_id) = value.mod_id
            && let Some(amount) = value.amount_sum
            && let Some(interval_start) = value.interval_start
        {
            let id_string = to_base62(mod_id as u64);
            if !hm.contains_key(&id_string) {
                hm.insert(id_string.clone(), HashMap::new());
            }
            if let Some(hm) = hm.get_mut(&id_string) {
                hm.insert(interval_start.timestamp(), amount);
            }
        }
    }

    Ok(HttpResponse::Ok().json(hm))
}

/// 获取一组项目或版本的国家数据
/// 数据以哈希映射的形式返回，项目/版本 ID 映射到国家下载量的哈希映射。
/// 未知国家标记为 ""。
/// 这可以用来查看每个项目的显著表现国家
/// 例如:
/// {
///     "4N1tEhnO": {
///         "CAN":  22
///    }
///}
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
/// 对于此端点，提供的日期是要聚合的范围，而不是要获取的特定日期
pub async fn countries_downloads_get(
    req: HttpRequest,
    clickhouse: web::Data<clickhouse::Client>,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ANALYTICS]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    // - 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    let project_ids =
        filter_allowed_ids(project_ids, user, &pool, &redis, None).await?;

    // 获取国家数据
    let countries = crate::clickhouse::fetch_countries_downloads(
        project_ids.unwrap_or_default(),
        start_date,
        end_date,
        clickhouse.into_inner(),
    )
    .await?;

    let mut hm = HashMap::new();
    for views in countries {
        let id_string = to_base62(views.id);
        if !hm.contains_key(&id_string) {
            hm.insert(id_string.clone(), HashMap::new());
        }
        if let Some(hm) = hm.get_mut(&id_string) {
            hm.insert(views.country, views.total);
        }
    }

    let hm: HashMap<String, HashMap<String, u64>> = hm
        .into_iter()
        .map(|(key, value)| (key, condense_countries(value)))
        .collect();

    Ok(HttpResponse::Ok().json(hm))
}

/// 获取一组项目或版本的国家数据
/// 数据以哈希映射的形式返回，项目/版本 ID 映射到国家浏览量的哈希映射。
/// 未知国家标记为 ""。
/// 这可以用来查看每个项目的显著表现国家
/// 例如:
/// {
///     "4N1tEhnO": {
///         "CAN":  56165
///    }
///}
/// 可以使用项目 ID 列表或版本 ID 列表，但不能同时使用。未经授权的项目/版本将被过滤掉。
/// 对于此端点，提供的日期是要聚合的范围，而不是要获取的特定日期
pub async fn countries_views_get(
    req: HttpRequest,
    clickhouse: web::Data<clickhouse::Client>,
    data: web::Query<GetData>,
    session_queue: web::Data<AuthQueue>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, ApiError> {
    let user = get_user_from_headers(
        &req,
        &**pool,
        &redis,
        &session_queue,
        Some(&[Scopes::ANALYTICS]),
    )
    .await
    .map(|x| x.1)?;

    let project_ids = data
        .project_ids
        .as_ref()
        .map(|ids| serde_json::from_str::<Vec<String>>(ids))
        .transpose()?;

    let start_date = data.start_date.unwrap_or(Utc::now() - Duration::weeks(2));
    let end_date = data.end_date.unwrap_or(Utc::now());

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    // - 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    let project_ids =
        filter_allowed_ids(project_ids, user, &pool, &redis, None).await?;

    // 获取国家数据
    let countries = crate::clickhouse::fetch_countries_views(
        project_ids.unwrap_or_default(),
        start_date,
        end_date,
        clickhouse.into_inner(),
    )
    .await?;

    let mut hm = HashMap::new();
    for views in countries {
        let id_string = to_base62(views.id);
        if !hm.contains_key(&id_string) {
            hm.insert(id_string.clone(), HashMap::new());
        }
        if let Some(hm) = hm.get_mut(&id_string) {
            hm.insert(views.country, views.total);
        }
    }

    let hm: HashMap<String, HashMap<String, u64>> = hm
        .into_iter()
        .map(|(key, value)| (key, condense_countries(value)))
        .collect();

    Ok(HttpResponse::Ok().json(hm))
}

fn condense_countries(countries: HashMap<String, u64>) -> HashMap<String, u64> {
    // 每个国家（视图或下载）低于 '15' 的应缩减为 'XX'
    let mut hm = HashMap::new();
    for (mut country, count) in countries {
        if count < 50 {
            country = "XX".to_string();
        }
        if !hm.contains_key(&country) {
            hm.insert(country.to_string(), 0);
        }
        if let Some(hm) = hm.get_mut(&country) {
            *hm += count;
        }
    }
    hm
}

async fn filter_allowed_ids(
    mut project_ids: Option<Vec<String>>,
    user: crate::models::users::User,
    pool: &web::Data<PgPool>,
    redis: &RedisPool,
    remove_defaults: Option<bool>,
) -> Result<Option<Vec<ProjectId>>, ApiError> {
    // 如果未提供项目 ID 或版本 ID，则默认使用用户有权访问的所有项目
    if project_ids.is_none() && !remove_defaults.unwrap_or(false) {
        project_ids = Some(
            user_item::User::get_projects(user.id.into(), &***pool, redis)
                .await?
                .into_iter()
                .map(|x| ProjectId::from(x).to_string())
                .collect(),
        );
    }

    // 将字符串列表转换为项目 ID 或版本 ID 列表
    // - 过滤掉未经授权的项目/版本
    let project_ids = if let Some(project_strings) = project_ids {
        let projects_data = database::models::Project::get_many(
            &project_strings,
            &***pool,
            redis,
        )
        .await?;

        let team_ids = projects_data
            .iter()
            .map(|x| x.inner.team_id)
            .collect::<Vec<database::models::TeamId>>();
        let team_members =
            database::models::TeamMember::get_from_team_full_many(
                &team_ids, &***pool, redis,
            )
            .await?;

        let organization_ids = projects_data
            .iter()
            .filter_map(|x| x.inner.organization_id)
            .collect::<Vec<database::models::OrganizationId>>();
        let organizations = database::models::Organization::get_many_ids(
            &organization_ids,
            &***pool,
            redis,
        )
        .await?;

        let organization_team_ids = organizations
            .iter()
            .map(|x| x.team_id)
            .collect::<Vec<database::models::TeamId>>();
        let organization_team_members =
            database::models::TeamMember::get_from_team_full_many(
                &organization_team_ids,
                &***pool,
                redis,
            )
            .await?;

        let ids = projects_data
            .into_iter()
            .filter(|project| {
                let team_member = team_members.iter().find(|x| {
                    x.team_id == project.inner.team_id
                        && x.user_id == user.id.into()
                });

                let organization = project
                    .inner
                    .organization_id
                    .and_then(|oid| organizations.iter().find(|x| x.id == oid));

                let organization_team_member =
                    if let Some(organization) = organization {
                        organization_team_members.iter().find(|x| {
                            x.team_id == organization.team_id
                                && x.user_id == user.id.into()
                        })
                    } else {
                        None
                    };

                let permissions = ProjectPermissions::get_permissions_by_role(
                    &user.role,
                    &team_member.cloned(),
                    &organization_team_member.cloned(),
                )
                .unwrap_or_default();

                permissions.contains(ProjectPermissions::VIEW_ANALYTICS)
            })
            .map(|x| x.inner.id.into())
            .collect::<Vec<_>>();

        Some(ids)
    } else {
        None
    };
    // 只有 project_ids 或 version_ids 之一会为 Some
    Ok(project_ids)
}
