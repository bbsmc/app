use std::num::NonZeroU32;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

use actix_web::web;
use chrono::Utc;
use database::redis::RedisPool;
use queue::{
    analytics::AnalyticsQueue, payouts::PayoutsQueue, session::AuthQueue,
    socket::ActiveSockets,
};
use sqlx::Postgres;
use tokio::sync::RwLock;

extern crate clickhouse as clickhouse_crate;
use clickhouse_crate::Client;
use governor::middleware::StateInformationMiddleware;
use governor::{Quota, RateLimiter};
use log::{info, warn};
use util::cors::default_cors;

use crate::database::Project;
use crate::database::models::notification_item::NotificationBuilder;
use crate::database::models::{
    OrganizationId, ProjectId, TeamId, User, UserId,
};
use crate::models::notifications::NotificationBody;
use crate::models::projects::{MonetizationStatus, ProjectStatus};
use crate::models::teams::ProjectPermissions;
use crate::models::users::{Badges, Role};
use crate::queue::moderation::AutomatedModerationQueue;
use crate::util::ratelimit::KeyedRateLimiter;
use crate::{
    search::indexing::index_projects,
    util::env::{parse_strings_from_var, parse_var},
};

pub mod auth;
pub mod clickhouse;
pub mod database;
pub mod file_hosting;
pub mod models;
pub mod queue;
pub mod routes;
pub mod scheduler;
pub mod search;
pub mod util;
pub mod validate;

#[derive(Clone)]
pub struct Pepper {
    pub pepper: String,
}

#[derive(Clone)]
pub struct LabrinthConfig {
    pub pool: sqlx::Pool<Postgres>,
    pub redis_pool: RedisPool,
    pub clickhouse: Client,
    pub file_host: Arc<dyn file_hosting::FileHost + Send + Sync>,
    /// 私有桶文件存储（用于付费插件），可选
    pub private_file_host: Option<Arc<file_hosting::S3PrivateHost>>,
    pub scheduler: Arc<scheduler::Scheduler>,
    pub ip_salt: Pepper,
    pub search_config: search::SearchConfig,
    pub session_queue: web::Data<AuthQueue>,
    pub payouts_queue: web::Data<PayoutsQueue>,
    pub analytics_queue: Arc<AnalyticsQueue>,
    pub active_sockets: web::Data<RwLock<ActiveSockets>>,
    pub automated_moderation_queue: web::Data<AutomatedModerationQueue>,
    pub rate_limiter: KeyedRateLimiter,
    // pub stripe_client: stripe::Client,
}

pub fn app_setup(
    pool: sqlx::Pool<Postgres>,
    redis_pool: RedisPool,
    search_config: search::SearchConfig,
    clickhouse: &mut Client,
    file_host: Arc<dyn file_hosting::FileHost + Send + Sync>,
    private_file_host: Option<Arc<file_hosting::S3PrivateHost>>,
) -> LabrinthConfig {
    info!("启动 Labrinth 于 {}", dotenvy::var("BIND_ADDR").unwrap());

    let automated_moderation_queue =
        web::Data::new(AutomatedModerationQueue::default());

    {
        let automated_moderation_queue_ref = automated_moderation_queue.clone();
        let pool_ref = pool.clone();
        let redis_pool_ref = redis_pool.clone();
        actix_rt::spawn(async move {
            automated_moderation_queue_ref
                .task(pool_ref, redis_pool_ref)
                .await;
        });
    }

    let mut scheduler = scheduler::Scheduler::new();

    let limiter: KeyedRateLimiter = Arc::new(
        RateLimiter::keyed(Quota::per_minute(NonZeroU32::new(500).unwrap()))
            .with_middleware::<StateInformationMiddleware>(),
    );
    let limiter_clone = Arc::clone(&limiter);
    scheduler.run(Duration::from_secs(60), move || {
        info!("清理速率限制器，存储大小：{}", limiter_clone.len());
        limiter_clone.retain_recent();
        info!("完成清理速率限制器，存储大小：{}", limiter_clone.len());

        async move {}
    });

    // 本地数据库索引的间隔时间，单位为秒。默认值为 1 小时。
    let local_index_interval = std::time::Duration::from_secs(
        parse_var("LOCAL_INDEX_INTERVAL").unwrap_or(3600),
    );

    let pool_ref = pool.clone();
    let search_config_ref = search_config.clone();
    let redis_pool_ref = redis_pool.clone();
    scheduler.run(local_index_interval, move || {
        let pool_ref = pool_ref.clone();
        let redis_pool_ref = redis_pool_ref.clone();
        let search_config_ref = search_config_ref.clone();
        async move {
            info!("索引本地数据库");
            let result = index_projects(
                pool_ref,
                redis_pool_ref.clone(),
                &search_config_ref,
            )
            .await;
            if let Err(e) = result {
                warn!("本地项目索引失败：{:?}", e);
            }
            info!("完成索引本地数据库");
        }
    });

    // Changes statuses of scheduled projects/versions
    let pool_ref = pool.clone();
    // TODO: Clear cache when these are run
    scheduler.run(std::time::Duration::from_secs(60 * 5), move || {
        let pool_ref = pool_ref.clone();
        // info!("发布计划的版本/项目！");

        async move {
            let projects_results = sqlx::query!(
                "
                UPDATE mods
                SET status = requested_status
                WHERE status = $1 AND approved < CURRENT_DATE AND requested_status IS NOT NULL
                ",
                crate::models::projects::ProjectStatus::Scheduled.as_str(),
            )
                .execute(&pool_ref)
                .await;

            if let Err(e) = projects_results {
                warn!("同步计划的项目发布失败：{:?}", e);
            }

            let versions_results = sqlx::query!(
                "
                UPDATE versions
                SET status = requested_status
                WHERE status = $1 AND date_published < CURRENT_DATE AND requested_status IS NOT NULL
                ",
                crate::models::projects::VersionStatus::Scheduled.as_str(),
            )
                .execute(&pool_ref)
                .await;

            if let Err(e) = versions_results {
                warn!("同步计划的版本发布失败：{:?}", e);
            }

            // info!("完成发布计划的版本/项目");
        }
    });

    scheduler::schedule_versions(
        &mut scheduler,
        pool.clone(),
        redis_pool.clone(),
    );

    scheduler::schedule_translation_tracking(
        &mut scheduler,
        pool.clone(),
        redis_pool.clone(),
    );

    // 每 5 分钟清理超过 12 小时未付款的待支付订单
    let pool_ref = pool.clone();
    scheduler.run(std::time::Duration::from_secs(60 * 5), move || {
        let pool_ref = pool_ref.clone();
        async move {
            match database::models::PaymentOrder::delete_stale_pending_orders(
                &pool_ref,
            )
            .await
            {
                Ok(count) if count > 0 => {
                    info!("已清理 {} 个过期未付款订单", count);
                }
                Err(e) => {
                    log::error!("清理过期订单失败: {:?}", e);
                }
                _ => {}
            }
        }
    });

    let session_queue = web::Data::new(AuthQueue::new());

    let pool_ref = pool.clone();
    let redis_ref = redis_pool.clone();
    let session_queue_ref = session_queue.clone();
    scheduler.run(std::time::Duration::from_secs(60 * 30), move || {
        let pool_ref = pool_ref.clone();
        let redis_ref = redis_ref.clone();
        let session_queue_ref = session_queue_ref.clone();

        async move {
            // info!("索引会话队列");
            let result = session_queue_ref.index(&pool_ref, &redis_ref).await;
            if let Err(e) = result {
                warn!("索引会话队列失败： {:?}", e);
            }
            // info!("完成索引会话队列");
        }
    });

    let analytics_queue = Arc::new(AnalyticsQueue::new());
    {
        let client_ref = clickhouse.clone();
        let analytics_queue_ref = analytics_queue.clone();
        let pool_ref = pool.clone();
        let redis_ref = redis_pool.clone();
        scheduler.run(std::time::Duration::from_secs(15), move || {
            let client_ref = client_ref.clone();
            let analytics_queue_ref = analytics_queue_ref.clone();
            let pool_ref = pool_ref.clone();
            let redis_ref = redis_ref.clone();

            async move {
                info!("开始索引分析服务");
                let result = analytics_queue_ref
                    .index(client_ref, &redis_ref, &pool_ref)
                    .await;
                if let Err(e) = result {
                    warn!("分析服务索引失败: {:?}", e);
                }
                info!("分析索引完成");
            }
        });
    }

    info!("启动检测超时百科编辑");
    {
        let pool_ref_clone2 = pool.clone();
        let redis_ref2 = redis_pool.clone();

        scheduler.run(std::time::Duration::from_secs(60), move || {
            let pool_ref_clone2 = pool_ref_clone2.clone();
            let redis_ref2 = redis_ref2.clone();

            async move {
                // info!("开始检测超时百科编辑");

                let result = database::models::WikiCache::get_all_draft(&pool_ref_clone2).await;
                match result {
                    Ok(items) => {
                        for item in items {
                            if item.again_time.add(Duration::from_secs(3600 * 5)) < Utc::now() {


                                let projects = sqlx::query!(
                                    "
                                    SELECT m.id id, m.name name, m.summary summary, m.downloads downloads, m.follows follows,
                                    m.icon_url icon_url, m.raw_icon_url raw_icon_url, m.description description, m.published published,
                                    m.updated updated, m.approved approved, m.queued, m.status status, m.requested_status requested_status,
                                    m.license_url license_url,
                                    m.team_id team_id, m.organization_id organization_id, m.license license, m.slug slug, m.moderation_message moderation_message, m.moderation_message_body moderation_message_body,
                                    m.webhook_sent, m.color, m.wiki_open,m.issues_type issues_type, m.translation_tracking, m.translation_tracker, m.is_paid,
                                    (SELECT slug FROM mods WHERE translation_tracker = m.slug AND m.slug IS NOT NULL LIMIT 1) as translation_source,
                                    t.id thread_id, m.monetization_status monetization_status,
                                    ARRAY_AGG(DISTINCT c.category) filter (where c.category is not null and mc.is_additional is false) categories,
                                    ARRAY_AGG(DISTINCT c.category) filter (where c.category is not null and mc.is_additional is true) additional_categories
                                    FROM mods m
                                    INNER JOIN threads t ON t.mod_id = m.id
                                    LEFT JOIN mods_categories mc ON mc.joining_mod_id = m.id
                                    LEFT JOIN categories c ON mc.joining_category_id = c.id
                                    WHERE m.id = ANY($1)
                                    GROUP BY t.id, m.id;
                                    ",
                                    &[item.project_id.clone().0]
                                ).fetch_all(&pool_ref_clone2).await;
                                match projects {
                                    Ok(projects) => {
                                        let m = projects.first().unwrap();
                                        let id = m.id;
                                        let inner = Project {
                                            id: ProjectId(id),
                                            team_id: TeamId(m.team_id),
                                            organization_id: m.organization_id.map(OrganizationId),
                                            name: m.name.clone(),
                                            summary: m.summary.clone(),
                                            downloads: m.downloads,
                                            icon_url: m.icon_url.clone(),
                                            raw_icon_url: m.raw_icon_url.clone(),
                                            published: m.published,
                                            updated: m.updated,
                                            license_url: m.license_url.clone(),
                                            status: ProjectStatus::from_string(
                                                &m.status,
                                            ),
                                            requested_status: None,
                                            license: m.license.clone(),
                                            slug: m.slug.clone(),
                                            description: m.description.clone(),
                                            follows: m.follows,
                                            moderation_message: None,
                                            moderation_message_body: None,
                                            approved: m.approved,
                                            webhook_sent: m.webhook_sent,
                                            wiki_open: m.wiki_open,
                                            color: m.color.map(|x| x as u32),
                                            queued: m.queued,
                                            monetization_status: MonetizationStatus::from_string(
                                                &m.monetization_status,
                                            ),
                                            loaders: vec![],
                                            issues_type: m.issues_type,
                                            forum: None,
                                            translation_tracking: m.translation_tracking,
                                            translation_tracker: m.translation_tracker.clone(),
                                            translation_source: m.translation_source.clone(),
                                            is_paid: m.is_paid,
                                        };
                                        // println!("{:?}", inner);

                                        let users = sqlx::query!(
                                            "
                                            SELECT id, email,
                                                avatar_url, raw_avatar_url, username, bio,
                                                created, role, badges,
                                                github_id, discord_id, gitlab_id, google_id, steam_id, microsoft_id,
                                                email_verified, password, totp_secret, paypal_id, paypal_country, paypal_email,
                                                venmo_handle, stripe_customer_id,wiki_overtake_count,wiki_ban_time
                                            FROM users
                                            WHERE id = $1
                                            ",
                                            &item.user_id.0
                                        ).fetch_all(&pool_ref_clone2).await;
                                        match users {
                                            Ok(users) => {
                                                let u = users.first().unwrap();
                                                let user = User {
                                                    id: UserId(u.id),
                                                    github_id: None,
                                                    discord_id: None,
                                                    gitlab_id: None,
                                                    google_id: None,
                                                    steam_id: None,
                                                    microsoft_id: None,
                                                    bilibili_id: None,
                                                    qq_id: None,
                                                    email: None,
                                                    email_verified: true,
                                                    avatar_url: None,
                                                    raw_avatar_url: None,
                                                    username: u.username.clone(),
                                                    bio: u.bio.clone(),
                                                    created: u.created,
                                                    role: u.role.clone(),
                                                    badges: Badges::from_bits(u.badges as u64).unwrap_or_default(),
                                                    password: None,
                                                    paypal_id: None,
                                                    paypal_country: None,
                                                    paypal_email: None,
                                                    venmo_handle: None,
                                                    stripe_customer_id: None,
                                                    totp_secret: None,
                                                    wiki_overtake_count: u.wiki_overtake_count,
                                                    wiki_ban_time: u.wiki_ban_time,
                                                    phone_number: None,
                                                    is_premium_creator: false,
                                                    creator_verified_at: None,
                                                    active_bans: vec![],
                                                    pending_profile_reviews: vec![],
                                                };

                                                let (team_member, organization_team_member) = match crate::database::models::TeamMember::get_for_project_permissions(&inner, item.user_id, &pool_ref_clone2).await {
                                                    Ok(result) => result,
                                                    Err(e) => {
                                                        warn!("Failed to get team member: {:?}", e);
                                                        continue;
                                                    }
                                                };
                                                // println!("{:?}", team_member);

                                                let permissions = ProjectPermissions::get_permissions_by_role(
                                                    &Role::from_string(&user.role),
                                                    &team_member,
                                                    &organization_team_member,
                                                );
                                                if permissions.is_some() && permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT) {
                                                    let mut transaction = match pool_ref_clone2.begin().await{
                                                        Ok(transaction) => transaction,
                                                        Err(e) => {
                                                            println!("Failed to get transaction: {:?}", e);
                                                            continue
                                                        }
                                                    };

                                                    match sqlx::query!(
                                                       "UPDATE wiki_cache SET status = $1, message = $2 WHERE id = $3",
                                                        "reject",
                                                        item.message,
                                                        item.id.0
                                                    ).execute( &mut *transaction).await{
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to update wiki_cache: {:?}", e);
                                                            continue
                                                        }
                                                    };
                                                    let n = NotificationBuilder {
                                                        body: NotificationBody::WikiCache {
                                                            project_id: crate::models::ids::ProjectId::from(inner.id),
                                                            project_title: inner.name.clone(),
                                                            wiki_cache_id: item.id,
                                                            type_: "reject".to_string(),
                                                            msg: "资源编辑超时，您是该资源的管理员可重新发起编辑".to_string(),
                                                        },
                                                    };
                                                    match n.insert(user.id, &mut transaction, &redis_ref2).await{
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to insert notification: {:?}", e);
                                                            continue
                                                        }
                                                    };
                                                    match transaction.commit().await{
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to commit transaction: {:?}", e);
                                                            continue
                                                        }
                                                    };

                                                }else {
                                                    println!("无权限超时");

                                                    let mut transaction = match pool_ref_clone2.begin().await{
                                                        Ok(transaction) => transaction,
                                                        Err(e) => {
                                                            println!("Failed to get transaction: {:?}", e);
                                                            continue
                                                        }
                                                    };
                                                    match sqlx::query!(
                                                       "UPDATE wiki_cache SET status = $1, message = $2 WHERE id = $3",
                                                        "timeout",
                                                        item.message,
                                                        item.id.0
                                                    ).execute(&mut *transaction).await {
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to update wiki_cache: {:?}", e);
                                                            continue
                                                        }
                                                    };

                                                    if user.wiki_overtake_count+1 >= 3 {
                                                        match sqlx::query!(
                                                           "UPDATE users SET wiki_ban_time = now() + interval '1 hour' * $1,wiki_overtake_count=0 WHERE id = $2",
                                                            72 as i64,
                                                            user.id.0
                                                        ).execute(&mut *transaction).await{
                                                            Ok(_) => {},
                                                            Err(e) => {
                                                                println!("Failed to update users: {:?}", e);
                                                                continue
                                                            }
                                                        };
                                                        println!("{:?} 超时未提交,并累计超过3次,已被禁止编辑72小时", user.username);
                                                        let n = NotificationBuilder {
                                                            body: NotificationBody::WikiCache {
                                                                project_id: crate::models::ids::ProjectId::from(inner.id),
                                                                project_title: inner.name.clone(),
                                                                wiki_cache_id: item.id,
                                                                type_: "time_out".to_string(),
                                                                msg: "资源编辑超时，并且累计3次超时/取消编辑 您已经被禁止编辑72小时".to_string(),
                                                            },
                                                        };
                                                        match n.insert(user.id, &mut transaction, &redis_ref2).await{
                                                            Ok(_) => {},
                                                            Err(e) => {
                                                                println!("Failed to insert notification: {:?}", e);
                                                                continue
                                                            }
                                                        };

                                                    }else {
                                                        match sqlx::query!(
                                                           "UPDATE users SET wiki_ban_time = now() + interval '1 hour' * $1,wiki_overtake_count=wiki_overtake_count+1 WHERE id = $2",
                                                            24 as i64,
                                                            user.id.0
                                                        ).execute(&mut *transaction).await{
                                                            Ok(_) => {},
                                                            Err(e) => {
                                                                println!("Failed to update users: {:?}", e);
                                                                continue
                                                            }
                                                        };
                                                        println!("{:?} 超时未提交 已被禁止编辑24小时",user.username);
                                                        let n = NotificationBuilder {
                                                            body: NotificationBody::WikiCache {
                                                                project_id: crate::models::ids::ProjectId::from(inner.id),
                                                                project_title: inner.name.clone(),
                                                                wiki_cache_id: item.id,
                                                                type_: "time_out".to_string(),
                                                                msg: "资源编辑超时，您已被禁止编辑24小时".to_string(),
                                                            },
                                                        };
                                                        match n.insert(user.id, &mut transaction, &redis_ref2)
                                                            .await{

                                                            Ok(_) => {},
                                                            Err(e) => {
                                                                println!("Failed to insert notification: {:?}", e);
                                                                continue
                                                            }
                                                        };
                                                    }


                                                    match transaction.commit().await{
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to commit transaction: {:?}", e);
                                                            continue
                                                        }
                                                    };
                                                    match User::clear_caches(&[(user.id, Some(user.username.clone()))], &redis_ref2)
                                                        .await{
                                                        Ok(_) => {},
                                                        Err(e) => {
                                                            println!("Failed to clear caches: {:?}", e);
                                                            continue
                                                        }
                                                    };
                                                    continue

                                                }

                                            }
                                            Err(e) => {
                                                println!("Failed to get user with ID {:?}: {:?}", item.user_id.clone(), e);
                                                continue
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to get user with ID {:?}: {:?}", item.user_id.clone(), e);
                                        continue
                                    }
                                }
                                // let project_result_ = database::models::Project::get_id(item.project_id.clone(), &pool_ref_clone2).await;
                                // if project_result_.is_err() {
                                // if project_result_.await.is_err() {
                                //     continue;
                                // }
                                //     .await;
                                // 处理 project_result_ 的逻辑

                                // if let DatabaseError(e) = project_result_ {
                                //     println!("Failed to get project with ID {:?}: {:?}", item.project_id.clone(), e);
                                //     continue;
                                // }
                                // if project_result_.is_err() {
                                //     continue
                                // }

                                // let project = project_result_.await.unwrap(); // 此时 unwrap 是安全的，因为前面已经检查过错误情况
                                // if project.is_none() {
                                //     continue
                                // }
                                // let project = project.unwrap();
                                // println!("{:?}", project);

                                // let user_option =
                                //     match database::models::User::get_id(
                                //         item.user_id.clone(),
                                //         &pool_ref_clone2, &redis_ref2,
                                //     ).await{
                                //         Ok(user) => user,
                                //         Err(e) => {
                                //             warn!("Failed to get user: {:?}", e);
                                //             return;
                                //         }
                                //     };
                                // if user_option.is_none() {
                                //     continue;
                                // }
                                // let user_option = user_option.unwrap();
                                // let (team_member, organization_team_member) = match crate::database::models::TeamMember::get_for_project_permissions(&project.inner, item.user_id.clone(), &pool_ref_clone2).await {
                                //     Ok(result) => result,
                                //     Err(e) => {
                                //         warn!("Failed to get team member: {:?}", e);
                                //         return;
                                //     }
                                // };
                                //
                                //
                                // let permissions = ProjectPermissions::get_permissions_by_role(
                                //     &Role::from_string(&user_option.role),
                                //     &team_member,
                                //     &organization_team_member,
                                // );
                                // if !permissions.is_none() && permissions.unwrap().contains(ProjectPermissions::WIKI_EDIT) {
                                //     println!("有权限超时");
                                //     continue
                                // }else {
                                //     println!("无权限超时")
                                // }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("检测超时百科编辑失败: {:?}", e);
                    }
                }
                // info!("检测超时百科编辑完成");
            }
        });
    }

    // {
    //     let pool_ref = pool.clone();
    //     let client_ref = clickhouse.clone();
    //     scheduler.run(std::time::Duration::from_secs(60 * 60 * 6), move || {
    //         let pool_ref = pool_ref.clone();
    //         let client_ref = client_ref.clone();
    //
    //         async move {
    //             info!("Started running payouts");
    //             let result = process_payout(&pool_ref, &client_ref).await;
    //             if let Err(e) = result {
    //                 warn!("Payouts run failed: {:?}", e);
    //             }
    //             info!("Done running payouts");
    //         }
    //     });
    // }

    // let stripe_client =
    //     stripe::Client::new(dotenvy::var("STRIPE_API_KEY").unwrap());
    // {
    //     let pool_ref = pool.clone();
    //     let redis_ref = redis_pool.clone();
    //     let stripe_client_ref = stripe_client.clone();
    //
    //     actix_rt::spawn(async move {
    //         routes::internal::billing::task(
    //             stripe_client_ref,
    //             pool_ref,
    //             redis_ref,
    //         )
    //         .await;
    //     });
    // }
    //
    // {
    //     let pool_ref = pool.clone();
    //     let redis_ref = redis_pool.clone();
    //
    //     actix_rt::spawn(async move {
    //         routes::internal::billing::subscription_task(pool_ref, redis_ref)
    //             .await;
    //     });
    // }

    let ip_salt = Pepper {
        pepper: models::ids::Base62Id(models::ids::random_base62(11))
            .to_string(),
    };

    let payouts_queue = web::Data::new(PayoutsQueue::new());
    let active_sockets = web::Data::new(RwLock::new(ActiveSockets::default()));

    LabrinthConfig {
        pool,
        redis_pool,
        clickhouse: clickhouse.clone(),
        file_host,
        private_file_host,
        scheduler: Arc::new(scheduler),
        ip_salt,
        search_config,
        session_queue,
        payouts_queue,
        analytics_queue,
        active_sockets,
        automated_moderation_queue,
        rate_limiter: limiter,
    }
}

pub fn app_config(
    cfg: &mut web::ServiceConfig,
    labrinth_config: LabrinthConfig,
) {
    cfg.app_data(web::FormConfig::default().error_handler(|err, _req| {
        routes::ApiError::Validation(err.to_string()).into()
    }))
    .app_data(web::PathConfig::default().error_handler(|err, _req| {
        routes::ApiError::Validation(err.to_string()).into()
    }))
    .app_data(web::QueryConfig::default().error_handler(|err, _req| {
        routes::ApiError::Validation(err.to_string()).into()
    }))
    .app_data(web::JsonConfig::default().error_handler(|err, _req| {
        routes::ApiError::Validation(err.to_string()).into()
    }))
    .app_data(web::Data::new(labrinth_config.redis_pool.clone()))
    .app_data(web::Data::new(labrinth_config.pool.clone()))
    .app_data(web::Data::new(labrinth_config.file_host.clone()))
    .app_data(web::Data::new(labrinth_config.private_file_host.clone()))
    .app_data(web::Data::new(labrinth_config.search_config.clone()))
    .app_data(labrinth_config.session_queue.clone())
    .app_data(labrinth_config.payouts_queue.clone())
    .app_data(web::Data::new(labrinth_config.ip_salt.clone()))
    .app_data(web::Data::new(labrinth_config.analytics_queue.clone()))
    .app_data(web::Data::new(labrinth_config.clickhouse.clone()))
    .app_data(labrinth_config.active_sockets.clone())
    .app_data(labrinth_config.automated_moderation_queue.clone())
    // .app_data(web::Data::new(labrinth_config.stripe_client.clone()))
    .configure(routes::v2::config)
    .configure(routes::v3::config)
    .configure(routes::internal::config)
    .configure(routes::root_config)
    .default_service(web::get().wrap(default_cors()).to(routes::not_found));
}

// This is so that env vars not used immediately don't panic at runtime
pub fn check_env_vars() -> bool {
    let mut failed = false;

    fn check_var<T: std::str::FromStr>(var: &'static str) -> bool {
        let check = parse_var::<T>(var).is_none();
        if check {
            warn!(
                "变量 `{}` 在 dotenv 中缺失或不是类型 `{}`",
                var,
                std::any::type_name::<T>()
            );
        }
        check
    }

    failed |= check_var::<String>("SITE_URL");
    failed |= check_var::<String>("CDN_URL");
    failed |= check_var::<String>("LABRINTH_ADMIN_KEY");
    failed |= check_var::<String>("RATE_LIMIT_IGNORE_KEY");
    failed |= check_var::<String>("DATABASE_URL");
    failed |= check_var::<String>("MEILISEARCH_ADDR");
    failed |= check_var::<String>("MEILISEARCH_KEY");
    failed |= check_var::<String>("REDIS_URL");
    failed |= check_var::<String>("BIND_ADDR");
    failed |= check_var::<String>("SELF_ADDR");

    failed |= check_var::<String>("STORAGE_BACKEND");

    let storage_backend = dotenvy::var("STORAGE_BACKEND").ok();
    match storage_backend.as_deref() {
        Some("backblaze") => {
            failed |= check_var::<String>("BACKBLAZE_KEY_ID");
            failed |= check_var::<String>("BACKBLAZE_KEY");
            failed |= check_var::<String>("BACKBLAZE_BUCKET_ID");
        }
        Some("s3") => {
            failed |= check_var::<String>("S3_ACCESS_TOKEN");
            failed |= check_var::<String>("S3_SECRET");
            failed |= check_var::<String>("S3_URL");
            failed |= check_var::<String>("S3_REGION");
            failed |= check_var::<String>("S3_BUCKET_NAME");
        }
        Some("local") => {
            failed |= check_var::<String>("MOCK_FILE_PATH");
        }
        Some(backend) => {
            warn!(
                "变量 `STORAGE_BACKEND` 包含无效值：{}。预期值为 \"backblaze\"、\"s3\" 或 \"local\"。",
                backend
            );
            failed |= true;
        }
        _ => {
            warn!("变量 `STORAGE_BACKEND` 未设置！");
            failed |= true;
        }
    }

    failed |= check_var::<usize>("LOCAL_INDEX_INTERVAL");
    failed |= check_var::<usize>("VERSION_INDEX_INTERVAL");

    if parse_strings_from_var("WHITELISTED_MODPACK_DOMAINS").is_none() {
        warn!(
            "变量 `WHITELISTED_MODPACK_DOMAINS` 在 dotenv 中缺失或不是字符串数组"
        );
        failed |= true;
    }

    if parse_strings_from_var("ALLOWED_CALLBACK_URLS").is_none() {
        warn!("变量 `ALLOWED_CALLBACK_URLS` 在 dotenv 中缺失或不是字符串数组");
        failed |= true;
    }

    failed |= check_var::<String>("GITHUB_CLIENT_ID");
    failed |= check_var::<String>("GITHUB_CLIENT_SECRET");
    failed |= check_var::<String>("GITLAB_CLIENT_ID");
    failed |= check_var::<String>("GITLAB_CLIENT_SECRET");
    failed |= check_var::<String>("DISCORD_CLIENT_ID");
    failed |= check_var::<String>("DISCORD_CLIENT_SECRET");
    failed |= check_var::<String>("MICROSOFT_CLIENT_ID");
    failed |= check_var::<String>("MICROSOFT_CLIENT_SECRET");
    failed |= check_var::<String>("GOOGLE_CLIENT_ID");
    failed |= check_var::<String>("GOOGLE_CLIENT_SECRET");
    failed |= check_var::<String>("STEAM_API_KEY");

    failed |= check_var::<String>("BILIBILI_CLIENT_ID");
    failed |= check_var::<String>("BILIBILI_CLIENT_SECRET");

    failed |= check_var::<String>("QQ_CLIENT_ID");
    failed |= check_var::<String>("QQ_CLIENT_SECRET");

    failed |= check_var::<String>("TREMENDOUS_API_URL");
    failed |= check_var::<String>("TREMENDOUS_API_KEY");
    failed |= check_var::<String>("TREMENDOUS_PRIVATE_KEY");

    failed |= check_var::<String>("PAYPAL_API_URL");
    failed |= check_var::<String>("PAYPAL_WEBHOOK_ID");
    failed |= check_var::<String>("PAYPAL_CLIENT_ID");
    failed |= check_var::<String>("PAYPAL_CLIENT_SECRET");

    failed |= check_var::<String>("HCAPTCHA_SECRET");

    failed |= check_var::<String>("SMTP_USERNAME");
    failed |= check_var::<String>("SMTP_PASSWORD");
    failed |= check_var::<String>("SMTP_HOST");

    failed |= check_var::<String>("SITE_VERIFY_EMAIL_PATH");
    failed |= check_var::<String>("SITE_RESET_PASSWORD_PATH");
    failed |= check_var::<String>("SITE_BILLING_PATH");

    failed |= check_var::<String>("BEEHIIV_PUBLICATION_ID");
    failed |= check_var::<String>("BEEHIIV_API_KEY");

    if parse_strings_from_var("ANALYTICS_ALLOWED_ORIGINS").is_none() {
        warn!(
            "变量 `ANALYTICS_ALLOWED_ORIGINS` 在 dotenv 中缺失或不是字符串数组"
        );
        failed |= true;
    }

    failed |= check_var::<String>("CLICKHOUSE_URL");
    failed |= check_var::<String>("CLICKHOUSE_USER");
    failed |= check_var::<String>("CLICKHOUSE_PASSWORD");
    failed |= check_var::<String>("CLICKHOUSE_DATABASE");

    failed |= check_var::<String>("FLAME_ANVIL_URL");

    failed |= check_var::<String>("STRIPE_API_KEY");
    failed |= check_var::<String>("STRIPE_WEBHOOK_SECRET");

    // failed |= check_var::<u64>("ADITUDE_API_KEY");

    failed |= check_var::<String>("PYRO_API_KEY");

    failed
}
