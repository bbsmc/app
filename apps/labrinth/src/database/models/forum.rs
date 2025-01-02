use crate::database::models::{DatabaseError, DiscussionId, PostId, UserId};
use crate::database::redis::RedisPool;
use crate::models::forum::{Replay, ReplayContent};
use crate::models::ids::base62_impl::parse_base62;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use log::info;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

pub const DISCUSSION_NAMESPACE: &str = "discussions";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostQuery {
    pub id: PostId,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostIndex {
    pub post_id: PostId,
    pub floor_number: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discussion {
    pub id: DiscussionId,
    pub title: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: UserId,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub state: String,
    pub pinned: bool,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryDiscussion {
    pub inner: Discussion,
    pub posts: Vec<PostIndex>,
}

impl Discussion {
    pub async fn get_many<'a, E>(
        keys: &[i64],
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<QueryDiscussion>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let val = redis
            .get_cached_keys(DISCUSSION_NAMESPACE, keys, |ids| async move {
                let mut exec = exec.acquire().await?;
                let posts: DashMap<i64, Vec<PostIndex>> = sqlx::query!(
                    "
                     SELECT DISTINCT discussion_id, p.id as id, p.floor_number
                    FROM discussions d
                    INNER JOIN posts p ON d.id = p.discussion_id
                    WHERE d.id = ANY($1) ORDER BY p.floor_number ASC
                    ",
                    &ids.clone()
                )
                .fetch(&mut *exec)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, Vec<PostIndex>>, m| {
                        let post_id = PostId(m.id);
                        let floor_number = m.floor_number;
                        acc.entry(m.discussion_id).or_default().push(
                            PostIndex {
                                post_id,
                                floor_number,
                            },
                        );
                        async move { Ok(acc) }
                    },
                )
                .await?;

                let posts = sqlx::query!(
                    "SELECT d.id id,
                           d.title title,
                           d.category category,
                           d.created_at created_at,
                           d.updated_at updated_at,
                           d.user_id user_id,
                           d.state state,
                           d.pinned pinned,
                           d.deleted deleted,
                           d.deleted_at deleted_at,
                           u.username user_name,
                           u.avatar_url avatar_url
                    FROM discussions d
                             LEFT JOIN users u ON d.user_id = u.id
                    WHERE d.id = ANY ($1) AND d.deleted = false",
                    &ids
                )
                .fetch(&mut *exec)
                .try_fold(DashMap::new(), |acc, m| {
                    // let id: i64 = m.id.clone();
                    let id = DiscussionId(m.id.unwrap());
                    acc.insert(
                        id.0,
                        QueryDiscussion {
                            posts: posts.get(&id.0).unwrap().clone(),
                            inner: Discussion {
                                id: id,
                                title: m.title.unwrap(),
                                category: m.category.unwrap(),
                                created_at: m.created_at.unwrap(),
                                updated_at: m.updated_at.unwrap(),
                                user_id: UserId(m.user_id.unwrap()),
                                user_name: m.user_name,
                                user_avatar: m.avatar_url,
                                state: m.state.unwrap(),
                                pinned: m.pinned.unwrap(),
                                deleted: m.deleted.unwrap(),
                                deleted_at: m.deleted_at,
                            },
                        },
                    );
                    async move { Ok(acc) }
                })
                .await?;
                Ok(posts)
            })
            .await?;
        Ok(val)
    }
}

impl PostQuery {

    pub async fn insert<'a, E>(
        discussion_id: DiscussionId,
        content: String,
        replied_to: Option<i64>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<(), sqlx::Error> { {
   {
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
    pub async fn get_many<'a, E>(
        ids: &Vec<i64>,
        discussion_id: &DiscussionId,
        pool: E,
        redis: &RedisPool,
    ) -> Result<Vec<PostQuery>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres> + Clone,
    {
        let posts = redis.get_cached_keys(
            &"post".to_string(),
            ids,
            |keys| async move {
                let mut executor = pool.acquire().await?;

                // 主查询
                let rows = sqlx::query!(
                        r#"
                        SELECT p.*, u.username as user_name, u.avatar_url as avatar_url
                        FROM posts p
                        LEFT JOIN users u ON p.user_id = u.id
                        WHERE p.id = ANY($1)
                        AND p.discussion_id = $2
                        "#,
                        &keys,
                        discussion_id.0
                    )
                    .fetch_all(&mut *executor)
                    .await?;

                info!("rows: {:?}", rows);

                let result = DashMap::new();

                // 处理每一行数据
                for w in rows {
                    // 查询回复内容
                    let reply_content = if let Some(replied_to) = w.replied_to {
                        let row_content = sqlx::query!(
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
                            .fetch_optional(&mut *executor)
                            .await?;

                        row_content.map(|row| ReplayContent {
                            content: row.content,
                            user_name: row.user_name,
                            user_avatar: row.avatar_url.unwrap_or_default(),
                        })
                    } else {
                        None
                    };

                    // 查询回复列表
                    let replies_rows = sqlx::query!(
                            "
                            SELECT p.*, u.username as user_name, u.avatar_url as avatar_url
                            FROM posts p
                            LEFT JOIN users u ON p.user_id = u.id
                            WHERE p.replied_to = $1
                            AND p.discussion_id = $2
                            ",
                            w.floor_number as i64,
                            w.discussion_id
                        )
                        .fetch_all(&mut *executor)
                        .await?;

                    let mut replies: Vec<Replay> = replies_rows
                        .into_iter()
                        .map(|r| Replay {
                            floor_number: r.floor_number,
                            content: r.content,
                            user_name: r.user_name,
                            user_avatar: r.avatar_url.unwrap_or_default(),
                            replied_to: r.replied_to,
                        })
                        .collect::<Vec<Replay>>();
                    replies.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

                    result.insert(
                        w.id,
                        PostQuery {
                            id: PostId(w.id),
                            discussion_id: DiscussionId(w.discussion_id),
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
                }

                Ok(result)
            },
        ).await?;

        Ok(posts)
    }
}
