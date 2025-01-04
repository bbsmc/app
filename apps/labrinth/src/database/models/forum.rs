use crate::database::models::{
    DatabaseError, DiscussionId, PostId, ProjectId, UserId,
};
use crate::database::redis::RedisPool;
use crate::models::forum::{Replay, ReplayContent};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const DISCUSSION_NAMESPACE: &str = "discussions";
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostQuery {
    pub id: PostId,
    pub discussion_id: DiscussionId,
    pub floor_number: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub user_name: String,
    pub user_avatar: String,
    pub replied_to: Option<i64>,
    pub reply_content: Option<ReplayContent>,
    pub replies: Vec<Replay>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostBuilder {
    pub id: PostId,
    pub discussion_id: DiscussionId,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub user_id: UserId,
    pub replied_to: Option<i64>,
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
    pub content: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
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
    pub async fn clear_cache(
        ids: &[DiscussionId],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;
        redis
            .delete_many(
                ids.iter()
                    .map(|id| (DISCUSSION_NAMESPACE, Some(id.0.to_string()))),
            )
            .await?;
        Ok(())
    }

    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO discussions(id, title, content, category, created_at, updated_at, user_id, state, pinned, deleted, deleted_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            self.id.0,
            self.title,
            self.content,
            self.category,
            self.created_at,
            self.updated_at,
            self.user_id.0,
            self.state,
            self.pinned,
            self.deleted,
            self.deleted_at
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    pub async fn update_project_discussion(
        &self,
        project_id: ProjectId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "update mods set forum=$1 where id=$2",
            self.id.0,
            project_id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    pub async fn get_id<'a, E>(
        key: i64,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Option<QueryDiscussion>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let discussions = Discussion::get_many(&[key], exec, redis).await?;
        Ok(discussions.into_iter().next())
    }

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
                let posts_index: DashMap<i64, Vec<PostIndex>> = sqlx::query!(
                    "
                     SELECT DISTINCT discussion_id, p.id as id, p.created_at
                    FROM discussions d
                    INNER JOIN posts p ON d.id = p.discussion_id
                    WHERE d.id = ANY($1) ORDER BY p.created_at ASC
                    ",
                    &ids.clone()
                )
                .fetch(&mut *exec)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, Vec<PostIndex>>, m| {
                        let post_id = PostId(m.id);
                        let floor_number =
                            acc.entry(m.discussion_id).or_default().len()
                                as i64
                                + 1;
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
                           d.content content,
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
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, QueryDiscussion>, m| {
                        // let id: i64 = m.id.clone();
                        let id = DiscussionId(m.id.unwrap());
                        let posts: Vec<PostIndex> = posts_index
                            .get(&id.0)
                            .map(|v| v.clone())
                            .unwrap_or_default();
                        acc.insert(
                            id.0,
                            QueryDiscussion {
                                posts,
                                inner: Discussion {
                                    id,
                                    title: m.title.unwrap(),
                                    content: m.content.unwrap(),
                                    category: m.category.unwrap(),
                                    created_at: m.created_at.unwrap(),
                                    updated_at: m.updated_at,
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
                    },
                )
                .await?;
                Ok(posts)
            })
            .await?;
        Ok(val)
    }
}

impl PostBuilder {
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO posts (id, discussion_id, content, user_id, created_at, replied_to) VALUES ($1, $2, $3, $4, $5, $6)",
            self.id.0,
            self.discussion_id.0,
            self.content,
            self.user_id.0,
            self.created_at,
            self.replied_to
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
}
impl PostQuery {
    pub async fn get_many<'a, E>(
        ids: &[i64],
        discussion_id: &DiscussionId,
        pool: E,
        redis: &RedisPool,
    ) -> Result<Vec<PostQuery>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres> + Clone,
    {
        let posts = redis.get_cached_keys(
            "post",
            ids,
            |keys| async move {
                let mut executor = pool.acquire().await?;
                let posts: DashMap<i64, Vec<PostIndex>> = sqlx::query!(
                    "
                     SELECT DISTINCT discussion_id, p.id as id, p.created_at
                    FROM discussions d
                    INNER JOIN posts p ON d.id = p.discussion_id
                    WHERE d.id = $1 ORDER BY p.created_at ASC
                    ",
                    &discussion_id.0
                )
                    .fetch(&mut *executor)
                    .try_fold(
                        DashMap::new(),
                        |acc: DashMap<i64, Vec<PostIndex>>, m| {
                            let post_id: PostId = PostId(m.id);
                            let floor_number = acc.entry(m.discussion_id).or_default().len() as i64 + 1;
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
                // 主查询
                let rows = sqlx::query!(
                        "
                        SELECT p.*, u.username as user_name, u.avatar_url as avatar_url
                        FROM posts p
                        LEFT JOIN users u ON p.user_id = u.id
                        WHERE p.id = ANY($1)
                        AND p.discussion_id = $2
                        ",
                        &keys,
                        discussion_id.0
                    )
                    .fetch_all(&mut *executor)
                    .await?;

                // info!("rows: {:?}", rows);

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
                                WHERE p.id = $1
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
                            w.id as i64,
                            w.discussion_id
                        )
                        .fetch_all(&mut *executor)
                        .await?;

                    let mut replies: Vec<Replay> = replies_rows
                        .into_iter()
                        .map(|r| {
                            let id: i64 = r.id;
                            // 查询post里 post_id=id的floor_number
                            let floor_number = posts.get(&r.discussion_id).unwrap().iter().find(|p| p.post_id == PostId(id)).unwrap().floor_number;
                            Replay {
                                floor_number,
                                content: r.content,
                                user_name: r.user_name,
                                user_avatar: r.avatar_url.unwrap_or_default(),
                            }
                        }
                           )
                        .collect::<Vec<Replay>>();
                    replies.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

                    // 判断 w.replied_to 是否为空
                    let replied_to = if let Some(replied_to) = w.replied_to {
                        Some(posts.get(&w.discussion_id).unwrap().iter().find(|p| p.post_id == PostId(replied_to)).unwrap().floor_number)
                    } else {
                        None
                    };
                    result.insert(
                        w.id,
                        PostQuery {
                            id: PostId(w.id),
                            discussion_id: DiscussionId(w.discussion_id),
                            floor_number: posts.get(&w.discussion_id).unwrap().iter().find(|p| p.post_id == PostId(w.id)).unwrap().floor_number,
                            content: w.content,
                            created_at: w.created_at,
                            updated_at: w.updated_at,
                            user_name: w.user_name,
                            user_avatar: w.avatar_url.unwrap_or_default(),
                            replied_to,
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
