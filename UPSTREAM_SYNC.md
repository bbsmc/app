# Modrinth 上游同步记录

本文件记录 BBSMC 与 Modrinth 上游代码库的同步历史。

## 同步配置

- **上游仓库**: https://github.com/modrinth/code
- **上游远程名**: `upstream`
- **Fork 时间**: 2024-11-16
- **Fork 起点**: commit `b188b3fe`

## 排除的功能（不同步）

以下 Modrinth 功能不适用于 BBSMC，同步时应跳过：

1. **服务器面板** - Pyro, Servers, server panel 相关
2. **桌面应用** - app-frontend, Tauri, theseus 相关
3. **支付系统** - Mural, Tremendous, 税务, billing 相关
4. **affiliate** - 推广码相关功能
5. **博客系统** - 新闻/博客功能

## BBSMC 自定义功能（优先保留）

以下是 BBSMC 独有的功能，同步时需避免冲突：

1. **论坛系统** - 完整的论坛功能
2. **用户封禁系统** - 多类型封禁（全局/资源/论坛）
3. **百科系统** - Wiki 功能
4. **翻译链接系统** - 版本翻译关联
5. **中文本地化** - 界面和消息中文化
6. **AutoMod 增强** - 云盘版本和许可证检测
7. **Turnstile 验证码** - 替代 hCaptcha

---

## 同步历史

### 2024-12-22 首次全面同步

**同步范围**: 2024-11-16 至 2024-12-22 (约 1023 个上游提交)

**执行分支**: `feature/sync-upstream-fixes-20251222`

**合并的修复数量**: 22 个

#### 创建的提交

| 提交哈希 | 描述 | 上游来源 |
|---------|------|---------|
| `6baf2e28c` | 安全: MRPACK 路径验证防止路径遍历攻击 | ab6e9dd5d |
| `8b4f68f4a` | 修复: 热门项目版本上传失败 | 71d63fbe1 |
| `3d2a87742` | 修复: 前端 UI 问题合集 | 267e0cb63, 94c0003c1 等 |
| `17a42c1d8` | 修复: 组织可见性检查 | 290c9fc19, 6f7618db1 |
| `4b7f5d3a0` | 修复: 搜索索引删除时序 | 97e4d8e13 |
| `c47d4cb34` | 修复: 后端多项改进 | 863bf62f8, b7f098839 等 |
| `683c3b8a4` | 修复: 前端多项改进 | fd9653e28, e368e35e7 等 |

#### 详细修复列表

**安全修复**:
- MRPACK 文件路径验证，防止路径遍历攻击

**后端修复**:
- 热门项目版本上传（过滤已删除用户）
- 组织可见性检查
- 搜索索引删除时序
- updated 字段排除已删除版本
- 项目删除与线程解耦
- 用户删除更新处理
- TOTP 时间偏移容错
- PGP 签名文件类型支持
- random_projects 性能优化

**前端修复**:
- 许可证 URL 清除
- scrollbar-gutter 稳定
- 移动端导航 z-index
- 按钮阴影对比度
- 空版本列表处理
- 认证重定向 URL 编码
- 举报按钮登录检查
- 多加载器资源包下载
- 图表竞态条件

**数据库迁移**:
- `20251222000001_decouple_threads_from_projects.sql`
- `20251222000002_random_project_index.sql`

#### 跳过的修复

- hCaptcha 相关（BBSMC 使用 Turnstile）
- 服务器面板功能
- 桌面应用修复
- affiliate/支付功能
- discover 页面（架构不同）

---

## 同步操作指南

### 准备工作

```bash
# 确保已添加上游远程
git remote add upstream https://github.com/modrinth/code.git

# 获取上游更新
git fetch upstream --tags
```

### 查找新的修复

```bash
# 查看上次同步后的提交
git log upstream/main --since="2024-12-22" --oneline --grep="fix"
git log upstream/main --since="2024-12-22" --oneline --grep="bug"
```

### 分析提交

```bash
# 查看具体提交内容
git show <commit-hash>

# 对比文件差异
git diff HEAD upstream/main -- <file-path>
```

### 创建同步分支

```bash
git checkout -b feature/sync-upstream-fixes-YYYYMMDD
```

### 同步后更新本文件

每次同步后，在"同步历史"部分添加新记录，包括：
- 日期
- 同步范围
- 合并的修复列表
- 跳过的内容及原因
