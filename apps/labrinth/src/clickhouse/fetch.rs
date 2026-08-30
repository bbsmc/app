use std::sync::Arc;

use crate::{models::ids::ProjectId, routes::ApiError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ClickHouse 桶函数（toStartOf*）的桶起点不受 client 端 session_timezone 影响，
// 必须显式传入时区参数。这里硬编码字面量是为了直接嵌入 SQL，对应
// `crate::util::date::APP_TZ_NAME`，修改时务必同步两处。
//
// 桶函数三参形式参考 ClickHouse 官方文档（≥ 22.3 支持）。

#[derive(clickhouse::Row, Serialize, Deserialize, Clone, Debug)]
pub struct ReturnIntervals {
    pub time: u32,
    pub id: u64,
    pub total: u64,
}

#[derive(clickhouse::Row, Serialize, Deserialize, Clone, Debug)]
pub struct ReturnCountry {
    pub country: String,
    pub id: u64,
    pub total: u64,
}

#[derive(clickhouse::Row, Serialize, Deserialize, Clone, Debug)]
pub struct ReturnGlobalInterval {
    pub time: u32,
    pub total: u64,
}

#[derive(clickhouse::Row, Serialize, Deserialize, Clone, Debug)]
pub struct ReturnProjectTotal {
    pub id: u64,
    pub total: u64,
}

// 只能使用 project_id 或 version_id 之一
// 获取播放时间，返回 ReturnPlaytimes 的 Vec
pub async fn fetch_playtimes(
    projects: Vec<ProjectId>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    resolution_minute: u32,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnIntervals>, ApiError> {
    let query = client
        .query(
            "
            SELECT
                toUnixTimestamp(toStartOfInterval(recorded, toIntervalMinute(?), 'Asia/Shanghai')) AS time,
                project_id AS id,
                SUM(seconds) AS total
            FROM playtime
            WHERE recorded BETWEEN ? AND ?
            AND project_id IN ?
            GROUP BY
                time,
                project_id
            ",
        )
        .bind(resolution_minute)
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(projects.iter().map(|x| x.0).collect::<Vec<_>>());

    Ok(query.fetch_all().await?)
}

// Fetches views as a Vec of ReturnViews
pub async fn fetch_views(
    projects: Vec<ProjectId>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    resolution_minutes: u32,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnIntervals>, ApiError> {
    let query = client
        .query(
            "
            SELECT  
                toUnixTimestamp(toStartOfInterval(recorded, toIntervalMinute(?), 'Asia/Shanghai')) AS time,
                project_id AS id,
                count(1) AS total
            FROM views
            WHERE recorded BETWEEN ? AND ?
                  AND project_id IN ?
            GROUP BY
            time, project_id
            ",
        )
        .bind(resolution_minutes)
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(projects.iter().map(|x| x.0).collect::<Vec<_>>());

    Ok(query.fetch_all().await?)
}

// Fetches downloads as a Vec of ReturnDownloads
pub async fn fetch_downloads(
    projects: Vec<ProjectId>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    resolution_minutes: u32,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnIntervals>, ApiError> {
    let query = client
        .query(
            "
            SELECT  
                toUnixTimestamp(toStartOfInterval(recorded, toIntervalMinute(?), 'Asia/Shanghai')) AS time,
                project_id as id,
                count(1) AS total
            FROM downloads
            WHERE recorded BETWEEN ? AND ?
                  AND project_id IN ?
            GROUP BY time, project_id
            ",
        )
        .bind(resolution_minutes)
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(projects.iter().map(|x| x.0).collect::<Vec<_>>());

    Ok(query.fetch_all().await?)
}

pub async fn fetch_countries_downloads(
    projects: Vec<ProjectId>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnCountry>, ApiError> {
    let query = client
        .query(
            "
            SELECT
                country,
                project_id AS id,
                count(1) AS total
            FROM downloads
            WHERE recorded BETWEEN ? AND ? AND project_id IN ?
            GROUP BY
                country,
                project_id
            ",
        )
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(projects.iter().map(|x| x.0).collect::<Vec<_>>());

    Ok(query.fetch_all().await?)
}

// 获取全站下载量的时间序列（不限定项目）
pub async fn fetch_global_downloads_timeseries(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    resolution_minutes: u32,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnGlobalInterval>, ApiError> {
    let query = client
        .query(
            "
            SELECT
                toUnixTimestamp(toStartOfInterval(recorded, toIntervalMinute(?), 'Asia/Shanghai')) AS time,
                count(1) AS total
            FROM downloads
            WHERE recorded BETWEEN ? AND ?
            GROUP BY time
            ORDER BY time
            ",
        )
        .bind(resolution_minutes)
        .bind(start_date.timestamp())
        .bind(end_date.timestamp());

    Ok(query.fetch_all().await?)
}

// 获取指定时间范围内下载量排行的项目（不限定项目）
pub async fn fetch_top_projects_downloads(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: u32,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnProjectTotal>, ApiError> {
    let query = client
        .query(
            "
            SELECT
                project_id AS id,
                count(1) AS total
            FROM downloads
            WHERE recorded BETWEEN ? AND ?
            GROUP BY project_id
            ORDER BY total DESC
            LIMIT ?
            ",
        )
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(limit);

    Ok(query.fetch_all().await?)
}

pub async fn fetch_countries_views(
    projects: Vec<ProjectId>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    client: Arc<clickhouse::Client>,
) -> Result<Vec<ReturnCountry>, ApiError> {
    let query = client
        .query(
            "
            SELECT
                country,
                project_id AS id,
                count(1) AS total
            FROM views
            WHERE recorded BETWEEN ? AND ? AND project_id IN ?
            GROUP BY
                country,
                project_id
            ",
        )
        .bind(start_date.timestamp())
        .bind(end_date.timestamp())
        .bind(projects.iter().map(|x| x.0).collect::<Vec<_>>());

    Ok(query.fetch_all().await?)
}
