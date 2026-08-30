<template>
  <div class="panel">
    <div class="filter-bar">
      <Chips v-model="rangeKey" :items="rangeOptions" :format-label="formatRangeLabel" />
      <div class="search-box">
        <input
          v-model="searchInput"
          type="text"
          placeholder="按资源名称或 slug 搜索…"
          @keyup.enter="applySearch"
        />
        <button class="btn btn-secondary" @click="applySearch">搜索</button>
        <button v-if="appliedSearch" class="btn btn-transparent" @click="clearSearch">
          清除
        </button>
      </div>
    </div>

    <div class="summary-bar">
      <span>
        共
        <b>{{ formatNumber(data?.total_count ?? 0, false) }}</b>
        个资源参与排行
      </span>
      <span>
        总下载
        <b>{{ formatNumber(data?.total_downloads ?? 0, false) }}</b>
      </span>
    </div>

    <div v-if="loading" class="loading-section">
      <UpdatedIcon aria-hidden="true" class="animate-spin" />
      <span>加载中...</span>
    </div>
    <div v-else-if="error" class="empty-section">加载失败：{{ error }}</div>
    <div v-else-if="!data?.items?.length" class="empty-section">
      所选区间内暂无下载记录
    </div>
    <div v-else class="ranking-list">
      <div
        v-for="item in data.items"
        :key="item.project_id"
        class="ranking-row"
      >
        <div class="rank">#{{ item.rank }}</div>
        <div class="project-icon">
          <img
            :src="item.icon_url || 'https://cdn.bbsmc.net/raw/placeholder-banner.svg'"
            :alt="item.name"
          />
        </div>
        <div class="project-info">
          <nuxt-link :to="`/project/${item.slug || item.project_id}`" class="project-name">
            {{ item.name }}
          </nuxt-link>
          <div v-if="item.slug" class="project-slug">{{ item.slug }}</div>
        </div>
        <div class="downloads-cell">
          <div class="downloads-value">{{ formatNumber(item.downloads, false) }}</div>
          <div class="percentage-bar">
            <span :style="{ width: percentage(item.downloads) }"></span>
          </div>
        </div>
      </div>
    </div>

    <div v-if="data && data.total_count > pageSize" class="pagination">
      <button
        class="btn btn-secondary"
        :disabled="page <= 1 || loading"
        @click="page = Math.max(1, page - 1)"
      >
        上一页
      </button>
      <span class="page-indicator">第 {{ page }} / {{ totalPages }} 页</span>
      <button
        class="btn btn-secondary"
        :disabled="page >= totalPages || loading"
        @click="page = Math.min(totalPages, page + 1)"
      >
        下一页
      </button>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import { formatNumber } from "~/plugins/shorthands.js";
import Chips from "~/components/ui/Chips.vue";
import { UpdatedIcon } from "@modrinth/assets";

const rangeKey = ref("7d");
const rangeOptions = ["24h", "7d", "30d", "90d", "365d"];
const formatRangeLabel = (v) => {
  const map = {
    "24h": "近 24 小时",
    "7d": "近 7 天",
    "30d": "近 30 天",
    "90d": "近 90 天",
    "365d": "近 1 年",
  };
  return map[v] || v;
};

const rangeToHours = (key) => {
  const map = { "24h": 24, "7d": 168, "30d": 720, "90d": 2160, "365d": 8760 };
  return map[key] || 168;
};

const pageSize = 20;
const page = ref(1);
const searchInput = ref("");
const appliedSearch = ref("");

const queryParams = computed(() => {
  const endDate = new Date();
  const startDate = new Date(endDate.getTime() - rangeToHours(rangeKey.value) * 3600 * 1000);
  const params = {
    start_date: startDate.toISOString(),
    end_date: endDate.toISOString(),
    limit: pageSize,
    offset: (page.value - 1) * pageSize,
  };
  if (appliedSearch.value) params.search = appliedSearch.value;
  return params;
});

const data = ref(null);
const loading = ref(false);
const error = ref(null);

const fetchData = async () => {
  loading.value = true;
  error.value = null;
  try {
    data.value = await useBaseFetch("moderation/analytics/top-projects", {
      method: "GET",
      internal: true,
      params: queryParams.value,
    });
  } catch (e) {
    error.value = e?.data?.description || e?.message || "未知错误";
    data.value = null;
  } finally {
    loading.value = false;
  }
};

// 顺序敏感：先重置 page，再监听 queryParams 拉数据
// 否则切换筛选时会先用旧 page 拉一次、page 重置后再拉一次
watch([rangeKey, appliedSearch], () => {
  page.value = 1;
});

watch(queryParams, fetchData, { immediate: true });

const applySearch = () => {
  appliedSearch.value = searchInput.value.trim();
};
const clearSearch = () => {
  searchInput.value = "";
  appliedSearch.value = "";
};

const totalPages = computed(() =>
  Math.max(1, Math.ceil((data.value?.total_count ?? 0) / pageSize)),
);

const maxDownloadsInPage = computed(() => {
  const items = data.value?.items || [];
  return items.reduce((acc, it) => Math.max(acc, Number(it.downloads)), 0);
});

const percentage = (value) => {
  const max = maxDownloadsInPage.value;
  if (!max) return "0%";
  return `${(Number(value) / max) * 100}%`;
};
</script>

<style scoped lang="scss">
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--gap-lg);
}

.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gap-md);
  align-items: center;
}

.search-box {
  display: flex;
  gap: var(--gap-sm);
  align-items: center;

  input {
    min-width: 240px;
  }
}

.summary-bar {
  display: flex;
  gap: var(--gap-lg);
  color: var(--color-secondary);
  font-size: var(--font-size-sm);
}

.ranking-list {
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-button-bg);
  overflow: hidden;
}

.ranking-row {
  display: grid;
  grid-template-columns: 4rem 3rem 1fr 14rem;
  align-items: center;
  gap: var(--gap-md);
  padding: var(--gap-md);
  border-bottom: 1px solid var(--color-button-bg);

  &:last-child {
    border-bottom: none;
  }

  .rank {
    font-weight: var(--font-weight-bold);
    color: var(--color-secondary);
  }

  .project-icon img {
    width: 3rem;
    height: 3rem;
    border-radius: var(--radius-sm);
    object-fit: cover;
  }

  .project-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .project-name {
    font-weight: var(--font-weight-bold);
    color: var(--color-contrast);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-slug {
    color: var(--color-secondary);
    font-size: var(--font-size-xs);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .downloads-cell {
    display: flex;
    flex-direction: column;
    gap: var(--gap-xs);
    align-items: stretch;

    .downloads-value {
      font-weight: var(--font-weight-bold);
      text-align: right;
    }

    .percentage-bar {
      width: 100%;
      height: 0.5rem;
      background-color: var(--color-raised-bg);
      border-radius: var(--radius-sm);
      overflow: hidden;

      span {
        display: block;
        height: 100%;
        background-color: var(--color-brand);
      }
    }
  }
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: var(--gap-md);

  .page-indicator {
    color: var(--color-secondary);
  }
}

.loading-section,
.empty-section {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--gap-sm);
  padding: var(--gap-xl);
  color: var(--color-secondary);
}

@media (max-width: 768px) {
  .ranking-row {
    grid-template-columns: 3rem 2.5rem 1fr 8rem;
    gap: var(--gap-sm);
  }

  .search-box input {
    min-width: 0;
    width: 100%;
  }
}
</style>
