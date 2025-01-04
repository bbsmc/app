use super::ids::*;
use crate::database::models::DatabaseError;
use crate::models::users::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub const WIKI_CACHE_NAMESPACE: &str = "wikis_cache";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct WikiCache {
    pub id: WikiCacheId,
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub created: DateTime<Utc>,
    pub status: String,
    pub cache: serde_json::Value,
    pub old: serde_json::Value,
    pub message: serde_json::Value,
    pub again_count: u64,
    pub again_time: DateTime<Utc>,
}

impl WikiCache {
    pub async fn message_add(&mut self, user: &User, msg: &str) {
        self.message
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "username": user.username,
                "avatar_url": user.avatar_url,
                "time": Utc::now(),
                "message": msg.to_owned()
            }));
    }
    pub async fn get_draft<'a, E>(
        project_id: ProjectId,
        user_id: UserId,
        exec: E,
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches, old, message,again_count,again_time FROM wiki_cache WHERE mod_id = $1 AND user_id = $2 AND status = 'draft'",
            &project_id.0,
            &user_id.0
        ).fetch_optional(&mut *exec)
            .await?
            .map(|row| WikiCache {
                id: WikiCacheId(row.id),
                project_id: ProjectId(row.mod_id),
                user_id: UserId(row.user_id),
                created: row.created,
                status: row.status,
                cache: row.caches,
                old: row.old,
                message: row.message,
                again_count: row.again_count as u64,
                again_time: row.again_time
            });

        Ok(wiki_cache)
    }

    pub async fn get_all_draft(
        exec: &PgPool,
    ) -> Result<Vec<WikiCache>, DatabaseError> {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches, old, message,again_count,again_time FROM wiki_cache WHERE status = 'draft'"
        ).fetch_all(&mut *exec)
            .await?
            .into_iter()
            .map(|row| WikiCache {
                id: WikiCacheId(row.id),
                project_id: ProjectId(row.mod_id),
                user_id: UserId(row.user_id),
                created: row.created,
                status: row.status,
                cache: row.caches,
                old: row.old,
                message: row.message,
                again_count: row.again_count as u64,
                again_time: row.again_time
            }).collect::<Vec<_>>();
        Ok(wiki_cache)
    }

    pub async fn get_reject_or_review<'a, E>(
        cache_id: i64,
        user_id: UserId,
        exec: E,
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches, old, message,again_count,again_time FROM wiki_cache WHERE id = $1 AND user_id = $2 AND (status = 'reject' OR status = 'review')",
            &cache_id,
            &user_id.0
        ).fetch_optional(&mut *exec)
            .await?
            .map(|row| WikiCache {
                id: WikiCacheId(row.id),
                project_id: ProjectId(row.mod_id),
                user_id: UserId(row.user_id),
                created: row.created,
                status: row.status,
                cache: row.caches,
                old: row.old,
                message: row.message,
                again_count: row.again_count as u64,
                again_time: row.again_time
            });

        Ok(wiki_cache)
    }

    pub async fn get_draft_or_review<'a, E>(
        project_id: ProjectId,
        exec: E,
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches, old, message,again_count,again_time FROM wiki_cache WHERE mod_id = $1 AND (status = 'draft' OR status = 'review')",
            &project_id.0,
        ).fetch_optional(&mut *exec)
            .await?
            .map(|row| WikiCache {
                id: WikiCacheId(row.id),
                project_id: ProjectId(row.mod_id),
                user_id: UserId(row.user_id),
                created: row.created,
                status: row.status,
                cache: row.caches,
                old: row.old,
                message: row.message,
                again_count: row.again_count as u64,
                again_time: row.again_time
            });

        Ok(wiki_cache)
    }

    pub async fn get_has_draft_or_review<'a, E>(
        project_id: ProjectId,
        exec: E,
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches, old, message,again_count,again_time FROM wiki_cache WHERE mod_id = $1 AND (status = 'draft' OR status = 'review')",
            &project_id.0,
        ).fetch_optional(&mut *exec)
            .await?
            .map(|row| WikiCache {
                id: WikiCacheId(row.id),
                project_id: ProjectId(row.mod_id),
                user_id: UserId(row.user_id),
                created: row.created,
                status: row.status,
                cache: row.caches,
                old: row.old,
                message: row.message,
                again_count: row.again_count as u64,
                again_time: row.again_time
            });

        Ok(wiki_cache)
    }

    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<WikiCache, sqlx::Error> {
        let row = sqlx::query!(
            "INSERT INTO wiki_cache (id, mod_id, user_id, caches,old, message) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
            self.id.0,
            self.project_id.0,
            self.user_id.0,
            self.cache,
            self.old,
            self.message
        ).fetch_one(&mut **transaction).await?;

        Ok(WikiCache {
            id: WikiCacheId(row.id),
            project_id: ProjectId(row.mod_id),
            user_id: UserId(row.user_id),
            created: row.created,
            status: row.status,
            cache: row.caches,
            old: row.old,
            message: row.message,
            again_count: row.again_count as u64,
            again_time: row.again_time,
        })
    }
    pub async fn update_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<WikiCache, sqlx::Error> {
        let row = sqlx::query!(
            "UPDATE wiki_cache SET caches = $1,status = $2,message=$3 WHERE id=$4 RETURNING *",
            self.cache,
            self.status,
            self.message,
            self.id.0
        ).fetch_one(&mut **transaction).await?;
        Ok(WikiCache {
            id: WikiCacheId(row.id),
            project_id: ProjectId(row.mod_id),
            user_id: UserId(row.user_id),
            created: row.created,
            status: row.status,
            cache: row.caches,
            old: row.old,
            message: row.message,
            again_count: row.again_count as u64,
            again_time: row.again_time,
        })
    }
    pub async fn finish_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wiki_cache SET status = $1,message=$2 WHERE id=$3",
            "success",
            self.message,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn reject_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wiki_cache SET status = $1,message=$2 WHERE id=$3",
            "reject",
            self.message,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
    pub async fn review_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wiki_cache SET status = $1,message=$2 WHERE id=$3",
            "review",
            self.message,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn again_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
           "UPDATE wiki_cache SET status = $1, message = $2, again_count = again_count + 1, again_time = now() WHERE id = $3",
            "draft",
            self.message,
            self.id.0
        ).execute(&mut **transaction).await?;
        Ok(())
    }

    pub async fn user_ban(
        &self,
        user_id: UserId,
        hour: i64,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
           "UPDATE users SET wiki_ban_time = now() + interval '1 hour' * $1 WHERE id = $2",
            hour as i64,
            user_id.0
        ).execute(&mut **transaction).await?;
        Ok(())
    }
    pub async fn user_overtake_count(
        &self,
        user_id: UserId,
        add_count: i64,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users
                    SET wiki_overtake_count = wiki_overtake_count + $1
                    WHERE id = $2",
            add_count,
            user_id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
    pub async fn user_overtake_count_set(
        &self,
        user_id: UserId,
        count: i64,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE users
                    SET wiki_overtake_count = $1
                    WHERE id = $2",
            count,
            user_id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn given_up_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wiki_cache SET status = $1, message = $2 WHERE id = $3",
            "given_up",
            self.message,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
    pub async fn timeout_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wiki_cache SET status = $1, message = $2 WHERE id = $3",
            "timeout",
            self.message,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
}
