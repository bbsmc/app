use super::ids::Base62Id;
use crate::database::models::forum::PostQuery;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct DiscussionId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Debug)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct PostId(pub u64);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostsQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostResponse {
    pub post_id: PostId,
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
pub struct ReplayContent {
    pub content: String,
    pub user_name: String,
    pub user_avatar: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostIndex {
    pub id: i64,
    pub floor_number: i64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Replay {
    pub floor_number: i64,
    pub content: String,
    pub user_name: String,
    pub user_avatar: String,
}

impl From<PostQuery> for PostResponse {
    fn from(post: PostQuery) -> Self {
        PostResponse {
            post_id: post.id.into(),
            discussion_id: post.discussion_id.into(),
            floor_number: post.floor_number,
            content: post.content,
            created_at: post.created_at,
            updated_at: post.updated_at,
            user_name: post.user_name,
            user_avatar: post.user_avatar,
            replied_to: post.replied_to,
            reply_content: post.reply_content,
            replies: post.replies,
        }
    }
}
