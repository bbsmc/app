use super::ids::{ProjectId, UserId};
use super::{CollectionId, ReportId, ThreadId};
use crate::database::models;
use crate::database::models::{DatabaseError, OrganizationId};
use crate::database::redis::RedisPool;
use crate::models::ids::base62_impl::{parse_base62, to_base62};
use crate::models::users::Badges;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::hash::Hash;

const USERS_NAMESPACE: &str = "users";
const USER_USERNAMES_NAMESPACE: &str = "users_usernames";
const USERS_PROJECTS_NAMESPACE: &str = "users_projects";

/// 用户活跃封禁摘要（用于 User 查询结果）
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UserBanInfo {
    pub id: i64,
    pub ban_type: String,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub appeal: Option<BanAppealInfo>,
}

/// 封禁申诉信息
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BanAppealInfo {
    pub id: i64,
    pub status: String,
    pub thread_id: Option<i64>,
}

/// 用户资料审核信息（用于 User 查询结果）
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProfileReviewInfo {
    pub id: i64,
    pub review_type: String, // "avatar" / "username" / "bio"
    pub old_value: Option<String>,
    pub new_value: String,
    pub risk_labels: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub id: UserId,

    pub github_id: Option<i64>,
    pub discord_id: Option<i64>,
    pub gitlab_id: Option<i64>,
    pub google_id: Option<String>,
    pub steam_id: Option<i64>,
    pub microsoft_id: Option<String>,
    pub bilibili_id: Option<String>,
    pub qq_id: Option<String>,
    pub password: Option<String>,

    pub paypal_id: Option<String>,
    pub paypal_country: Option<String>,
    pub paypal_email: Option<String>,
    pub venmo_handle: Option<String>,
    pub stripe_customer_id: Option<String>,

    pub totp_secret: Option<String>,

    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
    pub raw_avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created: DateTime<Utc>,
    pub role: String,
    pub badges: Badges,
    pub wiki_ban_time: DateTime<Utc>,
    pub wiki_overtake_count: i64,
    pub phone_number: Option<String>,

    /// 是否为高级创作者（可发布付费插件）
    pub is_premium_creator: bool,
    /// 高级创作者认证时间
    pub creator_verified_at: Option<DateTime<Utc>>,

    /// 用户当前的活跃封禁列表（需要单独调用 UserBan::get_user_active_bans 填充）
    #[serde(default)]
    pub active_bans: Vec<UserBanInfo>,

    /// 用户待审核的资料修改列表
    #[serde(default)]
    pub pending_profile_reviews: Vec<ProfileReviewInfo>,
}

impl User {
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::error::Error> {
        sqlx::query!(
            "
            INSERT INTO users (
                id, username, email,
                avatar_url, raw_avatar_url, bio, created,
                github_id, discord_id, gitlab_id, google_id, steam_id, microsoft_id,
                bilibili_id, qq_id,
                email_verified, password, paypal_id, paypal_country, paypal_email,
                venmo_handle, stripe_customer_id
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7,
                $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22
            )
            ",
            self.id as UserId,
            &self.username,
            self.email.as_ref(),
            self.avatar_url.as_ref(),
            self.raw_avatar_url.as_ref(),
            self.bio.as_ref(),
            self.created,
            self.github_id,
            self.discord_id,
            self.gitlab_id,
            self.google_id,
            self.steam_id,
            self.microsoft_id,
            self.bilibili_id,
            self.qq_id,
            self.email_verified,
            self.password,
            self.paypal_id,
            self.paypal_country,
            self.paypal_email,
            self.venmo_handle,
            self.stripe_customer_id
        )
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    pub async fn get<'a, 'b, E>(
        string: &str,
        executor: E,
        redis: &RedisPool,
    ) -> Result<Option<User>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        User::get_many(&[string], executor, redis)
            .await
            .map(|x| x.into_iter().next())
    }

    pub async fn get_id<'a, 'b, E>(
        id: UserId,
        executor: E,
        redis: &RedisPool,
    ) -> Result<Option<User>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        User::get_many(&[crate::models::ids::UserId::from(id)], executor, redis)
            .await
            .map(|x| x.into_iter().next())
    }

    pub async fn get_many_ids<'a, E>(
        user_ids: &[UserId],
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<User>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        let ids = user_ids
            .iter()
            .map(|x| crate::models::ids::UserId::from(*x))
            .collect::<Vec<_>>();
        User::get_many(&ids, exec, redis).await
    }

    pub async fn get_many<
        'a,
        E,
        T: Display + Hash + Eq + PartialEq + Clone + Debug,
    >(
        users_strings: &[T],
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<User>, DatabaseError>
    where
        E: sqlx::Acquire<'a, Database = sqlx::Postgres>,
    {
        use futures::TryStreamExt;

        let val = redis
            .get_cached_keys_with_slug(
                USERS_NAMESPACE,
                USER_USERNAMES_NAMESPACE,
                false,
                users_strings,
                |ids| async move {
                    let mut exec = exec.acquire().await?;

                    let user_ids: Vec<i64> = ids
                        .iter()
                        .flat_map(|x| parse_base62(&x.to_string()).ok())
                        .map(|x| x as i64)
                        .collect();
                    let slugs = ids
                        .into_iter()
                        .map(|x| x.to_string().to_lowercase())
                        .collect::<Vec<_>>();

                    // 第一步：查询用户基本信息
                    let users = sqlx::query!(
                        "
                        SELECT id, email,
                            avatar_url, raw_avatar_url, username, bio,
                            created, role, badges,
                            github_id, discord_id, gitlab_id, google_id, steam_id, microsoft_id,
                            bilibili_id, qq_id,
                            email_verified, password, totp_secret, paypal_id, paypal_country, paypal_email,
                            venmo_handle, stripe_customer_id,wiki_overtake_count,wiki_ban_time,phone_number,
                            is_premium_creator, creator_verified_at
                        FROM users
                        WHERE id = ANY($1) OR LOWER(username) = ANY($2)
                        ",
                        &user_ids,
                        &slugs,
                    )
                    .fetch(&mut *exec)
                    .try_fold(DashMap::new(), |acc, u| {
                        let user = User {
                            id: UserId(u.id),
                            github_id: u.github_id,
                            discord_id: u.discord_id,
                            gitlab_id: u.gitlab_id,
                            google_id: u.google_id,
                            steam_id: u.steam_id,
                            microsoft_id: u.microsoft_id,
                            bilibili_id: u.bilibili_id,
                            qq_id: u.qq_id,
                            email: u.email,
                            email_verified: u.email_verified,
                            avatar_url: u.avatar_url,
                            raw_avatar_url: u.raw_avatar_url,
                            username: u.username.clone(),
                            bio: u.bio,
                            created: u.created,
                            role: u.role,
                            badges: Badges::from_bits(u.badges as u64).unwrap_or_default(),
                            password: u.password,
                            paypal_id: u.paypal_id,
                            paypal_country: u.paypal_country,
                            paypal_email: u.paypal_email,
                            venmo_handle: u.venmo_handle,
                            stripe_customer_id: u.stripe_customer_id,
                            totp_secret: u.totp_secret,
                            wiki_overtake_count: u.wiki_overtake_count,
                            wiki_ban_time: u.wiki_ban_time,
                            phone_number: u.phone_number,
                            is_premium_creator: u.is_premium_creator,
                            creator_verified_at: u.creator_verified_at,
                            active_bans: Vec::new(),  // 第二步填充
                            pending_profile_reviews: Vec::new(),  // 第四步填充
                        };

                        acc.insert(u.id, (Some(u.username), user));
                        async move { Ok(acc) }
                    })
                    .await?;

                    // 第二步：查询所有用户的活跃封禁（包含申诉信息）
                    let queried_user_ids: Vec<i64> = users.iter().map(|entry| *entry.key()).collect();
                    if !queried_user_ids.is_empty() {
                        let active_bans: DashMap<i64, Vec<UserBanInfo>> = sqlx::query!(
                            "
                            SELECT
                                ub.user_id,
                                ub.id,
                                ub.ban_type,
                                ub.reason,
                                ub.banned_at,
                                ub.expires_at,
                                uba.id as appeal_id,
                                uba.status as appeal_status,
                                uba.thread_id as appeal_thread_id
                            FROM user_bans ub
                            LEFT JOIN user_ban_appeals uba ON uba.ban_id = ub.id
                            WHERE ub.user_id = ANY($1)
                              AND ub.is_active = true
                              AND (ub.expires_at IS NULL OR ub.expires_at > NOW())
                            ",
                            &queried_user_ids
                        )
                        .fetch(&mut *exec)
                        .try_fold(DashMap::new(), |acc: DashMap<i64, Vec<UserBanInfo>>, row| {
                            let appeal = if let Some(appeal_id) = row.appeal_id {
                                Some(BanAppealInfo {
                                    id: appeal_id,
                                    status: row.appeal_status.unwrap_or_default(),
                                    thread_id: row.appeal_thread_id,
                                })
                            } else {
                                None
                            };

                            let ban_info = UserBanInfo {
                                id: row.id,
                                ban_type: row.ban_type,
                                reason: row.reason,
                                banned_at: row.banned_at,
                                expires_at: row.expires_at,
                                appeal,
                            };

                            acc.entry(row.user_id).or_default().push(ban_info);
                            async move { Ok(acc) }
                        })
                        .await?;

                        // 第三步：填充 active_bans 到用户对象
                        for mut entry in users.iter_mut() {
                            let user_id = *entry.key();
                            if let Some(bans) = active_bans.get(&user_id) {
                                entry.value_mut().1.active_bans = bans.value().clone();
                            }
                        }

                        // 第四步：查询所有用户的待审核资料修改
                        let pending_reviews: DashMap<i64, Vec<ProfileReviewInfo>> = sqlx::query!(
                            "
                            SELECT user_id, id, review_type, old_value, new_value, risk_labels, status, created_at
                            FROM user_profile_reviews
                            WHERE user_id = ANY($1) AND status = 'pending'
                            ",
                            &queried_user_ids
                        )
                        .fetch(&mut *exec)
                        .try_fold(DashMap::new(), |acc: DashMap<i64, Vec<ProfileReviewInfo>>, row| {
                            let review_info = ProfileReviewInfo {
                                id: row.id,
                                review_type: row.review_type,
                                old_value: row.old_value,
                                new_value: row.new_value,
                                risk_labels: row.risk_labels,
                                status: row.status,
                                created_at: row.created_at,
                            };
                            acc.entry(row.user_id).or_default().push(review_info);
                            async move { Ok(acc) }
                        })
                        .await?;

                        // 第五步：填充 pending_profile_reviews 到用户对象
                        for mut entry in users.iter_mut() {
                            let user_id = *entry.key();
                            if let Some(reviews) = pending_reviews.get(&user_id) {
                                entry.value_mut().1.pending_profile_reviews = reviews.value().clone();
                            }
                        }
                    }

                    Ok(users)
                },
            )
            .await?;

        Ok(val)
    }

    pub async fn get_email<'a, E>(
        email: &str,
        exec: E,
    ) -> Result<Option<UserId>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        let user_pass = sqlx::query!(
            "
            SELECT id FROM users
            WHERE email = $1
            ",
            email
        )
        .fetch_optional(exec)
        .await?;

        Ok(user_pass.map(|x| UserId(x.id)))
    }

    // pub async fn get_phone_number<'a, E>(
    //     phone_number: &str,
    //     exec: E,
    // ) -> Result<Option<UserId>, sqlx::Error>
    // where
    //     E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    // {
    //     let user_pass = sqlx::query!(
    //         "
    //         SELECT id FROM users
    //         WHERE phone_number = $1
    //         ",
    //         phone_number
    //     )
    //     .fetch_optional(exec)
    //     .await?;

    //     Ok(user_pass.map(|x| UserId(x.id)))
    // }

    pub async fn get_projects<'a, E>(
        user_id: UserId,
        exec: E,
        redis: &RedisPool,
    ) -> Result<Vec<ProjectId>, DatabaseError>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let mut redis = redis.connect().await?;

        let cached_projects = redis
            .get_deserialized_from_json::<Vec<ProjectId>>(
                USERS_PROJECTS_NAMESPACE,
                &user_id.0.to_string(),
            )
            .await?;

        if let Some(projects) = cached_projects {
            return Ok(projects);
        }

        let db_projects = sqlx::query!(
            "
            SELECT m.id FROM mods m
            INNER JOIN team_members tm ON tm.team_id = m.team_id AND tm.accepted = TRUE
            WHERE tm.user_id = $1
            ORDER BY m.downloads DESC
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| ProjectId(m.id))
        .try_collect::<Vec<ProjectId>>()
        .await?;

        redis
            .set_serialized_to_json(
                USERS_PROJECTS_NAMESPACE,
                user_id.0,
                &db_projects,
                None,
            )
            .await?;

        Ok(db_projects)
    }

    pub async fn get_organizations<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Vec<OrganizationId>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let orgs = sqlx::query!(
            "
            SELECT o.id FROM organizations o
            INNER JOIN team_members tm ON tm.team_id = o.team_id AND tm.accepted = TRUE
            WHERE tm.user_id = $1
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| OrganizationId(m.id))
        .try_collect::<Vec<OrganizationId>>()
        .await?;

        Ok(orgs)
    }

    pub async fn get_collections<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Vec<CollectionId>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let projects = sqlx::query!(
            "
            SELECT c.id FROM collections c
            WHERE c.user_id = $1
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| CollectionId(m.id))
        .try_collect::<Vec<CollectionId>>()
        .await?;

        Ok(projects)
    }

    pub async fn get_follows<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Vec<ProjectId>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let projects = sqlx::query!(
            "
            SELECT mf.mod_id FROM mod_follows mf
            WHERE mf.follower_id = $1
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| ProjectId(m.mod_id))
        .try_collect::<Vec<ProjectId>>()
        .await?;

        Ok(projects)
    }

    pub async fn get_reports<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Vec<ReportId>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let reports = sqlx::query!(
            "
            SELECT r.id FROM reports r
            WHERE r.user_id = $1
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| ReportId(m.id))
        .try_collect::<Vec<ReportId>>()
        .await?;

        Ok(reports)
    }

    pub async fn get_backup_codes<'a, E>(
        user_id: UserId,
        exec: E,
    ) -> Result<Vec<String>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres> + Copy,
    {
        use futures::stream::TryStreamExt;

        let codes = sqlx::query!(
            "
            SELECT code FROM user_backup_codes
            WHERE user_id = $1
            ",
            user_id as UserId,
        )
        .fetch(exec)
        .map_ok(|m| to_base62(m.code as u64))
        .try_collect::<Vec<String>>()
        .await?;

        Ok(codes)
    }

    /// 清理用户缓存（常规场景）
    /// 只删除数据键，不删除锁键
    /// 锁键会在60秒后自动过期
    pub async fn clear_caches(
        user_ids: &[(UserId, Option<String>)],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis_conn = redis.connect().await?;

        let keys: Vec<_> = user_ids
            .iter()
            .flat_map(|(id, username)| {
                [
                    (USERS_NAMESPACE, Some(id.0.to_string())),
                    (
                        USER_USERNAMES_NAMESPACE,
                        username.clone().map(|i| i.to_lowercase()),
                    ),
                ]
            })
            .collect();

        redis_conn.delete_many(keys).await?;

        Ok(())
    }

    /// 清理用户缓存并强制清理锁键（封禁操作专用）
    /// 用于封禁/解封后立即访问的场景，防止锁超时问题
    pub async fn clear_caches_with_locks(
        user_ids: &[(UserId, Option<String>)],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis_conn = redis.connect().await?;

        // 第一步：先删除锁键，让等待的请求立即失败
        // 注意：锁键的 namespace 始终是 USERS_NAMESPACE，不是 USER_USERNAMES_NAMESPACE
        // 并且需要同时删除大小写两种形式的 username 锁（因为 Redis 内部处理的复杂性）
        let lock_keys: Vec<String> = user_ids
            .iter()
            .flat_map(|(id, username)| {
                let mut keys =
                    vec![format!("_{}:{}/lock", USERS_NAMESPACE, id.0)];
                if let Some(username) = username {
                    // 同时删除原始大小写和小写两种形式
                    keys.push(format!(
                        "_{}:{}/lock",
                        USERS_NAMESPACE, username
                    ));
                    keys.push(format!(
                        "_{}:{}/lock",
                        USERS_NAMESPACE,
                        username.to_lowercase()
                    ));
                }
                keys
            })
            .collect();

        if !lock_keys.is_empty() {
            use redis::cmd;
            cmd("DEL")
                .arg(&lock_keys)
                .query_async::<usize>(&mut redis_conn.connection)
                .await?;
        }

        // 第二步：再删除数据键
        let keys: Vec<_> = user_ids
            .iter()
            .flat_map(|(id, username)| {
                [
                    (USERS_NAMESPACE, Some(id.0.to_string())),
                    (
                        USER_USERNAMES_NAMESPACE,
                        username.clone().map(|i| i.to_lowercase()),
                    ),
                ]
            })
            .collect();

        redis_conn.delete_many(keys).await?;

        // 第三步：等待一小段时间，让正在等待的请求有机会检测到锁消失
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        Ok(())
    }

    pub async fn clear_project_cache(
        user_ids: &[UserId],
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        let mut redis = redis.connect().await?;

        redis
            .delete_many(
                user_ids.iter().map(|id| {
                    (USERS_PROJECTS_NAMESPACE, Some(id.0.to_string()))
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn remove(
        id: UserId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        redis: &RedisPool,
    ) -> Result<Option<()>, DatabaseError> {
        let user = Self::get_id(id, &mut **transaction, redis).await?;

        if let Some(delete_user) = user {
            User::clear_caches(&[(id, Some(delete_user.username))], redis)
                .await?;

            let deleted_user: UserId =
                crate::models::users::DELETED_USER.into();

            sqlx::query!(
                "
                UPDATE team_members
                SET user_id = $1
                WHERE (user_id = $2 AND is_owner = TRUE)
                ",
                deleted_user as UserId,
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                UPDATE versions
                SET author_id = $1
                WHERE (author_id = $2)
                ",
                deleted_user as UserId,
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            use futures::TryStreamExt;
            let notifications: Vec<i64> = sqlx::query!(
                "
                SELECT n.id FROM notifications n
                WHERE n.user_id = $1
                ",
                id as UserId,
            )
            .fetch(&mut **transaction)
            .map_ok(|m| m.id)
            .try_collect::<Vec<i64>>()
            .await?;

            sqlx::query!(
                "
                DELETE FROM notifications
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM notifications_actions
                 WHERE notification_id = ANY($1)
                ",
                &notifications
            )
            .execute(&mut **transaction)
            .await?;

            let user_collections = sqlx::query!(
                "
                SELECT id
                FROM collections
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .fetch(&mut **transaction)
            .map_ok(|x| CollectionId(x.id))
            .try_collect::<Vec<_>>()
            .await?;

            for collection_id in user_collections {
                models::Collection::remove(collection_id, transaction, redis)
                    .await?;
            }

            let report_threads = sqlx::query!(
                "
                SELECT t.id
                FROM threads t
                INNER JOIN reports r ON t.report_id = r.id AND (r.user_id = $1 OR r.reporter = $1)
                WHERE report_id IS NOT NULL
                ",
                id as UserId,
            )
            .fetch(&mut **transaction)
            .map_ok(|x| ThreadId(x.id))
            .try_collect::<Vec<_>>()
            .await?;

            for thread_id in report_threads {
                models::Thread::remove_full(thread_id, transaction).await?;
            }

            sqlx::query!(
                "
                DELETE FROM reports
                WHERE user_id = $1 OR reporter = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM mod_follows
                WHERE follower_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM team_members
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            // 更新 payouts_values 引用为 deleted_user，而不是直接删除
            // 这样可以保留支付历史记录
            sqlx::query!(
                "
                UPDATE payouts_values
                SET user_id = $1
                WHERE user_id = $2
                ",
                deleted_user as UserId,
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM payouts
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                r#"
                UPDATE threads_messages
                SET body = '{"type": "deleted"}', author_id = $2
                WHERE author_id = $1
                "#,
                id as UserId,
                deleted_user as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM threads_members
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM sessions
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM pats
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM user_backup_codes
                WHERE user_id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            sqlx::query!(
                "
                DELETE FROM users
                WHERE id = $1
                ",
                id as UserId,
            )
            .execute(&mut **transaction)
            .await?;

            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
