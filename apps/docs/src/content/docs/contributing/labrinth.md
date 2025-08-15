---
title: Labrinth（API/后端）
description: 为 BBSMC API 和后端做贡献的指南
sidebar:
  order: 5
---

# 为 Labrinth 做贡献

Labrinth 是 BBSMC 的后端 API 服务器，使用 Rust 编写。它处理所有的 API 请求、数据库操作、身份验证和业务逻辑。

## 开发环境设置

### 先决条件

- Rust（最新稳定版）
- PostgreSQL
- Redis
- 环境变量配置

### 快速开始

1. 克隆仓库：
```bash
git clone https://github.com/bbsmc/app.git
cd app/apps/labrinth
```

2. 设置环境变量（创建 `.env` 文件）

3. 运行数据库迁移：
```bash
sqlx migrate run
```

4. 启动开发服务器：
```bash
cargo run
```

## 项目结构

```
apps/labrinth/
├── src/
│   ├── routes/       # API 路由处理
│   ├── models/       # 数据模型
│   ├── database/     # 数据库交互
│   ├── auth/         # 身份验证
│   └── search/       # 搜索功能
├── migrations/       # 数据库迁移
└── tests/           # 测试文件
```

## 编码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 进行代码检查
- 使用 `cargo check` 而不是 `cargo build` 进行快速检查
- 编写单元测试和集成测试

## API 版本管理

BBSMC API 支持多个版本（v2 和 v3）。添加新功能时：

1. 在 v3 中实现新功能
2. 在 v2 中添加兼容层（如果适用）
3. 更新 OpenAPI 规范
4. 添加适当的测试

## 数据库

- 使用 SQLx 进行类型安全的数据库查询
- 迁移文件应放在 `apps/labrinth/migrations/`
- 使用事务处理复杂操作

## 贡献流程

1. 创建功能分支
2. 编写代码和测试
3. 运行 `cargo check` 确保编译通过
4. 提交更改并创建拉取请求
5. 等待代码审查