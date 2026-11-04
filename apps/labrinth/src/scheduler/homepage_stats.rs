//! 首页站点统计
//!
//! 每小时从 PostgreSQL 汇总一次首页展示用的统计数据（资源总数 / 累计下载 / 注册用户），
//! 写入 Redis；公开接口 `GET /v3/statistics/homepage` 只读 Redis，未命中时才回源计算。
//! 前端在构建期（nuxt `build:before`）请求该接口并写入 `generated/state.json`。

use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::database::models::DatabaseError;
use crate::database::redis::RedisPool;
use crate::models::projects::ProjectStatus;

use super::Scheduler;

const HOMEPAGE_STATS_NAMESPACE: &str = "site_stats";
const HOMEPAGE_STATS_KEY: &str = "homepage";
/// 刷新间隔：每小时一次
const REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// Redis 过期时间大于刷新间隔，单次刷新失败时仍保留上一次的数据
const CACHE_EXPIRY_SECS: i64 = 60 * 60 * 3;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HomepageStats {
    /// 公开可见（可搜索）的资源总数
    pub projects: i64,
    /// 全平台资源累计下载数
    pub downloads: i64,
    /// 注册用户数
    pub users: i64,
    /// 本次统计时间
    pub updated_at: DateTime<Utc>,
}

async fn compute_homepage_stats(
    pool: &PgPool,
) -> Result<HomepageStats, DatabaseError> {
    let searchable_statuses = ProjectStatus::iterator()
        .filter(|x| x.is_searchable())
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

    let projects = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM mods
        WHERE status = ANY($1::text[])
        "#,
        &searchable_statuses,
    )
    .fetch_one(pool)
    .await?;

    let downloads = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(downloads), 0)::bigint AS "total!"
        FROM mods
        "#
    )
    .fetch_one(pool)
    .await?;

    let users = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM users
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(HomepageStats {
        projects,
        downloads,
        users,
        updated_at: Utc::now(),
    })
}

/// 重新统计并覆盖写入 Redis
async fn refresh_homepage_stats(
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<HomepageStats, DatabaseError> {
    let stats = compute_homepage_stats(pool).await?;
    let mut conn = redis.connect().await?;
    conn.set_serialized_to_json(
        HOMEPAGE_STATS_NAMESPACE,
        HOMEPAGE_STATS_KEY,
        &stats,
        Some(CACHE_EXPIRY_SECS),
    )
    .await?;
    Ok(stats)
}

/// 读取首页统计：优先 Redis，未命中（如 Redis 被清空）时单飞回源计算并回填
pub async fn get_homepage_stats(
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<HomepageStats, DatabaseError> {
    redis
        .get_cached_key_singleflight_with_expiry(
            HOMEPAGE_STATS_NAMESPACE,
            HOMEPAGE_STATS_KEY,
            CACHE_EXPIRY_SECS,
            || compute_homepage_stats(pool),
        )
        .await
}

/// 启动时立即执行一次，之后每小时刷新
pub fn schedule_homepage_stats(
    scheduler: &mut Scheduler,
    pool: PgPool,
    redis: RedisPool,
) {
    scheduler.run(REFRESH_INTERVAL, move || {
        let pool = pool.clone();
        let redis = redis.clone();
        async move {
            match refresh_homepage_stats(&pool, &redis).await {
                Ok(stats) => info!(
                    "首页统计已刷新：资源 {}，累计下载 {}，注册用户 {}",
                    stats.projects, stats.downloads, stats.users
                ),
                Err(e) => warn!("首页统计刷新失败：{:?}", e),
            }
        }
    });
}
