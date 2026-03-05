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
pub const DISCUSSION_TYPES_NAMESPACE: &str = "discussions_types";
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
    pub reply_to_deleted: bool,
    pub replies: Vec<Replay>,
    pub deleted: bool,
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
    pub organization: Option<String>,
    pub organization_id: Option<String>,
    pub avatar: Option<String>,
    pub state: String,
    pub pinned: bool,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub last_post_time: DateTime<Utc>,
    pub project_id: Option<ProjectId>,
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

    pub async fn update_discussion_content(
        &self,
        content: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "update discussions set content=$1,updated_at=$2 where id=$3",
            content,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
    pub async fn update_discussion_title(
        &self,
        title: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "update discussions set title=$1,updated_at=$2 where id=$3",
            title,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
    pub async fn delete_discussion(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "update discussions set deleted=$1,deleted_at=$2 where id=$3",
            true,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
    pub async fn update_last_post_time(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE discussions SET last_post_time = $1 WHERE id = $2",
            self.last_post_time,
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn clear_cache_discussions(
        types: &[String],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;
        redis
            .delete_many(
                types
                    .iter()
                    .map(|id| (DISCUSSION_TYPES_NAMESPACE, Some(id.clone()))),
            )
            .await?;
        Ok(())
    }

    pub async fn forums<'a, E>(
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<DiscussionId>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let forums = redis
            .get_cached_key(
                DISCUSSION_TYPES_NAMESPACE,
                "all".to_string(),
                || async move {
                    let mut exec = exec.acquire().await?;

                    let forums: Vec<DiscussionId> = sqlx::query!(
                        "SELECT id
                FROM discussions
                WHERE deleted = false order by last_post_time desc limit 5"
                    )
                    .fetch(&mut *exec)
                    .try_fold(Vec::new(), |mut acc, row| {
                        acc.push(DiscussionId(row.id));
                        async move { Ok(acc) }
                    })
                    .await?;

                    Ok(forums)
                },
            )
            .await?;

        Ok(forums)
    }

    pub async fn get_forums<'a, E>(
        forum_type: String,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<DiscussionId>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let forums = redis.get_cached_key(DISCUSSION_TYPES_NAMESPACE, forum_type.clone(), || async move {
            let mut exec = exec.acquire().await?;

            let forums: Vec<DiscussionId> = sqlx::query!(
                "SELECT id
                FROM discussions
                WHERE deleted = false AND category = $1 order by last_post_time desc",
                &forum_type
            )
            .fetch(&mut *exec)
            .try_fold(Vec::new(), |mut acc, row| {
                acc.push(DiscussionId(row.id));
                async move { Ok(acc) }
            })
            .await?;
            Ok(forums)
        }).await?;

        Ok(forums)
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
                    WHERE d.deleted = false and d.id = ANY($1)
                    ORDER BY p.created_at ASC
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
                    r#"SELECT d.id as "id!",
                           d.title as "title!",
                           d.content as "content!",
                           d.category as "category!",
                           d.created_at as "created_at!",
                           d.updated_at,
                           d.user_id as "user_id!",
                           d.state as "state!",
                           d.pinned as "pinned!",
                           d.deleted as "deleted!",
                           d.deleted_at,
                           d.last_post_time,
                           u.username as "user_name?",
                           u.avatar_url as "avatar_url?",
                           (SELECT m.id FROM mods m WHERE m.forum = d.id LIMIT 1) as project_id
                    FROM discussions d
                             LEFT JOIN users u ON d.user_id = u.id
                    WHERE d.id = ANY ($1) AND d.deleted = false"#,
                    &ids
                )
                .fetch(&mut *exec)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, QueryDiscussion>, m| {
                        let id = DiscussionId(m.id);
                        let posts: Vec<PostIndex> = posts_index
                            .get(&id.0)
                            .map(|v| v.clone())
                            .unwrap_or_default();

                        // 使用子查询后，project_id 会被正确推断为 Option<i64>
                        let project_id: Option<ProjectId> =
                            m.project_id.map(ProjectId);

                        acc.insert(
                            id.0,
                            QueryDiscussion {
                                posts,
                                inner: Discussion {
                                    id,
                                    title: m.title,
                                    content: m.content,
                                    category: m.category,
                                    created_at: m.created_at,
                                    updated_at: m.updated_at,
                                    user_id: UserId(m.user_id),
                                    user_name: m.user_name.unwrap_or_default(),
                                    avatar: m.avatar_url,
                                    organization: None,
                                    organization_id: None,
                                    state: m.state,
                                    pinned: m.pinned,
                                    deleted: m.deleted,
                                    deleted_at: m.deleted_at,
                                    last_post_time: m.last_post_time.unwrap_or_else(chrono::Utc::now),
                                    project_id,
                                },
                            },
                        );
                        async move { Ok(acc) }
                    },
                )
                .await?;

                // 获取posts的所有keys
                let keys: Vec<i64> = posts.iter().map(|r| *r.key()).collect();

                // 遍历并更新每个帖子
                for key in keys {
                    if let Some(mut ele) = posts.get_mut(&key)
                        && ele.inner.project_id.is_some() {
                            let project =
                                crate::database::models::Project::get_id(
                                    ele.inner.project_id.unwrap(),
                                    &mut *exec,
                                    redis,
                                )
                                .await?;
                            if let Some(project) = project {
                                ele.inner.title = project.inner.name;
                                ele.inner.avatar = project.inner.icon_url;
                                if let Some(organization_id) =
                                    project.inner.organization_id
                                {
                                    let organization = crate::database::models::Organization::get_id(
                                        organization_id,
                                        &mut *exec,
                                        redis,
                                    )
                                    .await?;
                                    if let Some(organization) = organization {
                                        ele.inner.organization = Some(organization.name);
                                        ele.inner.organization_id = Some(organization.slug);
                                    }
                                }
                            }
                        }
                }

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
    pub async fn clear_cache(
        ids: &[PostId],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;
        redis
            .delete_many(ids.iter().map(|id| ("post", Some(id.0.to_string()))))
            .await?;
        Ok(())
    }

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
                    // 查询回复内容和删除状态
                    let (reply_content, reply_to_deleted) = if let Some(replied_to) = w.replied_to {
                        let row_content = sqlx::query!(
                                "
                                SELECT p.content, p.deleted, u.username as user_name, u.avatar_url as avatar_url
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

                        match row_content {
                            Some(row) if row.deleted => {
                                // 被回复的帖子已删除，返回用户信息但不返回内容
                                (Some(ReplayContent {
                                    content: String::new(),
                                    user_name: row.user_name,
                                    user_avatar: row.avatar_url.unwrap_or_default(),
                                }), true)
                            }
                            Some(row) => {
                                // 被回复的帖子存在且未删除，返回内容
                                (Some(ReplayContent {
                                    content: row.content,
                                    user_name: row.user_name,
                                    user_avatar: row.avatar_url.unwrap_or_default(),
                                }), false)
                            }
                            None => {
                                // 被回复的帖子不存在（可能被彻底删除）
                                (None, true)
                            }
                        }
                    } else {
                        (None, false)
                    };

                    // 查询回复列表（包含已删除的回复，但标记删除状态）
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
                                // 如果已删除，不返回内容
                                content: if r.deleted { String::new() } else { r.content },
                                user_name: r.user_name,
                                user_avatar: r.avatar_url.unwrap_or_default(),
                                deleted: r.deleted,
                            }
                        }
                           )
                        .collect::<Vec<Replay>>();
                    replies.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

                    // 判断 w.replied_to 是否为空
                    let replied_to = if let Some(replied_to) = w.replied_to {
                        if posts.get(&w.discussion_id).is_none() {
                            None
                        } else {
                            let posts = posts.get(&w.discussion_id).unwrap();
                            let p = posts.iter().find(|p| p.post_id == PostId(replied_to));
                            p.map(|p| p.floor_number)
                        }
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
                            reply_to_deleted,
                            deleted: w.deleted,
                        },
                    );
                }

                Ok(result)
            },
        ).await?;

        Ok(posts)
    }
}
