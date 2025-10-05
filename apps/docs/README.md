# BBSMC 文档

这是 BBSMC 的开发者文档站点，使用 Astro 和 Starlight 构建。

## 🚀 快速开始

### 安装依赖

```bash
pnpm install
```

### 开发服务器

```bash
pnpm dev
```

文档将在 http://localhost:4321 上运行。

### 构建

```bash
pnpm build
```

### 预览构建结果

```bash
pnpm preview
```

## 📁 项目结构

```
.
├── public/
│   └── openapi.yaml      # API 规范文件
├── src/
│   ├── assets/          # Logo 和图片资源
│   ├── content/
│   │   └── docs/        # 文档内容
│   │       ├── contributing/  # 贡献指南
│   │       ├── guide/         # 使用指南
│   │       └── index.mdx      # 主页
│   └── styles/          # 自定义样式
├── astro.config.mjs     # Astro 配置
├── package.json
└── tsconfig.json
```

## 📝 编写文档

文档使用 Markdown/MDX 格式编写，放在 `src/content/docs/` 目录下。

每个文档文件需要包含 frontmatter：

```yaml
---
title: 页面标题
description: 页面描述
sidebar:
  order: 1  # 侧边栏排序
---
```

## 🌏 国际化

文档默认语言为简体中文。配置在 `astro.config.mjs` 中：

```js
defaultLocale: 'zh-CN',
locales: {
  'zh-CN': {
    label: '简体中文',
  },
}
```

## 🔧 配置

主要配置文件：
- `astro.config.mjs` - Astro 和 Starlight 配置
- `public/openapi.yaml` - API 文档规范

## 🤝 贡献

欢迎贡献！请查看[贡献指南](./src/content/docs/contributing/getting-started.md)。