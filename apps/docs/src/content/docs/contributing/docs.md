---
title: 文档
description: 为 BBSMC 文档做贡献的指南
sidebar:
  order: 6
---

# 为文档做贡献

BBSMC 的文档使用 Astro 和 Starlight 构建，为开发者提供 API 文档和指南。

## 文档类型

### API 文档
- 从 OpenAPI 规范自动生成
- 位于 `public/openapi.yaml`
- 使用 `starlight-openapi` 插件渲染

### 指南和教程
- Markdown/MDX 格式
- 位于 `src/content/docs/`
- 支持代码高亮和交互式组件

## 本地开发

1. 进入文档目录：
```bash
cd apps/docs
```

2. 安装依赖：
```bash
pnpm install
```

3. 启动开发服务器：
```bash
pnpm dev
```

文档将在 `http://localhost:4321` 上运行。

## 文档结构

```
apps/docs/
├── src/
│   ├── content/
│   │   └── docs/
│   │       ├── contributing/  # 贡献指南
│   │       ├── guide/         # 使用指南
│   │       └── index.mdx      # 主页
│   ├── assets/                # 图片和 logo
│   └── styles/                # 自定义样式
├── public/
│   └── openapi.yaml          # API 规范
└── astro.config.mjs          # Astro 配置
```

## 编写指南

### Markdown 前言
每个文档文件都应包含前言：

```yaml
---
title: 页面标题
description: 页面描述
sidebar:
  order: 1  # 侧边栏排序
---
```

### 代码示例
使用代码围栏并指定语言：

````markdown
```rust
fn main() {
    println!("Hello, BBSMC!");
}
```
````

### 链接
- 内部链接使用相对路径：`[文本](./other-page)`
- 外部链接使用完整 URL：`[文本](https://example.com)`

## 更新 API 文档

1. 编辑 `public/openapi.yaml`
2. 运行开发服务器查看更改
3. API 文档会自动更新

## 贡献流程

1. 识别需要改进的文档
2. 创建分支并进行更改
3. 预览更改确保格式正确
4. 提交拉取请求
5. 等待审查和合并