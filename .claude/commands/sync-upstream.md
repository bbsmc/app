# Modrinth 上游同步命令

执行 BBSMC 与 Modrinth 上游代码库的同步分析和合并。

## 使用方法

```
/sync-upstream [选项]
```

## 选项

- `analyze` - 只分析不合并，列出可用的修复
- `merge` - 执行完整的分析和合并流程
- `since:YYYY-MM-DD` - 指定起始日期（默认读取上次同步记录）

## 执行步骤

$ARGUMENTS

请执行以下上游同步任务：

### 1. 环境准备

```bash
# 检查上游远程是否存在
git remote -v | grep upstream

# 如果不存在，添加上游
git remote add upstream https://github.com/modrinth/code.git

# 获取上游更新
git fetch upstream --tags
```

### 2. 读取同步记录

读取 `UPSTREAM_SYNC.md` 文件，获取：
- 上次同步日期
- 排除的功能列表
- BBSMC 自定义功能列表

### 3. 分析上游提交

```bash
# 查找 bug 修复
git log upstream/main --since="<上次同步日期>" --oneline --grep="fix"
git log upstream/main --since="<上次同步日期>" --oneline --grep="bug"
git log upstream/main --since="<上次同步日期>" --oneline --grep="hotfix"
```

对每个提交分析：
- 是否是 bug 修复或优化（排除新功能）
- 是否属于排除的功能
- 是否与 BBSMC 自定义功能冲突
- 影响的文件和重要程度

### 4. 创建工作分支

```bash
git checkout -b feature/sync-upstream-fixes-$(date +%Y%m%d)
```

### 5. 合并修复

按优先级顺序合并：
1. 安全修复（最高优先级）
2. 数据完整性修复
3. 性能优化
4. UI/UX 修复

每个修复：
- 手动将代码写入 BBSMC（不直接 cherry-pick）
- 添加注释标注上游来源
- 创建独立提交

### 6. 验证

```bash
# 后端编译检查
cd apps/labrinth && SQLX_OFFLINE=true cargo check

# 前端检查（如果修改了前端）
cd apps/frontend && pnpm lint
```

### 7. 更新同步记录

在 `UPSTREAM_SYNC.md` 添加本次同步记录。

### 8. 生成报告

生成完整的同步报告，包括：
- 分析的提交数量
- 合并的修复数量
- 跳过的内容及原因
- 创建的提交列表
- 部署注意事项
