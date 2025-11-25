//! 用户封禁系统数据库模型
//!
//! 提供封禁、封禁历史、申诉等功能的数据库操作。

use crate::database::models::{
    BanAppealId, BanHistoryId, DatabaseError, ThreadId, UserBanId, UserId,
};
use crate::database::redis::RedisPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== 缓存命名空间 ====================

/// 用户是否被封禁的状态缓存（按封禁类型）
pub const USER_BAN_ACTIVE_NAMESPACE: &str = "user_bans_active";
/// 用户活跃封禁列表缓存
pub const USER_BANS_LIST_NAMESPACE: &str = "user_bans_list";
/// 单个封禁详情缓存
pub const USER_BAN_DETAIL_NAMESPACE: &str = "user_ban_detail";
/// 缓存穿透保护：空结果标记命名空间
pub const USER_BAN_NULL_NAMESPACE: &str = "user_bans_null";

// ==================== 缓存工具函数 ====================

/// 生成带随机偏移的 TTL，防止缓存雪崩
/// base_ttl: 基础 TTL（秒）
/// 返回值范围: [base_ttl * 0.8, base_ttl * 1.2]
fn random_ttl(base_ttl: i64) -> i64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let offset = (base_ttl as f64 * 0.2) as i64;
    base_ttl + rng.gen_range(-offset..=offset)
}

// ==================== 封禁类型枚举 ====================

/// 封禁类型（数据库层）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BanType {
    /// 全局封禁：无法登录系统
    Global,
    /// 资源类封禁：禁止上传/创建/编辑/删除资源
    Resource,
    /// 论坛类封禁：禁止评论/发帖/百科/消息
    Forum,
}

impl BanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BanType::Global => "global",
            BanType::Resource => "resource",
            BanType::Forum => "forum",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "global" => Some(BanType::Global),
            "resource" => Some(BanType::Resource),
            "forum" => Some(BanType::Forum),
            _ => None,
        }
    }
}

impl std::fmt::Display for BanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ==================== 申诉状态枚举 ====================

/// 申诉状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppealStatus {
    Pending,
    Approved,
    Rejected,
}

impl AppealStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppealStatus::Pending => "pending",
            AppealStatus::Approved => "approved",
            AppealStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AppealStatus::Pending),
            "approved" => Some(AppealStatus::Approved),
            "rejected" => Some(AppealStatus::Rejected),
            _ => None,
        }
    }
}

// ==================== 用户封禁结构 ====================

/// 用户封禁记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserBan {
    pub id: UserBanId,
    pub user_id: UserId,
    pub ban_type: String,
    pub reason: String,
    pub internal_reason: Option<String>,
    pub banned_by: UserId,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
}

/// 用户封禁构建器
pub struct UserBanBuilder {
    pub user_id: UserId,
    pub ban_type: BanType,
    pub reason: String,
    pub internal_reason: Option<String>,
    pub banned_by: UserId,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

impl UserBan {
    /// 插入新的封禁记录
    pub async fn insert(
        id: UserBanId,
        builder: UserBanBuilder,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let now = Utc::now();
        let metadata = builder.metadata.unwrap_or(serde_json::json!({}));

        sqlx::query!(
            "INSERT INTO user_bans (id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9)",
            id.0,
            builder.user_id.0,
            builder.ban_type.as_str(),
            builder.reason,
            builder.internal_reason,
            builder.banned_by.0,
            now,
            builder.expires_at,
            metadata
        )
        .execute(&mut **transaction)
        .await?;

        Ok(UserBan {
            id,
            user_id: builder.user_id,
            ban_type: builder.ban_type.as_str().to_string(),
            reason: builder.reason,
            internal_reason: builder.internal_reason,
            banned_by: builder.banned_by,
            banned_at: now,
            expires_at: builder.expires_at,
            is_active: true,
            metadata,
        })
    }

    /// 根据 ID 获取封禁记录（带缓存和缓存穿透保护）
    pub async fn get(
        id: UserBanId,
        pool: &sqlx::PgPool,
        redis: &RedisPool,
    ) -> Result<Option<Self>, DatabaseError> {
        let cache_key = id.0.to_string();
        let mut redis_conn = redis.connect().await?;

        // 先检查是否有缓存的数据
        if let Some(cached) = redis_conn
            .get_deserialized_from_json::<Self>(
                USER_BAN_DETAIL_NAMESPACE,
                &cache_key,
            )
            .await?
        {
            return Ok(Some(cached));
        }

        // 检查是否有缓存穿透保护标记（表示数据库中不存在）
        if redis_conn
            .get_deserialized_from_json::<bool>(
                USER_BAN_NULL_NAMESPACE,
                &cache_key,
            )
            .await?
            .is_some()
        {
            return Ok(None);
        }

        // 查询数据库
        let result = sqlx::query!(
            "SELECT id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata
             FROM user_bans WHERE id = $1",
            id.0
        )
        .fetch_optional(pool)
        .await?;

        let ban = result.map(|row| UserBan {
            id: UserBanId(row.id),
            user_id: UserId(row.user_id),
            ban_type: row.ban_type,
            reason: row.reason,
            internal_reason: row.internal_reason,
            banned_by: UserId(row.banned_by),
            banned_at: row.banned_at,
            expires_at: row.expires_at,
            is_active: row.is_active,
            metadata: row.metadata,
        });

        // 写入缓存（带随机 TTL 防止缓存雪崩）
        if let Some(ref b) = ban {
            redis_conn
                .set_serialized_to_json(
                    USER_BAN_DETAIL_NAMESPACE,
                    &cache_key,
                    b,
                    Some(random_ttl(600)), // 基础 10 分钟，随机 ±20%
                )
                .await?;
        } else {
            // 缓存穿透保护：缓存空结果（较短 TTL）
            redis_conn
                .set_serialized_to_json(
                    USER_BAN_NULL_NAMESPACE,
                    &cache_key,
                    &true,
                    Some(random_ttl(60)), // 基础 1 分钟，随机 ±20%
                )
                .await?;
        }

        Ok(ban)
    }

    /// 获取用户的所有活跃封禁（带缓存和随机 TTL）
    pub async fn get_user_active_bans(
        user_id: UserId,
        pool: &sqlx::PgPool,
        redis: &RedisPool,
    ) -> Result<Vec<Self>, DatabaseError> {
        // 先检查缓存
        let cache_key = user_id.0.to_string();
        let mut redis_conn = redis.connect().await?;

        if let Some(cached) = redis_conn
            .get_deserialized_from_json::<Vec<Self>>(
                USER_BANS_LIST_NAMESPACE,
                &cache_key,
            )
            .await?
        {
            return Ok(cached);
        }

        // 查询数据库
        let results = sqlx::query!(
            "SELECT id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata
             FROM user_bans
             WHERE user_id = $1
               AND is_active = true
               AND (expires_at IS NULL OR expires_at > NOW())",
            user_id.0
        )
        .fetch_all(pool)
        .await?;

        let bans: Vec<Self> = results
            .into_iter()
            .map(|row| UserBan {
                id: UserBanId(row.id),
                user_id: UserId(row.user_id),
                ban_type: row.ban_type,
                reason: row.reason,
                internal_reason: row.internal_reason,
                banned_by: UserId(row.banned_by),
                banned_at: row.banned_at,
                expires_at: row.expires_at,
                is_active: row.is_active,
                metadata: row.metadata,
            })
            .collect();

        // 写入缓存（带随机 TTL，包括空结果以防止缓存穿透）
        // 基础 5 分钟，随机 ±20%，防止缓存雪崩
        redis_conn
            .set_serialized_to_json(
                USER_BANS_LIST_NAMESPACE,
                &cache_key,
                &bans,
                Some(random_ttl(300)),
            )
            .await?;

        Ok(bans)
    }

    /// 批量获取多个用户的活跃封禁（用于 User 查询）
    pub async fn get_active_bans_for_users(
        user_ids: &[i64],
        pool: &sqlx::PgPool,
        redis: &RedisPool,
    ) -> Result<Vec<Self>, DatabaseError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 尝试从缓存获取每个用户的封禁
        let mut result = Vec::new();
        let mut uncached_ids = Vec::new();

        // 复用同一个 Redis 连接
        let mut redis_conn = redis.connect().await?;

        for &user_id in user_ids {
            let cache_key = user_id.to_string();

            if let Some(cached) = redis_conn
                .get_deserialized_from_json::<Vec<Self>>(
                    USER_BANS_LIST_NAMESPACE,
                    &cache_key,
                )
                .await?
            {
                result.extend(cached);
            } else {
                uncached_ids.push(user_id);
            }
        }

        // 查询未缓存的用户
        if !uncached_ids.is_empty() {
            let results = sqlx::query!(
                "SELECT id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata
                 FROM user_bans
                 WHERE user_id = ANY($1)
                   AND is_active = true
                   AND (expires_at IS NULL OR expires_at > NOW())",
                &uncached_ids
            )
            .fetch_all(pool)
            .await?;

            // 按用户分组并缓存
            let mut user_bans: std::collections::HashMap<i64, Vec<Self>> =
                std::collections::HashMap::new();
            for row in results {
                let ban = UserBan {
                    id: crate::database::models::UserBanId(row.id),
                    user_id: UserId(row.user_id),
                    ban_type: row.ban_type,
                    reason: row.reason,
                    internal_reason: row.internal_reason,
                    banned_by: UserId(row.banned_by),
                    banned_at: row.banned_at,
                    expires_at: row.expires_at,
                    is_active: row.is_active,
                    metadata: row.metadata,
                };
                user_bans.entry(row.user_id).or_default().push(ban.clone());
                result.push(ban);
            }

            // 缓存结果（包括空结果以防止缓存穿透），带随机 TTL
            let mut redis_conn = redis.connect().await?;
            for &user_id in &uncached_ids {
                let bans = user_bans.get(&user_id).cloned().unwrap_or_default();
                redis_conn
                    .set_serialized_to_json(
                        USER_BANS_LIST_NAMESPACE,
                        &user_id.to_string(),
                        &bans,
                        Some(random_ttl(300)), // 基础 5 分钟，随机 ±20%
                    )
                    .await?;
            }
        }

        Ok(result)
    }

    /// 检查用户是否被指定类型封禁（带缓存和随机 TTL）
    pub async fn is_user_banned(
        user_id: UserId,
        ban_type: BanType,
        pool: &sqlx::PgPool,
        redis: &RedisPool,
    ) -> Result<bool, DatabaseError> {
        // 先检查缓存
        let cache_key = format!("{}:{}", user_id.0, ban_type.as_str());
        let mut redis_conn = redis.connect().await?;

        if let Some(cached) = redis_conn
            .get_deserialized_from_json::<bool>(
                USER_BAN_ACTIVE_NAMESPACE,
                &cache_key,
            )
            .await?
        {
            return Ok(cached);
        }

        // 查询数据库
        let result = sqlx::query!(
            "SELECT EXISTS(
                SELECT 1 FROM user_bans
                WHERE user_id = $1
                  AND ban_type = $2
                  AND is_active = true
                  AND (expires_at IS NULL OR expires_at > NOW())
            ) as exists",
            user_id.0,
            ban_type.as_str()
        )
        .fetch_one(pool)
        .await?;

        let is_banned = result.exists.unwrap_or(false);

        // 写入缓存（带随机 TTL 防止缓存雪崩）
        // 基础 5 分钟，随机 ±20%（也缓存 false 结果防止穿透）
        redis_conn
            .set_serialized_to_json(
                USER_BAN_ACTIVE_NAMESPACE,
                &cache_key,
                &is_banned,
                Some(random_ttl(300)),
            )
            .await?;

        Ok(is_banned)
    }

    /// 检查用户是否被任意封禁类型封禁
    pub async fn check_user_bans(
        user_id: UserId,
        ban_types: &[BanType],
        pool: &sqlx::PgPool,
        redis: &RedisPool,
    ) -> Result<Option<BanType>, DatabaseError> {
        for ban_type in ban_types {
            if Self::is_user_banned(user_id, ban_type.clone(), pool, redis)
                .await?
            {
                return Ok(Some(ban_type.clone()));
            }
        }
        Ok(None)
    }

    /// 获取用户的所有封禁记录（包括已过期/已解除的）
    pub async fn get_user_all_bans(
        user_id: UserId,
        include_inactive: bool,
        limit: i64,
        offset: i64,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<Self>, DatabaseError> {
        let results = sqlx::query!(
            "SELECT id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata
             FROM user_bans
             WHERE user_id = $1 AND ($2 OR is_active = true)
             ORDER BY banned_at DESC
             LIMIT $3 OFFSET $4",
            user_id.0,
            include_inactive,
            limit,
            offset
        )
        .fetch_all(exec)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| UserBan {
                id: UserBanId(row.id),
                user_id: UserId(row.user_id),
                ban_type: row.ban_type,
                reason: row.reason,
                internal_reason: row.internal_reason,
                banned_by: UserId(row.banned_by),
                banned_at: row.banned_at,
                expires_at: row.expires_at,
                is_active: row.is_active,
                metadata: row.metadata,
            })
            .collect())
    }

    /// 解除封禁（将 is_active 设为 false）
    pub async fn deactivate(
        id: UserBanId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE user_bans SET is_active = false WHERE id = $1",
            id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    /// 更新封禁信息
    pub async fn update(
        id: UserBanId,
        reason: Option<String>,
        internal_reason: Option<String>,
        expires_at: Option<Option<DateTime<Utc>>>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        if let Some(reason) = reason {
            sqlx::query!(
                "UPDATE user_bans SET reason = $1 WHERE id = $2",
                reason,
                id.0
            )
            .execute(&mut **transaction)
            .await?;
        }

        if let Some(internal_reason) = internal_reason {
            sqlx::query!(
                "UPDATE user_bans SET internal_reason = $1 WHERE id = $2",
                internal_reason,
                id.0
            )
            .execute(&mut **transaction)
            .await?;
        }

        if let Some(expires_at) = expires_at {
            sqlx::query!(
                "UPDATE user_bans SET expires_at = $1 WHERE id = $2",
                expires_at,
                id.0
            )
            .execute(&mut **transaction)
            .await?;
        }

        Ok(())
    }

    /// 清除用户封禁缓存（包括状态缓存和列表缓存）
    pub async fn clear_cache(
        user_id: UserId,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        use redis::cmd;

        let mut redis_conn = redis.connect().await?;
        let meta_namespace = std::env::var("REDIS_NAMESPACE")
            .unwrap_or_else(|_| "labrinth".to_string());

        let mut keys_to_delete = Vec::new();

        // 清除所有封禁类型的状态缓存（数据键 + 锁键）
        for ban_type in &[BanType::Global, BanType::Resource, BanType::Forum] {
            let cache_key = format!("{}:{}", user_id.0, ban_type.as_str());
            // 数据键
            keys_to_delete.push(format!(
                "{}_{}:{}",
                meta_namespace, USER_BAN_ACTIVE_NAMESPACE, cache_key
            ));
            // 锁键
            keys_to_delete.push(format!(
                "{}_{}:{}/lock",
                meta_namespace, USER_BAN_ACTIVE_NAMESPACE, cache_key
            ));
        }

        // 清除用户活跃封禁列表缓存（数据键 + 锁键）
        let list_key = user_id.0.to_string();
        keys_to_delete.push(format!(
            "{}_{}:{}",
            meta_namespace, USER_BANS_LIST_NAMESPACE, list_key
        ));
        keys_to_delete.push(format!(
            "{}_{}:{}/lock",
            meta_namespace, USER_BANS_LIST_NAMESPACE, list_key
        ));

        // 批量删除所有键（包括锁键）
        if !keys_to_delete.is_empty() {
            cmd("DEL")
                .arg(&keys_to_delete)
                .query_async::<()>(&mut redis_conn.connection)
                .await?;
        }

        Ok(())
    }

    /// 清除单个封禁的详情缓存（同时清除缓存穿透保护标记）
    pub async fn clear_ban_cache(
        ban_id: UserBanId,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        use redis::cmd;

        let mut redis_conn = redis.connect().await?;
        let meta_namespace = std::env::var("REDIS_NAMESPACE")
            .unwrap_or_else(|_| "labrinth".to_string());
        let cache_key = ban_id.0.to_string();

        let keys_to_delete = vec![
            // 清除封禁详情缓存（数据键 + 锁键）
            format!(
                "{}_{}:{}",
                meta_namespace, USER_BAN_DETAIL_NAMESPACE, cache_key
            ),
            format!(
                "{}_{}:{}/lock",
                meta_namespace, USER_BAN_DETAIL_NAMESPACE, cache_key
            ),
            // 清除缓存穿透保护标记（数据键 + 锁键）
            format!(
                "{}_{}:{}",
                meta_namespace, USER_BAN_NULL_NAMESPACE, cache_key
            ),
            format!(
                "{}_{}:{}/lock",
                meta_namespace, USER_BAN_NULL_NAMESPACE, cache_key
            ),
        ];

        // 批量删除所有键（包括锁键）
        cmd("DEL")
            .arg(&keys_to_delete)
            .query_async::<()>(&mut redis_conn.connection)
            .await?;

        Ok(())
    }
}

// ==================== 封禁历史记录 ====================

/// 封禁操作历史
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BanHistory {
    pub id: BanHistoryId,
    pub ban_id: UserBanId,
    pub action: String,
    pub operator_id: UserId,
    pub operated_at: DateTime<Utc>,
    pub old_data: Option<serde_json::Value>,
    pub new_data: serde_json::Value,
    pub reason: String,
}

/// 封禁历史构建器
pub struct BanHistoryBuilder {
    pub ban_id: UserBanId,
    pub action: String,
    pub operator_id: UserId,
    pub old_data: Option<serde_json::Value>,
    pub new_data: serde_json::Value,
    pub reason: String,
}

impl BanHistory {
    /// 插入新的历史记录
    pub async fn insert(
        id: BanHistoryId,
        builder: BanHistoryBuilder,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let now = Utc::now();

        sqlx::query!(
            "INSERT INTO user_ban_history (id, ban_id, action, operator_id, operated_at, old_data, new_data, reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            id.0,
            builder.ban_id.0,
            builder.action,
            builder.operator_id.0,
            now,
            builder.old_data,
            builder.new_data,
            builder.reason
        )
        .execute(&mut **transaction)
        .await?;

        Ok(BanHistory {
            id,
            ban_id: builder.ban_id,
            action: builder.action,
            operator_id: builder.operator_id,
            operated_at: now,
            old_data: builder.old_data,
            new_data: builder.new_data,
            reason: builder.reason,
        })
    }

    /// 获取封禁的所有历史记录
    pub async fn get_by_ban_id(
        ban_id: UserBanId,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<Self>, DatabaseError> {
        let results = sqlx::query!(
            "SELECT id, ban_id, action, operator_id, operated_at, old_data, new_data, reason
             FROM user_ban_history
             WHERE ban_id = $1
             ORDER BY operated_at DESC",
            ban_id.0
        )
        .fetch_all(exec)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| BanHistory {
                id: BanHistoryId(row.id),
                ban_id: UserBanId(row.ban_id),
                action: row.action,
                operator_id: UserId(row.operator_id),
                operated_at: row.operated_at,
                old_data: row.old_data,
                new_data: row.new_data,
                reason: row.reason,
            })
            .collect())
    }
}

// ==================== 封禁申诉 ====================

/// 封禁申诉记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BanAppeal {
    pub id: BanAppealId,
    pub ban_id: UserBanId,
    pub user_id: UserId,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub reviewed_by: Option<UserId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
    pub thread_id: Option<ThreadId>,
}

/// 封禁申诉构建器
pub struct BanAppealBuilder {
    pub ban_id: UserBanId,
    pub user_id: UserId,
    pub reason: String,
}

impl BanAppeal {
    /// 插入新的申诉记录
    pub async fn insert(
        id: BanAppealId,
        builder: BanAppealBuilder,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Self, DatabaseError> {
        let now = Utc::now();

        sqlx::query!(
            "INSERT INTO user_ban_appeals (id, ban_id, user_id, reason, status, created_at)
             VALUES ($1, $2, $3, $4, 'pending', $5)",
            id.0,
            builder.ban_id.0,
            builder.user_id.0,
            builder.reason,
            now
        )
        .execute(&mut **transaction)
        .await?;

        Ok(BanAppeal {
            id,
            ban_id: builder.ban_id,
            user_id: builder.user_id,
            reason: builder.reason,
            status: "pending".to_string(),
            created_at: now,
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            thread_id: None,
        })
    }

    /// 设置申诉的线程ID
    pub async fn set_thread_id(
        id: BanAppealId,
        thread_id: ThreadId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE user_ban_appeals SET thread_id = $1 WHERE id = $2",
            thread_id.0,
            id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    /// 根据 ID 获取申诉记录
    pub async fn get(
        id: BanAppealId,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Option<Self>, DatabaseError> {
        let result = sqlx::query!(
            "SELECT id, ban_id, user_id, reason, status, created_at, reviewed_by, reviewed_at, review_notes, thread_id
             FROM user_ban_appeals WHERE id = $1",
            id.0
        )
        .fetch_optional(exec)
        .await?;

        Ok(result.map(|row| BanAppeal {
            id: BanAppealId(row.id),
            ban_id: UserBanId(row.ban_id),
            user_id: UserId(row.user_id),
            reason: row.reason,
            status: row.status,
            created_at: row.created_at,
            reviewed_by: row.reviewed_by.map(UserId),
            reviewed_at: row.reviewed_at,
            review_notes: row.review_notes,
            thread_id: row.thread_id.map(ThreadId),
        }))
    }

    /// 根据封禁 ID 获取申诉记录
    pub async fn get_by_ban_id(
        ban_id: UserBanId,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Option<Self>, DatabaseError> {
        let result = sqlx::query!(
            "SELECT id, ban_id, user_id, reason, status, created_at, reviewed_by, reviewed_at, review_notes, thread_id
             FROM user_ban_appeals WHERE ban_id = $1",
            ban_id.0
        )
        .fetch_optional(exec)
        .await?;

        Ok(result.map(|row| BanAppeal {
            id: BanAppealId(row.id),
            ban_id: UserBanId(row.ban_id),
            user_id: UserId(row.user_id),
            reason: row.reason,
            status: row.status,
            created_at: row.created_at,
            reviewed_by: row.reviewed_by.map(UserId),
            reviewed_at: row.reviewed_at,
            review_notes: row.review_notes,
            thread_id: row.thread_id.map(ThreadId),
        }))
    }

    /// 获取用户的所有申诉记录
    pub async fn get_user_appeals(
        user_id: UserId,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<Self>, DatabaseError> {
        let results = sqlx::query!(
            "SELECT id, ban_id, user_id, reason, status, created_at, reviewed_by, reviewed_at, review_notes, thread_id
             FROM user_ban_appeals
             WHERE user_id = $1
             ORDER BY created_at DESC",
            user_id.0
        )
        .fetch_all(exec)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| BanAppeal {
                id: BanAppealId(row.id),
                ban_id: UserBanId(row.ban_id),
                user_id: UserId(row.user_id),
                reason: row.reason,
                status: row.status,
                created_at: row.created_at,
                reviewed_by: row.reviewed_by.map(UserId),
                reviewed_at: row.reviewed_at,
                review_notes: row.review_notes,
                thread_id: row.thread_id.map(ThreadId),
            })
            .collect())
    }

    /// 审核申诉
    pub async fn review(
        id: BanAppealId,
        status: AppealStatus,
        reviewer_id: UserId,
        review_notes: Option<String>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        sqlx::query!(
            "UPDATE user_ban_appeals
             SET status = $1, reviewed_by = $2, reviewed_at = $3, review_notes = $4
             WHERE id = $5",
            status.as_str(),
            reviewer_id.0,
            now,
            review_notes,
            id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    /// 获取待处理的申诉列表
    pub async fn get_pending_appeals(
        limit: i64,
        offset: i64,
        exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<Self>, DatabaseError> {
        let results = sqlx::query!(
            "SELECT id, ban_id, user_id, reason, status, created_at, reviewed_by, reviewed_at, review_notes, thread_id
             FROM user_ban_appeals
             WHERE status = 'pending'
             ORDER BY created_at ASC
             LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(exec)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| BanAppeal {
                id: BanAppealId(row.id),
                ban_id: UserBanId(row.ban_id),
                user_id: UserId(row.user_id),
                reason: row.reason,
                status: row.status,
                created_at: row.created_at,
                reviewed_by: row.reviewed_by.map(UserId),
                reviewed_at: row.reviewed_at,
                review_notes: row.review_notes,
                thread_id: row.thread_id.map(ThreadId),
            })
            .collect())
    }
}

// ==================== 辅助函数 ====================

/// 标记过期的封禁为不活跃
pub async fn deactivate_expired_bans(
    pool: &sqlx::PgPool,
) -> Result<u64, DatabaseError> {
    let result = sqlx::query!(
        "UPDATE user_bans
         SET is_active = false
         WHERE is_active = true
           AND expires_at IS NOT NULL
           AND expires_at < NOW()"
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// 获取即将到期的封禁（用于发送提醒通知）
pub async fn get_expiring_bans(
    days_before: i32,
    pool: &sqlx::PgPool,
) -> Result<Vec<UserBan>, DatabaseError> {
    let results = sqlx::query!(
        "SELECT id, user_id, ban_type, reason, internal_reason, banned_by, banned_at, expires_at, is_active, metadata
         FROM user_bans
         WHERE is_active = true
           AND expires_at IS NOT NULL
           AND expires_at BETWEEN NOW() AND NOW() + ($1 || ' days')::interval
           AND (metadata->>'expiry_notified')::boolean IS NOT TRUE",
        days_before.to_string()
    )
    .fetch_all(pool)
    .await?;

    Ok(results
        .into_iter()
        .map(|row| UserBan {
            id: UserBanId(row.id),
            user_id: UserId(row.user_id),
            ban_type: row.ban_type,
            reason: row.reason,
            internal_reason: row.internal_reason,
            banned_by: UserId(row.banned_by),
            banned_at: row.banned_at,
            expires_at: row.expires_at,
            is_active: row.is_active,
            metadata: row.metadata,
        })
        .collect())
}

/// 标记封禁已发送过期提醒
pub async fn mark_expiry_notified(
    ban_id: UserBanId,
    pool: &sqlx::PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query!(
        "UPDATE user_bans
         SET metadata = metadata || '{\"expiry_notified\": true}'::jsonb
         WHERE id = $1",
        ban_id.0
    )
    .execute(pool)
    .await?;

    Ok(())
}
