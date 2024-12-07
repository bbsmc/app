use std::process::Child;
use super::ids::*;
use crate::database::models::{DatabaseError, WikiCache};
use crate::database::redis::RedisPool;
use chrono::{DateTime, Utc};
use dashmap::{DashMap};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

pub const WIKI_NAMESPACE: &str = "wikis";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
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
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Wikis {
    pub wikis: Vec<WikiDisplays>,
    pub is_editor: bool,
    pub cache: Option<WikiCache>
}

impl Wiki {
    // pub async fn insert(
    //     &self,
    //     transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    // ) -> Result<(), sqlx::error::Error> {
    //     sqlx::query!(
    //         "
    //         INSERT INTO wikis (id, mod_id, sort_order, title, body, parent_wiki_id, featured, created, updated, slug)
    //         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    //         ",
    //         self.id.0,
    //         self.project_id.0,
    //         self.sort_order,
    //         self.title,
    //         self.body,
    //         self.parent_wiki_id.map(|x| x.0),
    //         self.featured,
    //         self.created,
    //         self.updated,
    //         self.slug,
    //     )
    //     .execute(&mut **transaction)
    //     .await?;
    //     Ok(())
    // }

    // pub async fn get<'a, 'b, E>(
    //     id: WikiId,
    //     executor: E,
    //     redis: &RedisPool,
    // ) -> Result<Option<Self>, DatabaseError>
    // where
    //     E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    // {
    //     Self::get_many(&[id], executor, redis)
    //         .await
    //         .map(|x| x.into_iter().next())
    // }

    pub async fn get_many<'a, E>(
        wiki_ids: &[WikiId],
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
                    WHERE id = ANY($1);
                    ",
                    &wiki_ids
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
}
