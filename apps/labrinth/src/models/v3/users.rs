use super::bans::{BanAppealId, BanType, UserBanId};
use super::ids::Base62Id;
use super::threads::ThreadId;
use crate::{auth::AuthProvider, bitflags_serde_impl};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]
#[serde(into = "Base62Id")]
pub struct UserId(pub u64);

pub const DELETED_USER: UserId = UserId(127155982985829);

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct Badges: u64 {
        const MIDAS = 1 << 0;
        const EARLY_MODPACK_ADOPTER = 1 << 1;
        const EARLY_RESPACK_ADOPTER = 1 << 2;
        const EARLY_PLUGIN_ADOPTER = 1 << 3;
        const ALPHA_TESTER = 1 << 4;
        const CONTRIBUTOR = 1 << 5;
        const TRANSLATOR = 1 << 6;

        const ALL = 0b1111111;
        const NONE = 0b0;
    }
}

bitflags_serde_impl!(Badges, u64);

impl Default for Badges {
    fn default() -> Badges {
        Badges::NONE
    }
}

/// 用户资料审核摘要（仅展示给用户本人，不含敏感的风控标签）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileReviewSummary {
    pub id: i64,
    pub review_type: String,
    pub new_value: String,
    pub created_at: DateTime<Utc>,
}

/// 用户封禁摘要（用于 User model）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserBanSummary {
    pub id: UserBanId,
    pub ban_type: BanType,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appeal: Option<BanAppealSummary>,
}

/// 申诉摘要
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BanAppealSummary {
    pub id: BanAppealId,
    pub status: String,
    pub thread_id: Option<ThreadId>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created: DateTime<Utc>,
    pub role: Role,
    pub badges: Badges,

    pub auth_providers: Option<Vec<AuthProvider>>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub has_password: Option<bool>,
    pub has_phonenumber: Option<bool>,
    pub has_totp: Option<bool>,
    pub payout_data: Option<UserPayoutData>,
    pub stripe_customer_id: Option<String>,

    // DEPRECATED. Always returns None
    pub github_id: Option<u64>,
    pub wiki_ban_time: DateTime<Utc>,
    pub wiki_overtake_count: i64,

    /// 是否为高级创作者（可发布付费插件）
    pub is_premium_creator: bool,
    /// 高级创作者认证时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_verified_at: Option<DateTime<Utc>>,

    /// 用户当前的活跃封禁列表（None 表示未查询）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_bans: Option<Vec<UserBanSummary>>,

    /// 用户待审核的资料修改（仅本人和管理员可见）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_profile_reviews: Option<Vec<ProfileReviewSummary>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserPayoutData {
    pub paypal_address: Option<String>,
    pub paypal_country: Option<String>,
    pub venmo_handle: Option<String>,
    #[serde(with = "rust_decimal::serde::float")]
    pub balance: Decimal,
}

use crate::database::models::user_item::User as DBUser;
impl From<DBUser> for User {
    fn from(data: DBUser) -> Self {
        // 转换封禁信息
        let active_bans = if data.active_bans.is_empty() {
            None
        } else {
            Some(
                data.active_bans
                    .into_iter()
                    .map(|ban| {
                        let ban_type = BanType::parse(&ban.ban_type).unwrap_or_else(|| {
                            log::error!("Invalid ban_type '{}' for user, defaulting to Global", ban.ban_type);
                            BanType::Global
                        });

                        let appeal = ban.appeal.map(|a| BanAppealSummary {
                            id: BanAppealId(a.id as u64),
                            status: a.status,
                            thread_id: a.thread_id.map(|id| ThreadId(id as u64)),
                        });

                        UserBanSummary {
                            id: UserBanId(ban.id as u64),
                            ban_type,
                            reason: ban.reason,
                            banned_at: ban.banned_at,
                            expires_at: ban.expires_at,
                            appeal,
                        }
                    })
                    .collect(),
            )
        };

        let pending_profile_reviews = if data.pending_profile_reviews.is_empty()
        {
            None
        } else {
            Some(
                data.pending_profile_reviews
                    .into_iter()
                    .map(|r| ProfileReviewSummary {
                        id: r.id,
                        review_type: r.review_type,
                        new_value: r.new_value,
                        created_at: r.created_at,
                    })
                    .collect(),
            )
        };

        Self {
            id: data.id.into(),
            username: data.username,
            email: None,
            email_verified: None,
            avatar_url: data.avatar_url,
            bio: data.bio,
            created: data.created,
            role: Role::from_string(&data.role),
            badges: data.badges,
            payout_data: None,
            auth_providers: None,
            has_password: None,
            has_phonenumber: None,
            has_totp: None,
            github_id: None,
            stripe_customer_id: None,
            wiki_ban_time: data.wiki_ban_time,
            wiki_overtake_count: data.wiki_overtake_count,
            is_premium_creator: data.is_premium_creator,
            creator_verified_at: data.creator_verified_at,
            active_bans,
            pending_profile_reviews,
        }
    }
}

impl User {
    pub fn from_full(db_user: DBUser) -> Self {
        let mut auth_providers = Vec::new();

        if db_user.github_id.is_some() {
            auth_providers.push(AuthProvider::GitHub)
        }
        if db_user.gitlab_id.is_some() {
            auth_providers.push(AuthProvider::GitLab)
        }
        if db_user.discord_id.is_some() {
            auth_providers.push(AuthProvider::Discord)
        }
        if db_user.google_id.is_some() {
            auth_providers.push(AuthProvider::Google)
        }
        if db_user.microsoft_id.is_some() {
            auth_providers.push(AuthProvider::Microsoft)
        }
        if db_user.steam_id.is_some() {
            auth_providers.push(AuthProvider::Steam)
        }
        if db_user.paypal_id.is_some() {
            auth_providers.push(AuthProvider::PayPal)
        }
        if db_user.bilibili_id.is_some() {
            auth_providers.push(AuthProvider::Bilibili)
        }
        if db_user.qq_id.is_some() {
            auth_providers.push(AuthProvider::QQ)
        }

        // 转换封禁信息
        let active_bans = if db_user.active_bans.is_empty() {
            None
        } else {
            Some(
                db_user
                    .active_bans
                    .into_iter()
                    .map(|ban| {
                        let ban_type = BanType::parse(&ban.ban_type).unwrap_or_else(|| {
                            log::error!("Invalid ban_type '{}' for user, defaulting to Global", ban.ban_type);
                            BanType::Global
                        });

                        let appeal = ban.appeal.map(|a| BanAppealSummary {
                            id: BanAppealId(a.id as u64),
                            status: a.status,
                            thread_id: a.thread_id.map(|id| ThreadId(id as u64)),
                        });

                        UserBanSummary {
                            id: UserBanId(ban.id as u64),
                            ban_type,
                            reason: ban.reason,
                            banned_at: ban.banned_at,
                            expires_at: ban.expires_at,
                            appeal,
                        }
                    })
                    .collect(),
            )
        };

        let pending_profile_reviews =
            if db_user.pending_profile_reviews.is_empty() {
                None
            } else {
                Some(
                    db_user
                        .pending_profile_reviews
                        .into_iter()
                        .map(|r| ProfileReviewSummary {
                            id: r.id,
                            review_type: r.review_type,
                            new_value: r.new_value,
                            created_at: r.created_at,
                        })
                        .collect(),
                )
            };

        Self {
            id: UserId::from(db_user.id),
            username: db_user.username,
            email: db_user.email,
            email_verified: Some(db_user.email_verified),
            avatar_url: db_user.avatar_url,
            bio: db_user.bio,
            created: db_user.created,
            role: Role::from_string(&db_user.role),
            badges: db_user.badges,
            auth_providers: Some(auth_providers),
            has_password: Some(db_user.password.is_some()),
            has_phonenumber: Some(db_user.phone_number.is_some()),
            has_totp: Some(db_user.totp_secret.is_some()),
            github_id: None,
            payout_data: Some(UserPayoutData {
                paypal_address: db_user.paypal_email,
                paypal_country: db_user.paypal_country,
                venmo_handle: db_user.venmo_handle,
                balance: Decimal::ZERO,
            }),
            stripe_customer_id: db_user.stripe_customer_id,
            wiki_ban_time: db_user.wiki_ban_time,
            wiki_overtake_count: db_user.wiki_overtake_count,
            is_premium_creator: db_user.is_premium_creator,
            creator_verified_at: db_user.creator_verified_at,
            active_bans,
            pending_profile_reviews,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Developer,
    Moderator,
    Admin,
}

impl std::fmt::Display for Role {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(self.as_str())
    }
}

impl Role {
    pub fn from_string(string: &str) -> Role {
        match string {
            "admin" => Role::Admin,
            "moderator" => Role::Moderator,
            _ => Role::Developer,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Developer => "developer",
            Role::Moderator => "moderator",
            Role::Admin => "admin",
        }
    }

    pub fn is_mod(&self) -> bool {
        match self {
            Role::Developer => false,
            Role::Moderator | Role::Admin => true,
        }
    }

    pub fn is_admin(&self) -> bool {
        match self {
            Role::Developer | Role::Moderator => false,
            Role::Admin => true,
        }
    }
}
