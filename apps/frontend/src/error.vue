<template>
  <NuxtLayout>
    <div class="error-page">
      <div class="error-content">
        <!-- Error Icon -->
        <div class="error-icon" :class="{ 'is-404': is404 }">
          <template v-if="is404">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9 5.25h.008v.008H12v-.008z"
              />
            </svg>
          </template>
          <template v-else>
            <!-- Sad face for server errors -->
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M15.182 16.318A4.486 4.486 0 0012.016 15a4.486 4.486 0 00-3.198 1.318M21 12a9 9 0 11-18 0 9 9 0 0118 0zM9.75 9.75c0 .414-.168.75-.375.75S9 10.164 9 9.75 9.168 9 9.375 9s.375.336.375.75zm-.375 0h.008v.015h-.008V9.75zm5.625 0c0 .414-.168.75-.375.75s-.375-.336-.375-.75.168-.75.375-.75.375.336.375.75zm-.375 0h.008v.015h-.008V9.75z"
              />
            </svg>
          </template>
        </div>

        <!-- Error Title -->
        <h1 class="error-title">{{ errorContent.title }}</h1>

        <!-- Error Subtitle -->
        <p class="error-subtitle">{{ errorContent.subtitle }}</p>

        <!-- Help List -->
        <div v-if="errorContent.listTitle" class="help-section">
          <p class="help-title">{{ errorContent.listTitle }}</p>
          <ul class="help-list">
            <li v-for="(item, index) in errorContent.listItems" :key="index">
              <component :is="item.component" v-if="item.component" />
              <span v-else>{{ item.text }}</span>
            </li>
          </ul>
        </div>

        <!-- Error Details (for non-404 errors) -->
        <div v-if="!is404" class="error-details">
          <span class="error-code">Error {{ error.statusCode }}</span>
          <span
            v-if="error.message && error.message !== errorContent.subtitle"
            class="error-message-detail"
          >
            {{ error.message }}
          </span>
        </div>

        <!-- Action Buttons -->
        <div class="error-actions">
          <NuxtLink to="/" class="btn-primary">
            <HomeIcon />
            返回首页
          </NuxtLink>
          <button class="btn-secondary" @click="handleError">
            <UndoIcon />
            重试
          </button>
        </div>

        <!-- Quick Links -->
        <div class="quick-links">
          <p class="quick-links-title">或者探索这些内容：</p>
          <div class="links-grid">
            <NuxtLink to="/mods" class="quick-link">
              <span class="link-icon"><BoxIcon /></span>
              <span>模组</span>
            </NuxtLink>
            <NuxtLink to="/modpacks" class="quick-link">
              <span class="link-icon"><PackageClosedIcon /></span>
              <span>整合包</span>
            </NuxtLink>
            <NuxtLink to="/shaders" class="quick-link">
              <span class="link-icon"><GlassesIcon /></span>
              <span>光影</span>
            </NuxtLink>
            <NuxtLink to="/languages" class="quick-link">
              <span class="link-icon"><LanguagesIcon /></span>
              <span>整合包汉化</span>
            </NuxtLink>
          </div>
        </div>
      </div>
    </div>
  </NuxtLayout>
</template>

<script setup>
import { h } from "vue";
import {
  BoxIcon,
  PackageClosedIcon,
  GlassesIcon,
  LanguagesIcon,
  HomeIcon,
  UndoIcon,
} from "@modrinth/assets";
import { addNotification } from "~/composables/notifs.js";

const props = defineProps({
  error: {
    type: Object,
    default() {
      return {
        statusCode: 404,
        message: "页面不存在",
      };
    },
  },
});

const route = useRoute();

const is404 = computed(() => props.error.statusCode === 404);

// 项目路径前缀
const PROJECT_PATH_PREFIXES = [
  "/mod/",
  "/datapack/",
  "/resourcepack/",
  "/plugin/",
  "/shader/",
  "/modpack/",
  "/project/",
];

// 检查是否是项目页面
const isProjectPage = computed(() =>
  PROJECT_PATH_PREFIXES.some((prefix) => route.path.startsWith(prefix)),
);

// 检查是否是用户页面
const isUserPage = computed(() => route.path.startsWith("/user/"));

// QQ群链接组件
const QQGroupLink = () =>
  h("span", {}, [
    "如果问题持续存在，欢迎加入我们的 ",
    h(
      "a",
      {
        href: "https://qm.qq.com/cgi-bin/qm/qr?k=YOUR_KEY&jump_from=webapi",
        target: "_blank",
        rel: "noopener",
        class: "help-link",
        onClick: (e) => {
          e.preventDefault();
          copyQQGroup();
        },
      },
      "QQ 群 (1078515449)",
    ),
    " 反馈问题。",
  ]);

// 复制QQ群号
const copyQQGroup = () => {
  navigator.clipboard.writeText("1078515449").then(() => {
    addNotification({
      group: "main",
      title: "已复制",
      text: "QQ 群号 1078515449 已复制到剪贴板",
      type: "success",
    });
  });
};

// 错误消息配置
const errorMessages = {
  // 404 错误
  404: {
    title: "页面走丢了",
    subtitle: "抱歉，您访问的页面不存在或已被移除。",
  },
  // 项目 404
  project404: {
    title: "项目不存在",
    subtitle: "找不到您要访问的项目。",
    listTitle: "可能的原因：",
    listItems: [
      { text: "您可能输入了错误的项目地址。" },
      { text: "项目作者可能已更改地址、设为私密或删除了项目。" },
      { text: "项目可能因违反社区规则被超级管理员移除。" },
    ],
  },
  // 用户 404
  user404: {
    title: "用户不存在",
    subtitle: "找不到您要访问的用户。",
    listTitle: "可能的原因：",
    listItems: [
      { text: "您可能输入了错误的用户名。" },
      { text: "该用户可能已更改用户名或注销账号。" },
    ],
  },
  // 429 请求过多
  429: {
    title: "请求过于频繁",
    subtitle: "您的请求速度过快，请稍后再试。",
    listTitle: "建议：",
    listItems: [{ text: "请等待几秒钟后再次尝试。" }, { text: "避免在短时间内频繁刷新页面。" }],
  },
  // 451 法律原因
  451: {
    title: "内容不可用",
    subtitle: "由于法律原因，此内容暂时无法访问。",
  },
  // 500 服务器错误
  500: {
    title: "糟糕！",
    subtitle: "服务器出错了。",
    listTitle: "请稍后再试：",
    listItems: [{ text: "服务器可能正在维护或遇到临时问题。" }, { component: QQGroupLink }],
  },
  // 502 网关错误
  502: {
    title: "糟糕！",
    subtitle: "服务器网关出错了。",
    listTitle: "请稍后再试：",
    listItems: [{ text: "服务器可能正在重启或部署更新。" }, { component: QQGroupLink }],
  },
  // 503 服务不可用
  503: {
    title: "服务暂时不可用",
    subtitle: "服务器正在维护中，请稍后再试。",
    listTitle: "可能的情况：",
    listItems: [
      { text: "服务器正在进行计划维护。" },
      { text: "服务器负载过高，暂时无法响应请求。" },
      { component: QQGroupLink },
    ],
  },
  // 504 网关超时
  504: {
    title: "请求超时",
    subtitle: "服务器响应时间过长。",
    listTitle: "请稍后再试：",
    listItems: [{ text: "服务器可能正忙，请稍后重试。" }, { component: QQGroupLink }],
  },
  // 默认错误
  default: {
    title: "糟糕！",
    subtitle: "出错了。",
    listTitle: "请在几分钟后再试：",
    listItems: [{ text: "如果您在执行某个操作，请稍后重试。" }, { component: QQGroupLink }],
  },
};

// 获取错误内容
const errorContent = computed(() => {
  const code = props.error.statusCode;

  // 特殊路由处理
  if (code === 404) {
    if (isProjectPage.value) {
      return errorMessages.project404;
    }
    if (isUserPage.value) {
      return errorMessages.user404;
    }
    return errorMessages[404];
  }

  // 其他错误码
  return errorMessages[code] || errorMessages.default;
});

const handleError = () => clearError({ redirect: route.fullPath });
</script>

<style lang="scss" scoped>
.error-page {
  min-height: calc(100vh - 200px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}

.error-content {
  text-align: center;
  max-width: 520px;
  width: 100%;
}

.error-icon {
  width: 80px;
  height: 80px;
  margin: 0 auto 24px;
  padding: 20px;
  background: var(--color-red-bg, rgba(239, 68, 68, 0.1));
  border-radius: 50%;

  svg {
    width: 100%;
    height: 100%;
    color: var(--color-red, #ef4444);
  }

  &.is-404 {
    background: var(--color-orange-bg, rgba(241, 100, 54, 0.1));

    svg {
      color: var(--color-orange, #f16436);
    }
  }
}

.error-title {
  font-size: 32px;
  font-weight: 700;
  color: var(--color-contrast);
  margin: 0 0 12px;
}

.error-subtitle {
  font-size: 16px;
  color: var(--color-secondary);
  margin: 0 0 24px;
  line-height: 1.6;
}

.help-section {
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: 12px;
  padding: 20px 24px;
  margin-bottom: 24px;
  text-align: left;
}

.help-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-contrast);
  margin: 0 0 12px;
}

.help-list {
  margin: 0;
  padding: 0 0 0 20px;
  list-style: disc;

  li {
    font-size: 14px;
    color: var(--color-secondary);
    line-height: 1.6;
    margin-bottom: 8px;

    &:last-child {
      margin-bottom: 0;
    }
  }
}

:deep(.help-link) {
  color: var(--color-brand);
  text-decoration: none;
  font-weight: 500;

  &:hover {
    text-decoration: underline;
  }
}

.error-details {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 24px;
  font-size: 13px;
  color: var(--color-secondary);

  .error-code {
    background: var(--color-red-bg, rgba(239, 68, 68, 0.1));
    color: var(--color-red, #ef4444);
    padding: 4px 10px;
    border-radius: 6px;
    font-weight: 600;
    font-family: monospace;
  }

  .error-message-detail {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.error-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
  margin-bottom: 48px;
}

.btn-primary,
.btn-secondary {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 24px;
  border-radius: 12px;
  font-size: 15px;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.2s ease;
  cursor: pointer;

  svg {
    width: 18px;
    height: 18px;
  }
}

.btn-primary {
  background: var(--color-brand);
  color: white;
  border: none;
  box-shadow: 0 4px 12px var(--color-brand-shadow);

  &:hover {
    filter: brightness(1.1);
    transform: translateY(-2px);
    box-shadow: 0 6px 20px var(--color-brand-shadow);
  }
}

.btn-secondary {
  background: var(--color-button-bg);
  color: var(--color-text);
  border: 1px solid var(--color-divider);

  &:hover {
    background: var(--color-raised-bg);
    border-color: var(--color-brand);
    color: var(--color-brand);
  }
}

.quick-links {
  padding-top: 32px;
  border-top: 1px solid var(--color-divider);
}

.quick-links-title {
  font-size: 14px;
  color: var(--color-secondary);
  margin: 0 0 16px;
}

.links-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;

  @media (max-width: 480px) {
    grid-template-columns: repeat(2, 1fr);
  }
}

.quick-link {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 16px 12px;
  background: var(--color-button-bg);
  border: 1px solid var(--color-divider);
  border-radius: 12px;
  text-decoration: none;
  color: var(--color-text);
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s ease;

  .link-icon {
    display: flex;
    align-items: center;
    justify-content: center;

    svg {
      width: 1.5rem;
      height: 1.5rem;
      color: var(--color-brand);
    }
  }

  &:hover {
    background: var(--color-brand-highlight);
    border-color: var(--color-brand);
    color: var(--color-brand);
    transform: translateY(-2px);

    .link-icon svg {
      transform: scale(1.1);
    }
  }
}
</style>
