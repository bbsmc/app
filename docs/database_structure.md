# Modrinth 数据库结构文档

## 数据库概述

Modrinth 使用 PostgreSQL 作为主数据库，存储用户、项目（模组）、版本、文件等核心数据。数据库通过 SQLx 进行 Rust 端的查询构建和编译时类型检查。

## 核心表结构

### 用户系统 (User System)

#### `users` - 用户表
- **主键**: `id` (bigint)
- **核心字段**:
  - `username` (varchar 255) - 用户名，唯一
  - `email` (varchar 255) - 邮箱，唯一
  - `avatar_url` (varchar 1024) - 头像 URL
  - `bio` (varchar 160) - 个人简介
  - `created` (timestamp) - 创建时间
  - `role` (varchar 50) - 用户角色，默认 'developer'
  - `badges` (bigint) - 徽章标识
  - `balance` (numeric 40,20) - 账户余额
  - `email_verified` (boolean) - 邮箱是否已验证
  - `allow_friend_requests` (boolean) - 是否允许好友请求
- **OAuth 字段**:
  - `github_id` (bigint) - GitHub ID
  - `discord_id` (bigint) - Discord ID
  - `gitlab_id` (bigint) - GitLab ID
  - `google_id` (varchar 256) - Google ID
  - `steam_id` (bigint) - Steam ID
  - `microsoft_id` (varchar 256) - Microsoft ID
- **支付相关**:
  - `paypal_country`, `paypal_email`, `paypal_id` (text) - PayPal 信息
  - `venmo_handle` (text) - Venmo 账号
  - `stripe_customer_id` (text) - Stripe 客户 ID
- **Wiki 相关**:
  - `wiki_overtake_count` (bigint) - Wiki 接管次数
  - `wiki_ban_time` (timestamp) - Wiki 禁用时间

**关联关系**:
- 一对多关联到 `collections` (用户收藏集)
- 一对多关联到 `mods` (通过 teams)
- 一对多关联到 `versions` (版本作者)
- 一对多关联到 `notifications` (通知)
- 一对多关联到 `reports` (举报)
- 一对多关联到 `payouts` (收益)
- 一对多关联到 `sessions` (会话)

#### `sessions` - 用户会话表
- **主键**: `id`
- **外键**: `user_id` → `users.id`
- 存储用户登录会话信息

#### `user_backup_codes` - 用户备份码表
- **外键**: `user_id` → `users.id`
- 存储两步验证的备份码

#### `friends` - 好友关系表
- **外键**: 
  - `user_id` → `users.id`
  - `friend_id` → `users.id`
- 存储用户好友关系

### 项目系统 (Project System)

#### `mods` - 项目表（主要实体）
- **主键**: `id` (bigint)
- **外键**: 
  - `team_id` → `teams.id` (项目团队)
  - `organization_id` → `organizations.id` (所属组织，可为空)
  - `forum` → `discussions.id` (论坛讨论，唯一)
- **核心字段**:
  - `name` (varchar 255) - 项目名称
  - `slug` (varchar 255) - URL 友好的标识符，唯一
  - `summary` (varchar 2048) - 项目简介
  - `description` (varchar 65536) - 详细描述
  - `published` (timestamp) - 发布时间
  - `updated` (timestamp) - 最后更新时间
  - `downloads` (integer) - 下载次数
  - `follows` (integer) - 关注数
- **展示相关**:
  - `icon_url` (varchar 2048) - 图标 URL
  - `raw_icon_url` (text) - 原始图标 URL
  - `color` (integer) - 主题色
- **状态相关**:
  - `status` (varchar 128) - 当前状态
  - `requested_status` (varchar 128) - 请求的状态
  - `approved` (timestamp) - 审核通过时间
  - `moderation_message` (varchar 2000) - 审核消息
  - `moderation_message_body` (varchar 65536) - 审核详情
  - `webhook_sent` (boolean) - Webhook 是否已发送
  - `queued` (timestamp) - 队列时间
- **许可和商业化**:
  - `license` (varchar 2048) - 许可证，默认 'LicenseRef-All-Rights-Reserved'
  - `license_url` (varchar 1000) - 许可证 URL
  - `monetization_status` (varchar 64) - 商业化状态，默认 'monetized'
- **Wiki 和论坛**:
  - `wiki_open` (boolean) - Wiki 是否开放
  - `issues_type` (integer) - 问题跟踪类型

**关联关系**:
- 一对多关联到 `versions` (项目版本)
- 一对多关联到 `mods_categories` (项目分类)
- 一对多关联到 `mods_gallery` (项目图库)
- 一对多关联到 `mods_links` (项目链接)
- 一对多关联到 `mod_follows` (关注)
- 一对多关联到 `dependencies` (依赖)
- 一对多关联到 `reports` (举报)
- 一对多关联到 `issues` (问题)

#### `versions` - 版本表
- **主键**: `id` (bigint)
- **外键**: 
  - `mod_id` → `mods.id` (所属项目)
  - `author_id` → `users.id` (版本作者)
- **核心字段**:
  - `name` (varchar 255) - 版本名称
  - `version_number` (varchar 255) - 版本号
  - `date_published` (timestamp) - 发布时间
  - `downloads` (integer) - 下载次数
  - `changelog` (varchar 65536) - 更新日志
  - `version_type` (varchar 255) - 版本类型
  - `featured` (boolean) - 是否为特色版本
  - `status` (varchar 128) - 状态，默认 'listed'
  - `requested_status` (varchar 128) - 请求的状态
  - `ordering` (integer) - 排序

**关联关系**:
- 一对多关联到 `files` (版本文件)
- 一对多关联到 `dependencies` (作为依赖/被依赖)
- 一对多关联到 `loaders_versions` (支持的加载器)
- 一对多关联到 `version_fields` (版本字段)

#### `files` - 文件表
- **主键**: `id` (bigint)
- **外键**: `version_id` → `versions.id`
- **字段**:
  - `url` (varchar 2048) - 文件 URL
  - `filename` (varchar 2048) - 文件名
  - `is_primary` (boolean) - 是否为主文件
  - `size` (integer) - 文件大小
  - `file_type` (varchar 128) - 文件类型
  - `metadata` (jsonb) - 元数据

**关联关系**:
- 一对多关联到 `hashes` (文件哈希)

#### `teams` - 团队表
- **主键**: `id` (bigint)
- 简单的团队标识表

**关联关系**:
- 一对多关联到 `team_members` (团队成员)
- 一对多关联到 `mods` (团队项目)
- 一对多关联到 `organizations` (组织)

#### `team_members` - 团队成员表
- **主键**: `id` (bigint)
- **外键**: 
  - `team_id` → `teams.id`
  - `user_id` → `users.id`
- **字段**:
  - `role` (varchar 255) - 角色
  - `permissions` (bigint) - 权限位掩码
  - `accepted` (boolean) - 是否已接受邀请
  - `payouts_split` (numeric 96,48) - 收益分成比例
  - `ordering` (bigint) - 排序
  - `organization_permissions` (bigint) - 组织权限
  - `is_owner` (boolean) - 是否为所有者

### 分类和加载器系统

#### `categories` - 分类表
- **主键**: `id` (integer)
- **外键**: `project_type` → `project_types.id`
- **字段**:
  - `category` (varchar 255) - 分类名称
  - `icon` (varchar 20000) - 分类图标 SVG
  - `header` (varchar 256) - 分类标题
  - `ordering` (bigint) - 排序

#### `loaders` - 加载器表
- **主键**: `id` (integer)
- **字段**:
  - `loader` (varchar 255) - 加载器名称，唯一
  - `icon` (varchar 20000) - 加载器图标 SVG
  - `hidable` (boolean) - 是否可隐藏
  - `metadata` (jsonb) - 元数据

#### `mods_categories` - 项目分类关联表
- **外键**: 
  - `joining_mod_id` → `mods.id`
  - `joining_category_id` → `categories.id`

#### `loaders_versions` - 加载器版本关联表
- **外键**: 
  - `loader_id` → `loaders.id`
  - `version_id` → `versions.id`

### 依赖系统

#### `dependencies` - 依赖关系表
- **主键**: `id` (integer)
- **外键**: 
  - `dependent_id` → `versions.id` (依赖者版本)
  - `dependency_id` → `versions.id` (被依赖版本)
  - `mod_dependency_id` → `mods.id` (被依赖模组)
- **字段**:
  - `dependency_type` (varchar 255) - 依赖类型
  - `dependency_file_name` (varchar 1024) - 依赖文件名

### 收藏系统

#### `collections` - 收藏集表
- **主键**: `id` (bigint)
- **外键**: `user_id` → `users.id`
- **字段**:
  - `name` (varchar 255) - 收藏集名称
  - `description` (varchar 2048) - 描述
  - `created` (timestamp) - 创建时间
  - `updated` (timestamp) - 更新时间
  - `status` (varchar 64) - 状态，默认 'listed'
  - `icon_url` (varchar 2048) - 图标 URL
  - `raw_icon_url` (text) - 原始图标 URL
  - `color` (integer) - 主题色

#### `collections_mods` - 收藏集模组关联表
- **外键**: 
  - `collection_id` → `collections.id`
  - `mod_id` → `mods.id`

### 通知系统

#### `notifications` - 通知表
- **主键**: `id` (bigint)
- **外键**: `user_id` → `users.id`
- **字段**:
  - `name` (varchar 255) - 通知名称
  - `text` (varchar 2048) - 通知内容
  - `link` (varchar 2048) - 相关链接
  - `created` (timestamp) - 创建时间
  - `read` (boolean) - 是否已读
  - `type` (varchar 256) - 通知类型
  - `body` (jsonb) - 通知详情

#### `notifications_actions` - 通知操作表
- **外键**: `notification_id` → `notifications.id`

### 举报系统

#### `reports` - 举报表
- **主键**: `id` (bigint)
- **外键**: 
  - `report_type_id` → `report_types.id` (举报类型)
  - `mod_id` → `mods.id` (被举报项目，可选)
  - `version_id` → `versions.id` (被举报版本，可选)
  - `user_id` → `users.id` (被举报用户，可选)
  - `reporter` → `users.id` (举报者)
- **字段**:
  - `body` (varchar 65536) - 举报内容
  - `created` (timestamp) - 创建时间
  - `closed` (boolean) - 是否已处理

#### `report_types` - 举报类型表
- **主键**: `id`

### 支付系统

#### `payouts` - 收益表
- **主键**: `id` (bigint)
- **外键**: `user_id` → `users.id`
- **字段**:
  - `amount` (numeric 96,48) - 金额
  - `created` (timestamp) - 创建时间
  - `status` (varchar 128) - 状态
  - `method` (text) - 付款方式
  - `method_address` (text) - 付款地址
  - `platform_id` (text) - 平台 ID
  - `fee` (numeric 40,20) - 手续费

#### `payouts_values` - 收益明细表
- **外键**: 
  - `user_id` → `users.id`
  - `mod_id` → `mods.id`

#### `charges` - 费用表
- **外键**: `user_id` → `users.id`

### OAuth 系统

#### `oauth_clients` - OAuth 客户端表
- **主键**: `id`
- **外键**: `created_by` → `users.id`

#### `oauth_client_authorizations` - OAuth 授权表
- **外键**: 
  - `user_id` → `users.id`
  - OAuth 客户端相关

#### `oauth_access_tokens` - OAuth 访问令牌表

#### `pats` - 个人访问令牌表
- **外键**: `user_id` → `users.id`

### 组织系统

#### `organizations` - 组织表
- **主键**: `id` (bigint)
- **外键**: `team_id` → `teams.id`
- **字段**:
  - `slug` (varchar 255) - 组织标识符
  - `name` (text) - 组织名称
  - `description` (text) - 描述
  - `created_at` (timestamp) - 创建时间
  - `updated_at` (timestamp) - 更新时间
  - `icon_url` (varchar 255) - 图标 URL
  - `raw_icon_url` (text) - 原始图标 URL
  - `color` (integer) - 主题色

### 论坛和讨论系统

#### `discussions` - 讨论表
- **主键**: `id`
- **外键**: `user_id` → `users.id`

#### `threads` - 话题表
- **外键**: 
  - `mod_id` → `mods.id`
  - `report_id` → `reports.id`

#### `threads_members` - 话题成员表
- **外键**: `user_id` → `users.id`

#### `threads_messages` - 话题消息表
- **外键**: `author_id` → `users.id`

### 问题跟踪系统

#### `issues` - 问题表
- **外键**: 
  - `mod_id` → `mods.id`
  - `author_id` → `users.id`

#### `issue_assignees` - 问题指派表
- **外键**: 
  - `user_id` → `users.id`
  - `assigned_by` → `users.id`

#### `issue_comments` - 问题评论表
- **外键**: `author_id` → `users.id`

#### `issue_labels` - 问题标签表
#### `issue_label_associations` - 问题标签关联表

### 其他支持表

#### `uploaded_images` - 上传图片表
- **外键**: 
  - `owner_id` → `users.id`
  - `mod_id` → `mods.id`
  - `version_id` → `versions.id`
  - `report_id` → `reports.id`

#### `hashes` - 文件哈希表
- **外键**: `file_id` → `files.id`

#### `games` - 游戏表
#### `project_types` - 项目类型表
#### `link_platforms` - 链接平台表

#### `mod_follows` - 模组关注表
- **外键**: 
  - `follower_id` → `users.id`
  - `mod_id` → `mods.id`

#### `mods_gallery` - 项目图库表
- **外键**: `mod_id` → `mods.id`

#### `mods_links` - 项目链接表
- **外键**: `joining_mod_id` → `mods.id`

#### `posts` - 帖子表
- **外键**: `user_id` → `users.id`

#### `products` - 产品表
#### `products_prices` - 产品价格表
#### `users_subscriptions` - 用户订阅表
- **外键**: `user_id` → `users.id`

#### `version_fields` - 版本字段表
- **外键**: `version_id` → `versions.id`

#### `wiki_cache` - Wiki 缓存表
- **外键**: 
  - `user_id` → `users.id`
  - `mod_id` → `mods.id`

#### `wikis` - Wiki 表
- **外键**: `mod_id` → `mods.id`

#### `disk_urls` - 磁盘 URL 表
- **外键**: `version_id` → `versions.id`

## 数据库关系总结

### 核心实体关系图
```
users (用户)
  ├─ sessions (会话)
  ├─ friends (好友)
  ├─ notifications (通知)
  ├─ collections (收藏集)
  ├─ team_members (团队成员) ─ teams (团队)
  └─ payouts (收益)

teams (团队)
  ├─ team_members (团队成员) ─ users (用户)
  ├─ mods (项目)
  └─ organizations (组织)

mods (项目)
  ├─ versions (版本)
  │   ├─ files (文件)
  │   │   └─ hashes (哈希)
  │   ├─ dependencies (依赖)
  │   └─ loaders_versions (加载器)
  ├─ mods_categories (分类)
  ├─ mods_gallery (图库)
  ├─ mods_links (链接)
  ├─ mod_follows (关注)
  ├─ reports (举报)
  └─ issues (问题)

collections (收藏集)
  └─ collections_mods (收藏项目) ─ mods (项目)
```

### 关键约束和索引
- 所有主要实体都有 `id` 作为主键
- 用户名和邮箱具有唯一约束
- 项目 slug 具有唯一约束
- 团队每个项目只能有一个所有者（`idx_one_owner_per_team`）
- 用户在同一团队中只能有一个成员记录（`idx_unique_user_team`）

### 数据完整性
- 使用外键约束确保引用完整性
- 级联删除和更新规则保护数据一致性
- 时间戳字段默认使用 `CURRENT_TIMESTAMP`
- 数值字段有合理的默认值（如下载数、关注数默认为 0）