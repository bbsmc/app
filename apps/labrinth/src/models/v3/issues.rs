use super::ids::Base62Id;
use crate::database::models::issues::{
    IssueAssignee, IssueCommentQuery, IssueLabel, IssueReply, QueryIssue,
};
use crate::models::ids::{ProjectId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct IssuesId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct IssuesCommentsId(pub u64);

// 查询参数
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssuesQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub state: Option<String>, // open, closed, all
    pub labels: Option<Vec<String>>,
    pub assignee: Option<String>,
    pub author: Option<String>,
    pub sort: Option<String>, // created, updated, comments
    pub direction: Option<String>, // asc, desc
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentsQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

// Issue响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct IssueResponse {
    pub id: IssuesId,
    pub project_id: ProjectId,
    pub title: String,
    pub body: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub author: UserInfo,
    pub locked: bool,
    pub labels: Vec<LabelResponse>,
    pub assignees: Vec<AssigneeResponse>,
    pub comments_count: i32,
}

// 评论响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: IssuesCommentsId,
    pub issue_id: IssuesId,
    pub author: UserInfo,
    pub body: String,
    pub comment_type: String,
    pub reply_to_id: Option<IssuesCommentsId>,
    pub reply_to_floor: Option<i64>,
    pub reply_to_deleted: bool,
    pub floor_number: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub replies: Vec<ReplyResponse>,
}

// 标签响应结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LabelResponse {
    pub id: i32,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

// 指派人响应结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssigneeResponse {
    pub user: UserInfo,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: UserInfo,
}

// 回复响应结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReplyResponse {
    pub id: IssuesCommentsId,
    pub floor_number: i64,
    pub body: String,
    pub author: UserInfo,
    pub created_at: DateTime<Utc>,
}

// 用户信息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: UserId,
    pub username: String,
    pub avatar: String,
}

// 评论索引结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentIndex {
    pub comment_id: IssuesCommentsId,
    pub floor_number: i64,
}

// 创建Issue请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: String,
    pub labels: Option<Vec<i32>>,
    pub assignees: Option<Vec<UserId>>,
}

// 更新Issue请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    pub state: Option<String>,
    pub labels: Option<Vec<i32>>,
}

// 创建评论请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub body: String,
    pub comment_type: Option<String>,
    pub reply_to_id: Option<IssuesCommentsId>,
}

// 更新评论请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCommentRequest {
    pub body: String,
}

// 从数据库模型转换为API模型
impl From<QueryIssue> for IssueResponse {
    fn from(issue: QueryIssue) -> Self {
        let mut avatar = issue.inner.author_avatar.unwrap_or_default();
        if avatar.is_empty() {
            avatar = "https://cdn.bbsmc.net/raw/bbsmc-logo.png".to_string();
        }

        IssueResponse {
            id: issue.inner.id.into(),
            project_id: issue.inner.mod_id.into(),
            title: issue.inner.title,
            body: issue.inner.body,
            state: issue.inner.state,
            created_at: issue.inner.created_at,
            updated_at: issue.inner.updated_at,
            closed_at: issue.inner.closed_at,
            author: UserInfo {
                id: issue.inner.author_id.into(),
                username: issue.inner.author_name,
                avatar,
            },
            locked: issue.inner.locked,
            labels: issue
                .inner
                .labels
                .into_iter()
                .map(|label| label.into())
                .collect(),
            assignees: issue
                .inner
                .assignees
                .into_iter()
                .map(|assignee| assignee.into())
                .collect(),
            comments_count: issue.comments.len() as i32,
        }
    }
}

impl From<IssueCommentQuery> for CommentResponse {
    fn from(comment: IssueCommentQuery) -> Self {
        let mut avatar = comment.author_avatar.unwrap_or_default();
        if avatar.is_empty() {
            avatar = "https://cdn.bbsmc.net/raw/bbsmc-logo.png".to_string();
        }

        CommentResponse {
            id: comment.id.into(),
            issue_id: comment.issue_id.into(),
            author: UserInfo {
                id: comment.author_id.into(),
                username: comment.author_name,
                avatar,
            },
            body: comment.body,
            comment_type: comment.comment_type,
            reply_to_id: comment.reply_to_id.map(|id| id.into()),
            reply_to_floor: comment.reply_to_floor,
            reply_to_deleted: comment.reply_to_deleted,
            floor_number: comment.floor_number,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
            replies: comment
                .replies
                .into_iter()
                .map(|reply| reply.into())
                .collect(),
        }
    }
}

impl From<IssueLabel> for LabelResponse {
    fn from(label: IssueLabel) -> Self {
        LabelResponse {
            id: label.id,
            name: label.name,
            color: label.color,
            description: label.description,
        }
    }
}

impl From<IssueAssignee> for AssigneeResponse {
    fn from(assignee: IssueAssignee) -> Self {
        let mut user_avatar = assignee.user_avatar.unwrap_or_default();
        if user_avatar.is_empty() {
            user_avatar =
                "https://cdn.bbsmc.net/raw/bbsmc-logo.png".to_string();
        }

        // 注意：这里assigned_by的信息在数据库模型中只有ID，没有详细信息
        // 在实际使用时可能需要额外查询
        AssigneeResponse {
            user: UserInfo {
                id: assignee.user_id.into(),
                username: assignee.user_name,
                avatar: user_avatar,
            },
            assigned_at: assignee.assigned_at,
            assigned_by: UserInfo {
                id: assignee.assigned_by.into(),
                username: "".to_string(), // 需要额外查询
                avatar: "https://cdn.bbsmc.net/raw/bbsmc-logo.png".to_string(),
            },
        }
    }
}

impl From<IssueReply> for ReplyResponse {
    fn from(reply: IssueReply) -> Self {
        let mut avatar = reply.author_avatar.unwrap_or_default();
        if avatar.is_empty() {
            avatar = "https://cdn.bbsmc.net/raw/bbsmc-logo.png".to_string();
        }

        ReplyResponse {
            id: reply.id.into(),
            floor_number: reply.floor_number,
            body: reply.body,
            author: UserInfo {
                id: UserId(0), // 数据库模型中没有author_id，需要补充
                username: reply.author_name,
                avatar,
            },
            created_at: reply.created_at,
        }
    }
}
