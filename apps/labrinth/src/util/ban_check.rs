//! 用户封禁检查工具
//!
//! 提供在各路由中检查用户封禁状态的辅助函数。

use crate::database::models::UserId;
use crate::database::models::user_ban_item::{BanType, UserBan};
use crate::database::redis::RedisPool;
use crate::routes::ApiError;
use chrono::{DateTime, Utc};

/// 封禁检查结果
#[derive(Debug)]
pub struct BanCheckResult {
    pub ban_type: BanType,
    pub reason: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl BanCheckResult {
    /// 转换为 API 错误
    pub fn into_api_error(self) -> ApiError {
        ApiError::CustomAuthentication(format!(
            "您已被{}，原因：{}{}",
            self.ban_type_display(),
            self.reason,
            self.expires_display()
        ))
    }

    fn ban_type_display(&self) -> &'static str {
        match self.ban_type {
            BanType::Global => "禁止登录系统",
            BanType::Resource => "禁止资源操作（上传、创建、编辑等）",
            BanType::Forum => "禁止论坛互动（评论、发帖、编辑百科等）",
        }
    }

    fn expires_display(&self) -> String {
        match self.expires_at {
            Some(expires_at) => {
                format!("。解封时间：{}", expires_at.format("%Y-%m-%d %H:%M"))
            }
            None => "。此为永久封禁".to_string(),
        }
    }
}

/// 检查用户是否被指定类型封禁
///
/// # Arguments
/// * `user_id` - 用户 ID
/// * `ban_type` - 要检查的封禁类型
/// * `pool` - 数据库连接池
/// * `redis` - Redis 连接池
///
/// # Returns
/// * `Ok(())` - 用户未被封禁
/// * `Err(ApiError)` - 用户被封禁，包含封禁信息
pub async fn check_user_ban(
    user_id: UserId,
    ban_type: BanType,
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    if UserBan::is_user_banned(user_id, ban_type.clone(), pool, redis).await? {
        // 获取封禁详情
        let bans = UserBan::get_user_active_bans(user_id, pool, redis).await?;
        if let Some(ban) = bans
            .iter()
            .find(|b| BanType::parse(&b.ban_type) == Some(ban_type.clone()))
        {
            return Err(BanCheckResult {
                ban_type,
                reason: ban.reason.clone(),
                expires_at: ban.expires_at,
            }
            .into_api_error());
        }
    }
    Ok(())
}

/// 检查用户是否被任意封禁类型封禁（批量检查）
///
/// # Arguments
/// * `user_id` - 用户 ID
/// * `ban_types` - 要检查的封禁类型列表
/// * `pool` - 数据库连接池
/// * `redis` - Redis 连接池
///
/// # Returns
/// * `Ok(())` - 用户未被任何指定类型封禁
/// * `Err(ApiError)` - 用户被封禁，包含第一个匹配的封禁信息
pub async fn check_user_bans(
    user_id: UserId,
    ban_types: &[BanType],
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    for ban_type in ban_types {
        check_user_ban(user_id, ban_type.clone(), pool, redis).await?;
    }
    Ok(())
}

/// 检查资源类操作的封禁状态
///
/// 这是一个便捷函数，自动检查 Global 和 Resource 封禁。
/// 用于：上传版本、创建/编辑/删除项目、团队管理、提现等操作。
pub async fn check_resource_ban(
    user_id: UserId,
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    check_user_bans(user_id, &[BanType::Global, BanType::Resource], pool, redis)
        .await
}

/// 检查论坛类操作的封禁状态
///
/// 这是一个便捷函数，自动检查 Global 和 Forum 封禁。
/// 用于：评论、发帖/回帖、编辑百科、发送消息、举报等操作。
pub async fn check_forum_ban(
    user_id: UserId,
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    check_user_bans(user_id, &[BanType::Global, BanType::Forum], pool, redis)
        .await
}

/// 检查全局封禁状态
///
/// 用于：登录、敏感操作等。
pub async fn check_global_ban(
    user_id: UserId,
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<(), ApiError> {
    check_user_ban(user_id, BanType::Global, pool, redis).await
}

/// 获取用户所有活跃封禁的简要信息
///
/// 用于在用户界面显示封禁状态。
pub async fn get_user_ban_summary(
    user_id: UserId,
    pool: &sqlx::PgPool,
    redis: &RedisPool,
) -> Result<Vec<BanSummary>, ApiError> {
    let bans = UserBan::get_user_active_bans(user_id, pool, redis).await?;

    Ok(bans
        .into_iter()
        .map(|ban| BanSummary {
            ban_type: ban.ban_type,
            reason: ban.reason,
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
        })
        .collect())
}

/// 封禁摘要（用户视角）
#[derive(Debug)]
pub struct BanSummary {
    pub ban_type: String,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
