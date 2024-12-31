use super::ids::Base62Id;
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

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Replay {
    pub floor_number: i64,
    pub content: String,
    pub user_name: String,
    pub user_avatar: String,
    pub replied_to: Option<i64>,
}
