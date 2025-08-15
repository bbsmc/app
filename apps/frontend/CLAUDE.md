# BBSMC 前端开发文档

本文档为 BBSMC 前端应用程序提供完整的开发指引和架构说明。

## 目录

- [项目概述](#项目概述)
- [技术栈](#技术栈)
- [项目架构](#项目架构)
- [开发环境设置](#开发环境设置)
- [目录结构详解](#目录结构详解)
- [核心概念](#核心概念)
- [组件系统](#组件系统)
- [状态管理](#状态管理)
- [路由系统](#路由系统)
- [样式系统](#样式系统)
- [API 集成](#api-集成)
- [国际化 (i18n)](#国际化-i18n)
- [开发规范](#开发规范)
- [常见问题](#常见问题)

## 项目概述

BBSMC 是一个 Minecraft 资源社区平台，基于 Modrinth 开源项目修改开发。前端应用使用现代化的 Vue 3 + Nuxt 3 技术栈构建，提供用户友好的界面来浏览、下载和管理 Minecraft 模组、整合包、插件等资源。

### 主要功能

- **资源浏览**：模组、整合包、插件、资源包、光影、数据包等
- **用户管理**：注册、登录、个人资料
- **项目管理**：创建、编辑、版本管理
- **社区功能**：论坛、评论、收藏
- **管理功能**：内容审核、用户管理
- **翻译系统**：多语言支持和翻译链接管理

## 技术栈

### 核心框架
- **Nuxt 3** - Vue.js 全栈框架
- **Vue 3** - 前端框架 (组合式 API)
- **TypeScript** - 类型安全的 JavaScript
- **Vite** - 构建工具

### UI 和样式
- **Tailwind CSS** - 原子化 CSS 框架
- **SCSS** - CSS 预处理器
- **@modrinth/ui** - 共享组件库
- **@modrinth/assets** - 图标和资源

### 状态管理和数据
- **Pinia** - Vue 3 状态管理
- **VueUse** - Vue 组合式函数工具集
- **$fetch/ofetch** - HTTP 客户端

### 工具库
- **@vintl/nuxt** - 国际化
- **dayjs** - 日期处理
- **markdown-it** - Markdown 解析
- **highlight.js** - 代码高亮
- **fuse.js** - 模糊搜索

## 项目架构

### 架构概览

```
BBSMC Frontend
├── 用户界面层 (UI Layer)
│   ├── 页面组件 (Pages)
│   ├── 布局组件 (Layouts)
│   └── UI 组件 (Components)
├── 逻辑层 (Logic Layer)
│   ├── 组合式函数 (Composables)
│   ├── 工具函数 (Helpers)
│   └── 中间件 (Middleware)
├── 状态管理层 (State Layer)
│   ├── 用户状态 (Auth)
│   ├── 应用状态 (App State)
│   └── 缓存管理 (Cache)
└── 数据层 (Data Layer)
    ├── API 调用 (API Calls)
    ├── 数据转换 (Data Transform)
    └── 错误处理 (Error Handling)
```

### 模块化设计

项目采用单体仓库 (monorepo) 架构，分为以下主要模块：

- **apps/frontend** - 主前端应用
- **packages/ui** - 共享 UI 组件
- **packages/utils** - 共享工具函数
- **packages/assets** - 图标和静态资源

## 开发环境设置

### 环境要求

- Node.js 18+ 
- pnpm 8+
- TypeScript 5+

### 安装步骤

```bash
# 1. 克隆仓库
git clone https://github.com/bbsmc/app.git
cd app

# 2. 安装依赖
pnpm install

# 3. 启动开发服务器
pnpm web:dev

# 4. 或者启动所有服务
turbo run dev
```

### 环境变量

在 `apps/frontend` 目录下创建 `.env` 文件：

```env
# API 基础地址
BROWSER_BASE_URL=http://api.bbsmc.net/v2/
BASE_URL=http://api.bbsmc.net/v2/

# 网站地址
SITE_URL=https://bbsmc.net

# 功能标志覆盖
FLAG_OVERRIDES={}
```

### 开发命令

```bash
# 开发相关
pnpm dev              # 启动开发服务器
pnpm build            # 生产构建
pnpm preview          # 预览构建结果

# 代码质量
pnpm lint             # 检查代码规范
pnpm fix              # 自动修复代码问题

# 国际化
pnpm intl:extract     # 提取国际化字符串
```

## 目录结构详解

```
src/
├── assets/                    # 静态资源
│   ├── icons/                 # SVG 图标
│   ├── images/                # 图片资源
│   └── styles/                # 样式文件
│       ├── global.scss        # 全局样式和主题
│       ├── layout.scss        # 布局样式
│       ├── components.scss    # 组件样式
│       └── utils.scss         # 工具样式
├── components/                # Vue 组件
│   ├── brand/                 # 品牌相关组件
│   └── ui/                    # 通用 UI 组件
│       ├── charts/            # 图表组件
│       ├── report/            # 举报相关组件
│       ├── search/            # 搜索相关组件
│       ├── servers/           # 服务器相关组件
│       └── thread/            # 消息线程组件
├── composables/               # 组合式函数
│   ├── auth/                  # 认证相关
│   ├── auth.js                # 用户认证
│   ├── fetch.js               # HTTP 请求
│   ├── loading.js             # 加载状态
│   ├── notifs.js              # 通知系统
│   └── ...                    # 其他组合式函数
├── helpers/                   # 工具函数
│   ├── projects.js            # 项目相关工具
│   ├── users.js               # 用户相关工具
│   ├── notifications.js       # 通知工具
│   └── ...                    # 其他工具函数
├── layouts/                   # 布局组件
│   └── default.vue            # 默认布局
├── middleware/                # 中间件
│   └── auth.ts                # 认证中间件
├── pages/                     # 页面组件 (路由)
│   ├── [type]/[id].vue        # 动态项目页面
│   ├── auth/                  # 认证相关页面
│   ├── dashboard/             # 用户面板
│   ├── moderation/            # 管理面板
│   ├── settings/              # 设置页面
│   └── ...                    # 其他页面
├── plugins/                   # 插件
│   ├── theme/                 # 主题系统
│   ├── dayjs.js               # 日期插件
│   └── ...                    # 其他插件
├── server/                    # 服务端代码
│   └── routes/                # 服务端路由
├── types/                     # TypeScript 类型定义
└── utils/                     # 工具函数
    ├── analytics.js           # 分析工具
    ├── permissions.ts         # 权限工具
    └── ...                    # 其他工具
```

## 核心概念

### 1. 组合式 API

项目全面采用 Vue 3 组合式 API：

```vue
<script setup>
import { ref, computed, watch, onMounted } from 'vue'

// 响应式状态
const loading = ref(false)
const data = ref(null)

// 计算属性
const processedData = computed(() => {
  return data.value ? processData(data.value) : null
})

// 监听器
watch(loading, (newVal) => {
  if (newVal) {
    fetchData()
  }
})

// 生命周期
onMounted(() => {
  init()
})
</script>
```

### 2. 组合式函数 (Composables)

可重用的逻辑封装在组合式函数中：

```javascript
// composables/useProject.js
export const useProject = (projectId) => {
  const project = ref(null)
  const loading = ref(false)
  
  const fetchProject = async () => {
    loading.value = true
    try {
      project.value = await useBaseFetch(`project/${projectId}`)
    } finally {
      loading.value = false
    }
  }
  
  return {
    project: readonly(project),
    loading: readonly(loading),
    fetchProject
  }
}
```

### 3. 类型安全

使用 TypeScript 确保类型安全：

```typescript
// types/modrinth.d.ts
interface Project {
  id: string
  title: string
  description: string
  project_type: ProjectType
  status: ProjectStatus
  // ...
}

type ProjectType = 'mod' | 'modpack' | 'plugin' | 'resourcepack' | 'shader' | 'datapack'
```

## 组件系统

### UI 组件库

项目使用自定义的 UI 组件库，位于 `components/ui/` 目录：

#### 基础组件

```vue
<!-- Avatar 头像组件 -->
<Avatar 
  :src="user.avatar_url" 
  :alt="user.username"
  size="md"
  circle
/>

<!-- Badge 标签组件 -->
<Badge 
  :type="project.status"
  class="status-badge"
>
  {{ getStatusLabel(project.status) }}
</Badge>

<!-- Button 按钮组件 -->
<ButtonStyled color="primary">
  <CheckIcon />
  确认
</ButtonStyled>
```

#### 表单组件

```vue
<!-- Checkbox 复选框 -->
<Checkbox v-model="agreed">
  我同意服务条款
</Checkbox>

<!-- FileInput 文件上传 -->
<FileInput
  v-model="files"
  accept="image/*"
  multiple
  @upload-progress="handleProgress"
/>

<!-- Chips 选项卡 -->
<Chips
  v-model="selectedCategory"
  :items="categories"
  :format-label="getCategoryLabel"
/>
```

#### 复杂组件

```vue
<!-- ProjectCard 项目卡片 -->
<ProjectCard
  :id="project.id"
  :type="project.project_type"
  :name="project.title"
  :author="project.author"
  :description="project.description"
  :icon-url="project.icon_url"
  :downloads="project.downloads"
  :follows="project.follows"
  :featured-image="project.featured_gallery"
/>

<!-- NotificationItem 通知项 -->
<NotificationItem
  :notification="notification"
  :notifications="notifications"
  :compact="false"
  :auth="auth"
  @update:notifications="updateNotifications"
/>
```

### 组件开发规范

#### 1. 组件结构

```vue
<template>
  <div class="component-name">
    <!-- 组件内容 -->
  </div>
</template>

<script setup>
// 导入
import { ref, computed } from 'vue'

// Props 定义
const props = defineProps({
  title: {
    type: String,
    required: true
  },
  size: {
    type: String,
    default: 'md',
    validator: (value) => ['sm', 'md', 'lg'].includes(value)
  }
})

// Emits 定义
const emit = defineEmits(['update:modelValue', 'change'])

// 响应式状态
const isActive = ref(false)

// 计算属性
const classes = computed(() => [
  'component-name',
  `component-name--${props.size}`,
  { 'component-name--active': isActive.value }
])
</script>

<style scoped lang="scss">
.component-name {
  // 样式定义
  
  &--active {
    // 激活状态样式
  }
  
  &--sm {
    // 小尺寸样式
  }
}
</style>
```

#### 2. Props 设计原则

- 使用 TypeScript 类型定义
- 提供合理的默认值
- 添加 validator 验证
- 遵循 Vue 命名约定

#### 3. 事件处理

```vue
<script setup>
// 定义事件
const emit = defineEmits(['update:modelValue', 'change', 'submit'])

// 处理输入
const handleInput = (value) => {
  emit('update:modelValue', value)
  emit('change', value)
}

// 处理提交
const handleSubmit = (data) => {
  emit('submit', data)
}
</script>
```

## 状态管理

### 组合式状态管理

项目主要使用 Vue 3 的 `useState` 进行状态管理：

#### 1. 用户认证状态

```javascript
// composables/auth.js
export const useAuth = async (oldToken = null) => {
  const auth = useState('auth', () => ({
    user: null,
    token: '',
    headers: {}
  }))

  if (!auth.value.user || oldToken) {
    auth.value = await initAuth(oldToken)
  }

  return auth
}

// 使用示例
const auth = await useAuth()
console.log(auth.value.user) // 当前用户信息
```

#### 2. 应用状态

```javascript
// composables/loading.js
export const useLoading = () => {
  const loading = useState('loading', () => false)
  
  const startLoading = () => {
    loading.value = true
  }
  
  const stopLoading = () => {
    loading.value = false
  }
  
  return {
    loading: readonly(loading),
    startLoading,
    stopLoading
  }
}
```

#### 3. 通知系统

```javascript
// composables/notifs.js
export const useNotifications = () => useState('notifications', () => [])

export const addNotification = (notification) => {
  const notifications = useNotifications()
  
  // 避免重复通知
  const existingNotif = notifications.value.find(
    x => x.text === notification.text && 
        x.title === notification.title &&
        x.type === notification.type
  )
  
  if (existingNotif) {
    setNotificationTimer(existingNotif)
    return
  }

  notification.id = new Date()
  setNotificationTimer(notification)
  notifications.value.push(notification)
}
```

### 状态持久化

重要状态通过 Cookie 或 LocalStorage 持久化：

```javascript
// 认证 Token 持久化
const authCookie = useCookie('auth-token', {
  maxAge: 60 * 60 * 24 * 365 * 10, // 10 年
  sameSite: 'lax',
  secure: true,
  httpOnly: false,
  path: '/'
})

// 主题设置持久化
const themeSettings = useCookie('theme-settings', {
  default: () => ({ preferred: 'system' }),
  sameSite: 'lax'
})
```

## 路由系统

### 基于文件的路由

Nuxt 3 使用基于文件的路由系统，`pages/` 目录中的文件自动生成路由：

```
pages/
├── index.vue                   # /
├── auth.vue                    # /auth
├── auth/
│   ├── sign-in.vue            # /auth/sign-in
│   └── sign-up.vue            # /auth/sign-up
├── [type]/
│   └── [id].vue               # /mod/example-mod
├── dashboard/
│   ├── index.vue              # /dashboard
│   └── projects.vue           # /dashboard/projects
└── settings/
    ├── index.vue              # /settings
    └── profile.vue            # /settings/profile
```

### 动态路由

#### 1. 项目页面路由

```vue
<!-- pages/[type]/[id].vue -->
<script setup>
const route = useRoute()
const { type, id } = route.params

// 获取项目数据
const project = await useBaseFetch(`project/${id}`)

// SEO 设置
useHead({
  title: project.title,
  meta: [
    { name: 'description', content: project.description }
  ]
})
</script>
```

#### 2. 路由参数处理

```javascript
// composables/route-params.js
export const useRouteParams = () => {
  const route = useRoute()
  
  // 获取项目类型
  const projectType = computed(() => {
    return getProjectTypeFromRoute(route.params.type)
  })
  
  // 获取项目 ID
  const projectId = computed(() => {
    return route.params.id
  })
  
  return {
    projectType,
    projectId
  }
}
```

### 路由守卫

#### 1. 认证中间件

```typescript
// middleware/auth.ts
export default defineNuxtRouteMiddleware((to, from) => {
  const { $auth } = useNuxtApp()
  
  if (!$auth.user) {
    return navigateTo('/auth/sign-in')
  }
})
```

#### 2. 权限检查

```vue
<!-- pages/moderation/index.vue -->
<script setup>
const auth = await useAuth()

// 检查管理员权限
if (!auth.value?.user || 
    (auth.value.user.role !== 'admin' && auth.value.user.role !== 'moderator')) {
  throw createError({
    statusCode: 403,
    statusMessage: '权限不足'
  })
}
</script>
```

### 路由配置

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  hooks: {
    'pages:extend'(routes) {
      // 自定义搜索路由
      const types = ['mods', 'modpacks', 'plugins', 'resourcepacks', 'shaders', 'datapacks']
      
      types.forEach(type => {
        routes.push({
          name: `search-${type}`,
          path: `/${type}`,
          file: resolve(__dirname, 'src/pages/search/[searchProjectType].vue'),
          children: []
        })
      })
    }
  }
})
```

## 样式系统

### 主题系统

项目支持多主题切换，包括浅色、深色、OLED 和复古主题：

#### 1. 主题定义

```scss
// assets/styles/global.scss

// 浅色主题
.light-mode {
  --color-bg: #e5e7eb;
  --color-raised-bg: #ffffff;
  --color-text: hsl(221, 39%, 11%);
  --color-text-dark: #1a202c;
  --color-brand: var(--color-green);
  // ...
}

// 深色主题
.dark-mode {
  --color-bg: #131313;
  --color-raised-bg: #1a1a1b;
  --color-text: var(--dark-color-text);
  --color-text-dark: var(--dark-color-text-dark);
  --color-brand: var(--color-gray);
  // ...
}
```

#### 2. 主题切换

```typescript
// plugins/theme/index.ts
export default defineNuxtPlugin({
  setup() {
    const { cycle } = useTheme()
    
    // 切换主题
    function changeTheme() {
      const nextTheme = cycle()
      return nextTheme
    }
    
    return {
      provide: {
        theme: {
          changeTheme
        }
      }
    }
  }
})
```

#### 3. 组件中使用主题

```vue
<template>
  <div class="themed-component" :style="themeVars">
    <!-- 组件内容 -->
  </div>
</template>

<script setup>
import { isDarkTheme } from '~/plugins/theme/themes'

const { $theme } = useNuxtApp()

const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      '--component-bg': 'rgba(0, 0, 0, 0.8)',
      '--component-text': 'var(--color-text-dark)'
    }
  } else {
    return {
      '--component-bg': 'rgba(255, 255, 255, 0.9)',
      '--component-text': 'var(--color-text)'
    }
  }
})
</script>
```

### CSS 架构

#### 1. 全局变量

```scss
:root {
  // 间距变量
  --gap-2: 0.125rem;
  --gap-4: calc(2 * var(--gap-2));
  --gap-8: calc(2 * var(--gap-4));
  
  // 圆角变量
  --radius-sm: 0.5rem;
  --radius-md: 0.75rem;
  --radius-lg: 1rem;
  
  // 字体变量
  --text-14: 0.875rem;
  --text-16: 1rem;
  --text-18: 1.125rem;
  
  // 图标变量
  --icon-16: 1rem;
  --icon-20: 1.25rem;
  --icon-24: 1.5rem;
}
```

#### 2. 组件样式

```scss
// 使用 BEM 命名约定
.project-card {
  display: grid;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  
  &__title {
    font-size: var(--text-18);
    font-weight: 600;
    color: var(--color-text-dark);
  }
  
  &__description {
    font-size: var(--text-14);
    color: var(--color-text);
    line-height: 1.5;
  }
  
  &--featured {
    border: 2px solid var(--color-brand);
  }
}
```

#### 3. 响应式设计

```scss
.container {
  max-width: 1224px;
  margin: 0 auto;
  padding: 0 1rem;
  
  @media (max-width: 768px) {
    padding: 0 0.5rem;
  }
  
  @media (max-width: 480px) {
    padding: 0 0.25rem;
  }
}

.grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 1rem;
  
  @media (max-width: 768px) {
    grid-template-columns: repeat(2, 1fr);
  }
  
  @media (max-width: 480px) {
    grid-template-columns: 1fr;
  }
}
```

### Tailwind CSS 集成

#### 1. 配置

```javascript
// tailwind.config.js
module.exports = {
  content: [
    "./src/components/**/*.{js,vue,ts}",
    "./src/layouts/**/*.vue",
    "./src/pages/**/*.vue",
    "./src/plugins/**/*.{js,ts}",
    "./src/app.vue",
    "./src/error.vue",
    "../../packages/**/*.{js,vue,ts}"
  ],
  theme: {
    extend: {
      colors: {
        primary: "var(--color-text)",
        secondary: "var(--color-text-secondary)",
        brand: "var(--color-brand)"
      }
    }
  }
}
```

#### 2. 自定义工具类

```scss
// assets/styles/utils.scss
.text-brand {
  color: var(--color-brand);
}

.bg-raised {
  background-color: var(--color-raised-bg);
}

.shadow-card {
  box-shadow: var(--shadow-card);
}
```

## API 集成

### HTTP 客户端

项目使用自定义的 `useBaseFetch` 进行 API 调用：

#### 1. 基础用法

```javascript
// 获取数据
const project = await useBaseFetch(`project/${projectId}`)

// POST 请求
const result = await useBaseFetch('project', {
  method: 'POST',
  body: {
    title: 'New Project',
    description: 'Project description'
  }
})

// 使用不同 API 版本
const data = await useBaseFetch('endpoint', {
  apiVersion: 3
})

// 跳过认证
const publicData = await useBaseFetch('public/data', {}, true)
```

#### 2. 文件上传

```javascript
// composables/fetch.js
export const useBaseFetchFile = async (url, options = {}, skipAuth = false) => {
  // 处理文件上传
  if (options.body instanceof FormData) {
    const xhr = new XMLHttpRequest()
    
    // 设置上传进度监听
    if (options.onUploadProgress && xhr.upload) {
      xhr.upload.onprogress = function(event) {
        if (event.lengthComputable) {
          const percentComplete = (event.loaded / event.total) * 100
          const uploadSpeed = calculateSpeed(event.loaded, event.total)
          options.onUploadProgress(percentComplete, uploadSpeed)
        }
      }
    }
    
    return new Promise((resolve, reject) => {
      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve(JSON.parse(xhr.responseText))
        } else {
          reject(JSON.parse(xhr.responseText))
        }
      }
      
      xhr.send(options.body)
    })
  }
  
  return await $fetch(`${base}${url}`, options)
}
```

#### 3. 错误处理

```javascript
// 统一错误处理
const handleApiError = (error) => {
  const app = useNuxtApp()
  
  let message = '发生未知错误'
  
  if (error.data?.description) {
    message = error.data.description
  } else if (error.message) {
    message = error.message
  }
  
  app.$notify({
    group: 'main',
    title: '错误',
    text: message,
    type: 'error'
  })
}

// 使用示例
try {
  const result = await useBaseFetch('api/endpoint')
} catch (error) {
  handleApiError(error)
}
```

### API 数据处理

#### 1. 数据转换

```javascript
// helpers/projects.js

// 获取项目链接
export const getProjectLink = (project) => {
  return `/${getProjectTypeForUrl(project.project_type, project.loaders)}/${
    project.slug ? project.slug : project.id
  }`
}

// 格式化版本显示
export const formatVersionsForDisplay = (gameVersions) => {
  const inputVersions = gameVersions.slice()
  const tags = useTags().value
  
  // 处理版本分组逻辑
  const releaseVersions = inputVersions.filter(version => 
    tags.gameVersions.some(gameVer => 
      gameVer.version === version && gameVer.version_type === 'release'
    )
  )
  
  return groupVersions(releaseVersions)
}
```

#### 2. 缓存策略

```javascript
// 使用 Nuxt 缓存
const { data: project } = await useFetch(`project/${id}`, {
  key: `project-${id}`,
  server: true,
  default: () => null
})

// 手动缓存控制
const projectCache = new Map()

const getCachedProject = async (id) => {
  if (projectCache.has(id)) {
    return projectCache.get(id)
  }
  
  const project = await useBaseFetch(`project/${id}`)
  projectCache.set(id, project)
  
  return project
}
```

## 国际化 (i18n)

### 国际化配置

项目使用 `@vintl/nuxt` 进行国际化：

#### 1. Nuxt 配置

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vintl/nuxt'],
  vintl: {
    defaultLocale: 'zh-Hans',
    locales: [
      {
        tag: 'zh-Hans',
        meta: {
          static: { iso: 'en' }
        }
      }
    ],
    storage: 'cookie',
    parserless: 'only-prod'
  }
})
```

#### 2. 消息定义

```vue
<script setup>
import { defineMessages } from '@vintl/vintl'

// 定义消息
const messages = defineMessages({
  title: {
    id: 'page.title',
    defaultMessage: '页面标题'
  },
  description: {
    id: 'page.description', 
    defaultMessage: '这是页面描述'
  }
})

const { formatMessage } = useVIntl()
</script>

<template>
  <div>
    <h1>{{ formatMessage(messages.title) }}</h1>
    <p>{{ formatMessage(messages.description) }}</p>
  </div>
</template>
```

#### 3. 提取和管理翻译

```bash
# 提取翻译字符串
pnpm intl:extract

# 生成的翻译文件结构
src/locales/
├── zh-Hans/
│   ├── index.json          # 主要翻译
│   ├── languages.json      # 语言名称
│   └── meta.json          # 元数据
└── en-US/
    ├── index.json
    └── ...
```

#### 4. 动态翻译

```vue
<script setup>
const { formatMessage } = useVIntl()

// 带参数的翻译
const welcomeMessage = formatMessage({
  id: 'welcome.message',
  defaultMessage: '欢迎, {username}!'
}, {
  username: user.username
})

// 复数处理
const downloadCount = formatMessage({
  id: 'download.count',
  defaultMessage: '{count, plural, one {# download} other {# downloads}}'
}, {
  count: project.downloads
})
</script>
```

### 项目类型本地化

```typescript
// utils/i18n-project-type.ts
export const getProjectTypeMessage = (type: string, plural = false) => {
  const key = plural ? `project_type.${type}.plural` : `project_type.${type}.singular`
  
  return {
    id: key,
    defaultMessage: getDefaultProjectTypeName(type, plural)
  }
}

// 使用示例
const projectTypeLabel = formatMessage(getProjectTypeMessage('mod', true))
// 输出: "模组"
```

## 开发规范

### 代码风格

#### 1. Vue 组件规范

```vue
<!-- ✅ 好的示例 -->
<template>
  <div class="user-profile">
    <Avatar 
      :src="user.avatarUrl" 
      :alt="`${user.username} 的头像`"
      size="lg"
    />
    <div class="user-info">
      <h2 class="username">{{ user.username }}</h2>
      <p class="bio">{{ user.bio }}</p>
    </div>
  </div>
</template>

<script setup>
// 导入按字母顺序排列
import Avatar from '~/components/ui/Avatar.vue'
import { computed, ref } from 'vue'

// Props 定义
const props = defineProps({
  userId: {
    type: String,
    required: true
  }
})

// 状态定义
const user = ref(null)
const loading = ref(false)

// 计算属性
const displayName = computed(() => 
  user.value?.displayName || user.value?.username
)

// 方法定义
const fetchUser = async () => {
  loading.value = true
  try {
    user.value = await useBaseFetch(`user/${props.userId}`)
  } finally {
    loading.value = false
  }
}

// 生命周期
onMounted(fetchUser)
</script>

<style scoped lang="scss">
.user-profile {
  display: flex;
  align-items: center;
  gap: var(--spacing-card-md);
}

.user-info {
  flex: 1;
}

.username {
  margin: 0 0 var(--spacing-card-sm) 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}

.bio {
  margin: 0;
  color: var(--color-text-secondary);
}
</style>
```

#### 2. JavaScript/TypeScript 规范

```javascript
// ✅ 使用 const/let 而不是 var
const API_BASE_URL = 'https://api.bbsmc.net/v2'

// ✅ 函数命名使用动词开头
const fetchProjects = async () => { /* ... */ }
const validateForm = (data) => { /* ... */ }

// ✅ 使用箭头函数
const processData = (data) => data.filter(item => item.active)

// ✅ 解构赋值
const { title, description, author } = project
const [first, second, ...rest] = items

// ✅ 模板字符串
const message = `欢迎 ${user.username} 来到 BBSMC`

// ✅ 异步错误处理
const handleSubmit = async () => {
  try {
    const result = await submitForm(formData)
    showSuccessMessage()
  } catch (error) {
    showErrorMessage(error.message)
  }
}
```

#### 3. 样式规范

```scss
// ✅ 使用 CSS 变量
.component {
  background: var(--color-raised-bg);
  color: var(--color-text);
  padding: var(--spacing-card-md);
}

// ✅ BEM 命名规范
.project-card {
  &__title {
    font-size: var(--font-size-lg);
  }
  
  &__description {
    font-size: var(--font-size-sm);
  }
  
  &--featured {
    border: 2px solid var(--color-brand);
  }
}

// ✅ 移动优先的响应式设计
.grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
  
  @media (min-width: 768px) {
    grid-template-columns: repeat(2, 1fr);
  }
  
  @media (min-width: 1024px) {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### 性能优化

#### 1. 懒加载

```vue
<script setup>
// 组件懒加载
const LazyModal = defineAsyncComponent(() => import('~/components/ui/Modal.vue'))

// 路由懒加载
const routes = [
  {
    path: '/dashboard',
    component: () => import('~/pages/dashboard.vue')
  }
]
</script>

<template>
  <div>
    <!-- 条件渲染懒加载组件 -->
    <LazyModal v-if="showModal" />
  </div>
</template>
```

#### 2. 图片优化

```vue
<template>
  <!-- 使用 loading="lazy" 懒加载图片 -->
  <img 
    :src="project.iconUrl" 
    :alt="project.title"
    loading="lazy"
    class="project-icon"
  />
  
  <!-- 提供多种尺寸 -->
  <img 
    :srcset="`${project.iconUrl}?w=64 64w, ${project.iconUrl}?w=128 128w`"
    sizes="(max-width: 768px) 64px, 128px"
    :src="project.iconUrl"
    :alt="project.title"
  />
</template>
```

#### 3. 缓存策略

```javascript
// 使用计算属性缓存复杂计算
const expensiveValue = computed(() => {
  return heavyCalculation(props.data)
})

// 使用 readonly 防止意外修改
const readonlyState = readonly(state)

// 合理使用 watch 的 immediate 选项
watch(
  () => props.projectId,
  (newId) => fetchProject(newId),
  { immediate: true }
)
```

### 错误处理

#### 1. 全局错误处理

```vue
<!-- error.vue -->
<template>
  <div class="error-page">
    <h1>{{ error.statusCode }}</h1>
    <p>{{ error.statusMessage }}</p>
    <button @click="handleError">
      {{ error.statusCode === 404 ? '返回首页' : '重试' }}
    </button>
  </div>
</template>

<script setup>
const props = defineProps(['error'])

const handleError = () => {
  if (props.error.statusCode === 404) {
    navigateTo('/')
  } else {
    window.location.reload()
  }
}
</script>
```

#### 2. 组件级错误处理

```vue
<script setup>
import { ref, onErrorCaptured } from 'vue'

const error = ref(null)

// 捕获子组件错误
onErrorCaptured((err) => {
  error.value = err
  console.error('Component error:', err)
  return false // 阻止错误继续传播
})
</script>

<template>
  <div>
    <div v-if="error" class="error-boundary">
      <p>组件加载失败</p>
      <button @click="error = null">重试</button>
    </div>
    <slot v-else />
  </div>
</template>
```

### 测试规范

#### 1. 单元测试

```javascript
// tests/components/Avatar.test.js
import { mount } from '@vue/test-utils'
import Avatar from '~/components/ui/Avatar.vue'

describe('Avatar', () => {
  it('renders with correct src', () => {
    const wrapper = mount(Avatar, {
      props: {
        src: 'https://example.com/avatar.jpg',
        alt: 'User Avatar'
      }
    })
    
    expect(wrapper.find('img').attributes('src')).toBe('https://example.com/avatar.jpg')
    expect(wrapper.find('img').attributes('alt')).toBe('User Avatar')
  })
  
  it('applies size class correctly', () => {
    const wrapper = mount(Avatar, {
      props: {
        size: 'lg'
      }
    })
    
    expect(wrapper.classes()).toContain('avatar--lg')
  })
})
```

#### 2. 集成测试

```javascript
// tests/pages/project.test.js
import { mountSuspended } from '@nuxt/test-utils/runtime'
import ProjectPage from '~/pages/[type]/[id].vue'

describe('Project Page', () => {
  it('loads project data correctly', async () => {
    const wrapper = await mountSuspended(ProjectPage, {
      route: '/mod/example-mod'
    })
    
    // 等待数据加载
    await wrapper.vm.$nextTick()
    
    expect(wrapper.find('.project-title').text()).toBe('Example Mod')
    expect(wrapper.find('.project-description').exists()).toBe(true)
  })
})
```

## 常见问题

### 1. 开发环境问题

#### Q: 启动开发服务器时出现端口冲突
```bash
# 解决方案：指定不同端口
PORT=3001 pnpm dev

# 或者在 nuxt.config.ts 中配置
export default defineNuxtConfig({
  devServer: {
    port: 3001
  }
})
```

#### Q: 热重载不工作
```bash
# 检查文件监听限制
echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# 或者使用轮询模式
export NUXT_DEV_SERVER_POLLING=true
```

### 2. 构建问题

#### Q: 构建时内存不足
```bash
# 增加 Node.js 内存限制
NODE_OPTIONS="--max-old-space-size=4096" pnpm build

# 或者在 package.json 中配置
{
  "scripts": {
    "build": "cross-env NODE_OPTIONS=--max-old-space-size=4096 nuxt build"
  }
}
```

#### Q: TypeScript 类型检查失败
```typescript
// 忽略类型检查错误（临时解决方案）
// @ts-ignore
const problematicCode = something.that.typescript.doesnt.like

// 更好的解决方案：添加类型定义
interface CustomWindow extends Window {
  customProperty: any
}
declare const window: CustomWindow
```

### 3. 样式问题

#### Q: CSS 变量在某些浏览器中不工作
```scss
// 提供降级方案
.component {
  background: #ffffff; /* 降级 */
  background: var(--color-raised-bg); /* 现代浏览器 */
}
```

#### Q: Tailwind 类名不生效
```javascript
// 确保在 tailwind.config.js 中包含了正确的路径
module.exports = {
  content: [
    "./src/**/*.{vue,js,ts}",
    "./components/**/*.{vue,js,ts}",
    "./layouts/**/*.vue",
    "./pages/**/*.vue"
  ]
}
```

### 4. API 相关问题

#### Q: CORS 跨域问题
```javascript
// 在开发环境中配置代理
// nuxt.config.ts
export default defineNuxtConfig({
  devServer: {
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true
      }
    }
  }
})
```

#### Q: 认证 Token 失效处理
```javascript
// 在 fetch 拦截器中处理
const handleAuthError = async (error) => {
  if (error.status === 401) {
    // 清除无效 Token
    const authCookie = useCookie('auth-token')
    authCookie.value = null
    
    // 重定向到登录页
    await navigateTo('/auth/sign-in')
  }
}
```

### 5. 性能问题

#### Q: 页面加载缓慢
```javascript
// 使用懒加载
const HeavyComponent = defineAsyncComponent(() => 
  import('~/components/HeavyComponent.vue')
)

// 预加载关键资源
useHead({
  link: [
    { rel: 'preload', href: '/critical.css', as: 'style' },
    { rel: 'prefetch', href: '/next-page-data.json', as: 'fetch' }
  ]
})
```

#### Q: 大列表渲染性能问题
```vue
<template>
  <!-- 使用虚拟滚动 -->
  <VirtualList
    :items="largeDataSet"
    :item-height="60"
    :container-height="400"
  >
    <template #item="{ item, index }">
      <ListItem :data="item" :index="index" />
    </template>
  </VirtualList>
</template>
```

## 部署和构建

### 生产构建

```bash
# 构建生产版本
pnpm build

# 预览构建结果
pnpm preview

# 生成静态站点
pnpm generate
```

### 环境变量配置

```bash
# .env.production
BROWSER_BASE_URL=https://api.bbsmc.net/v2/
SITE_URL=https://bbsmc.net
NODE_ENV=production
```

### 性能监控

```javascript
// 添加性能监控
export default defineNuxtPlugin(() => {
  if (process.client) {
    // Web Vitals 监控
    import('web-vitals').then(({ getCLS, getFID, getFCP, getLCP, getTTFB }) => {
      getCLS(console.log)
      getFID(console.log)
      getFCP(console.log)
      getLCP(console.log)
      getTTFB(console.log)
    })
  }
})
```

---

## 总结

BBSMC 前端应用是一个现代化的 Vue 3 + Nuxt 3 应用程序，具有以下特点：

- **现代化技术栈**：Vue 3 组合式 API + Nuxt 3 + TypeScript
- **完整的组件系统**：可重用的 UI 组件库
- **强大的状态管理**：基于组合式 API 的状态管理
- **灵活的路由系统**：基于文件的路由和动态路由
- **丰富的主题系统**：支持多主题切换
- **完善的国际化**：多语言支持
- **性能优化**：懒加载、缓存、虚拟滚动等
- **开发工具**：完整的开发、测试、构建工具链

本文档涵盖了前端开发的方方面面，从基础概念到高级特性，从开发规范到性能优化。开发者可以根据需要查阅相应章节，快速上手和深入开发 BBSMC 前端应用。

## 更新日志

- **2024-08-14**: 初始版本创建，包含完整的前端架构分析和开发指引
- **版本**: 基于当前代码库状态分析生成

---

*本文档基于 BBSMC 前端代码库深度分析生成，为 AI 程序员提供全面的开发参考。*