//! 汉化追踪调度器
//!
//! 每 5 分钟执行一次，检查启用了汉化追踪的项目，
//! 同步上游更新并更新汉化内容。

use crate::database::models::DatabaseError;
use crate::database::models::ids::{ProjectId, generate_project_id};
use crate::database::models::project_item::{Project, ProjectBuilder};
use crate::database::models::team_item::TeamBuilder;
use crate::database::models::thread_item::ThreadBuilder;
use crate::database::redis::RedisPool;
use crate::models::projects::{MonetizationStatus, ProjectStatus};
use crate::models::threads::ThreadType;
use chrono::Utc;
use log::debug;
use thiserror::Error;

use super::Scheduler;

/// 汉化组织的 slug（从环境变量读取，默认为 bbsmc-cn）
fn get_cn_org_slug() -> String {
    dotenvy::var("CN_ORG_SLUG").unwrap_or_else(|_| "bbsmc-cn".to_string())
}

/// 汉化组 QQ 群号（从环境变量读取）
fn get_qq_group() -> String {
    dotenvy::var("CN_QQ_GROUP").unwrap_or_default()
}

/// 汉化包描述模板 - 第一部分：基本信息
fn get_description_part1(slug: &str) -> String {
    format!(
        "# {} 汉化包\n\n此资源包含 {} 的中文本地化内容。\n\n由 BBSMC 汉化组维护。",
        slug, slug
    )
}

/// 汉化包描述模板 - 第二部分：使用教程
fn get_description_part2() -> &'static str {
    r#"

## 使用教程

此汉化包为覆盖文件类型，需要将压缩包内的所有文件解压后覆盖到游戏运行目录中。

**[>>> 点击查看图文安装教程 <<<](https://bbsmc.net/install-tutorial)**

### 安装步骤

1. **下载汉化包**：点击上方的下载按钮获取最新版本的汉化包
2. **解压文件**：将下载的压缩包解压
3. **定位游戏目录**：找到整合包的运行目录（推荐通过 PCL2 的「版本设置」→「版本文件夹」快速定位）
4. **覆盖文件**：将解压后的所有文件和文件夹复制到游戏目录中，**选择替换原有文件**
5. **启动游戏**：重新启动游戏即可看到汉化效果

> **注意**：每次整合包更新后，请重新下载对应版本的汉化包并重复上述步骤。

## 特别说明

本汉化包已删除所有内置整合包中的广告内容，游戏内多人游戏列表和单人游戏创建世界的导航页面均不会再显示任何广告。

游戏内如有任何汉化质量问题，欢迎前往 QQ 群反馈，我们将及时校准并重新发布修改后的汉化包。"#
}

/// 汉化包描述模板 - 第三部分：QQ 群信息
fn get_description_part3() -> String {
    format!(
        r#"

## 反馈与交流

如果您在使用汉化包时遇到任何问题，或者想要游玩的整合包还没有汉化包，欢迎加入 BBSMC 汉化组 QQ 群进行反馈：

**QQ 群号：{}**

我们会尽快处理您的反馈和汉化请求！"#,
        get_qq_group()
    )
}

/// 生成完整的汉化包描述
fn generate_cn_description(slug: &str) -> String {
    format!(
        "{}{}{}",
        get_description_part1(slug),
        get_description_part2(),
        get_description_part3()
    )
}

/// 汉化组织信息
#[derive(Debug)]
struct CnOrganization {
    id: i64,
    name: String,
}

/// 启用汉化追踪的项目信息
#[derive(Debug)]
#[allow(dead_code)]
pub struct TrackedProject {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: i32,
    pub follows: i32,
    pub organization_id: Option<i64>,
}

/// 调度汉化追踪任务
///
/// 每 5 分钟执行一次，处理所有启用了 `translation_tracking` 的项目
pub fn schedule_translation_tracking(
    scheduler: &mut Scheduler,
    pool: sqlx::Pool<sqlx::Postgres>,
    redis: RedisPool,
) {
    // 每 1 分钟执行一次
    let interval = std::time::Duration::from_secs(60);

    scheduler.run(interval, move || {
        let pool_ref = pool.clone();
        let redis_ref = redis.clone();

        async move {
            debug!("开始执行汉化追踪任务");

            if let Err(e) =
                run_translation_tracking(&pool_ref, &redis_ref).await
            {
                debug!("汉化追踪任务执行失败：{}", e);
            }

            debug!("完成汉化追踪任务");
        }
    });
}

/// 获取所有启用汉化追踪的项目
pub async fn get_tracked_projects(
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Vec<TrackedProject>, TranslationTrackingError> {
    let projects = sqlx::query_as!(
        TrackedProject,
        r#"
        SELECT
            id,
            name,
            slug,
            description,
            icon_url,
            downloads,
            follows,
            organization_id
        FROM mods
        WHERE translation_tracking = true
        ORDER BY downloads DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(projects)
}

/// 获取 bbsmc-cn 组织信息
async fn get_cn_organization(
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Option<CnOrganization>, TranslationTrackingError> {
    let org = sqlx::query_as!(
        CnOrganization,
        r#"
        SELECT id, name
        FROM organizations
        WHERE LOWER(slug) = LOWER($1)
        LIMIT 1
        "#,
        get_cn_org_slug()
    )
    .fetch_optional(pool)
    .await?;

    Ok(org)
}

/// 汉化资源信息
#[derive(Debug)]
struct CnProjectInfo {
    id: i64,
    icon_url: Option<String>,
    description: String,
}

/// 获取汉化资源信息（如果存在）
async fn get_cn_project(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cn_slug: &str,
) -> Result<Option<CnProjectInfo>, TranslationTrackingError> {
    let result = sqlx::query_as!(
        CnProjectInfo,
        r#"
        SELECT id, icon_url, description
        FROM mods
        WHERE LOWER(slug) = LOWER($1)
        LIMIT 1
        "#,
        cn_slug
    )
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

/// 同步汉化资源图标
async fn sync_cn_project_icon(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cn_project_id: i64,
    new_icon_url: Option<&str>,
) -> Result<(), TranslationTrackingError> {
    sqlx::query!(
        r#"
        UPDATE mods
        SET icon_url = $1, raw_icon_url = $2
        WHERE id = $3
        "#,
        new_icon_url,
        new_icon_url,
        cn_project_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// 同步汉化资源描述
async fn sync_cn_project_description(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cn_project_id: i64,
    new_description: &str,
) -> Result<(), TranslationTrackingError> {
    sqlx::query!(
        r#"
        UPDATE mods
        SET description = $1
        WHERE id = $2
        "#,
        new_description,
        cn_project_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// 执行汉化追踪逻辑
async fn run_translation_tracking(
    pool: &sqlx::Pool<sqlx::Postgres>,
    redis: &RedisPool,
) -> Result<(), TranslationTrackingError> {
    // 获取所有启用汉化追踪的项目
    let projects = get_tracked_projects(pool).await?;

    if projects.is_empty() {
        debug!("没有启用汉化追踪的项目");
        return Ok(());
    }

    debug!("找到 {} 个启用汉化追踪的项目", projects.len());

    // 获取 bbsmc-cn 组织信息（用于创建汉化资源）
    let cn_org = get_cn_organization(pool).await?;
    let cn_org = match cn_org {
        Some(org) => org,
        None => {
            debug!("未找到 {} 组织，跳过汉化资源创建", get_cn_org_slug());
            return Ok(());
        }
    };

    debug!("找到汉化组织: {} (id={})", cn_org.name, cn_org.id);

    // 遍历所有启用追踪的项目
    for project in &projects {
        debug!(
            "处理项目: {} (id={}, slug={}, downloads={})",
            project.name,
            project.id,
            project.slug.as_deref().unwrap_or("none"),
            project.downloads
        );

        if let Err(e) =
            process_tracked_project(pool, project, &cn_org, redis).await
        {
            debug!("处理项目 {} ({}) 失败: {}", project.name, project.id, e);
            // 继续处理下一个项目，不中断整个任务
        }
    }

    debug!("汉化追踪任务处理完成，共处理 {} 个项目", projects.len());
    Ok(())
}

/// 处理单个追踪项目
///
/// 1. 检查是否有对应的汉化资源（slug + "-cn"）
/// 2. 如果没有，则创建汉化资源
async fn process_tracked_project(
    pool: &sqlx::Pool<sqlx::Postgres>,
    project: &TrackedProject,
    cn_org: &CnOrganization,
    redis: &RedisPool,
) -> Result<(), TranslationTrackingError> {
    let Some(original_slug) = &project.slug else {
        debug!("项目 {} 没有 slug，跳过", project.name);
        return Ok(());
    };

    // 构建汉化资源的 slug
    let cn_slug = format!("{}-cn", original_slug);

    // 检查汉化资源是否已存在
    if let Some(cn_project) = get_cn_project(pool, &cn_slug).await? {
        debug!("汉化资源 {} 已存在，检查同步", cn_slug);

        let mut need_clear_cache = false;

        // 检查图标是否需要同步
        if cn_project.icon_url != project.icon_url {
            debug!(
                "同步汉化资源图标: {} -> {}",
                cn_project.icon_url.as_deref().unwrap_or("无"),
                project.icon_url.as_deref().unwrap_or("无")
            );
            sync_cn_project_icon(
                pool,
                cn_project.id,
                project.icon_url.as_deref(),
            )
            .await?;
            need_clear_cache = true;
            debug!("汉化资源 {} 图标同步完成", cn_slug);
        }

        // 检查描述是否需要同步
        let expected_description = generate_cn_description(original_slug);
        if cn_project.description != expected_description {
            debug!(
                "同步汉化资源描述: {} (长度 {} -> {})",
                cn_slug,
                cn_project.description.len(),
                expected_description.len()
            );
            sync_cn_project_description(
                pool,
                cn_project.id,
                &expected_description,
            )
            .await?;
            need_clear_cache = true;
            debug!("汉化资源 {} 描述同步完成", cn_slug);
        }

        // 如果有任何更新，清除缓存
        if need_clear_cache {
            Project::clear_cache(
                ProjectId(cn_project.id),
                Some(cn_slug.clone()),
                None,
                redis,
            )
            .await?;
        }

        return Ok(());
    }

    // 创建汉化资源
    debug!("开始创建汉化资源: {}", cn_slug);

    let mut transaction = pool.begin().await?;

    // 生成新的项目 ID
    let project_id = generate_project_id(&mut transaction).await?;

    // 创建团队（空团队，后续由组织管理）
    let team_builder = TeamBuilder { members: vec![] };
    let team_id = team_builder.insert(&mut transaction).await?;

    // 构建项目信息
    let project_builder = ProjectBuilder {
        project_id,
        team_id,
        organization_id: Some(crate::database::models::ids::OrganizationId(
            cn_org.id,
        )),
        name: format!("{} 汉化包", original_slug),
        summary: "整合包汉化包，长期稳定更新".to_string(),
        description: generate_cn_description(original_slug),
        icon_url: project.icon_url.clone(),
        raw_icon_url: project.icon_url.clone(),
        license_url: None,
        categories: vec![],
        additional_categories: vec![],
        initial_versions: vec![],
        status: ProjectStatus::Approved,
        requested_status: None,
        license: "LicenseRef-All-Rights-Reserved".to_string(),
        slug: Some(cn_slug.clone()),
        link_urls: vec![],
        gallery_items: vec![],
        color: None,
        monetization_status: MonetizationStatus::Monetized,
        is_paid: false,
    };

    project_builder.insert(&mut transaction).await?;

    // 创建项目关联的 thread（项目必须有 thread 才能被查询到）
    let thread_builder = ThreadBuilder {
        type_: ThreadType::Project,
        members: vec![],
        project_id: Some(project_id),
        report_id: None,
        ban_appeal_id: None,
        creator_application_id: None,
    };
    thread_builder.insert(&mut transaction).await?;

    // 设置汉化资源的审核通过时间戳
    sqlx::query!(
        r#"
        UPDATE mods
        SET approved = $1
        WHERE id = $2
        "#,
        Utc::now(),
        project_id.0
    )
    .execute(&mut *transaction)
    .await?;

    // 更新原项目的 translation_tracker 字段
    sqlx::query!(
        r#"
        UPDATE mods
        SET translation_tracker = $1
        WHERE id = $2
        "#,
        &cn_slug,
        project.id
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    // 清除原项目缓存（translation_tracker 已更新）
    Project::clear_cache(
        ProjectId(project.id),
        project.slug.clone(),
        None,
        redis,
    )
    .await?;

    // 清除新创建的汉化资源缓存
    Project::clear_cache(project_id, Some(cn_slug.clone()), None, redis)
        .await?;

    debug!(
        "成功创建汉化资源: {} (id={})，已更新原项目 translation_tracker={}",
        cn_slug, project_id.0, cn_slug
    );

    Ok(())
}

#[derive(Error, Debug)]
pub enum TranslationTrackingError {
    #[error("数据库错误：{0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("数据库模型错误：{0}")]
    DatabaseError(#[from] DatabaseError),
}
