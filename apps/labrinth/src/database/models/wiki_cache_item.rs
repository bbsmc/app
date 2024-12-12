use super::ids::*;
use crate::database::models::DatabaseError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const WIKI_CACHE_NAMESPACE: &str = "wikis_cache";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct WikiCache {
    pub id: WikiCacheId,
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub created: DateTime<Utc>,
    pub status: String,
    pub cache: serde_json::Value,
}

impl WikiCache {
    pub async fn get_draft<'a, E>(
        project_id: ProjectId,
        user_id: UserId,
        exec: E
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        // let mut redis = redis.connect().await?;
        // let id = format!("{}:{}", project_id.0, user_id.0);
        // let cached: Option<String> =
        //     redis.get(WIKI_CACHE_NAMESPACE, &id).await?;
        // if let Some(cached) = cached {
        //     return Ok(Some(serde_json::from_str(&cached)?));
        // }

        // let key = format!("{}:{}", project_id.0, user_id.0);
        //
        // let mut redis = redis.connect().await?;
        // let cached = redis.get(WIKI_CACHE_NAMESPACE,&key).await?.and_then(|cached| serde_json::from_str(&cached).ok());


        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches FROM wiki_cache WHERE mod_id = $1 AND user_id = $2 AND status = 'draft'",
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
            });


        // if let Some(&wiki_cache) = wiki_cache {
        //     redis
        //         .set(
        //             WIKI_CACHE_NAMESPACE,
        //             &id,
        //             &serde_json::to_string(&wiki_cache)?,
        //             Option::from(crate::database::redis::DEFAULT_EXPIRY),
        //         )
        //         .await?;
        // }

        Ok(wiki_cache)
    }

    pub async fn get_has_draft_or_review<'a, E>(
        project_id: ProjectId,
        exec: E
    ) -> Result<Option<WikiCache>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;
        let wiki_cache= sqlx::query!(
            "SELECT id, mod_id, user_id, created, status ,caches FROM wiki_cache WHERE mod_id = $1 AND (status = 'draft' OR status = 'review')",
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
            });

        Ok(wiki_cache)
    }

    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<WikiCache, sqlx::Error>
    {

        let row = sqlx::query!(
            "INSERT INTO wiki_cache (id, mod_id, user_id, caches) VALUES ($1, $2, $3, $4) RETURNING *",
            self.id.0,
            self.project_id.0,
            self.user_id.0,
            self.cache
        ).fetch_one(&mut **transaction).await?;

        Ok(WikiCache {
            id: WikiCacheId(row.id),
            project_id: ProjectId(row.mod_id),
            user_id: UserId(row.user_id),
            created: row.created,
            status: row.status,
            cache: row.caches,
        })
    }
    pub async fn update_cache(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<WikiCache, sqlx::Error>
    {
        let row = sqlx::query!(
            "UPDATE wiki_cache SET caches = $1,status = $2 WHERE id=$3 RETURNING *",
            self.cache,
            self.status,
            self.id.0
        ).fetch_one(&mut **transaction).await?;
        Ok(WikiCache {
            id: WikiCacheId(row.id),
            project_id: ProjectId(row.mod_id),
            user_id: UserId(row.user_id),
            created: row.created,
            status: row.status,
            cache: row.caches,
        })
    }
}
