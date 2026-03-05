//! 高级创作者申请数据库模型
//!
//! 提供高级创作者申请的数据库操作。
//! 身份证号等敏感信息使用 AES-256-GCM 加密存储。

use crate::database::models::{DatabaseError, UserId};
use crate::util::encrypt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== 申请状态枚举 ====================

/// 申请状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationStatus {
    Pending,
    Approved,
    Rejected,
}

impl ApplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationStatus::Pending => "pending",
            ApplicationStatus::Approved => "approved",
            ApplicationStatus::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(ApplicationStatus::Pending),
            "approved" => Some(ApplicationStatus::Approved),
            "rejected" => Some(ApplicationStatus::Rejected),
            _ => None,
        }
    }
}

impl std::fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ==================== 创作者申请结构体 ====================

/// 高级创作者申请
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorApplication {
    pub id: i64,
    pub user_id: UserId,
    pub real_name: String,
    pub contact_info: String,
    pub id_card_number: Option<String>,
    pub portfolio_links: Option<String>,
    pub application_reason: Option<String>,
    pub status: ApplicationStatus,
    pub reviewer_id: Option<UserId>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// 关联的对话线程 ID
    pub thread_id: Option<crate::database::models::ThreadId>,
}

/// 创建申请的构建器
#[derive(Debug, Clone)]
pub struct CreatorApplicationBuilder {
    pub user_id: UserId,
    pub real_name: String,
    pub contact_info: String,
    pub id_card_number: Option<String>,
    pub portfolio_links: Option<String>,
    pub application_reason: Option<String>,
}

impl CreatorApplicationBuilder {
    pub fn new(
        user_id: UserId,
        real_name: String,
        contact_info: String,
    ) -> Self {
        Self {
            user_id,
            real_name,
            contact_info,
            id_card_number: None,
            portfolio_links: None,
            application_reason: None,
        }
    }

    /// 插入新申请
    ///
    /// 身份证号会在存储前使用 AES-256-GCM 加密
    pub async fn insert(
        self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<i64, DatabaseError> {
        // 加密身份证号
        let encrypted_id_card = match &self.id_card_number {
            Some(id_card) if !id_card.is_empty() => {
                Some(encrypt::encrypt(id_card).map_err(|e| {
                    DatabaseError::Database2(format!("加密身份证号失败: {}", e))
                })?)
            }
            _ => None,
        };

        let result = sqlx::query!(
            r#"
            INSERT INTO creator_applications (
                user_id, real_name, contact_info, id_card_number,
                portfolio_links, application_reason, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending')
            RETURNING id
            "#,
            self.user_id.0,
            self.real_name,
            self.contact_info,
            encrypted_id_card,
            self.portfolio_links,
            self.application_reason,
        )
        .fetch_one(&mut **transaction)
        .await?;

        Ok(result.id)
    }
}

impl CreatorApplication {
    /// 解密身份证号
    ///
    /// 如果解密失败，记录警告日志并返回 None（不返回加密值以避免泄露）
    fn decrypt_id_card(encrypted: Option<String>) -> Option<String> {
        encrypted.and_then(|encrypted_value| {
            match encrypt::decrypt(&encrypted_value) {
                Ok(decrypted) => Some(decrypted),
                Err(e) => {
                    log::warn!(
                        "解密身份证号失败: {}，可能是旧数据或密钥变更",
                        e
                    );
                    // 解密失败时返回 None，避免泄露加密后的数据
                    None
                }
            }
        })
    }

    /// 根据 ID 获取申请
    pub async fn get_by_id<'a, E>(
        id: i64,
        executor: E,
    ) -> Result<Option<CreatorApplication>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            r#"
            SELECT id, user_id, real_name, contact_info, id_card_number,
                   portfolio_links, application_reason, status,
                   reviewer_id, review_note, created_at, reviewed_at, thread_id
            FROM creator_applications
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|r| CreatorApplication {
            id: r.id,
            user_id: UserId(r.user_id),
            real_name: r.real_name,
            contact_info: r.contact_info,
            id_card_number: Self::decrypt_id_card(r.id_card_number),
            portfolio_links: r.portfolio_links,
            application_reason: r.application_reason,
            status: ApplicationStatus::parse(&r.status)
                .unwrap_or(ApplicationStatus::Pending),
            reviewer_id: r.reviewer_id.map(UserId),
            review_note: r.review_note,
            created_at: r.created_at,
            reviewed_at: r.reviewed_at,
            thread_id: r.thread_id.map(crate::database::models::ThreadId),
        }))
    }

    /// 获取用户的最新申请
    pub async fn get_by_user<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<Option<CreatorApplication>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            r#"
            SELECT id, user_id, real_name, contact_info, id_card_number,
                   portfolio_links, application_reason, status,
                   reviewer_id, review_note, created_at, reviewed_at, thread_id
            FROM creator_applications
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id.0
        )
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|r| CreatorApplication {
            id: r.id,
            user_id: UserId(r.user_id),
            real_name: r.real_name,
            contact_info: r.contact_info,
            id_card_number: Self::decrypt_id_card(r.id_card_number),
            portfolio_links: r.portfolio_links,
            application_reason: r.application_reason,
            status: ApplicationStatus::parse(&r.status)
                .unwrap_or(ApplicationStatus::Pending),
            reviewer_id: r.reviewer_id.map(UserId),
            review_note: r.review_note,
            created_at: r.created_at,
            reviewed_at: r.reviewed_at,
            thread_id: r.thread_id.map(crate::database::models::ThreadId),
        }))
    }

    /// 获取待审核的申请列表
    pub async fn get_pending<'a, E>(
        limit: i64,
        offset: i64,
        executor: E,
    ) -> Result<Vec<CreatorApplication>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let results = sqlx::query!(
            r#"
            SELECT id, user_id, real_name, contact_info, id_card_number,
                   portfolio_links, application_reason, status,
                   reviewer_id, review_note, created_at, reviewed_at, thread_id
            FROM creator_applications
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(executor)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| CreatorApplication {
                id: r.id,
                user_id: UserId(r.user_id),
                real_name: r.real_name,
                contact_info: r.contact_info,
                id_card_number: Self::decrypt_id_card(r.id_card_number),
                portfolio_links: r.portfolio_links,
                application_reason: r.application_reason,
                status: ApplicationStatus::parse(&r.status)
                    .unwrap_or(ApplicationStatus::Pending),
                reviewer_id: r.reviewer_id.map(UserId),
                review_note: r.review_note,
                created_at: r.created_at,
                reviewed_at: r.reviewed_at,
                thread_id: r.thread_id.map(crate::database::models::ThreadId),
            })
            .collect())
    }

    /// 获取申请列表（支持状态筛选）
    pub async fn get_list(
        status: Option<ApplicationStatus>,
        limit: i64,
        offset: i64,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<Vec<CreatorApplication>, DatabaseError> {
        let status_str = status.map(|s| s.to_string());

        // 使用 sqlx::query_as 动态构建查询
        let results = sqlx::query_as::<
            _,
            (
                i64,
                i64,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                Option<i64>,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<i64>,
            ),
        >(
            r#"
            SELECT id, user_id, real_name, contact_info, id_card_number,
                   portfolio_links, application_reason, status,
                   reviewer_id, review_note, created_at, reviewed_at, thread_id
            FROM creator_applications
            WHERE ($1::text IS NULL OR status = $1::text)
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status_str.clone())
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| CreatorApplication {
                id: r.0,
                user_id: UserId(r.1),
                real_name: r.2,
                contact_info: r.3,
                id_card_number: Self::decrypt_id_card(r.4),
                portfolio_links: r.5,
                application_reason: r.6,
                status: ApplicationStatus::parse(&r.7)
                    .unwrap_or(ApplicationStatus::Pending),
                reviewer_id: r.8.map(UserId),
                review_note: r.9,
                created_at: r.10,
                reviewed_at: r.11,
                thread_id: r.12.map(crate::database::models::ThreadId),
            })
            .collect())
    }

    /// 按状态统计申请数量
    pub async fn count_by_status(
        status: Option<ApplicationStatus>,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<i64, DatabaseError> {
        let status_str = status.map(|s| s.to_string());

        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM creator_applications WHERE ($1::text IS NULL OR status = $1::text)"
        )
        .bind(status_str)
        .fetch_one(executor)
        .await?;

        Ok(result)
    }

    /// 设置申请关联的线程 ID
    pub async fn set_thread_id(
        id: i64,
        thread_id: crate::database::models::ThreadId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        let result = sqlx::query!(
            r#"
            UPDATE creator_applications
            SET thread_id = $1
            WHERE id = $2
            "#,
            thread_id.0,
            id
        )
        .execute(&mut **transaction)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::Database2(format!(
                "未找到创作者申请记录: {}",
                id
            )));
        }

        Ok(())
    }

    /// 批准申请
    pub async fn approve(
        id: i64,
        reviewer_id: UserId,
        review_note: Option<&str>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        // 更新申请状态
        sqlx::query!(
            r#"
            UPDATE creator_applications
            SET status = 'approved',
                reviewer_id = $2,
                review_note = $3,
                reviewed_at = NOW()
            WHERE id = $1
            "#,
            id,
            reviewer_id.0,
            review_note
        )
        .execute(&mut **transaction)
        .await?;

        // 获取申请者的 user_id
        let application = sqlx::query!(
            "SELECT user_id FROM creator_applications WHERE id = $1",
            id
        )
        .fetch_one(&mut **transaction)
        .await?;

        // 更新用户的高级创作者状态
        sqlx::query!(
            r#"
            UPDATE users
            SET is_premium_creator = TRUE,
                creator_verified_at = NOW()
            WHERE id = $1
            "#,
            application.user_id
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    /// 拒绝申请
    pub async fn reject(
        id: i64,
        reviewer_id: UserId,
        review_note: Option<&str>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            UPDATE creator_applications
            SET status = 'rejected',
                reviewer_id = $2,
                review_note = $3,
                reviewed_at = NOW()
            WHERE id = $1
            "#,
            id,
            reviewer_id.0,
            review_note
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    /// 获取待审核申请数量
    pub async fn count_pending<'a, E>(executor: E) -> Result<i64, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM creator_applications WHERE status = 'pending'"
        )
        .fetch_one(executor)
        .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// 检查用户是否有待处理的申请
    pub async fn has_pending_application<'a, E>(
        user_id: UserId,
        executor: E,
    ) -> Result<bool, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM creator_applications WHERE user_id = $1 AND status = 'pending') as exists",
            user_id.0
        )
        .fetch_one(executor)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }
}
