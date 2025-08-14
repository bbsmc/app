# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。
必须使用中文回复内容
在进行git提交的时候 不要使用 claude的账户参与提交

## 主题事项

开发的时候先学习 /docs 里的文档
后端代码修改完成后在apps/labrinth 下运行cargo check测试，并解决遇到的问题
不要用cargo build测试。用check测试
前端不需要运行pnpm run dev , 因为我一直在开着前端的测试

migrations 应该创建在 apps/labrinth/migrations

## 项目概述

这是 Modrinth 单体仓库，一个 Minecraft 模组托管平台。架构包括：

- **前端**: Vue 3 + Nuxt 应用程序 (`apps/frontend/`)
- **后端 API**: 基于 Rust 的 API 服务器，名为 Labrinth (`apps/labrinth/`)
- **共享包**: UI 组件、工具和资源 (`packages/`)
- **应用程序库**: 基于 Tauri 的桌面应用程序库 (`packages/app-lib/`)

项目使用 pnpm 工作空间和 Turbo 进行构建编排。

## 开发命令

### 根级别命令（从项目根目录运行）：
```bash
# 开发
pnpm web:dev              # 启动前端开发服务器
turbo run dev             # 启动所有服务的开发模式

# 构建
pnpm build                # 构建所有包
pnpm web:build            # 仅构建前端
pnpm pages:build          # 构建用于 Cloudflare Pages 部署

# 代码质量
pnpm lint                 # 检查所有包的代码规范
pnpm fix                  # 自动修复代码规范问题
pnpm test                 # 运行所有测试
```

### 前端特定命令（`apps/frontend/`）：
```bash
pnpm dev                  # 开发服务器
pnpm build                # 生产构建
pnpm lint                 # ESLint + Prettier 检查
pnpm fix                  # 使用 ESLint + Prettier 自动修复
pnpm intl:extract         # 提取国际化字符串
```

### 后端特定命令（`apps/labrinth/`）：
```bash
cargo run                 # 启动开发服务器
cargo build --release    # 生产构建
cargo fmt --check        # 检查 Rust 格式化
cargo clippy             # Rust 代码检查
cargo test               # 运行测试（注意：在 package.json 中命名为 test-labrinth，由于 CI 约束）
```

## 架构详情

### 单体仓库结构
工作空间由 pnpm 管理，使用 Turbo 进行任务编排。主要工作空间成员：
- `apps/frontend/` - 主要 Web 应用程序（Nuxt/Vue）
- `apps/labrinth/` - API 服务器（Rust/Actix）
- `packages/ui/` - 共享 Vue 组件
- `packages/utils/` - 共享 TypeScript 工具
- `packages/assets/` - 图标、图像和样式
- `packages/app-lib/` - 桌面应用核心库（Rust）
- `packages/daedalus/` - Minecraft 相关工具（Rust）

### 前端架构
- **框架**: Nuxt 3 配合 Vue 3 组合式 API
- **样式**: Tailwind CSS + SCSS
- **状态管理**: Pinia
- **国际化**: @vintl/nuxt
- **组件**: `packages/ui/` 中的自定义组件库
- **开发**: 热重载、TypeScript、ESLint + Prettier

### 后端架构  
- **语言**: Rust 配合 Actix Web 框架
- **数据库**: PostgreSQL 配合 SQLx 进行查询构建
- **缓存**: Redis 用于会话和数据缓存
- **文件存储**: 可配置（Backblaze B2、S3 或本地）
- **身份验证**: 自定义 OAuth 实现
- **搜索**: Meilisearch 集成用于项目搜索

### 核心技术
- **前端**: Vue 3、Nuxt 3、TypeScript、Tailwind CSS、Pinia
- **后端**: Rust、Actix Web、SQLx、PostgreSQL、Redis
- **构建**: Turbo、pnpm 工作空间、Vite
- **测试**: Cargo test（Rust）、前端手动测试设置
- **部署**: 支持 Cloudflare Pages、Docker

## 数据库迁移

数据库迁移位于 `apps/labrinth/migrations/` 目录中，遵循时间戳命名约定。后端使用 SQLx 进行编译时检查的查询。

## 环境配置

前端和后端都需要环境文件：
- 前端：Nuxt 支持的 `.env` 文件
- 后端：数据库、Redis、文件存储等环境变量

关键环境变量在 `turbo.json` 中定义，用于构建缓存。

## 开发工作流程

1. 安装依赖：`pnpm install`
2. 为两个应用设置环境文件
3. 启动开发：`pnpm web:dev`（仅前端）或 `turbo run dev`（所有服务）
4. 提交前检查代码规范：`pnpm lint`
5. 运行测试：`pnpm test`

项目遵循传统的 git 工作流，主分支为 `main`。

## Thread（消息线程）系统集成

### 翻译链接审核的Thread集成
**位置**：`apps/frontend/src/pages/[type]/[id]/settings/translations.vue`

**功能特性**：
1. 每个翻译链接可展开显示消息线程
2. 支持公开消息和内部备注（仅管理员可见）
3. 拒绝链接时可附带拒绝原因消息
4. 被拒绝的链接保留消息历史，便于申请者查看反馈

**关键实现**：
```javascript
// 获取thread
const fetchThread = async (link) => {
  if (!link.thread_id) return;
  const thread = await useBaseFetch(`thread/${link.thread_id}`);
  threads.value[link.id] = thread;
};

// 发送消息
await useBaseFetch(`thread/${link.thread_id}`, {
  method: 'POST',
  body: {
    body: {
      type: 'text',
      body: messageText,
      private: isPrivate,  // 内部备注标记
    },
  },
});
```

**注意事项**：
- 目前需要后端为version_link_version表添加thread_id字段
- 创建翻译链接时应自动创建关联的thread
- Thread权限检查需确保翻译双方项目成员都可访问

## 重要技术细节与注意事项

> **开发要求**：遇到任何技术问题和解决方案都要简短记录在此，作为未来参考

### 前端图标文件检查
**问题**：使用不存在的SVG图标文件导致编译错误
**解决**：使用前先检查 `apps/frontend/src/assets/images/utils/` 目录确认图标存在
```bash
ls apps/frontend/src/assets/images/utils/ | grep refresh
# 如果不存在，查找类似功能的图标：undo, update, sync等
```

### Thread系统延迟初始化
**原则**：Thread不需要预先创建，在第一次发送消息时自动创建
**实现**：
```rust
let thread_id = if let Some(existing_thread_id) = link.thread_id {
    database::models::ids::ThreadId(existing_thread_id)
} else {
    // 创建新thread
    let new_thread_id = ThreadBuilder {
        type_: ThreadType::VersionLink,
        members: vec![],  // Version link threads不需要固定成员
        project_id: None,
        report_id: None,
    }.insert(&mut transaction).await?;
    
    // 更新关联表的thread_id
    sqlx::query!("UPDATE ... SET thread_id = $1 ...", new_thread_id.0)
        .execute(&mut *transaction).await?;
    
    new_thread_id
};
```

### 版本链接重新提交功能完整实现
**需求**：被拒绝的翻译链接可以修改后重新提交审核
**实现要点**：
1. 状态检查：只有rejected状态可以重新提交
2. 权限检查：只有翻译项目成员可以重新提交
3. 状态更新：rejected → pending
4. 消息记录：自动在thread中添加重新提交原因
5. 缓存清理：清除两个版本和两个项目的缓存

## 技术问题与解决方案记录

### 开发流程问题总结（2024-08-14）

#### 1. API版本遗漏问题
**错误**：只在v3添加了新端点，忘记在v2中添加
**影响**：前端默认使用v2 API，导致404错误
**教训**：
- 开发前先确认前端使用的API版本（查看 `nuxt.config.ts` 中的 `STAGING_API_URL`）
- 新功能必须同时在v2和v3中实现
- v2实现应该调用v3的逻辑，避免重复代码

#### 2. 前端资源文件未确认
**错误**：直接import不存在的SVG文件
**影响**：编译失败
**教训**：使用任何静态资源前先确认文件存在

#### 3. 权限检查位置错误
**错误**：检查翻译项目权限而不是目标项目权限
**影响**：权限控制失效
**教训**：审核类功能应该检查被审核资源的权限，不是提交者的权限

#### 4. Thread创建时机
**错误**：试图预先创建thread
**正确**：延迟创建，第一次使用时自动创建
**教训**：遵循按需创建原则，减少不必要的数据库操作

### ID 类型转换问题
**问题**：数据库查询返回空，但数据存在  
**原因**：API层（u64）和数据库层（i64）ID类型不同  
**解决**：
```rust
// 错误：id.0 as i64
// 正确：
let db_id: database::models::ids::VersionId = api_id.into();
sqlx::query!("...", db_id.0)
```

### 版本链接表结构
**表名**：`version_link_version`  
**字段**：`version_id`(翻译版本) → `joining_version_id`(原版本)  
**查询**：找翻译用 `WHERE joining_version_id = ?`

### API版本兼容
**问题**：前端使用v2 API，后端只改了v3  
**解决**：同时更新 `v2/` 和 `v3/` 的结构体（如 LegacyVersion）

**重要规则**：
1. **添加新API端点时必须同时在v2和v3中添加**
2. v2端点实现模式：
```rust
// v2/versions.rs
pub async fn new_endpoint(
    req: HttpRequest,
    info: web::Path<(String, String)>,  // v2使用String参数
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    // 转换字符串ID为类型化ID
    let id = VersionId(parse_base62(&info.0)?);
    let target_id = VersionId(parse_base62(&info.1)?);
    
    // 调用v3版本的实现
    v3::versions::new_endpoint(
        req,
        web::Path::from((id, target_id)),  // 使用Path::from构造
        pool,
        redis,
        session_queue,
        body,
    )
    .await
    .or_else(v2_reroute::flatten_404_error)
}
```
3. 路由配置必须同步添加：
```rust
// v2/versions.rs config函数
.route("{id}/link/{target_id}/new", web::post().to(new_endpoint))
// v3/versions.rs config函数  
.route("{id}/link/{target_id}/new", web::post().to(new_endpoint))
```

### Vue3响应式不更新
**问题**：修改数组元素属性后视图不更新  
**解决**：
```javascript
// 重新赋值整个元素
this.array[i] = { ...this.array[i], newProp: value };
// 或强制更新
this.$forceUpdate();
```

### 版本链接不保存
**问题**：EditVersion结构体缺少version_links字段  
**解决**：v2和v3的EditVersion都要加上，version_edit_helper中处理保存逻辑

### 版本API返回数据不完整
**问题**：`/project/{id}/version` API返回的translated_by字段为空数组，但数据库有数据  
**原因**：该API返回的是简化版本列表，不包含完整的版本详情字段  
**解决**：
1. 修改`version_item.rs`中的`Version::get_many`，添加反向查询获取translated_by数据
2. 在`QueryVersion`结构体中加入`translated_by`字段
3. 更新`From<QueryVersion>`实现，正确映射translated_by数据
```rust
// 反向查询找出哪些版本翻译了当前版本
let translated_by = sqlx::query!(
    "SELECT DISTINCT version_id, joining_version_id, link_type, language_code, description
     FROM version_link_version
     WHERE joining_version_id = ANY($1)",
    &version_ids
)
```

## Notifications 系统实现

### 后端通知系统

#### 1. 数据模型
- **位置**：`apps/labrinth/src/models/v3/notifications.rs` 和 `database/models/notification_item.rs`
- **核心结构**：
  - `Notification`：通知实体，包含id、用户、已读状态、创建时间、消息体
  - `NotificationBody`：枚举类型，定义所有通知类型
  - `NotificationAction`：通知操作（如接受/拒绝邀请）

#### 2. 通知类型（NotificationBody）
```rust
// 定义在 models/v3/notifications.rs
pub enum NotificationBody {
    ProjectUpdate { project_id, version_id },      // 项目更新
    TeamInvite { project_id, team_id, ... },      // 团队邀请
    OrganizationInvite { ... },                   // 组织邀请
    StatusChange { project_id, old_status, ... }, // 状态变更
    ModeratorMessage { thread_id, ... },          // 管理员消息
    WikiCache { project_id, msg, type_, ... },    // 百科编辑状态
    Forum { forum_id, forum_title, ... },         // 论坛回复
    LegacyMarkdown { name, text, link, ... },     // 旧格式（兼容）
}
```

#### 3. 发送通知
```rust
use crate::database::models::notification_item::NotificationBuilder;
use crate::models::notifications::NotificationBody;

// 创建通知
NotificationBuilder {
    body: NotificationBody::ProjectUpdate {
        project_id: project_id,
        version_id: version_id,
    }
}
.insert(user_id, &mut transaction, &redis)  // 单用户
.await?;

// 或批量发送
.insert_many(vec![user1, user2], &mut transaction, &redis)
.await?;
```

#### 4. 通知文本生成
- **位置**：`models/v3/notifications.rs` 的 `From<DBNotification>` 实现
- 每种通知类型自动生成 `name`（标题）、`text`（内容）、`link`（链接）、`actions`（操作按钮）
- 示例：
```rust
NotificationBody::TeamInvite { project_id, role, team_id, .. } => (
    "您已被邀请加入团队！".to_string(),
    format!("已向您发送邀请，成为团队的 {}", role),
    format!("/project/{}", project_id),
    vec![接受按钮, 拒绝按钮]
)
```

### 前端通知系统

#### 1. 显示通知
- **组件**：`apps/frontend/src/components/ui/NotificationItem.vue`
- 根据 `notification.body.type` 显示不同UI
- 支持分组显示（如多个版本更新合并）

#### 2. 通知管理
- **工具函数**：`apps/frontend/src/helpers/notifications.js`
  - `fetchExtraNotificationData`：批量获取关联数据（项目、用户、版本等）
  - `groupNotifications`：合并相似通知
  - `markAsRead`：标记已读

#### 3. 临时通知（Toast）
- **位置**：`apps/frontend/src/composables/notifs.js`
- 使用 `addNotification` 添加临时消息
- 30秒后自动消失
```javascript
app.$notify({
    group: "main",
    title: "成功",
    text: "操作完成",
    type: "success"
});
```

### 添加新通知类型步骤

#### 后端：
1. 在 `NotificationBody` 枚举添加新类型
2. 在 `From<DBNotification>` 实现中添加文本生成逻辑
3. 在需要发送通知的地方使用 `NotificationBuilder`

#### 前端：
1. 在 `NotificationItem.vue` 添加新类型的显示模板
2. 如需额外数据，在 `fetchExtraNotificationData` 添加获取逻辑
3. 处理通知操作（如有）

### 常见使用场景
- 版本发布：`routes/v3/version_creation.rs:469`
- 团队邀请：`routes/v3/teams.rs:624`
- 状态变更：`routes/v3/projects.rs:554`
- 论坛回复：`routes/v3/forum.rs:673`
- 百科审核：`routes/v3/wikis.rs:979`

## 翻译链接审核系统实现总结

### 系统架构理解
**版本链接表结构**（`version_link_version`）：
- `version_id`: 翻译版本ID（如汉化包版本）
- `joining_version_id`: 目标版本ID（被翻译的原版本）
- `approval_status`: 审批状态（pending/approved）
- **重要**：一个翻译版本在 `version_links` 中，多个翻译版本在目标版本的 `translated_by` 中

### 1. API接口开发规范

#### 版本兼容性
**问题**：前端使用v2 API，只在v3添加新接口导致404错误
**解决**：新功能需要同时在v2和v3路由中添加
```rust
// v3/projects.rs - 添加新路由
.route("translation_links", web::get().to(get_translation_links))

// v2/projects.rs - 必须同时添加
.service(get_translation_links)

// v2函数实现 - 直接调用v3
pub async fn get_translation_links(
    req: HttpRequest,
    info: web::Path<(String,)>,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    session_queue: web::Data<AuthQueue>,
) -> Result<HttpResponse, ApiError> {
    v3::projects::get_translation_links(req, info, pool, redis, session_queue)
        .await
        .or_else(v2_reroute::flatten_404_error)
}
```

#### 路由参数处理
**v2和v3的Path参数差异**：
```rust
// v2使用String参数
info: web::Path<(String, String)>
// 需要转换为v3的类型
let version_id = VersionId(parse_base62(&version_id_str)?);
// 使用Path::from构造
web::Path::from((version_id, target_id))
```

### 2. ID类型转换完整指南

#### 三层ID体系
1. **字符串层**：Base62编码的字符串（如 "Wcl3Ah1p"）
2. **API层**：`models::ids::VersionId(u64)`
3. **数据库层**：`database::models::ids::VersionId(i64)`

#### 转换方法
```rust
// 字符串 -> API层ID
use crate::models::ids::base62_impl::parse_base62;
let version_id = VersionId(parse_base62(&version_id_str)?);

// API层 -> 数据库层（使用.into()）
let db_version_id: database::models::ids::VersionId = api_version_id.into();

// 数据库层 -> API层
let api_version_id: models::ids::VersionId = db_version_id.into();

// SQL查询中的类型转换
sqlx::query!(
    "UPDATE ... WHERE version_id = $1",
    version_id.0 as i64  // u64转i64
)
```

### 3. 缓存更新策略（关键）

#### 需要清理的四个缓存
修改版本链接状态后，必须清理所有相关缓存：
```rust
// 1. 翻译版本缓存 - 更新其version_links字段
database::models::Version::clear_cache(&translation_version, &redis).await?;

// 2. 目标版本缓存 - 更新其translated_by字段  
database::models::Version::clear_cache(&target_version, &redis).await?;

// 3. 翻译项目缓存 - 项目可能缓存了版本列表
database::models::Project::clear_cache(
    translation_version.inner.project_id,
    None,
    Some(true),  // clear_versions参数
    &redis,
).await?;

// 4. 目标项目缓存
database::models::Project::clear_cache(
    target_version.inner.project_id,
    None,
    Some(true),
    &redis,
).await?;
```

#### 缓存机制理解
- Redis使用版本ID作为缓存key：`VERSIONS_NAMESPACE + version.id.0`
- `get_many`函数内部查询`version_links`和`translated_by`
- 清除缓存后，下次获取会重新查询数据库

### 4. 权限检查位置

#### 常见错误
检查当前项目权限而不是目标项目权限

#### 正确实现
```rust
// 批准链接时，需要检查目标项目的权限
let target_project_id = target_version.inner.project_id;
let team_member = TeamMember::get_from_user_id_project(
    target_project_id,  // 不是translation_project_id！
    user.id.into(),
    false,
    &**pool,
).await?;

// 检查权限
let permissions = ProjectPermissions::get_permissions_by_role(
    &user.role,
    &team_member,
    &organization_team_member,
);
```

### 5. 前端权限处理

#### 权限使用位运算
```javascript
// 定义权限常量（位标志）
const UPLOAD_VERSION = 1 << 0;
const DELETE_VERSION = 1 << 1;
const EDIT_DETAILS = 1 << 2;

// 检查权限 - 使用位运算，不是includes！
const hasPermission = computed(() => {
  return (props.currentMember?.permissions & UPLOAD_VERSION) === UPLOAD_VERSION ||
         (props.currentMember?.permissions & EDIT_DETAILS) === EDIT_DETAILS;
});
```

### 6. 数据查询差异

#### version_links vs translated_by
```sql
-- version_links查询（显示所有状态）
SELECT ... FROM version_link_version
WHERE version_id = ANY($1)

-- translated_by查询（只显示已批准）
SELECT ... FROM version_link_version vlv
INNER JOIN versions v ON v.id = vlv.version_id
WHERE vlv.joining_version_id = ANY($1)
AND v.status NOT IN ('draft', 'scheduled', 'unknown')
AND vlv.approval_status = 'approved'  -- 关键差异！
```

### 7. 前端缓存处理

#### $fetch缓存问题
- `useBaseFetch`使用`$fetch`，有内部缓存
- 后端缓存清理后，前端可能仍显示旧数据

#### 解决方案
```javascript
// 1. 操作后重新获取数据
setTimeout(() => {
  fetchTranslationLinks();
}, 500);

// 2. 清除Nuxt缓存（可能无效）
await clearNuxtData();

// 3. 提示用户刷新
addNotification({
  text: '已批准链接。请刷新页面查看最新状态。'
});
```

### 8. 审批状态保留逻辑

#### 编辑版本时的状态处理
```rust
// 只有新增或修改的链接需要重新审核
let approval_status = if let Some(status) = existing_status {
    status.clone()  // 保留原有状态
} else {
    // 新链接需要权限检查
    if has_permission {
        "approved".to_string()
    } else {
        "pending".to_string()
    }
};
```

### 9. 调试技巧

#### 添加日志
```rust
log::info!(
    "批准翻译链接: 翻译版本 {} -> 目标版本 {}, 已清理缓存",
    translation_version_id.0,
    target_version_id.0
);
```

#### 检查SQL查询
使用`cargo check`而不是`cargo build`，SQLx会在编译时检查查询

### 10. 常见陷阱总结

1. **API版本**：忘记在v2和v3同时添加接口
2. **ID转换**：混淆API层和数据库层的ID类型
3. **缓存清理**：只清理版本缓存，忘记清理项目缓存
4. **权限检查**：检查错误的项目权限
5. **前端权限**：使用includes而不是位运算
6. **数据差异**：不理解translated_by只返回已批准的链接
7. **Path参数**：直接构造Path而不是使用Path::from
8. **类型转换**：忘记as i64转换导致SQLx编译错误