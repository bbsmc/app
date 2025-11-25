# BBSMC前端开发指南

如果您不是开发者，并且偶然发现了这个仓库，您可以访问 [BBSMC 网站](https://bbsmc.net) 上的网页界面。

## 开发

### 开发环境

开始之前，请确保您的电脑上安装了以下软件：

- [Node.js](https://nodejs.org/en/)
- [pnpm](https://pnpm.io/)

### 初始化

请按照以下步骤设置您的开发环境：

```bash
pnpm install
pnpm web:dev
```

现在，您应该已经拥有一个已启用热重载的开发版 Web 界面。您对代码所做的任何更改都会自动刷新浏览器。

### 增设新板块:

1. apps/frontend/nuxt.config.ts 增加types
2. apps/frontend/src/composables/tag.js 增加projectTypes
3. apps/frontend/src/helpers/fileUtils.js 增加该板块可接受上传的文件类型
4. apps/frontend/src/layouts/default.vue 增加Nav导航
5. apps/frontend/src/plugins/cosmetics.ts 增加视图类型
