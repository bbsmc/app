use thiserror::Error;

pub mod categories;
pub mod charge_item;
pub mod collection_item;
pub mod flow_item;
pub mod forum;
pub mod ids;
pub mod image_item;
pub mod legacy_loader_fields;
pub mod loader_fields;
pub mod notification_item;
pub mod oauth_client_authorization_item;
pub mod oauth_client_item;
pub mod oauth_token_item;
pub mod organization_item;
pub mod pat_item;
pub mod payout_item;
pub mod product_item;
pub mod project_item;
pub mod report_item;
pub mod session_item;
pub mod team_item;
pub mod thread_item;
pub mod user_item;
pub mod user_subscription_item;
pub mod version_item;
pub mod wiki_item;

pub mod creator_application_item;
pub mod issues;
pub mod payment_merchant_item;
pub mod payment_order_item;
pub mod project_pricing_item;
pub mod user_ban_item;
pub mod user_purchase_item;
pub mod wiki_cache_item;

pub use collection_item::Collection;
pub use creator_application_item::{
    ApplicationStatus, CreatorApplication, CreatorApplicationBuilder,
};
pub use forum::Discussion;
pub use forum::PostBuilder;
pub use forum::PostQuery;
pub use forum::QueryDiscussion;
pub use ids::*;
pub use image_item::Image;
pub use oauth_client_item::OAuthClient;
pub use organization_item::Organization;
pub use payment_merchant_item::{PaymentMerchant, PaymentMerchantBuilder};
pub use payment_order_item::{OrderStatus, PaymentMethod, PaymentOrder};
pub use project_item::Project;
pub use project_pricing_item::ProjectPricing;
pub use team_item::Team;
pub use team_item::TeamMember;
pub use thread_item::{Thread, ThreadMessage};
pub use user_ban_item::{
    AppealStatus, BanAppeal, BanAppealBuilder, BanHistory, BanHistoryBuilder,
    BanType, UserBan, UserBanBuilder,
};
pub use user_item::User;
pub use user_purchase_item::{PurchaseStatus, UserPurchase};
pub use version_item::Version;
pub use wiki_cache_item::WikiCache;
pub use wiki_item::Wiki;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Error while interacting with the database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Error while interacting with the database: {0}")]
    Database2(String),
    #[error("Error while trying to generate random ID")]
    RandomId,
    #[error("Error while interacting with the cache: {0}")]
    CacheError(#[from] redis::RedisError),
    #[error("Redis Pool Error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),
    #[error("Error while serializing with the cache: {0}")]
    SerdeCacheError(#[from] serde_json::Error),
    #[error("Schema error: {0}")]
    SchemaError(String),
    // 上游优化: 添加更多调试信息到缓存超时错误
    #[error(
        "Timeout when waiting for cache subscriber (released {locks_released}/{locks_waiting} locks, pool wait: {time_spent_pool_wait_ms}ms, total: {time_spent_total_ms}ms)"
    )]
    CacheTimeout {
        locks_released: usize,
        locks_waiting: usize,
        time_spent_pool_wait_ms: u64,
        time_spent_total_ms: u64,
    },
}
