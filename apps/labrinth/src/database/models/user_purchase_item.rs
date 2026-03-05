use super::DatabaseError;
use super::ids::*;
use crate::database::redis::RedisPool;
use chrono::{DateTime, Utc};
use redis::cmd;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Redis 缓存 namespace 和过期时间
const USER_PURCHASES_NAMESPACE: &str = "user_purchases";
const USER_PURCHASES_EXPIRY_SECS: i64 = 300; // 5 分钟

/// 购买状态
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStatus {
    Active,   // 有效
    Expired,  // 已过期
    Refunded, // 已退款
}

impl PurchaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Refunded => "refunded",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "expired" => Self::Expired,
            "refunded" => Self::Refunded,
            // 未知状态默认为 Expired，避免意外授权
            _ => {
                tracing::warn!("未知的购买状态: {}, 默认为 expired", s);
                Self::Expired
            }
        }
    }
}

/// 用户购买记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPurchase {
    pub id: UserPurchaseId,
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub order_no: Option<String>,
    pub amount: Decimal,
    pub purchased_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: PurchaseStatus,
}

impl UserPurchase {
    /// 创建购买记录
    pub async fn create(
        user_id: UserId,
        project_id: ProjectId,
        order_no: Option<String>,
        amount: Decimal,
        expires_at: Option<DateTime<Utc>>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let new_id = generate_user_purchase_id(&mut *transaction).await?;
        let now = Utc::now();

        // 使用 RETURNING 获取实际的记录 ID 和 purchased_at
        // 当发生冲突（续费）时：
        // - id 和 purchased_at 保持原值（首次购买时间）
        // - amount 更新为本次支付金额（不累加，因为有订单表记录历史）
        // - expires_at 和 status 更新
        let result = sqlx::query!(
            "
            INSERT INTO user_purchases (id, user_id, project_id, order_no, amount, purchased_at, expires_at, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active')
            ON CONFLICT (user_id, project_id) DO UPDATE SET
                order_no = COALESCE($4, user_purchases.order_no),
                amount = $5,
                expires_at = $7,
                status = 'active'
            RETURNING id, purchased_at
            ",
            new_id.0,
            user_id.0,
            project_id.0,
            order_no.as_deref(),
            amount,
            now,
            expires_at,
        )
        .fetch_one(&mut **transaction)
        .await?;

        Ok(Self {
            id: UserPurchaseId(result.id),
            user_id,
            project_id,
            order_no,
            amount,
            purchased_at: result.purchased_at,
            expires_at,
            status: PurchaseStatus::Active,
        })
    }

    /// 检查用户是否已购买项目（且未过期）
    pub async fn check_access<'a, E>(
        user_id: UserId,
        project_id: ProjectId,
        executor: E,
    ) -> Result<bool, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            SELECT id FROM user_purchases
            WHERE user_id = $1 AND project_id = $2 AND status = 'active'
              AND (expires_at IS NULL OR expires_at > NOW())
            ",
            user_id.0,
            project_id.0,
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.is_some())
    }

    /// 获取用户对项目的购买记录
    pub async fn get<'a, E>(
        user_id: UserId,
        project_id: ProjectId,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "
            SELECT id, user_id, project_id, order_no, amount, purchased_at, expires_at, status
            FROM user_purchases
            WHERE user_id = $1 AND project_id = $2
            ",
            user_id.0,
            project_id.0,
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| Self {
            id: UserPurchaseId(row.id),
            user_id: UserId(row.user_id),
            project_id: ProjectId(row.project_id),
            order_no: row.order_no,
            amount: row.amount,
            purchased_at: row.purchased_at,
            expires_at: row.expires_at,
            status: PurchaseStatus::from_string(&row.status),
        }))
    }

    /// 获取用户的所有购买记录
    pub async fn get_user_purchases<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<Vec<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let results = sqlx::query!(
            "
            SELECT id, user_id, project_id, order_no, amount, purchased_at, expires_at, status
            FROM user_purchases
            WHERE user_id = $1
            ORDER BY purchased_at DESC
            ",
            user_id.0,
        )
        .fetch_all(executor)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| Self {
                id: UserPurchaseId(row.id),
                user_id: UserId(row.user_id),
                project_id: ProjectId(row.project_id),
                order_no: row.order_no,
                amount: row.amount,
                purchased_at: row.purchased_at,
                expires_at: row.expires_at,
                status: PurchaseStatus::from_string(&row.status),
            })
            .collect())
    }

    /// 获取项目的所有购买者
    pub async fn get_project_purchasers<'a, E>(
        project_id: ProjectId,
        executor: E,
    ) -> Result<Vec<Self>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let results = sqlx::query!(
            "
            SELECT id, user_id, project_id, order_no, amount, purchased_at, expires_at, status
            FROM user_purchases
            WHERE project_id = $1 AND status = 'active'
            ORDER BY purchased_at DESC
            ",
            project_id.0,
        )
        .fetch_all(executor)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| Self {
                id: UserPurchaseId(row.id),
                user_id: UserId(row.user_id),
                project_id: ProjectId(row.project_id),
                order_no: row.order_no,
                amount: row.amount,
                purchased_at: row.purchased_at,
                expires_at: row.expires_at,
                status: PurchaseStatus::from_string(&row.status),
            })
            .collect())
    }

    /// 更新过期的购买记录状态
    pub async fn update_expired_purchases(
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            "
            UPDATE user_purchases
            SET status = 'expired'
            WHERE status = 'active' AND expires_at IS NOT NULL AND expires_at <= NOW()
            "
        )
        .execute(&mut **transaction)
        .await?;

        Ok(result.rows_affected())
    }

    /// 删除用户的购买记录（撤销授权）
    pub async fn revoke(
        user_id: UserId,
        project_id: ProjectId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query!(
            "
            DELETE FROM user_purchases
            WHERE user_id = $1 AND project_id = $2
            ",
            user_id.0,
            project_id.0,
        )
        .execute(&mut **transaction)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== 带 Redis 缓存的方法 ====================

    /// 获取用户已购买的项目 ID 集合（带缓存）
    ///
    /// 缓存策略：
    /// - Key: `user_purchases:{user_id}`
    /// - Value: Set<project_id>
    /// - 过期时间: 5 分钟
    pub async fn get_user_purchased_project_ids_cached<'a, E>(
        user_id: UserId,
        executor: E,
        redis: &RedisPool,
    ) -> Result<HashSet<i64>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let cache_key = format!("{}:{}", USER_PURCHASES_NAMESPACE, user_id.0);

        // 尝试从缓存获取
        if let Ok(mut conn) = redis.pool.get().await {
            let cached: Result<Vec<String>, _> = cmd("SMEMBERS")
                .arg(&cache_key)
                .query_async(&mut *conn)
                .await;

            if let Ok(members) = cached
                && !members.is_empty()
            {
                let project_ids: HashSet<i64> = members
                    .into_iter()
                    .filter_map(|s| s.parse::<i64>().ok())
                    .collect();
                return Ok(project_ids);
            }
        }

        // 缓存未命中，从数据库查询
        let now = Utc::now();
        let results = sqlx::query!(
            "
            SELECT project_id FROM user_purchases
            WHERE user_id = $1 AND status = 'active'
              AND (expires_at IS NULL OR expires_at > $2)
            ",
            user_id.0,
            now,
        )
        .fetch_all(executor)
        .await?;

        let project_ids: HashSet<i64> =
            results.into_iter().map(|row| row.project_id).collect();

        // 写入缓存
        if let Ok(mut conn) = redis.pool.get().await {
            // 使用 pipeline 批量操作
            let mut pipe = redis::pipe();

            // 先删除旧缓存
            pipe.del(&cache_key);

            if !project_ids.is_empty() {
                // 添加所有项目 ID 到集合
                for pid in &project_ids {
                    pipe.sadd(&cache_key, pid.to_string());
                }
            } else {
                // 如果没有购买记录，添加一个占位符（避免缓存穿透）
                pipe.sadd(&cache_key, "_empty_");
            }

            // 设置过期时间
            pipe.expire(&cache_key, USER_PURCHASES_EXPIRY_SECS);

            let _: Result<(), _> = pipe.query_async(&mut *conn).await;
        }

        Ok(project_ids)
    }

    /// 批量检查用户是否购买了指定项目（带缓存）
    ///
    /// 返回用户已购买的项目 ID 集合（只包含在 project_ids 中且已购买的）
    pub async fn check_purchases_batch_cached<'a, E>(
        user_id: UserId,
        project_ids: &[ProjectId],
        executor: E,
        redis: &RedisPool,
    ) -> Result<HashSet<i64>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        if project_ids.is_empty() {
            return Ok(HashSet::new());
        }

        let all_purchased = Self::get_user_purchased_project_ids_cached(
            user_id, executor, redis,
        )
        .await?;

        // 过滤出用户购买的项目
        let purchased: HashSet<i64> = project_ids
            .iter()
            .filter(|pid| all_purchased.contains(&pid.0))
            .map(|pid| pid.0)
            .collect();

        Ok(purchased)
    }

    /// 清除用户购买缓存
    ///
    /// 在以下场景调用：
    /// - 用户完成新购买
    /// - 用户退款
    /// - 购买过期
    pub async fn clear_user_purchase_cache(
        user_id: UserId,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let cache_key = format!("{}:{}", USER_PURCHASES_NAMESPACE, user_id.0);

        if let Ok(mut conn) = redis.pool.get().await {
            let _: Result<(), _> =
                cmd("DEL").arg(&cache_key).query_async(&mut *conn).await;
        }

        Ok(())
    }

    /// 向用户购买缓存添加项目（用于购买成功后立即更新缓存）
    pub async fn add_to_user_purchase_cache(
        user_id: UserId,
        project_id: ProjectId,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let cache_key = format!("{}:{}", USER_PURCHASES_NAMESPACE, user_id.0);

        if let Ok(mut conn) = redis.pool.get().await {
            // 先检查缓存是否存在
            let exists: Result<bool, _> =
                cmd("EXISTS").arg(&cache_key).query_async(&mut *conn).await;

            if let Ok(true) = exists {
                // 缓存存在，添加新的项目 ID
                let mut pipe = redis::pipe();
                // 移除可能的空占位符
                pipe.srem(&cache_key, "_empty_");
                // 添加新项目
                pipe.sadd(&cache_key, project_id.0.to_string());
                // 刷新过期时间
                pipe.expire(&cache_key, USER_PURCHASES_EXPIRY_SECS);
                let _: Result<(), _> = pipe.query_async(&mut *conn).await;
            }
            // 如果缓存不存在，不需要处理（下次查询时会重新加载）
        }

        Ok(())
    }

    /// 从用户购买缓存移除项目（用于退款后立即更新缓存）
    pub async fn remove_from_user_purchase_cache(
        user_id: UserId,
        project_id: ProjectId,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let cache_key = format!("{}:{}", USER_PURCHASES_NAMESPACE, user_id.0);

        if let Ok(mut conn) = redis.pool.get().await {
            let _: Result<(), _> = cmd("SREM")
                .arg(&cache_key)
                .arg(project_id.0.to_string())
                .query_async(&mut *conn)
                .await;
        }

        Ok(())
    }
}
