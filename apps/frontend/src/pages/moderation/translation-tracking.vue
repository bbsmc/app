<template>
  <div>
    <section class="universal-card">
      <div class="header-section">
        <h2>汉化追踪监控</h2>
        <p class="description">监控所有开启汉化追踪的整合包，检查最新版本是否已绑定汉化</p>
      </div>

      <!-- 状态栏 -->
      <div class="status-bar">
        <div class="auto-refresh">
          <label class="refresh-toggle">
            <input v-model="autoRefresh" type="checkbox" />
            <span>自动刷新（每分钟）</span>
          </label>
          <span v-if="autoRefresh && nextRefreshIn > 0" class="next-refresh">
            {{ nextRefreshIn }}秒后刷新
          </span>
        </div>
        <div class="status-info">
          <span class="total-count">共 {{ items.length }} 个追踪项目</span>
          <span class="missing-count" :class="{ 'has-missing': missingCount > 0 }">
            {{ missingCount }} 个缺少汉化
          </span>
          <button class="refresh-btn" :disabled="loading" @click="fetchData">
            <UpdatedIcon aria-hidden="true" :class="{ 'animate-spin': loading }" />
            刷新
          </button>
        </div>
      </div>

      <!-- 加载中 -->
      <div v-if="loading && items.length === 0" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>

      <!-- 项目列表 -->
      <div v-else-if="items.length > 0" class="items-container">
        <div class="items-list">
          <div
            v-for="item in sortedItems"
            :key="item.project_id"
            class="tracking-item"
            :class="{
              'has-translation': item.has_approved_translation,
              'missing-translation': !item.has_approved_translation && item.latest_version_id,
            }"
          >
            <div class="item-main">
              <!-- 项目信息 -->
              <div class="project-info">
                <nuxt-link :to="`/modpack/${item.project_slug}`" class="project-link">
                  <Avatar :src="item.project_icon" :alt="item.project_name" size="md" />
                  <div class="project-details">
                    <span class="project-name">{{ item.project_name }}</span>
                    <span class="project-slug">{{ item.project_slug }}</span>
                  </div>
                </nuxt-link>
              </div>

              <!-- 版本信息 -->
              <div class="version-info">
                <template v-if="item.latest_version_id">
                  <div class="version-badge">
                    <VersionIcon aria-hidden="true" />
                    <span>{{ item.latest_version_number }}</span>
                  </div>
                  <div class="publish-time">
                    <CalendarIcon aria-hidden="true" />
                    <span>{{ formatPublishTime(item.latest_version_published) }}</span>
                  </div>
                </template>
                <span v-else class="no-version">暂无版本</span>
              </div>

              <!-- 汉化状态 -->
              <div class="translation-status">
                <template v-if="item.has_approved_translation">
                  <div class="status-badge status-ok">
                    <CheckIcon aria-hidden="true" />
                    已绑定汉化
                  </div>
                  <div class="translation-version">
                    <span class="label">汉化版本：</span>
                    <nuxt-link
                      v-if="item.translation_pack_slug"
                      :to="`/resourcepack/${item.translation_pack_slug}/version/${item.approved_translation_version_number}`"
                      class="version-link"
                    >
                      {{ item.approved_translation_version_number }}
                    </nuxt-link>
                  </div>
                </template>
                <template v-else-if="item.latest_version_id">
                  <div class="status-badge status-missing">
                    <XIcon aria-hidden="true" />
                    缺少汉化
                  </div>
                  <div class="overtime-info">
                    <span class="overtime-label">已超时：</span>
                    <span
                      class="overtime-value"
                      :class="getOvertimeClass(item.seconds_since_published)"
                    >
                      {{ formatDuration(item.seconds_since_published) }}
                    </span>
                  </div>
                </template>
                <template v-else>
                  <div class="status-badge status-no-version">
                    <InfoIcon aria-hidden="true" />
                    无版本
                  </div>
                </template>
              </div>

              <!-- 操作 -->
              <div class="item-actions">
                <nuxt-link
                  v-if="item.latest_version_id"
                  :to="`/modpack/${item.project_slug}/version/${item.latest_version_number}`"
                  class="btn btn-secondary"
                >
                  <EyeIcon aria-hidden="true" />
                  查看版本
                </nuxt-link>
                <nuxt-link
                  v-if="item.translation_pack_slug"
                  :to="`/resourcepack/${item.translation_pack_slug}`"
                  class="btn btn-primary"
                >
                  <LanguagesIcon aria-hidden="true" />
                  汉化包
                </nuxt-link>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>暂无开启汉化追踪的项目</p>
      </div>

      <!-- 底部信息 -->
      <div v-if="queriedAt" class="footer-info">
        <span>最后查询时间：{{ formatDateTime(queriedAt) }}</span>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useAuth } from "~/composables/auth.js";
import { useBaseFetch } from "~/composables/fetch.js";
import { addNotification } from "~/composables/notifs.js";
import Avatar from "~/components/ui/Avatar.vue";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import EyeIcon from "~/assets/images/utils/eye.svg?component";
import VersionIcon from "~/assets/images/utils/version.svg?component";
import CalendarIcon from "~/assets/images/utils/calendar.svg?component";
import LanguagesIcon from "~/assets/images/utils/languages.svg?component";
import XIcon from "~/assets/images/utils/x.svg?component";

const router = useRouter();
const auth = await useAuth();
const app = useNuxtApp();

// 页面标题
useHead({
  title: "汉化追踪监控 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

// 响应式状态
const loading = ref(false);
const items = ref([]);
const queriedAt = ref(null);
const autoRefresh = ref(true);
const nextRefreshIn = ref(60);

let refreshInterval = null;
let countdownInterval = null;

// 计算属性
const missingCount = computed(() => {
  return items.value.filter((item) => !item.has_approved_translation && item.latest_version_id)
    .length;
});

// 按超时时间排序（缺少汉化的排前面，按超时时间从长到短）
const sortedItems = computed(() => {
  return [...items.value].sort((a, b) => {
    // 首先按是否有汉化排序
    if (a.has_approved_translation !== b.has_approved_translation) {
      return a.has_approved_translation ? 1 : -1;
    }
    // 然后按超时时间排序（无版本的排最后）
    const aTime = a.seconds_since_published || 0;
    const bTime = b.seconds_since_published || 0;
    return bTime - aTime;
  });
});

// 格式化发布时间
const formatPublishTime = (date) => {
  if (!date) return "-";
  return app.$dayjs(date).format("YYYY-MM-DD HH:mm");
};

// 格式化日期时间
const formatDateTime = (date) => {
  if (!date) return "-";
  return app.$dayjs(date).format("YYYY-MM-DD HH:mm:ss");
};

// 格式化持续时间
const formatDuration = (seconds) => {
  if (!seconds || seconds <= 0) return "-";

  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}天${hours}小时`;
  } else if (hours > 0) {
    return `${hours}小时${minutes}分钟`;
  } else {
    return `${minutes}分钟`;
  }
};

// 获取超时等级样式
const getOvertimeClass = (seconds) => {
  if (!seconds) return "";
  const hours = seconds / 3600;
  if (hours >= 24) return "overtime-critical"; // 超过24小时
  if (hours >= 6) return "overtime-warning"; // 超过6小时
  return "overtime-normal"; // 6小时内
};

// 获取数据
const fetchData = async () => {
  loading.value = true;
  try {
    const response = await useBaseFetch("moderation/translation-tracking-status", {
      method: "GET",
      internal: true,
    });

    if (response) {
      items.value = response.items || [];
      queriedAt.value = response.queried_at;
    }
  } catch (error) {
    console.error("获取汉化追踪状态失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "获取汉化追踪状态失败",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

// 启动自动刷新
const startAutoRefresh = () => {
  stopAutoRefresh();

  // 每分钟刷新一次
  refreshInterval = setInterval(() => {
    fetchData();
    nextRefreshIn.value = 60;
  }, 60000);

  // 倒计时
  countdownInterval = setInterval(() => {
    if (nextRefreshIn.value > 0) {
      nextRefreshIn.value--;
    }
  }, 1000);
};

// 停止自动刷新
const stopAutoRefresh = () => {
  if (refreshInterval) {
    clearInterval(refreshInterval);
    refreshInterval = null;
  }
  if (countdownInterval) {
    clearInterval(countdownInterval);
    countdownInterval = null;
  }
};

// 监听自动刷新开关
watch(autoRefresh, (newVal) => {
  if (newVal) {
    nextRefreshIn.value = 60;
    startAutoRefresh();
  } else {
    stopAutoRefresh();
  }
});

// 检查权限
onMounted(() => {
  if (
    !auth.value?.user ||
    (auth.value.user.role !== "admin" && auth.value.user.role !== "moderator")
  ) {
    router.push("/");
    addNotification({
      group: "main",
      title: "权限不足",
      text: "您没有权限访问此页面",
      type: "error",
    });
  } else {
    fetchData();
    if (autoRefresh.value) {
      startAutoRefresh();
    }
  }
});

onUnmounted(() => {
  stopAutoRefresh();
});
</script>

<style scoped lang="scss">
.header-section {
  margin-bottom: 1.5rem;

  h2 {
    margin: 0 0 0.5rem 0;
    color: var(--color-heading);
  }

  .description {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: 0.95rem;
  }
}

.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
  margin-bottom: 1.5rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-divider);
  flex-wrap: wrap;
  gap: 1rem;
}

.auto-refresh {
  display: flex;
  align-items: center;
  gap: 1rem;

  .refresh-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;

    input {
      width: 1rem;
      height: 1rem;
      cursor: pointer;
    }

    span {
      font-size: 0.9rem;
    }
  }

  .next-refresh {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
}

.status-info {
  display: flex;
  align-items: center;
  gap: 1rem;

  .total-count {
    font-size: 0.9rem;
    color: var(--color-text-secondary);
  }

  .missing-count {
    font-size: 0.9rem;
    color: var(--color-text-secondary);
    padding: 0.25rem 0.75rem;
    background: var(--color-bg);
    border-radius: var(--radius-md);

    &.has-missing {
      background: rgba(239, 68, 68, 0.15);
      color: rgb(185, 28, 28);
      font-weight: 600;
    }
  }
}

.refresh-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: var(--color-button-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

.loading-section,
.empty-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem;
  gap: 1rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
  }

  p {
    margin: 0;
    font-size: 0.95rem;
  }
}

.items-container {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.items-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.tracking-item {
  padding: 1rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 2px solid var(--color-divider);
  transition: all 0.2s;

  &.has-translation {
    border-color: rgba(34, 197, 94, 0.3);
    background: rgba(34, 197, 94, 0.05);
  }

  &.missing-translation {
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.05);
  }

  &:hover {
    border-color: var(--color-primary);
  }
}

.item-main {
  display: grid;
  grid-template-columns: 1fr auto auto auto;
  align-items: center;
  gap: 1.5rem;

  @media (max-width: 900px) {
    grid-template-columns: 1fr;
    gap: 1rem;
  }
}

.project-info {
  .project-link {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    text-decoration: none;
    color: var(--color-text);

    &:hover .project-name {
      color: var(--color-primary);
    }
  }

  .project-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    .project-name {
      font-weight: 600;
      font-size: 1rem;
      transition: color 0.2s;
    }

    .project-slug {
      font-size: 0.85rem;
      color: var(--color-text-secondary);
    }
  }
}

.version-info {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 150px;

  .version-badge,
  .publish-time {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;

    svg {
      width: 1rem;
      height: 1rem;
      color: var(--color-text-secondary);
    }
  }

  .no-version {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
}

.translation-status {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 180px;
}

.status-badge {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  font-weight: 600;
  width: fit-content;

  svg {
    width: 1rem;
    height: 1rem;
  }

  &.status-ok {
    background: rgba(34, 197, 94, 0.15);
    color: rgb(21, 128, 61);
    border: 1px solid rgba(34, 197, 94, 0.3);
  }

  &.status-missing {
    background: rgba(239, 68, 68, 0.15);
    color: rgb(185, 28, 28);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  &.status-no-version {
    background: rgba(156, 163, 175, 0.15);
    color: rgb(107, 114, 128);
    border: 1px solid rgba(156, 163, 175, 0.3);
  }
}

.translation-version {
  font-size: 0.85rem;
  color: var(--color-text-secondary);

  .label {
    margin-right: 0.25rem;
  }

  .version-link {
    color: var(--color-primary);
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }
}

.overtime-info {
  font-size: 0.9rem;

  .overtime-label {
    color: var(--color-text-secondary);
  }

  .overtime-value {
    font-weight: 600;

    &.overtime-normal {
      color: rgb(217, 119, 6);
    }

    &.overtime-warning {
      color: rgb(234, 88, 12);
    }

    &.overtime-critical {
      color: rgb(185, 28, 28);
    }
  }
}

.item-actions {
  display: flex;
  gap: 0.5rem;
}

.btn {
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  border: none;
  font-weight: 500;
  font-size: 0.85rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  text-decoration: none;
  transition: all 0.2s;

  &.btn-primary {
    background: var(--color-primary);
    color: white;

    &:hover {
      background: var(--color-primary-dark);
    }
  }

  &.btn-secondary {
    background: var(--color-button-bg);
    color: var(--color-text);

    &:hover {
      background: var(--color-raised-bg);
    }
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

.footer-info {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 0.85rem;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.animate-spin {
  animation: spin 1s linear infinite;
}
</style>
