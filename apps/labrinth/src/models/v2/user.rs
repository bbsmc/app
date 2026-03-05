use crate::{
    auth::AuthProvider,
    models::{
        ids::UserId,
        users::{Badges, Role, UserPayoutData},
        v3::users::{ProfileReviewSummary, UserBanSummary},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LegacyUser {
    pub id: UserId,
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created: DateTime<Utc>,
    pub role: Role,
    pub badges: Badges,

    pub auth_providers: Option<Vec<AuthProvider>>, // this was changed in v3, but not changes ones we want to keep out of v2
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub has_password: Option<bool>,
    pub has_totp: Option<bool>,
    pub payout_data: Option<UserPayoutData>, // this was changed in v3, but not ones we want to keep out of v2
    pub has_phonenumber: Option<bool>,

    // DEPRECATED. Always returns None
    pub github_id: Option<u64>,

    /// 是否为高级创作者（可发布付费插件）
    pub is_premium_creator: bool,
    /// 高级创作者认证时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_verified_at: Option<DateTime<Utc>>,

    /// 用户当前的活跃封禁列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_bans: Option<Vec<UserBanSummary>>,

    /// 用户待审核的资料修改（仅本人和管理员可见）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_profile_reviews: Option<Vec<ProfileReviewSummary>>,
}

impl From<crate::models::v3::users::User> for LegacyUser {
    fn from(data: crate::models::v3::users::User) -> Self {
        Self {
            id: data.id,
            username: data.username,
            name: None,
            email: data.email,
            email_verified: data.email_verified,
            avatar_url: data.avatar_url,
            bio: data.bio,
            created: data.created,
            role: data.role,
            badges: data.badges,
            payout_data: data.payout_data,
            auth_providers: data.auth_providers,
            has_password: data.has_password,
            has_phonenumber: data.has_phonenumber,
            has_totp: data.has_totp,
            github_id: data.github_id,
            is_premium_creator: data.is_premium_creator,
            creator_verified_at: data.creator_verified_at,
            active_bans: data.active_bans,
            pending_profile_reviews: data.pending_profile_reviews,
        }
    }
}
