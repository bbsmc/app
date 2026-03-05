use crate::database::models::{
    DatabaseError, IssuesCommentsId, IssuesId, ProjectId, UserId,
};
use crate::database::redis::RedisPool;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const ISSUE_NAMESPACE: &str = "issues";
pub const ISSUE_LABELS_NAMESPACE: &str = "issue_labels";

// Issue标签结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueLabel {
    pub id: i32,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Issue评论查询结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueCommentQuery {
    pub id: IssuesCommentsId,
    pub issue_id: IssuesId,
    pub author_id: UserId,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub body: String,
    pub comment_type: String,
    pub reply_to_id: Option<IssuesCommentsId>,
    pub reply_to_floor: Option<i64>,
    pub reply_to_deleted: bool, // 被回复的评论是否已删除
    pub floor_number: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub replies: Vec<IssueReply>,
}

// Issue评论回复结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueReply {
    pub id: IssuesCommentsId,
    pub floor_number: i64,
    pub body: String,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Issue评论构建器
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueCommentBuilder {
    pub id: IssuesCommentsId,
    pub issue_id: IssuesId,
    pub author_id: UserId,
    pub body: String,
    pub comment_type: String,
    pub reply_to_id: Option<IssuesCommentsId>,
    pub created_at: DateTime<Utc>,
}

// Issue评论索引
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueCommentIndex {
    pub comment_id: IssuesCommentsId,
    pub floor_number: i64,
}

// Issue指派人结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueAssignee {
    pub issue_id: IssuesId,
    pub user_id: UserId,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: UserId,
}

// Issue主结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: IssuesId,
    pub mod_id: ProjectId,
    pub title: String,
    pub body: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub author_id: UserId,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub locked: bool,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub labels: Vec<IssueLabel>,
    pub assignees: Vec<IssueAssignee>,
}

// 查询Issue结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryIssue {
    pub inner: Issue,
    pub comments: Vec<IssueCommentIndex>,
}

impl Issue {
    // 清除缓存
    pub async fn clear_cache(
        ids: &[IssuesId],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;
        redis
            .delete_many(
                ids.iter()
                    .map(|id| (ISSUE_NAMESPACE, Some(id.0.to_string()))),
            )
            .await?;
        Ok(())
    }

    // 插入新Issue
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO issues(id, mod_id, title, body, state, created_at, updated_at, closed_at, author_id, locked, deleted, deleted_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            self.id.0,
            self.mod_id.0,
            self.title,
            self.body,
            self.state,
            self.created_at,
            self.updated_at,
            self.closed_at,
            self.author_id.0,
            self.locked,
            self.deleted,
            self.deleted_at
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 更新Issue内容
    pub async fn update_body(
        &self,
        body: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE issues SET body=$1, updated_at=$2 WHERE id=$3",
            body,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 更新Issue标题
    pub async fn update_title(
        &self,
        title: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE issues SET title=$1, updated_at=$2 WHERE id=$3",
            title,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 更新Issue状态
    pub async fn update_state(
        &self,
        state: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        let closed_at = if state == "closed" {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query!(
            "UPDATE issues SET state=$1, closed_at=$2, updated_at=$3 WHERE id=$4",
            state,
            closed_at,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 更新Issue标签
    pub async fn update_labels(
        &self,
        label_ids: Vec<i32>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        // 先删除现有的标签关联
        sqlx::query!(
            "DELETE FROM issue_label_associations WHERE issue_id = $1",
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        // 添加新的标签关联
        for label_id in label_ids {
            sqlx::query!(
                "INSERT INTO issue_label_associations (issue_id, label_id) VALUES ($1, $2)",
                self.id.0,
                label_id
            )
            .execute(&mut **transaction)
            .await?;
        }

        // 更新Issue的updated_at时间
        sqlx::query!(
            "UPDATE issues SET updated_at=$1 WHERE id=$2",
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 锁定/解锁Issue
    pub async fn update_locked(
        &self,
        locked: bool,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE issues SET locked=$1, updated_at=$2 WHERE id=$3",
            locked,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 删除Issue
    pub async fn delete_issue(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE issues SET deleted=$1, deleted_at=$2 WHERE id=$3",
            true,
            Utc::now(),
            self.id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 获取项目的所有Issues
    pub async fn get_project_issues<'a, E>(
        project_id: ProjectId,
        state: Option<String>,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<IssuesId>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let cache_key = format!(
            "project_{}_{}",
            project_id.0,
            state.as_deref().unwrap_or("all")
        );

        let issues = redis.get_cached_key(ISSUE_NAMESPACE, cache_key, || async move {
            let mut exec = exec.acquire().await?;

            let issues: Vec<IssuesId> = if let Some(state) = state {
                let query_result = sqlx::query!(
                    "SELECT id FROM issues WHERE mod_id = $1 AND state = $2 AND deleted = false ORDER BY created_at DESC",
                    project_id.0,
                    state
                )
                .fetch_all(&mut *exec)
                .await;
                match query_result {
                    Ok(rows) => {
                        rows.into_iter().map(|row| {
                            IssuesId(row.id)
                        }).collect()
                    },
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            } else {
                let query_result = sqlx::query!(
                    "SELECT id FROM issues WHERE mod_id = $1 AND deleted = false ORDER BY created_at DESC",
                    project_id.0
                )
                .fetch_all(&mut *exec)
                .await;
                match query_result {
                    Ok(rows) => {
                        rows.into_iter().map(|row| {
                            IssuesId(row.id)
                        }).collect()
                    },
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            };

            Ok(issues)
        }).await?;

        Ok(issues)
    }

    // 根据ID获取单个Issue
    pub async fn get_id<'a, E>(
        key: i64,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Option<QueryIssue>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let issues = Issue::get_many(&[key], exec, redis).await?;
        Ok(issues.into_iter().next())
    }

    // 批量获取Issues（简化版本）
    pub async fn get_many<'a, E>(
        keys: &[i64],
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<QueryIssue>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let val = redis
            .get_cached_keys(ISSUE_NAMESPACE, keys, |ids| async move {
                let mut exec = exec.acquire().await?;
                // 获取评论索引 - 包含已删除的评论来保持楼层号连续性
                let comments_index: DashMap<i64, Vec<IssueCommentIndex>> = sqlx::query!(
                    "
                    SELECT DISTINCT issue_id, ic.id as id, ic.created_at
                    FROM issues i
                    INNER JOIN issue_comments ic ON i.id = ic.issue_id
                    WHERE i.deleted = false AND i.id = ANY($1)
                    ORDER BY ic.created_at ASC
                    ",
                    &ids.clone()
                )
                .fetch(&mut *exec)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, Vec<IssueCommentIndex>>, m| {
                        let comment_id = IssuesCommentsId(m.id);
                        let floor_number = acc.entry(m.issue_id).or_default().len() as i64 + 1;
                        acc.entry(m.issue_id).or_default().push(
                            IssueCommentIndex {
                                comment_id,
                                floor_number,
                            },
                        );
                        async move { Ok(acc) }
                    },
                )
                .await?;

                // 获取Issues主要信息
                let issues = sqlx::query!(
                    r#"SELECT i.id as "id!", i.mod_id as "mod_id!", i.title as "title!",
                            i.body as "body!", i.state as "state!", i.created_at as "created_at!",
                            i.updated_at as "updated_at!", i.closed_at,
                            i.author_id as "author_id!", i.locked as "locked!",
                            i.deleted as "deleted!", i.deleted_at,
                            u.username as "author_name?", u.avatar_url as "author_avatar?"
                     FROM issues i
                     LEFT JOIN users u ON i.author_id = u.id
                     WHERE i.id = ANY($1) AND i.deleted = false"#,
                    &ids
                )
                .fetch(&mut *exec)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, QueryIssue>, m| {
                        let id = IssuesId(m.id);
                        let comments: Vec<IssueCommentIndex> = comments_index
                            .get(&id.0)
                            .map(|v| v.clone())
                            .unwrap_or_default();

                        acc.insert(
                            id.0,
                            QueryIssue {
                                comments,
                                inner: Issue {
                                    id,
                                    mod_id: ProjectId(m.mod_id),
                                    title: m.title,
                                    body: m.body,
                                    state: m.state,
                                    created_at: m.created_at,
                                    updated_at: m.updated_at,
                                    closed_at: m.closed_at,
                                    author_id: UserId(m.author_id),
                                    author_name: m.author_name.unwrap_or_default(),
                                    author_avatar: m.author_avatar,
                                    locked: m.locked,
                                    deleted: m.deleted,
                                    deleted_at: m.deleted_at,
                                    labels: Vec::new(), // 稍后填充
                                    assignees: Vec::new(), // 稍后填充
                                },
                            },
                        );
                        async move { Ok(acc) }
                    },
                )
                .await?;

                // 获取标签数据
                let labels_data = sqlx::query!(
                    "SELECT ila.issue_id, il.id, il.name, il.color, il.description, il.created_at
                     FROM issue_label_associations ila
                     JOIN issue_labels il ON ila.label_id = il.id
                     WHERE ila.issue_id = ANY($1)",
                    &ids
                )
                .fetch_all(&mut *exec)
                .await?;

                // 将标签数据添加到对应的Issue中
                for label_row in labels_data {
                    if let Some(mut issue) = issues.get_mut(&label_row.issue_id) {
                        issue.inner.labels.push(IssueLabel {
                            id: label_row.id,
                            name: label_row.name,
                            color: label_row.color,
                            description: Some(label_row.description.unwrap_or_default()),
                            created_at: label_row.created_at,
                        });
                    }
                }

                // 获取指派人数据
                let assignees_data = sqlx::query!(
                    "SELECT ia.issue_id, ia.user_id, ia.assigned_at, ia.assigned_by,
                            u.username as user_name, u.avatar_url as user_avatar
                     FROM issue_assignees ia
                     JOIN users u ON ia.user_id = u.id
                     WHERE ia.issue_id = ANY($1)",
                    &ids
                )
                .fetch_all(&mut *exec)
                .await?;

                // 将指派人数据添加到对应的Issue中
                for assignee_row in assignees_data {
                    if let Some(mut issue) = issues.get_mut(&assignee_row.issue_id) {
                        issue.inner.assignees.push(IssueAssignee {
                            issue_id: IssuesId(assignee_row.issue_id),
                            user_id: UserId(assignee_row.user_id),
                            user_name: assignee_row.user_name,
                            user_avatar: assignee_row.user_avatar,
                            assigned_at: assignee_row.assigned_at,
                            assigned_by: UserId(assignee_row.assigned_by),
                        });
                    }
                }

                Ok(issues)
            })
            .await?;
        Ok(val)
    }
}

impl IssueCommentBuilder {
    // 插入新评论
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO issue_comments (id, issue_id, author_id, body, comment_type, reply_to_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            self.id.0,
            self.issue_id.0,
            self.author_id.0,
            self.body,
            self.comment_type,
            self.reply_to_id.map(|id| id.0),
            self.created_at,
            self.created_at
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
}

impl IssueLabel {
    // 获取所有标签
    pub async fn get_all<'a, E>(
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<IssueLabel>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let labels = redis
            .get_cached_key(
                ISSUE_LABELS_NAMESPACE,
                "all".to_string(),
                || async move {
                    let mut exec = exec.acquire().await?;

                    let labels: Vec<IssueLabel> = sqlx::query!(
                        "SELECT id, name, color, description, created_at FROM issue_labels ORDER BY id ASC"
                    )
                    .fetch(&mut *exec)
                    .try_fold(Vec::new(), |mut acc, row| {
                        acc.push(IssueLabel {
                            id: row.id,
                            name: row.name,
                            color: row.color,
                            description: Some(row.description.unwrap_or_default()),
                            created_at: row.created_at,
                        });
                        async move { Ok(acc) }
                    })
                    .await?;

                    Ok(labels)
                },
            )
            .await?;

        Ok(labels)
    }
}

impl IssueCommentQuery {
    // 批量获取评论
    pub async fn get_many<'a, E>(
        ids: &[i64],
        issue_id: &IssuesId,
        pool: E,
        redis: &RedisPool,
    ) -> Result<Vec<IssueCommentQuery>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres> + Clone,
    {
        let comments = redis.get_cached_keys(
            "issue_comment",
            ids,
            |keys| async move {
                let mut executor = pool.acquire().await?;
                // 获取评论索引（楼层号）- 包含已删除的评论来保持楼层号连续性
                let comments_index: DashMap<i64, Vec<IssueCommentIndex>> = sqlx::query!(
                    "
                    SELECT DISTINCT issue_id, ic.id as id, ic.created_at
                    FROM issues i
                    INNER JOIN issue_comments ic ON i.id = ic.issue_id
                    WHERE i.id = $1 ORDER BY ic.created_at ASC
                    ",
                    &issue_id.0
                )
                .fetch(&mut *executor)
                .try_fold(
                    DashMap::new(),
                    |acc: DashMap<i64, Vec<IssueCommentIndex>>, m| {
                        let comment_id = IssuesCommentsId(m.id);
                        let floor_number = acc.entry(m.issue_id).or_default().len() as i64 + 1;
                        acc.entry(m.issue_id).or_default().push(
                            IssueCommentIndex {
                                comment_id,
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
                    SELECT ic.*, u.username as author_name, u.avatar_url as author_avatar
                    FROM issue_comments ic
                    LEFT JOIN users u ON ic.author_id = u.id
                    WHERE ic.id = ANY($1)
                    AND ic.issue_id = $2
                    ",
                    &keys,
                    issue_id.0
                )
                .fetch_all(&mut *executor)
                .await?;

                let result = DashMap::new();

                // 处理每一行数据
                for w in rows {
                    let comment_id = w.id;  // 先保存ID
                    // 查询回复列表
                    let replies_rows = sqlx::query!(
                        "
                        SELECT ic.*, u.username as author_name, u.avatar_url as author_avatar
                        FROM issue_comments ic
                        LEFT JOIN users u ON ic.author_id = u.id
                        WHERE ic.reply_to_id = $1
                        AND ic.issue_id = $2
                        AND ic.deleted = false
                        ",
                        comment_id as i64,
                        w.issue_id
                    )
                    .fetch_all(&mut *executor)
                    .await?;

                    let mut replies: Vec<IssueReply> = replies_rows
                        .into_iter()
                        .map(|r| {
                            let id: i64 = r.id;
                            // 查询楼层号
                            let floor_number = comments_index
                                .get(&r.issue_id)
                                .unwrap()
                                .iter()
                                .find(|p| p.comment_id == IssuesCommentsId(id))
                                .unwrap()
                                .floor_number;
                            IssueReply {
                                id: IssuesCommentsId(id),
                                floor_number,
                                body: r.body,
                                author_name: r.author_name,
                                author_avatar: r.author_avatar,
                                created_at: r.created_at,
                            }
                        })
                        .collect::<Vec<IssueReply>>();
                    replies.sort_by(|a, b| a.floor_number.cmp(&b.floor_number));

                    // 获取回复的楼层号和被回复评论的删除状态
                    let (reply_to_floor, reply_to_deleted) = if let Some(reply_to_id) = w.reply_to_id {
                        if comments_index.get(&w.issue_id).is_none() {
                            (None, false)
                        } else {
                            let comments = comments_index.get(&w.issue_id).unwrap();
                            if let Some(p) = comments.iter().find(|p| p.comment_id == IssuesCommentsId(reply_to_id)) {
                                // 查询被回复评论是否被删除
                                let reply_deleted = sqlx::query!(
                                    "SELECT deleted FROM issue_comments WHERE id = $1",
                                    reply_to_id
                                )
                                .fetch_optional(&mut *executor)
                                .await?
                                .map(|row| row.deleted)
                                .unwrap_or(false);
                                (Some(p.floor_number), reply_deleted)
                            } else {
                                (None, false)
                            }
                        }
                    } else {
                        (None, false)
                    };

                    result.insert(
                        comment_id,
                        IssueCommentQuery {
                            id: IssuesCommentsId(comment_id),
                            issue_id: IssuesId(w.issue_id),
                            author_id: UserId(w.author_id),
                            author_name: w.author_name,
                            author_avatar: w.author_avatar,
                            body: w.body,
                            comment_type: w.comment_type,
                            reply_to_id: w.reply_to_id.map(IssuesCommentsId),
                            reply_to_floor,
                            reply_to_deleted,
                            floor_number: comments_index
                                .get(&w.issue_id)
                                .unwrap()
                                .iter()
                                .find(|p| p.comment_id == IssuesCommentsId(comment_id))
                                .unwrap()
                                .floor_number,
                            created_at: w.created_at,
                            updated_at: w.updated_at,
                            deleted: w.deleted,
                            deleted_at: w.deleted_at,
                            replies,
                        },
                    );
                }

                Ok(result)
            },
        ).await?;

        Ok(comments)
    }

    // 获取单个评论
    pub async fn get_id<'a, E>(
        comment_id: IssuesCommentsId,
        issue_id: &IssuesId,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Option<IssueCommentQuery>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres> + Clone,
    {
        let comments =
            IssueCommentQuery::get_many(&[comment_id.0], issue_id, exec, redis)
                .await?;
        Ok(comments.into_iter().next())
    }

    // 更新评论内容
    pub async fn update_body(
        comment_id: IssuesCommentsId,
        body: String,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE issue_comments SET body=$1, updated_at=$2 WHERE id=$3",
            body,
            Utc::now(),
            comment_id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    // 删除评论
    pub async fn delete_comment(
        comment_id: IssuesCommentsId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // 软删除评论，保留原始内容和 reply_to_id 关系
        // 前端根据 deleted 字段来控制显示方式
        sqlx::query!(
            "UPDATE issue_comments SET deleted=$1, deleted_at=$2 WHERE id=$3",
            true,
            now,
            comment_id.0
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }
}
