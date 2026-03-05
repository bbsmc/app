<template>
  <div>
    <section class="universal-card">
      <div class="header__row">
        <h2 class="header__title text-2xl">已购买资源</h2>
        <span class="purchase-count">共 {{ purchases.length }} 个</span>
      </div>

      <div v-if="loading" class="loading-container">
        <div class="loading-spinner" />
        <p>加载中...</p>
      </div>

      <template v-else-if="purchases.length === 0">
        <div class="empty-state">
          <CoinsIcon class="empty-icon" />
          <p class="empty-message">您还没有购买任何资源</p>
          <p class="empty-hint">浏览资源市场，发现优质内容</p>
        </div>
      </template>

      <template v-else>
        <div class="purchases-grid">
          <div v-for="purchase in purchases" :key="purchase.project_id" class="purchase-card">
            <nuxt-link
              :to="purchase.project_slug ? `/mod/${purchase.project_slug}` : '#'"
              class="card-link"
            >
              <div class="card-header">
                <Avatar
                  :src="purchase.icon_url"
                  size="64px"
                  aria-hidden="true"
                  :alt="purchase.project_title"
                  class="project-icon"
                />
                <div class="card-title-section">
                  <h3 class="project-title">{{ purchase.project_title || "未知资源" }}</h3>
                  <span class="project-type">{{ formatProjectType(purchase.project_type) }}</span>
                </div>
                <div class="card-status">
                  <Badge v-if="purchase.is_active" type="approved"> 有效 </Badge>
                  <Badge v-else type="rejected"> 已过期 </Badge>
                </div>
              </div>

              <p class="project-description">
                {{ truncateDescription(purchase.project_description) }}
              </p>

              <div class="card-footer">
                <div class="purchase-info">
                  <div class="info-item">
                    <CoinsIcon class="info-icon" />
                    <span class="amount">¥{{ purchase.amount }}</span>
                  </div>
                  <div class="info-item">
                    <CalendarIcon class="info-icon" />
                    <span>{{ formatDate(purchase.purchased_at) }}</span>
                  </div>
                </div>
                <div class="expire-info">
                  <template v-if="purchase.expires_at">
                    <span :class="['expire-date', { expired: isExpired(purchase.expires_at) }]">
                      {{ isExpired(purchase.expires_at) ? "已于" : "有效期至" }}
                      {{ formatDate(purchase.expires_at) }}
                      {{ isExpired(purchase.expires_at) ? "过期" : "" }}
                    </span>
                  </template>
                  <span v-else class="permanent">
                    <CheckIcon class="info-icon" />
                    永久有效
                  </span>
                </div>
              </div>
            </nuxt-link>
          </div>
        </div>
      </template>
    </section>
  </div>
</template>

<script setup>
import Avatar from "~/components/ui/Avatar.vue";
import Badge from "~/components/ui/Badge.vue";
import CoinsIcon from "~/assets/images/utils/coins.svg?component";
import CalendarIcon from "~/assets/images/utils/calendar.svg?component";
import CheckIcon from "~/assets/images/utils/check.svg?component";

definePageMeta({
  middleware: "auth",
});

const loading = ref(true);
const purchases = ref([]);

const formatDate = (dateString) => {
  if (!dateString) return "-";
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
};

const isExpired = (dateString) => {
  if (!dateString) return false;
  return new Date(dateString) < new Date();
};

const truncateDescription = (desc) => {
  if (!desc) return "暂无描述";
  return desc.length > 80 ? desc.substring(0, 80) + "..." : desc;
};

const formatProjectType = (type) => {
  const typeMap = {
    mod: "模组",
    modpack: "整合包",
    resourcepack: "资源包",
    shader: "光影",
    datapack: "数据包",
    plugin: "插件",
  };
  return typeMap[type] || type || "资源";
};

onMounted(async () => {
  try {
    const data = await useBaseFetch("user/purchases", { apiVersion: 3 });
    purchases.value = data.purchases || [];
  } catch (error) {
    console.error("获取购买记录失败:", error);
  } finally {
    loading.value = false;
  }
});

useHead({
  title: "已购买资源 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});
</script>

<style lang="scss" scoped>
.header__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-card-lg);
}

.purchase-count {
  color: var(--color-text-secondary);
  font-size: 0.9rem;
}

.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 3rem;
  gap: 1rem;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--color-divider);
  border-top-color: var(--color-brand);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 4rem 2rem;
  text-align: center;
}

.empty-icon {
  width: 64px;
  height: 64px;
  color: var(--color-text-secondary);
  opacity: 0.5;
  margin-bottom: 1rem;
}

.empty-message {
  color: var(--color-text);
  font-size: 1.1rem;
  font-weight: 500;
  margin-bottom: 0.5rem;
}

.empty-hint {
  color: var(--color-text-secondary);
  font-size: 0.9rem;
}

.purchases-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: var(--spacing-card-md);

  @media screen and (max-width: 480px) {
    grid-template-columns: 1fr;
  }
}

.purchase-card {
  background: var(--color-raised-bg);
  border-radius: var(--size-rounded-card);
  border: 1px solid var(--color-divider);
  overflow: hidden;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease,
    border-color 0.2s ease;

  &:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-card);
    border-color: var(--color-brand);
  }
}

.card-link {
  display: block;
  padding: var(--spacing-card-md);
  color: inherit;
  text-decoration: none;
}

.card-header {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-card-sm);
  margin-bottom: var(--spacing-card-sm);
}

.project-icon {
  flex-shrink: 0;
  border-radius: var(--size-rounded-sm);
}

.card-title-section {
  flex: 1;
  min-width: 0;
}

.project-title {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--color-text);
  margin: 0 0 4px 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-type {
  font-size: 0.8rem;
  color: var(--color-text-secondary);
  background: var(--color-button-bg);
  padding: 2px 8px;
  border-radius: var(--size-rounded-sm);
}

.card-status {
  flex-shrink: 0;
}

.project-description {
  font-size: 0.9rem;
  color: var(--color-text-secondary);
  line-height: 1.5;
  margin: 0 0 var(--spacing-card-md) 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-footer {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-card-xs);
  padding-top: var(--spacing-card-sm);
  border-top: 1px solid var(--color-divider);
}

.purchase-info {
  display: flex;
  align-items: center;
  gap: var(--spacing-card-md);
  flex-wrap: wrap;
}

.info-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.85rem;
  color: var(--color-text-secondary);
}

.info-icon {
  width: 16px;
  height: 16px;
  color: var(--color-text-secondary);
}

.amount {
  font-weight: 600;
  color: var(--color-brand);
}

.expire-info {
  font-size: 0.85rem;
}

.expire-date {
  color: var(--color-text-secondary);

  &.expired {
    color: var(--color-red);
  }
}

.permanent {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--color-green);
  font-weight: 500;

  .info-icon {
    color: var(--color-green);
  }
}
</style>
