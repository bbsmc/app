# Labrinth 数据库操作参考文档

本文档为 Labrinth 后端开发提供数据库操作、ID 转换机制和架构模式的参考指南。

## 目录结构

### 1. Database 层 (`apps/labrinth/src/database/`)

```
database/
├── mod.rs                    # 数据库模块主入口
├── postgres_database.rs      # PostgreSQL 连接管理
├── redis.rs                  # Redis 缓存层
└── models/                   # 数据库模型
    ├── ids.rs               # ID 生成器和类型定义
    ├── project_item.rs      # 项目相关模型
    ├── user_item.rs         # 用户相关模型
    ├── version_item.rs      # 版本相关模型
    ├── issues.rs            # Issues 功能模型
    └── ...                  # 其他业务模型
```

### 2. Models 层 (`apps/labrinth/src/models/`)

```
models/
├── v2/                      # API v2 模型（旧版本）
├── v3/                      # API v3 模型（当前版本）
│   ├── ids.rs              # Base62 ID 转换逻辑
│   ├── projects.rs         # 项目 API 模型
│   ├── issues.rs           # Issues API 模型
│   └── ...
└── error.rs                # 错误处理
```

### 3. Routes 层 (`apps/labrinth/src/routes/`)

```
routes/
├── v2/                      # API v2 路由（旧版本）
├── v3/                      # API v3 路由（当前版本）
│   ├── projects.rs         # 项目相关路由
│   ├── issues.rs           # Issues 相关路由
│   └── ...
└── internal/               # 内部管理路由
```

## ID 系统架构

### ID 类型定义

Labrinth 使用两层 ID 系统：

1. **数据库层 ID** (`database/models/ids.rs`)
   - 内部使用 `i64` 存储
   - 例如：`pub struct ProjectId(pub i64)`

2. **API 层 ID** (`models/v3/ids.rs`)
   - 外部使用 `u64` 和 Base62 编码
   - 例如：`pub struct ProjectId(pub u64)`

### ID 转换机制

#### Base62 编码系统

```rust
// Base62 字符集：0-9, A-Z, a-z
const BASE62_CHARS: [u8; 62] = 
    *b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

// u64 转 Base62 字符串
pub fn to_base62(num: u64) -> String

// Base62 字符串转 u64
pub fn parse_base62(string: &str) -> Result<u64, DecodingError>
```

#### 自动序列化/反序列化

每个 ID 类型都实现了自动转换：

```rust
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Hash)]
#[serde(from = "Base62Id")]  // 反序列化：Base62 字符串 → u64
#[serde(into = "Base62Id")]  // 序列化：u64 → Base62 字符串
pub struct ProjectId(pub u64);
```

转换宏实现：

```rust
// 从 Base62Id 转换到具体 ID 类型
impl From<Base62Id> for ProjectId {
    fn from(id: Base62Id) -> ProjectId {
        ProjectId(id.0)
    }
}

// 从具体 ID 类型转换到 Base62Id
impl From<ProjectId> for Base62Id {
    fn from(id: ProjectId) -> Base62Id {
        Base62Id(id.0)
    }
}

// Display 特性实现（用于打印）
impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base62_impl::to_base62(self.0))
    }
}
```

### ID 生成策略

#### 单个 ID 生成

```rust
generate_ids!(
    pub generate_project_id,
    ProjectId,
    8,  // Base62 长度（8个字符）
    "SELECT EXISTS(SELECT 1 FROM mods WHERE id=$1)",
    ProjectId
);
```

生成流程：
1. 使用 ChaCha20 随机数生成器
2. 生成指定长度的 Base62 ID（通常 8 个字符）
3. 检查数据库中是否存在重复
4. 使用 Censor 库过滤不当词汇
5. 最多重试 20 次

#### 批量 ID 生成

```rust
generate_bulk_ids!(
    pub generate_bulk_version_ids,
    VersionId,
    "SELECT EXISTS(SELECT 1 FROM versions WHERE id = ANY($1))",
    VersionId
);
```

批量生成使用连续 ID 序列，提高效率。

## 数据库操作模式

### 1. 数据插入

#### 单条插入
```rust
impl Project {
    pub async fn insert(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO mods (id, name, slug, ...) 
             VALUES ($1, $2, $3, ...)",
            self.id.0,
            self.name,
            self.slug,
            // ...
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
}
```

#### 批量插入
```rust
impl LinkUrl {
    pub async fn insert_many_projects(
        links: Vec<Self>,
        project_id: ProjectId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::error::Error> {
        let (project_ids, platform_ids, urls): (Vec<_>, Vec<_>, Vec<_>) = 
            links.into_iter()
                .map(|url| (project_id.0, url.platform_id.0, url.url))
                .multiunzip();
        
        sqlx::query!(
            "INSERT INTO mods_links (joining_mod_id, joining_platform_id, url)
             SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::varchar[])",
            &project_ids[..],
            &platform_ids[..],
            &urls[..]
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }
}
```

### 2. 数据查询

#### 单条查询
```rust
impl Issue {
    pub async fn get(
        id: IssuesId,
        exec: &mut sqlx::PgConnection,
        redis: &RedisPool,
    ) -> Result<Option<Self>, DatabaseError> {
        // 先查 Redis 缓存
        if let Some(issue) = redis.get_cached_value(
            ISSUE_NAMESPACE,
            &id.0.to_string()
        ).await? {
            return Ok(Some(issue));
        }
        
        // 查询数据库
        let result = sqlx::query!(
            "SELECT * FROM issues WHERE id = $1",
            id.0
        )
        .fetch_optional(exec)
        .await?;
        
        // 写入缓存
        if let Some(row) = result {
            let issue = Self::from_row(row);
            redis.set_cached_value(
                ISSUE_NAMESPACE,
                &id.0.to_string(),
                &issue
            ).await?;
            Ok(Some(issue))
        } else {
            Ok(None)
        }
    }
}
```

#### 批量查询
```rust
impl Issue {
    pub async fn get_many(
        ids: &[IssuesId],
        exec: &mut sqlx::PgConnection,
        redis: &RedisPool,
    ) -> Result<Vec<Self>, DatabaseError> {
        // 使用 DashMap 并发处理
        let not_found = DashMap::new();
        
        // 批量查询缓存
        let cached = redis.get_many_cached_values(
            ISSUE_NAMESPACE,
            ids.iter().map(|x| x.0.to_string())
        ).await?;
        
        // 查询未缓存的数据
        let db_ids: Vec<i64> = not_found.iter()
            .map(|x| x.0)
            .collect();
            
        if !db_ids.is_empty() {
            let results = sqlx::query!(
                "SELECT * FROM issues WHERE id = ANY($1)",
                &db_ids
            )
            .fetch_all(exec)
            .await?;
            
            // 批量写入缓存
            for row in results {
                let issue = Self::from_row(row);
                redis.set_cached_value(
                    ISSUE_NAMESPACE,
                    &issue.id.0.to_string(),
                    &issue
                ).await?;
            }
        }
        
        Ok(issues)
    }
}
```

### 3. 数据更新

```rust
impl Issue {
    pub async fn update(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE issues 
             SET title = $2, body = $3, state = $4, updated_at = $5
             WHERE id = $1",
            self.id.0,
            self.title,
            self.body,
            self.state,
            self.updated_at
        )
        .execute(&mut **transaction)
        .await?;
        
        // 清除缓存
        redis.delete_cached_value(
            ISSUE_NAMESPACE,
            &self.id.0.to_string()
        ).await?;
        
        Ok(())
    }
}
```

### 4. 数据删除

```rust
impl Issue {
    pub async fn remove(
        id: IssuesId,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        redis: &RedisPool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "DELETE FROM issues WHERE id = $1",
            id.0
        )
        .execute(&mut **transaction)
        .await?;
        
        // 清除缓存
        redis.delete_cached_value(
            ISSUE_NAMESPACE,
            &id.0.to_string()
        ).await?;
        
        Ok(())
    }
}
```

## Redis 缓存策略

### 命名空间定义
```rust
pub const PROJECTS_NAMESPACE: &str = "projects";
pub const PROJECTS_SLUGS_NAMESPACE: &str = "projects_slugs";
pub const ISSUE_NAMESPACE: &str = "issues";
pub const ISSUE_LABELS_NAMESPACE: &str = "issue_labels";
```

### 缓存键格式
- 单个对象：`{namespace}:{id}`
- 列表：`{namespace}:list:{filter_key}`
- 关联数据：`{namespace}:{parent_id}:{child_type}`

### 缓存操作
```rust
// 设置缓存
redis.set_cached_value(namespace, key, value).await?;

// 获取缓存
redis.get_cached_value(namespace, key).await?;

// 删除缓存
redis.delete_cached_value(namespace, key).await?;

// 批量操作
redis.get_many_cached_values(namespace, keys).await?;
```

## 事务处理

### 基本事务模式
```rust
let mut transaction = pool.begin().await?;

// 执行多个操作
project.insert(&mut transaction).await?;
version.insert(&mut transaction).await?;

// 提交或回滚
transaction.commit().await?;
// 或
transaction.rollback().await?;
```

### 嵌套事务（Savepoint）
```rust
let mut transaction = pool.begin().await?;
let mut sub_transaction = transaction.begin().await?;

// 子事务操作
sub_transaction.commit().await?;
// 或
sub_transaction.rollback().await?;

transaction.commit().await?;
```

## 错误处理

### 数据库错误类型
```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("ID generation failed")]
    RandomId,
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

### 错误处理模式
```rust
// 使用 ? 操作符传播错误
let result = sqlx::query!(...).fetch_one(&mut conn).await?;

// 自定义错误转换
let project = Project::get(id, &mut conn, &redis)
    .await?
    .ok_or(DatabaseError::NotFound)?;

// 错误恢复
match operation().await {
    Ok(value) => value,
    Err(DatabaseError::Redis(_)) => {
        // Redis 失败时直接查数据库
        fetch_from_database().await?
    }
    Err(e) => return Err(e),
}
```

## 性能优化建议

### 1. 查询优化
- 使用索引覆盖查询
- 避免 N+1 查询问题
- 使用批量查询代替循环查询
- 合理使用 JOIN 和子查询

### 2. 缓存策略
- 热点数据优先缓存
- 设置合理的 TTL
- 使用缓存预热
- 实现缓存穿透保护

### 3. 连接池管理
```rust
// 连接池配置
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .connect_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(300))
    .connect(&database_url)
    .await?;
```

### 4. 批量操作
- 使用 UNNEST 进行批量插入
- 使用 ANY 进行批量查询
- 使用事务批处理相关操作

## 开发规范

### 1. ID 使用规范
- API 层使用 u64 + Base62 编码
- 数据库层使用 i64
- 生成 ID 长度统一为 8 个字符
- 必须检查 ID 唯一性

### 2. 缓存规范
- 所有查询优先走缓存
- 更新/删除操作必须清理缓存
- 使用统一的命名空间
- 实现缓存版本控制

### 3. 事务规范
- 写操作使用事务
- 保持事务简短
- 避免长事务
- 合理使用事务隔离级别

### 4. 错误处理规范
- 使用 Result 类型
- 提供详细的错误信息
- 实现错误恢复机制
- 记录错误日志

## 常见模式示例

### 1. 创建新实体
```rust
pub async fn create_issue(
    data: CreateIssueRequest,
    user: User,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Issue, ApiError> {
    let mut transaction = pool.begin().await?;
    
    // 生成 ID
    let issue_id = generate_issues_id(&mut transaction).await?;
    
    // 创建实体
    let issue = Issue {
        id: issue_id,
        title: data.title,
        // ...
    };
    
    // 插入数据库
    issue.insert(&mut transaction).await?;
    
    // 提交事务
    transaction.commit().await?;
    
    // 写入缓存
    redis.set_cached_value(
        ISSUE_NAMESPACE,
        &issue_id.0.to_string(),
        &issue
    ).await?;
    
    Ok(issue)
}
```

### 2. 更新实体
```rust
pub async fn update_issue(
    id: IssuesId,
    data: UpdateIssueRequest,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<Issue, ApiError> {
    let mut transaction = pool.begin().await?;
    
    // 获取现有数据
    let mut issue = Issue::get(id, &mut transaction, redis)
        .await?
        .ok_or(ApiError::NotFound)?;
    
    // 更新字段
    if let Some(title) = data.title {
        issue.title = title;
    }
    issue.updated_at = Utc::now();
    
    // 保存到数据库
    issue.update(&mut transaction, redis).await?;
    
    // 提交事务
    transaction.commit().await?;
    
    Ok(issue)
}
```

### 3. 查询列表
```rust
pub async fn list_issues(
    project_id: ProjectId,
    params: IssuesQueryParams,
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<PaginatedResponse<Issue>, ApiError> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    
    // 构建缓存键
    let cache_key = format!("{}:{}:{}", project_id, page, page_size);
    
    // 尝试从缓存获取
    if let Some(cached) = redis.get_cached_value(
        ISSUE_NAMESPACE,
        &cache_key
    ).await? {
        return Ok(cached);
    }
    
    // 查询数据库
    let mut conn = pool.acquire().await?;
    let issues = Issue::get_project_issues(
        project_id,
        params.state,
        &mut conn,
        redis
    ).await?;
    
    // 分页
    let total = issues.len();
    let offset = ((page - 1) * page_size) as usize;
    let issues = issues.into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();
    
    let response = PaginatedResponse {
        data: issues,
        total,
        page,
        page_size,
    };
    
    // 写入缓存
    redis.set_cached_value_with_ttl(
        ISSUE_NAMESPACE,
        &cache_key,
        &response,
        300 // 5 分钟 TTL
    ).await?;
    
    Ok(response)
}
```

## 注意事项

1. **ID 转换**：注意区分 API 层和数据库层的 ID 类型
2. **缓存一致性**：确保数据变更时清理相关缓存
3. **事务边界**：合理划分事务范围，避免死锁
4. **并发控制**：使用 DashMap/DashSet 处理并发访问
5. **错误处理**：提供友好的错误信息，记录详细日志
6. **性能监控**：关注慢查询，定期优化索引

## 相关文件参考

- ID 生成：`apps/labrinth/src/database/models/ids.rs`
- ID 转换：`apps/labrinth/src/models/v3/ids.rs`
- 数据库模型：`apps/labrinth/src/database/models/`
- API 模型：`apps/labrinth/src/models/v3/`
- 路由处理：`apps/labrinth/src/routes/v3/`
- Redis 操作：`apps/labrinth/src/database/redis.rs`