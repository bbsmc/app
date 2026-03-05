use actix_web::{App, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;
use labrinth::database::redis::RedisPool;
use labrinth::file_hosting::{S3Host, S3PrivateHost};
use labrinth::search;
use labrinth::util::ratelimit::RateLimit;
use labrinth::{check_env_vars, clickhouse, database, file_hosting};
use std::sync::Arc;
use tracing::{error, info};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Clone)]
pub struct Pepper {
    pub pepper: String,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    modrinth_log::init().expect("日志系统初始化失败");

    if check_env_vars() {
        error!("某些环境变量丢失！");
    }

    // DSN is from SENTRY_DSN env variable.
    // Has no effect if not set.
    let sentry = sentry::init(sentry::ClientOptions {
        release: sentry::release_name!(),
        traces_sample_rate: 0.1,
        ..Default::default()
    });
    if sentry.is_enabled() {
        info!("启用 Sentry 集成");
        // SAFETY: Setting RUST_BACKTRACE environment variable is safe in this context
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    info!("启动 Labrinth 于 {}", dotenvy::var("BIND_ADDR").unwrap());

    database::check_for_migrations()
        .await
        .expect("An error occurred while running migrations.");

    // Database Connector
    let pool = database::connect().await.expect("数据库连接失败");

    // Redis connector
    info!("初始化 Redis 连接");
    let redis_pool = RedisPool::new(None);

    info!("Redis 连接已建立");
    let storage_backend =
        dotenvy::var("STORAGE_BACKEND").unwrap_or_else(|_| "local".to_string());

    let file_host: Arc<dyn file_hosting::FileHost + Send + Sync> =
        match storage_backend.as_str() {
            "backblaze" => Arc::new(
                file_hosting::BackblazeHost::new(
                    &dotenvy::var("BACKBLAZE_KEY_ID").unwrap(),
                    &dotenvy::var("BACKBLAZE_KEY").unwrap(),
                    &dotenvy::var("BACKBLAZE_BUCKET_ID").unwrap(),
                )
                .await,
            ),
            "s3" => Arc::new(
                S3Host::new(
                    &dotenvy::var("S3_BUCKET_NAME").unwrap(),
                    &dotenvy::var("S3_URL").unwrap(),
                    &dotenvy::var("S3_ACCESS_TOKEN").unwrap(),
                    &dotenvy::var("S3_SECRET").unwrap(),
                )
                .unwrap(),
            ),
            "local" => Arc::new(file_hosting::MockHost::new()),
            _ => panic!("指定了无效的存储后端。启动中止！"),
        };

    // 初始化私有桶存储（用于付费插件）
    let private_file_host: Option<Arc<S3PrivateHost>> =
        if storage_backend == "s3" {
            match dotenvy::var("S3_PRIVATE_BUCKET_NAME") {
                Ok(bucket) if bucket != "none" && !bucket.is_empty() => {
                    let cdn_url = dotenvy::var("CDN_PRIVATE_URL").ok();
                    let cdn_url = cdn_url
                        .as_deref()
                        .filter(|s| *s != "none" && !s.is_empty());

                    match S3PrivateHost::new_with_cdn(
                        &bucket,
                        &dotenvy::var("S3_URL").unwrap(),
                        &dotenvy::var("S3_ACCESS_TOKEN").unwrap(),
                        &dotenvy::var("S3_SECRET").unwrap(),
                        cdn_url,
                    ) {
                        Ok(host) => {
                            info!("私有桶存储已启用: {}", bucket);
                            if cdn_url.is_some() {
                                info!("私有桶 CDN URL 已配置");
                            }
                            Some(Arc::new(host))
                        }
                        Err(e) => {
                            error!("初始化私有桶存储失败: {:?}", e);
                            None
                        }
                    }
                }
                _ => {
                    info!("私有桶存储未配置，付费插件功能将不可用");
                    None
                }
            }
        } else {
            None
        };

    info!("初始化 clickhouse 连接");
    let mut clickhouse = clickhouse::init_client().await.unwrap();
    let prometheus = PrometheusMetricsBuilder::new("labrinth")
        .endpoint("/metrics")
        .exclude_regex("^/v[23]/project/[^/]+(/.*)?$") // 排除所有 /project/{id} 相关路由
        .exclude_regex("^/v[23]/version/[^/]+/download$")
        .build()
        .expect("创建 prometheus 指标中间件失败");
    println!("prometheus: 正常");
    let search_config = search::SearchConfig::new(None);
    println!("search_config: 正常");
    let labrinth_config = labrinth::app_setup(
        pool.clone(),
        redis_pool.clone(),
        search_config.clone(),
        &mut clickhouse,
        file_host.clone(),
        private_file_host,
    );

    info!("启动 Actix HTTP 服务器！");

    // Init App
    HttpServer::new(move || {
        App::new()
            .wrap(prometheus.clone())
            .wrap(RateLimit(Arc::clone(&labrinth_config.rate_limiter)))
            .wrap(actix_web::middleware::Compress::default())
            .wrap(sentry_actix::Sentry::new())
            .configure(|cfg| labrinth::app_config(cfg, labrinth_config.clone()))
    })
    .bind(dotenvy::var("BIND_ADDR").unwrap())?
    .run()
    .await
}
