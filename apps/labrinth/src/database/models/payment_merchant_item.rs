//! 支付商户数据库模型
//!
//! 提供卖家支付平台配置的数据库操作。
//! 密钥等敏感信息使用 AES-256-GCM 加密存储。

use crate::database::models::{DatabaseError, UserId};
use crate::util::encrypt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== 支付商户结构体 ====================

/// 支付商户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMerchant {
    pub user_id: UserId,
    pub sid: i32,
    #[serde(skip_serializing)]
    pub secret_key: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建/更新商户配置的构建器
#[derive(Debug, Clone)]
pub struct PaymentMerchantBuilder {
    pub user_id: UserId,
    pub sid: i32,
    pub secret_key: String,
}

impl PaymentMerchantBuilder {
    pub fn new(user_id: UserId, sid: i32, secret_key: String) -> Self {
        Self {
            user_id,
            sid,
            secret_key,
        }
    }

    /// 插入或更新商户配置
    ///
    /// 密钥会在存储前使用 AES-256-GCM 加密
    pub async fn upsert(
        self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        // 加密密钥
        let encrypted_key =
            encrypt::encrypt(&self.secret_key).map_err(|e| {
                DatabaseError::Database2(format!("加密密钥失败: {}", e))
            })?;

        sqlx::query!(
            r#"
            INSERT INTO payment_merchants (user_id, sid, secret_key, verified, created_at, updated_at)
            VALUES ($1, $2, $3, FALSE, NOW(), NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET
                sid = EXCLUDED.sid,
                secret_key = EXCLUDED.secret_key,
                verified = FALSE,
                updated_at = NOW()
            "#,
            self.user_id.0,
            self.sid,
            encrypted_key,
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
}

impl PaymentMerchant {
    /// 解密密钥
    ///
    /// 解密失败时返回错误，由调用方决定如何处理
    fn decrypt_secret_key(encrypted: &str) -> Result<String, DatabaseError> {
        encrypt::decrypt(encrypted).map_err(|e| {
            log::error!("解密商户密钥失败: {}", e);
            DatabaseError::Database2(format!("解密密钥失败: {}", e))
        })
    }

    /// 根据用户 ID 获取商户配置
    pub async fn get_by_user<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<Option<PaymentMerchant>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            r#"
            SELECT user_id, sid, secret_key, verified, created_at, updated_at
            FROM payment_merchants
            WHERE user_id = $1
            "#,
            user_id.0
        )
        .fetch_optional(executor)
        .await?;

        match result {
            Some(r) => Ok(Some(PaymentMerchant {
                user_id: UserId(r.user_id),
                sid: r.sid,
                secret_key: Self::decrypt_secret_key(&r.secret_key)?,
                verified: r.verified.unwrap_or(false),
                created_at: r.created_at.unwrap_or_else(Utc::now),
                updated_at: r.updated_at.unwrap_or_else(Utc::now),
            })),
            None => Ok(None),
        }
    }

    /// 根据 SID 获取商户配置
    pub async fn get_by_sid<'a, E>(
        sid: i32,
        executor: E,
    ) -> Result<Option<PaymentMerchant>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            r#"
            SELECT user_id, sid, secret_key, verified, created_at, updated_at
            FROM payment_merchants
            WHERE sid = $1
            "#,
            sid
        )
        .fetch_optional(executor)
        .await?;

        match result {
            Some(r) => Ok(Some(PaymentMerchant {
                user_id: UserId(r.user_id),
                sid: r.sid,
                secret_key: Self::decrypt_secret_key(&r.secret_key)?,
                verified: r.verified.unwrap_or(false),
                created_at: r.created_at.unwrap_or_else(Utc::now),
                updated_at: r.updated_at.unwrap_or_else(Utc::now),
            })),
            None => Ok(None),
        }
    }

    /// 设置验证状态
    pub async fn set_verified(
        user_id: UserId,
        verified: bool,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        let result = sqlx::query!(
            r#"
            UPDATE payment_merchants
            SET verified = $2, updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id.0,
            verified,
        )
        .execute(&mut **transaction)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::Database2(format!(
                "未找到商户配置: user_id={}",
                user_id.0
            )));
        }

        Ok(())
    }

    /// 删除商户配置
    pub async fn delete(
        user_id: UserId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query!(
            "DELETE FROM payment_merchants WHERE user_id = $1",
            user_id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 检查用户是否已配置商户
    pub async fn exists<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<bool, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM payment_merchants WHERE user_id = $1) as exists",
            user_id.0
        )
        .fetch_one(executor)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// 检查 SID 是否已被其他用户使用
    pub async fn is_sid_used_by_other<'a, E>(
        sid: i32,
        current_user_id: UserId,
        executor: E,
    ) -> Result<bool, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM payment_merchants WHERE sid = $1 AND user_id != $2) as exists",
            sid,
            current_user_id.0
        )
        .fetch_one(executor)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }
}
