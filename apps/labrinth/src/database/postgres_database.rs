use crate::util::date::APP_TZ_NAME;
use log::info;
use sqlx::Executor;
use sqlx::migrate::{Migrate, MigrateDatabase, MigrateError, Migrator};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Connection, PgConnection, Postgres};
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    info!("Initializing database connection");
    let database_url =
        dotenvy::var("DATABASE_URL").expect("`DATABASE_URL` not in .env");
    let pool = PgPoolOptions::new()
        .min_connections(
            dotenvy::var("DATABASE_MIN_CONNECTIONS")
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0),
        )
        .max_connections(
            dotenvy::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(16),
        )
        .max_lifetime(Some(Duration::from_secs(60 * 60)))
        // 让所有连接默认使用 APP_TZ_NAME（北京时间），从而 CURRENT_DATE / DATE_TRUNC /
        // DATE() 等针对 timestamptz 的隐式时区转换自动按北京日聚合。
        // timestamptz 字段的存储/读取仍然是 UTC，由 sqlx + chrono::DateTime<Utc> 强类型保证。
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                let stmt = format!("SET TIME ZONE '{APP_TZ_NAME}'");
                conn.execute(stmt.as_str()).await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await?;

    Ok(pool)
}
pub async fn check_for_migrations() -> Result<(), sqlx::Error> {
    let uri =
        dotenvy::var("DATABASE_URL").expect("`DATABASE_URL` 未在 .env 中设置");
    let uri = uri.as_str();
    if !Postgres::database_exists(uri).await? {
        info!("正在创建数据库...");
        Postgres::create_database(uri).await?;
    }

    info!("正在检查数据结构版本更新...");

    let mut conn: PgConnection = PgConnection::connect(uri).await?;
    let mut migrator = sqlx::migrate!();
    conn.lock().await?;

    let migration_result =
        run_migrations_accepting_existing_versions(&mut conn, &mut migrator)
            .await;
    let unlock_result = conn.unlock().await;
    if let Err(unlock_error) = unlock_result {
        return Err(unlock_error.into());
    }

    migration_result.expect("运行数据库迁移时出错！");

    Ok(())
}

async fn run_migrations_accepting_existing_versions(
    conn: &mut PgConnection,
    migrator: &mut Migrator,
) -> Result<(), MigrateError> {
    accept_existing_migrations_by_version(conn, migrator).await?;
    migrator.set_locking(false);
    migrator.run(conn).await
}

async fn accept_existing_migrations_by_version(
    conn: &mut PgConnection,
    migrator: &mut Migrator,
) -> Result<(), MigrateError> {
    conn.ensure_migrations_table().await?;

    let applied_migrations: HashMap<i64, Vec<u8>> = conn
        .list_applied_migrations()
        .await?
        .into_iter()
        .map(|migration| (migration.version, migration.checksum.into_owned()))
        .collect();

    let mut accepted_versions = 0;
    let mut replaced_checksums = 0;
    for migration in migrator.migrations.to_mut() {
        if let Some(checksum) = applied_migrations.get(&migration.version) {
            accepted_versions += 1;

            if migration.checksum.as_ref() != checksum {
                migration.checksum = Cow::Owned(checksum.clone());
                replaced_checksums += 1;
            }
        }
    }

    if accepted_versions > 0 {
        info!(
            "Accepted {accepted_versions} already-applied SQLx migrations by version; replaced {replaced_checksums} local checksums with database checksums"
        );
    }

    Ok(())
}
