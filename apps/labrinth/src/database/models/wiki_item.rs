use super::ids::*;
use crate::database::models::{DatabaseError, WikiCache};
use crate::database::redis::RedisPool;
use crate::models::users::User;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

pub const WIKI_NAMESPACE: &str = "wikis";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Wiki {
    pub id: WikiId,
    pub project_id: ProjectId,
    pub sort_order: i32,
    pub title: String,
    pub body: String,
    pub parent_wiki_id: WikiId,
    pub featured: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub slug: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WikiDisplays {
    pub id: WikiId,
    pub project_id: ProjectId,
    pub sort_order: i32,
    pub title: String,
    pub body: String,
    pub parent_wiki_id: WikiId,
    pub featured: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub slug: String,
    pub child: Vec<Wiki>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Wikis {
    pub wikis: Vec<WikiDisplays>,
    pub is_editor: bool,
    pub cache: Option<WikiCache>,
    pub is_editor_user: bool,
    pub editor_user: Option<User>,
    pub is_visitors: bool,
}

impl Wiki {
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Wiki, sqlx::Error> {
        let row = sqlx::query!(
            "
            INSERT INTO wikis (id, mod_id, sort_order, title, body, parent_wiki_id, featured, slug)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *
            ",
            self.id.0,
            self.project_id.0,
            self.sort_order,
            self.title,
            self.body,
            self.parent_wiki_id.0,
            self.featured,
            self.slug,
        )
        .fetch_one(&mut **transaction)
        .await?;
        // println!("row_wiki_insert {:?}", row);

        Ok(Wiki {
            id: WikiId(row.id),
            project_id: ProjectId(row.mod_id),
            sort_order: row.sort_order,
            title: row.title,
            body: row.body,
            parent_wiki_id: WikiId(row.parent_wiki_id),
            featured: row.featured,
            created: row.created,
            updated: row.updated,
            slug: row.slug,
        })
    }

    pub async fn update(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE wikis SET body = $1,sort_order = $2,title = $3,featured = $4,updated = $5,draft = false WHERE id=$6",
            self.body,
            self.sort_order,
            self.title,
            self.featured,
            self.updated,
            self.id.0
        ).execute(&mut **transaction)
            .await?;

        Ok(())
    }
    pub async fn delete(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM wikis WHERE id = $1", self.id.0)
            .execute(&mut **transaction)
            .await?;

        Ok(())
    }

    pub async fn get_many<'a, E>(
        wiki_ids: &[WikiId],
        draft: bool,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<Wiki>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut val = redis.get_cached_keys(
            WIKI_NAMESPACE,
            &wiki_ids.iter().map(|x| x.0).collect::<Vec<_>>(),
            |wiki_ids| async move {
                let mut exec = exec.acquire().await?;


                let res = sqlx::query!("
                    SELECT id, mod_id, sort_order, title, body, parent_wiki_id, featured, created, updated, slug
                    FROM wikis
                    WHERE id = ANY($1) and draft = $2;
                    ",
                    &wiki_ids,
                    draft
                )
                    .fetch(&mut *exec)
                    .try_fold(DashMap::new(), | acc, w| {

                        let row = Wiki {
                            id: WikiId(w.id),
                            project_id: ProjectId(w.mod_id),
                            sort_order: w.sort_order,
                            title: w.title,
                            body: w.body,
                            parent_wiki_id: WikiId(w.parent_wiki_id),
                            featured: w.featured,
                            created: w.created,
                            updated: w.updated,
                            slug: w.slug,
                        };
                        acc.insert(w.id, row);
                        async move { Ok(acc) }

                    })
                    .await?;
                Ok(res)
            },
        ).await?;

        val.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        Ok(val)
    }

    pub async fn get<'a, E>(
        wiki_id: i64,
        exec: E,
    ) -> Result<Wiki, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let mut exec = exec.acquire().await?;

        let row = sqlx::query!("
                    SELECT id, mod_id, sort_order, title, body, parent_wiki_id, featured, created, updated, slug
                    FROM wikis
                    WHERE id = $1;
                    ",
                    &wiki_id
                ).fetch_one(&mut *exec)
            .await?;
        Ok(Wiki {
            id: WikiId(row.id),
            project_id: ProjectId(row.mod_id),
            sort_order: row.sort_order,
            title: row.title,
            body: row.body,
            parent_wiki_id: WikiId(row.parent_wiki_id),
            featured: row.featured,
            created: row.created,
            updated: row.updated,
            slug: row.slug,
        })
    }
    pub async fn clear_cache(
        &self,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;

        redis
            .delete_many([(WIKI_NAMESPACE, Some(self.id.0.to_string()))])
            .await?;
        Ok(())
    }
}
