//! 上游 f466470d0 (#4430) + ddb131a70 (#7591)：每用户发布限额（本 fork 只移植每日限额）。
//!
//! 表由迁移 `202611041200001_create_user_limits.sql` 创建。迁移运行前本地开发库里没有这张表，
//! `cargo sqlx prepare` 无法校验，所以这里用运行时查询（`sqlx::query_as`）而不是 `query!` 宏；
//! 改列名时要同时改迁移和这里。

use super::ids::UserId;
use sqlx::PgPool;

// 全局默认行缺失时的兜底额度，与迁移写入的默认值一致。
const FALLBACK_PROJECTS_PER_DAY: u64 = 10;
const FALLBACK_VERSIONS_PER_PROJECT: u64 = 10_000;
const FALLBACK_VERSIONS_PER_DAY: u64 = 50;

#[derive(Debug, Clone)]
pub struct DBUserLimits {
    pub projects_per_day: u64,
    pub versions_per_project: u64,
    pub versions_per_day: u64,
}

impl DBUserLimits {
    /// 取该用户的额度：有单独覆盖行就用覆盖行，否则用全局默认行（user_id IS NULL）。
    pub async fn get(
        user_id: UserId,
        pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let row: Option<(i32, i32, i32)> = sqlx::query_as(
            "SELECT projects_per_day, versions_per_project, versions_per_day
            FROM user_limits
            WHERE user_id = $1 OR user_id IS NULL
            ORDER BY user_id NULLS LAST
            LIMIT 1",
        )
        .bind(user_id.0)
        .fetch_optional(pool)
        .await?;

        let Some((projects_per_day, versions_per_project, versions_per_day)) =
            row
        else {
            log::warn!("user_limits 缺少全局默认行，使用内置默认额度");
            return Ok(Self {
                projects_per_day: FALLBACK_PROJECTS_PER_DAY,
                versions_per_project: FALLBACK_VERSIONS_PER_PROJECT,
                versions_per_day: FALLBACK_VERSIONS_PER_DAY,
            });
        };

        // 迁移里已有 CHECK (>= 0)，这里再防御一次，避免负数转成 u64 后变成"无限"。
        Ok(Self {
            projects_per_day: projects_per_day.max(0) as u64,
            versions_per_project: versions_per_project.max(0) as u64,
            versions_per_day: versions_per_day.max(0) as u64,
        })
    }
}
