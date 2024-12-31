use crate::database::models::{DatabaseError, DiscussionId};
use crate::database::redis::RedisPool;
use crate::models::forum::{Replay, ReplayContent};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct PostQuery {
    pub discussion_id: DiscussionId,
    pub floor_number: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_name: String,
    pub user_avatar: String,
    pub replied_to: Option<i64>,
    pub reply_content: Option<ReplayContent>,
    pub replies: Vec<Replay>,
}

impl PostQuery {
    // pub async fn get_many_ids<
    //     'a,
    //     E,
    //     T: Display + Hash + Eq + PartialEq + Clone + Debug,
    // >(
    //     post_ids: &[PostId],
    //     exec: E,
    //     redis: &RedisPool,
    // ) -> Result<Vec<Post>, DatabaseError>
    // where
    //     E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    // {
    //
    //     let ids = post_ids
    //         .iter()
    //         .map(|x| crate::models::ids::PatId::from(*x))
    //         .collect::<Vec<_>>();
    //     Post::get_many(&ids, exec, redis).await
    // }
    pub async fn get_many<'a, E>(
        floor_number: &Vec<i64>,
        discussion_id: &i64,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<PostQuery>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let posts: Vec<PostQuery> = redis
            .get_cached_keys(
                &"post".to_string(),
                floor_number,
                |keys| async move {


                    let posts = sqlx::query!(
                    r#"
                    SELECT p.*, u.username as user_name, u.avatar_url as avatar_url
                    FROM posts p
                    LEFT JOIN users u ON p.user_id = u.id
                    WHERE p.floor_number = ANY($1)
                    AND p.discussion_id = $2
                    "#,
                    &keys,
                    discussion_id.parse::<i64>().unwrap()
                )
                        .fetch(&mut *exec)
                        .try_fold(DashMap::new(), |acc, w|  async move  {

                                let mut exec = exec.acquire().await?;

                                let mut reply_content = None;
                                if let Some(replied_to) = w.replied_to {
                                    reply_content = sqlx::query!(
                                "
                                SELECT p.content, u.username as user_name, u.avatar_url as avatar_url
                                FROM posts p
                                LEFT JOIN users u ON p.user_id = u.id
                                WHERE p.floor_number = $1
                                AND p.discussion_id = $2
                                ",
                                replied_to as i64,
                                w.discussion_id
                            )
                                        .fetch_optional( &mut *exec)
                                        .await?
                                        .map(|row| ReplayContent {
                                            content: row.content,
                                            user_name: row.user_name,
                                            user_avatar: row.avatar_url.unwrap_or_default(),
                                        });
                                }

                                let replies = sqlx::query!(
                            "
                            SELECT p.*, u.username as user_name, u.avatar_url as avatar_url
                            FROM posts p
                            LEFT JOIN users u ON p.user_id = u.id
                            WHERE p.replied_to = $1
                            AND p.discussion_id = $2
                            ",
                            w.floor_number as i64,
                            w.discussion_id,
                        ).fetch(&mut *exec)
                                    .try_fold(vec![], |mut acc, w| {
                                        async move {
                                            acc.push(Replay {
                                                floor_number: w.floor_number,
                                                content: w.content,
                                                user_name: w.user_name,
                                                user_avatar: w.avatar_url.unwrap_or_default(),
                                                replied_to: w.replied_to,
                                            });
                                            Ok(acc)
                                        }
                                    }).await?;

                                acc.insert(
                                    w.id.to_string(),
                                    PostQuery {
                                        discussion_id: w.discussion_id,
                                        floor_number: w.floor_number,
                                        content: w.content,
                                        created_at: w.created_at,
                                        updated_at: w.updated_at,
                                        user_name: w.user_name,
                                        user_avatar: w.avatar_url.unwrap_or_default(),
                                        replied_to: w.replied_to,
                                        replies,
                                        reply_content,
                                    },
                                );
                                Ok(acc)

                        })
                        .await?;
                    Ok(posts)
                },
            )
            .await?;
        Ok(posts)
    }
}
