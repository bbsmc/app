<template>
  <div class="panel">
    <div class="filter-bar">
      <Chips v-model="rangeKey" :items="rangeOptions" :format-label="formatRangeLabel" />
    </div>

    <div class="kpi-grid">
      <div class="kpi-card">
        <div class="label">区间内总下载</div>
        <div class="value">{{ formatNumber(data?.total ?? 0, false) }}</div>
      </div>
      <div class="kpi-card">
        <div class="label">日均下载</div>
        <div class="value">{{ formatNumber(averagePerDay, false) }}</div>
      </div>
      <div class="kpi-card">
        <div class="label">峰值</div>
        <div class="value">{{ formatNumber(peakValue, false) }}</div>
        <div v-if="peakLabel" class="hint">{{ peakLabel }}</div>
      </div>
    </div>

    <div class="chart-block">
      <div v-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>
      <div v-else-if="error" class="empty-section">
        加载失败：{{ error }}
      </div>
      <div v-else-if="!chartData[0]?.data?.length" class="empty-section">
        所选区间内暂无下载数据
      </div>
      <client-only v-else>
        <Chart
          name="下载数"
          type="area"
          :labels="chartLabels"
          :data="chartData"
          :colors="['var(--color-brand)']"
          :hide-legend="true"
          :hide-toolbar="true"
          :format-labels="formatXLabel"
        />
      </client-only>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";
import { formatNumber } from "~/plugins/shorthands.js";
import Chart from "~/components/ui/charts/Chart.client.vue";
import Chips from "~/components/ui/Chips.vue";
import { UpdatedIcon } from "@modrinth/assets";

dayjs.locale("zh-cn");

const rangeKey = ref("14d");

const rangeOptions = ["24h", "7d", "14d", "30d", "90d"];

const formatRangeLabel = (v) => {
  const map = {
    "24h": "近 24 小时",
    "7d": "近 7 天",
    "14d": "近 14 天",
    "30d": "近 30 天",
    "90d": "近 90 天",
  };
  return map[v] || v;
};

const rangeConfig = (key) => {
  const map = {
    "24h": { hours: 24, resolution: 60 },
    "7d": { hours: 168, resolution: 60 * 6 },
    "14d": { hours: 336, resolution: 60 * 24 },
    "30d": { hours: 720, resolution: 60 * 24 },
    "90d": { hours: 2160, resolution: 60 * 24 },
  };
  return map[key] || map["14d"];
};

const queryParams = computed(() => {
  const cfg = rangeConfig(rangeKey.value);
  const endDate = new Date();
  const startDate = new Date(endDate.getTime() - cfg.hours * 3600 * 1000);
  return {
    start_date: startDate.toISOString(),
    end_date: endDate.toISOString(),
    resolution_minutes: cfg.resolution,
  };
});

const data = ref(null);
const loading = ref(false);
const error = ref(null);

const fetchData = async () => {
  loading.value = true;
  error.value = null;
  try {
    data.value = await useBaseFetch("moderation/analytics/downloads", {
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

watch(queryParams, fetchData, { immediate: true });

const chartLabels = computed(() => (data.value?.points || []).map((p) => p.time * 1000));

const chartData = computed(() => [
  {
    name: "下载数",
    data: (data.value?.points || []).map((p) => ({ x: p.time * 1000, y: Number(p.total) })),
  },
]);

const averagePerDay = computed(() => {
  if (!data.value?.points?.length) return 0;
  const startMs = new Date(data.value.start_date).getTime();
  const endMs = new Date(data.value.end_date).getTime();
  const days = Math.max((endMs - startMs) / 86400000, 1);
  return Math.round(Number(data.value.total) / days);
});

const peakPoint = computed(() => {
  const pts = data.value?.points || [];
  if (!pts.length) return null;
  return pts.reduce((acc, p) => (Number(p.total) > Number(acc.total) ? p : acc), pts[0]);
});

const peakValue = computed(() => Number(peakPoint.value?.total ?? 0));

const peakLabel = computed(() => {
  if (!peakPoint.value) return "";
  const cfg = rangeConfig(rangeKey.value);
  const fmt = cfg.resolution < 60 * 24 ? "YYYY-MM-DD HH:mm" : "YYYY-MM-DD";
  return dayjs(peakPoint.value.time * 1000).tz("Asia/Shanghai").format(fmt);
});

// 强制按北京时间格式化，避免管理员浏览器在其他时区时显示偏差
const formatXLabel = (label) => {
  const cfg = rangeConfig(rangeKey.value);
  const d = dayjs(label).tz("Asia/Shanghai");
  return cfg.resolution < 60 * 24 ? d.format("MM-DD HH:mm") : d.format("MM-DD");
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
}

.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--gap-md);
}

.kpi-card {
  background-color: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-button-bg);
  padding: var(--gap-md);
  display: flex;
  flex-direction: column;
  gap: var(--gap-xs);

  .label {
    color: var(--color-secondary);
    font-size: var(--font-size-sm);
  }

  .value {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .hint {
    color: var(--color-secondary);
    font-size: var(--font-size-xs);
  }
}

.chart-block {
  background-color: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-button-bg);
  padding: var(--gap-md);
  min-height: 320px;
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
</style>
