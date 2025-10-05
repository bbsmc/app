---
title: Knossos（前端）
description: 为 BBSMC 前端做贡献的指南
sidebar:
  order: 2
---

# 为 Knossos 做贡献

Knossos 是 BBSMC 的前端应用程序，使用 Vue 3 和 Nuxt 3 构建。它提供了用户界面，用于浏览、搜索和管理模组。

## 开发环境设置

### 先决条件

- Node.js 18+ 和 pnpm
- 运行中的 Labrinth 后端实例（或使用生产 API）

### 快速开始

1. 克隆仓库：
```bash
git clone https://github.com/bbsmc/app.git
cd app
```

2. 安装依赖：
```bash
pnpm install
```

3. 启动开发服务器：
```bash
pnpm web:dev
```

应用程序将在 `http://localhost:3000` 上运行。

## 项目结构

```
apps/frontend/
├── assets/          # 静态资源
├── components/      # Vue 组件
├── composables/     # 组合式函数
├── layouts/         # 页面布局
├── pages/           # 路由页面
├── plugins/         # Nuxt 插件
├── public/          # 公共文件
├── store/           # 状态管理
└── utils/           # 工具函数
```

## 技术栈

- **框架**: Nuxt 3 + Vue 3
- **样式**: Tailwind CSS + SCSS
- **状态管理**: Pinia
- **国际化**: @vintl/nuxt
- **组件库**: @modrinth/ui

## 编码规范

- 使用组合式 API（Composition API）
- 遵循 Vue 3 最佳实践
- 使用 TypeScript 进行类型安全
- 运行 `pnpm lint` 和 `pnpm fix` 进行代码格式化

## 主题系统

BBSMC 支持多个主题：
- 浅色主题
- 深色主题
- OLED 主题
- 复古主题（Modrinth Plus）

开发时请确保新功能在所有主题下都能正常工作。

## 国际化

- 使用 `$t()` 函数进行文本翻译
- 翻译键定义在 `locales/` 目录
- 始终为新的用户界面文本添加翻译键

## 贡献流程

1. 查看现有组件和代码风格
2. 创建功能分支
3. 开发新功能或修复
4. 测试所有主题和响应式布局
5. 提交拉取请求