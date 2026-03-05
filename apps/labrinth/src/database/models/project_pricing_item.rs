use super::DatabaseError;
use super::ids::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 项目定价信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectPricing {
    pub project_id: ProjectId,
    pub price: Decimal,             // 价格（单位：元）
    pub validity_days: Option<i32>, // 有效期天数，None 表示永久
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectPricing {
    /// 插入或更新项目定价
    pub async fn upsert(
        project_id: ProjectId,
        price: Decimal,
        validity_days: Option<i32>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let now = Utc::now();

        // 使用 RETURNING 获取实际的 created_at 和 updated_at
        // 当发生冲突（更新）时，created_at 保持原值，updated_at 是新值
        let result = sqlx::query!(
            "
            INSERT INTO project_pricing (project_id, price, validity_days, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            ON CONFLICT (project_id) DO UPDATE SET
                price = $2,
                validity_days = $3,
                updated_at = $4
            RETURNING created_at, updated_at
            ",
            project_id.0,
            price,
            validity_days,
            now,
        )
        .fetch_one(&mut **transaction)
        .await?;

        Ok(Self {
            project_id,
            price,
            validity_days,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }

    /// 获取项目定价
    pub async fn get<'a, E>(
        project_id: ProjectId,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            SELECT project_id, price, validity_days, created_at, updated_at
            FROM project_pricing
            WHERE project_id = $1
            ",
            project_id.0,
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| Self {
            project_id: ProjectId(row.project_id),
            price: row.price,
            validity_days: row.validity_days,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    /// 删除项目定价
    pub async fn delete(
        project_id: ProjectId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query!(
            "DELETE FROM project_pricing WHERE project_id = $1",
            project_id.0,
        )
        .execute(&mut **transaction)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
