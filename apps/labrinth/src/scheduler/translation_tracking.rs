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

/// 汉化包描述模板 - 系统介绍部分
fn get_description_system_intro() -> &'static str {
    r#"

## 关于 BBSMC汉化组 整合包自动汉化系统

这是一套完整的 Minecraft 整合包汉化自动化工具，覆盖整合包中几乎所有可翻译内容来源。系统采用 25+ 专用提取器、多级翻译引擎和智能过滤机制，实现从扫描、提取、翻译到打包的全流程自动化。所有文本均逐条提取、逐条过滤、逐条翻译，而非将整个文件丢给 AI 批量处理——每一条翻译都经过独立的上下文分析、占位符保护和质量校验。

### 核心翻译引擎

系统内置异步 AI 翻译引擎，支持流式翻译和并发批量处理。翻译前自动通过 59+ 条规则过滤不需要翻译的内容（资源路径、NBT 数据、代码片段、快捷键标记等），避免误翻译。翻译过程中通过占位符保护系统自动识别并保护 Minecraft 格式代码、颜色代码、变量占位符等特殊标记，确保翻译后格式完整不被破坏。

### 多源翻译合并

对于模组语言文件，系统实现了四级优先级自动合并：优先使用整合包作者自带的翻译，其次查找 CFPA 社区语言包中已有的翻译，再检查模组 JAR 内置的中文翻译，最后才通过 AI 生成翻译。这保证了翻译质量的同时最大化利用社区已有成果。

### 模组语言文件翻译

最基础也最核心的模块。系统自动扫描整合包中所有模组 JAR 文件，提取英文语言文件，检测哪些模组缺少中文翻译或存在"假中文"文件（文件名是 zh_cn 但内容实际为英文），然后通过多源合并生成完整的中文语言包，最终打包为 Minecraft 资源包。

### KubeJS 脚本翻译

KubeJS 是现代整合包中最常用的自定义脚本系统，大量物品名称、描述、工具提示都直接写在 JavaScript 脚本中。系统使用 esprima 和 tree-sitter 双引擎进行 JavaScript AST 解析，精确提取脚本中的可翻译字符串。配合 290+ 条跳过规则（涵盖 GregTech 机器类型、TFC 配方函数、化学式方法等），以及基于数据流分析的意图识别系统，准确判断每个字符串是否应该被翻译。翻译完成后通过基于行列号的精确替换写回脚本，保留原始引号类型和代码结构。对于通过 event.create() 注册但缺少显示名的物品和方块，还能自动检测并生成合理的中文名称。

### FTB Quests 任务翻译

FTB Quests 是整合包中最主要的任务系统，使用 SNBT 格式存储任务数据。系统能自动检测整合包使用的是哪种 FTB Quests 版本模式（LangFile、Localizer 或 Legacy），然后用对应的策略提取所有任务标题、描述和奖励文本。翻译时从章节结构、任务依赖关系和物品翻译中构建上下文场景，帮助 AI 更准确地理解每条文本的含义。支持 JSON Text Component 富文本格式和 Minecraft 格式代码的处理。

### 硬编码文本提取

许多模组将物品名称、工具提示等文本直接写死在 Java 代码中，而非使用语言文件。系统通过 CFR 反编译器将模组 class 文件反编译为 Java 源码，然后分析方法签名、类结构和调用上下文，精确定位那些流向 addTooltip、appendText、setCustomName 等渲染方法的字符串。配合 10 个专用模式检测器（NBT、资源路径、JEI/REI、Lore、TextComponent 等）过滤误报，最终生成 VaultPatcher 运行时文本替换配置或 ASM 字节码替换规则。内置 VM 汉化组提供的 2,451 条白名单数据，覆盖 583 个常见类。支持 Forge、NeoForge 和 Fabric 三大加载器，兼容 MC 1.12 到 1.21+。

### Patchouli 手册翻译

Patchouli（帕秋莉）是 Minecraft 中最流行的模组手册系统。系统能从整合包目录和模组 JAR 中同时提取手册内容，覆盖书籍名称、分类描述、词条标题和所有页面文本。翻译后正确区分 assets/（资源包）和 data/（数据包）两种路径，对于 data/ 路径的手册通过完整 JAR 重打包注入翻译，确保游戏能正确加载。

### Datapack 内容翻译

整合包中的数据包可能包含自定义进度、技能树、法术描述等需要翻译的内容。系统支持从 Paxi、OpenLoader 和 KubeJS 等多种数据包加载器中提取内容，覆盖 MMORPG Spells 法术名称、Passive Skill Tree 技能描述、Puffish Skills 天赋定义等模组的翻译需求。

### Advancement 成就翻译

Minecraft 的成就系统存在三种不同的文本格式：语言键引用格式、纯字符串格式和 JSON Text Component 格式。系统为每种格式建立了独立的翻译管线，分别通过语言文件注入、JAR 修改和 AI 翻译来处理，确保所有成就都能被正确翻译。

### Origins 起源翻译

Origins 模组允许玩家选择不同的起源获得独特能力。系统能从 ZIP 数据包、文件夹数据包和模组 JAR 三种来源中提取起源名称、能力描述和起源层定义，翻译后写回对应的来源格式。

### Lavender 手册翻译

Lavender 是另一种模组手册系统，使用 Markdown 格式编写。系统从模组 JAR 中提取手册的书籍定义、条目正文和分类描述，检测已有官方中文翻译的书籍并跳过，只翻译缺少中文版本的内容。

### FancyMenu 界面翻译

FancyMenu 允许整合包自定义游戏主菜单的按钮、文本和布局。系统不仅提取本地配置中的可翻译文本，还能自动检测引用的网络资源（如 GitHub 上的 Markdown 文件），下载后翻译并转为本地资源，实现完整的菜单汉化。

### CustomNPCs 翻译

CustomNPCs 模组的对话和任务数据存储在存档中，包括 JSON 对话文件和嵌入在 Region 文件中的 NBT 数据。系统能解析这两种格式，提取 NPC 对话、任务描述等文本，翻译后直接写回对应的数据结构。

### CraftTweaker 脚本翻译

CraftTweaker 使用 ZenScript 脚本修改游戏内容。系统通过正则匹配提取脚本中的 displayName 和 tooltip 设置，翻译后替换回原脚本。

### HQM 任务翻译

Hardcore Questing Mode 是另一种任务系统，常见于较老版本的整合包。系统兼容 HQM 的新旧两种数据结构，提取任务名称和描述进行翻译。

### CustomMainMenu 翻译

CustomMainMenu 模组定义了主菜单的按钮文本和悬停提示。系统提取这些文本并翻译，配合配置文件复制确保翻译生效。

### The Vault 专用翻译

针对 Vault Hunters 整合包，系统包含专门的提取器，覆盖 30+ 配置文件中的技能描述、天赋属性、装备词缀、秘境主题、卡牌系统、传说文本等内容。

### Excavated Variants 矿石变体翻译

Excavated Variants 模组为不同岩石类型生成对应的矿石变体。系统提取 JSON5 配置中的石头和矿石名称，翻译后生成对应的中文语言文件。

### Guidebook 手册翻译

支持 Modern Industrialization Guidebook 和 Applied Energistics 2 Guide 两种手册格式的提取和翻译。

### 其他翻译模块

Config Lang 处理 config 目录下模组自带的语言文件。StarterKit 翻译初始装备和职业选择界面的 JSON5 配置。Tips Mod 翻译加载界面的自定义提示文本。Resources Override 处理 1.12.2 及更早版本的资源覆盖目录。Mod Content Pack 扫描非标准目录下的语言文件。Orphan Namespace 检测没有语言文件的模组命名空间并补全翻译。

### 打包输出

所有翻译完成后，系统自动将结果打包为标准的 Minecraft 资源包，同时根据整合包的游戏版本和加载器类型（Forge/NeoForge/Fabric）附带对应的辅助模组，生成开箱即用的汉化补丁。整个流程由工作流编排器统一调度，从扫描到打包全自动完成。"#
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
        "{}{}{}{}",
        get_description_part1(slug),
        get_description_part2(),
        get_description_part3(),
        get_description_system_intro()
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
