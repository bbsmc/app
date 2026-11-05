//! 上游 ddb131a70 (#7591)：每用户每日发布限额（本 fork 只移植每日限额，不含总量上限）。
//!
//! - 每日新建项目数：你创建（`mods.created_by`）、且今天创建（`mods.published`）的项目数。
//!   与上游"按团队成员身份计数"不同：在组织名下建项目、被邀请进别人的项目、项目转让都不影响计数。
//! - 每日上传版本数：你上传（`versions.author_id`）、且今天发布的版本数，跨所有项目。
//! - 单项目版本数：该项目的全部版本数（不论作者）。
//!
//! "今天"按北京时间自然日（`app_tz()`，00:00 重置）。管理员（admin）不受限。
//! 计数与写入不在同一事务内，并发上传时可能略超上限，属软限制（与上游一致）。

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::database::models::ids::{
    ProjectId as DBProjectId, UserId as DBUserId,
};
use crate::database::models::user_limits::DBUserLimits;
use crate::models::users::User;
use crate::util::date::app_tz;

/// 北京时间"今天"的 [起, 止) 区间，以 UTC 表示。
fn app_day_bounds(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let tz = app_tz();
    let local_midnight = now
        .with_timezone(&tz)
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("午夜总是合法时间");
    let day_start = local_midnight
        .and_local_timezone(tz)
        .single()
        .expect("固定时区偏移下本地时间唯一")
        .with_timezone(&Utc);
    (day_start, day_start + chrono::Duration::days(1))
}

#[derive(Debug, Clone)]
pub struct UserLimits {
    pub current: u64,
    pub max: u64,
}

impl UserLimits {
    /// 再新增 `adding` 个是否会超过上限。
    pub fn would_exceed(&self, adding: u64) -> bool {
        self.current.saturating_add(adding) > self.max
    }

    fn adjust_for_user(self, user: &User) -> Self {
        if user.role.is_admin() {
            Self {
                current: self.current,
                max: u64::MAX,
            }
        } else {
            self
        }
    }

    pub async fn get_for_projects_per_day(
        user: &User,
        now: DateTime<Utc>,
        pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let user_id: DBUserId = user.id.into();
        let limits = DBUserLimits::get(user_id, pool).await?;
        let (day_start, day_end) = app_day_bounds(now);
        // mods.created_by 由迁移 202611041200001 新增，迁移运行前开发库里没有该列，
        // 无法用 query! 宏在编译期校验，故用运行时查询。
        let current: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mods
            WHERE created_by = $1
                AND published >= $2
                AND published < $3",
        )
        .bind(user_id.0)
        .bind(day_start)
        .bind(day_end)
        .fetch_one(pool)
        .await?;

        Ok(Self {
            current: current.max(0) as u64,
            max: limits.projects_per_day,
        }
        .adjust_for_user(user))
    }

    pub async fn get_for_versions_per_project(
        user: &User,
        project_id: DBProjectId,
        pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let limits = DBUserLimits::get(user.id.into(), pool).await?;
        let current = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM versions WHERE mod_id = $1",
            project_id as DBProjectId,
        )
        .fetch_one(pool)
        .await?
        .map_or(0, |x| x as u64);

        Ok(Self {
            current,
            max: limits.versions_per_project,
        }
        .adjust_for_user(user))
    }

    pub async fn get_for_versions_per_day(
        user: &User,
        now: DateTime<Utc>,
        pool: &PgPool,
    ) -> Result<Self, sqlx::Error> {
        let user_id: DBUserId = user.id.into();
        let limits = DBUserLimits::get(user_id, pool).await?;
        let (day_start, day_end) = app_day_bounds(now);
        let current = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM versions
            WHERE author_id = $1
                AND date_published >= $2
                AND date_published < $3",
            user_id as DBUserId,
            day_start,
            day_end,
        )
        .fetch_one(pool)
        .await?
        .map_or(0, |x| x as u64);

        Ok(Self {
            current,
            max: limits.versions_per_day,
        }
        .adjust_for_user(user))
    }
}
