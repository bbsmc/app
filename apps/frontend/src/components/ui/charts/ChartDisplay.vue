<template>
  <div>
    <div v-if="analytics.error.value" class="universal-card">
      <h2>
        <span class="label__title">Error</span>
      </h2>
      <div>
        {{ analytics.error.value }}
      </div>
    </div>
    <div v-else-if="!isInitialized || analytics.loading.value" class="universal-card">
      <h2>
        <span class="label__title">Loading analytics...</span>
      </h2>
    </div>
    <div v-else class="graphs">
      <div class="graphs__vertical-bar">
        <client-only>
          <CompactChart
            v-if="analytics.formattedData.value.downloads"
            ref="tinyDownloadChart"
            :title="` ${formatDateTime(startDate, 'YYYY-MM-DD')} - ${formatDateTime(endDate, 'YYYY-MM-DD')}`"
            color="var(--color-brand)"
            :value="formatNumber(analytics.formattedData.value.downloads.sum, false)"
            :data="analytics.formattedData.value.downloads.chart.sumData"
            :labels="analytics.formattedData.value.downloads.chart.labels"
            suffix="<svg xmlns='http://www.w3.org/2000/svg' class='h-6 w-6' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4' /></svg>"
            :class="`clickable chart-button-base button-base ${
              selectedChart === 'downloads'
                ? 'chart-button-base__selected button-base__selected'
                : ''
            }`"
            :onclick="() => (selectedChart = 'downloads')"
            role="button"
          />
        </client-only>
        <client-only>
          <CompactChart
            v-if="analytics.formattedData.value.views"
            ref="tinyViewChart"
            :title="` ${formatDateTime(startDate, 'YYYY-MM-DD')} - ${formatDateTime(endDate, 'YYYY-MM-DD')}`"
            color="var(--color-blue)"
            :value="formatNumber(analytics.formattedData.value.views.sum, false)"
            :data="analytics.formattedData.value.views.chart.sumData"
            :labels="analytics.formattedData.value.views.chart.labels"
            suffix="<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z'/><circle cx='12' cy='12' r='3'/></svg>"
            :class="`clickable chart-button-base button-base ${
              selectedChart === 'views' ? 'chart-button-base__selected button-base__selected' : ''
            }`"
            :onclick="() => (selectedChart = 'views')"
            role="button"
          />
        </client-only>
        <!--        <client-only>-->
        <!--          <CompactChart-->
        <!--            v-if="analytics.formattedData.value.revenue"-->
        <!--            ref="tinyRevenueChart"-->
        <!--            :title="` ${formatDateTime(startDate, 'YYYY-MM-DD')} - ${formatDateTime(endDate, 'YYYY-MM-DD')}`"-->
        <!--            color="var(&#45;&#45;color-purple)"-->
        <!--            :value="formatMoney(analytics.formattedData.value.revenue.sum, false)"-->
        <!--            :data="analytics.formattedData.value.revenue.chart.sumData"-->
        <!--            :labels="analytics.formattedData.value.revenue.chart.labels"-->
        <!--            is-money-->
        <!--            :class="`clickable chart-button-base button-base ${-->
        <!--              selectedChart === 'revenue' ? 'chart-button-base__selected button-base__selected' : ''-->
        <!--            }`"-->
        <!--            :onclick="() => (selectedChart = 'revenue')"-->
        <!--            role="button"-->
        <!--          />-->
        <!--        </client-only>-->
      </div>
      <div class="graphs__main-graph">
        <div class="universal-card">
          <div class="chart-controls">
            <h2>
              <span class="label__title">
                {{ formatCategoryHeader(selectedChart) }}
              </span>
            </h2>
            <div class="chart-controls__buttons">
              <Button v-tooltip="'切换颜色'" icon-only @click="onToggleColors">
                <PaletteIcon />
              </Button>
              <Button v-tooltip="'导出统计表'" icon-only @click="openExportModal">
                <DownloadIcon />
              </Button>
              <Button v-tooltip="'刷新'" icon-only @click="resetCharts">
                <UpdatedIcon />
              </Button>
            </div>
          </div>
          <div class="chart-filters">
            <div class="chart-filters__range">
              <DropdownSelect
                v-model="selectedRange"
                :options="selectableRanges"
                name="区间"
                style="width: 100%"
                :display-name="
                  (o: (typeof selectableRanges)[number] | undefined) => o?.label || '自定义'
                "
              />
            </div>
            <DatePicker
              :model-value="startDate"
              label="开始日期"
              :max-date="endDate"
              @change="onStartPick"
            />
            <span class="chart-filters__dash">至</span>
            <DatePicker :model-value="endDate" label="结束日期" align="right" @change="onEndPick" />
            <span class="chart-filters__hint">每点：{{ currentBucketLabel }}</span>
          </div>
          <div class="chart-area">
            <div class="chart">
              <client-only>
                <Chart
                  v-if="analytics.formattedData.value.downloads && selectedChart === 'downloads'"
                  ref="downloadsChart"
                  type="line"
                  name="Download data"
                  :hide-legend="true"
                  :data="analytics.formattedData.value.downloads.chart.data"
                  :labels="analytics.formattedData.value.downloads.chart.labels"
                  :format-labels="chartFormatLabels"
                  suffix="<svg xmlns='http://www.w3.org/2000/svg' class='h-6 w-6' fill='none' viewBox='0 0 24 24' stroke='currentColor' stroke-width='2'><path stroke-linecap='round' stroke-linejoin='round' d='M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4' /></svg>"
                  :colors="
                    isUsingProjectColors
                      ? analytics.formattedData.value.downloads.chart.colors
                      : analytics.formattedData.value.downloads.chart.defaultColors
                  "
                />
                <Chart
                  v-if="analytics.formattedData.value.views && selectedChart === 'views'"
                  ref="viewsChart"
                  type="line"
                  name="View data"
                  :hide-legend="true"
                  :data="analytics.formattedData.value.views.chart.data"
                  :labels="analytics.formattedData.value.views.chart.labels"
                  :format-labels="chartFormatLabels"
                  suffix="<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z'/><circle cx='12' cy='12' r='3'/></svg>"
                  :colors="
                    isUsingProjectColors
                      ? analytics.formattedData.value.views.chart.colors
                      : analytics.formattedData.value.views.chart.defaultColors
                  "
                />
                <Chart
                  v-if="analytics.formattedData.value.revenue && selectedChart === 'revenue'"
                  ref="revenueChart"
                  type="line"
                  name="Revenue data"
                  :hide-legend="true"
                  :data="analytics.formattedData.value.revenue.chart.data"
                  :labels="analytics.formattedData.value.revenue.chart.labels"
                  :format-labels="chartFormatLabels"
                  is-money
                  :colors="
                    isUsingProjectColors
                      ? analytics.formattedData.value.revenue.chart.colors
                      : analytics.formattedData.value.revenue.chart.defaultColors
                  "
                />
              </client-only>
            </div>
            <div class="legend">
              <div class="legend__items">
                <template v-for="project in selectedDataSetProjects" :key="project">
                  <button
                    v-tooltip="project.title"
                    :class="`legend__item button-base btn-transparent ${
                      !projectIsOnDisplay(project.id) ? 'btn-dimmed' : ''
                    }`"
                    @click="
                      () =>
                        projectIsOnDisplay(project.id) &&
                        analytics.validProjectIds.value.includes(project.id)
                          ? removeProjectFromDisplay(project.id)
                          : addProjectToDisplay(project.id)
                    "
                  >
                    <div
                      :style="{
                        '--color-brand': isUsingProjectColors
                          ? intToRgba(project.color, project.id, theme.active ?? undefined)
                          : getDefaultColor(project.id),
                      }"
                      class="legend__item__color"
                    ></div>
                    <div class="legend__item__text">{{ project.title }}</div>
                  </button>
                </template>
              </div>
            </div>
          </div>
        </div>
        <div class="country-data">
          <!--          <Card-->
          <!--            v-if="-->
          <!--              analytics.formattedData.value?.downloadsByCountry &&-->
          <!--              selectedChart === 'downloads' &&-->
          <!--              analytics.formattedData.value.downloadsByCountry.data.length > 0-->
          <!--            "-->
          <!--            class="country-downloads"-->
          <!--          >-->
          <!--            <label>-->
          <!--              <span class="label__title">Downloads by region</span>-->
          <!--            </label>-->
          <!--            <div class="country-values">-->
          <!--              <div-->
          <!--                v-for="[name, count] in analytics.formattedData.value.downloadsByCountry.data"-->
          <!--                :key="name"-->
          <!--                class="country-value"-->
          <!--              >-->
          <!--                <div class="country-flag-container">-->
          <!--                  <template v-if="name.toLowerCase() === 'xx' || !name">-->
          <!--                    <img-->
          <!--                      src="https://cdn.bbsmc.net/raw/placeholder-banner.svg"-->
          <!--                      alt="Placeholder flag"-->
          <!--                      class="country-flag"-->
          <!--                    />-->
          <!--                  </template>-->
          <!--                  <template v-else>-->
          <!--                    <img-->
          <!--                      :src="countryCodeToFlag(name)"-->
          <!--                      :alt="`${countryCodeToName(name)}'s flag`"-->
          <!--                      class="country-flag"-->
          <!--                    />-->
          <!--                  </template>-->
          <!--                </div>-->
          <!--                <div class="country-text">-->
          <!--                  <strong class="country-name"-->
          <!--                    ><template v-if="name.toLowerCase() === 'xx' || !name">Hidden</template>-->
          <!--                    <template v-else>{{ countryCodeToName(name) }}</template>-->
          <!--                  </strong>-->
          <!--                  <span class="data-point">{{ formatNumber(count) }}</span>-->
          <!--                </div>-->
          <!--                <div-->
          <!--                  v-tooltip="-->
          <!--                    formatPercent(count, analytics.formattedData.value.downloadsByCountry.sum)-->
          <!--                  "-->
          <!--                  class="percentage-bar"-->
          <!--                >-->
          <!--                  <span-->
          <!--                    :style="{-->
          <!--                      width: formatPercent(-->
          <!--                        count,-->
          <!--                        analytics.formattedData.value.downloadsByCountry.sum,-->
          <!--                      ),-->
          <!--                      backgroundColor: 'var(&#45;&#45;color-brand)',-->
          <!--                    }"-->
          <!--                  ></span>-->
          <!--                </div>-->
          <!--              </div>-->
          <!--            </div>-->
          <!--          </Card>-->
          <!--          <Card-->
          <!--            v-if="-->
          <!--              analytics.formattedData.value?.viewsByCountry &&-->
          <!--              selectedChart === 'views' &&-->
          <!--              analytics.formattedData.value.viewsByCountry.data.length > 0-->
          <!--            "-->
          <!--            class="country-downloads"-->
          <!--          >-->
          <!--            <label>-->
          <!--              <span class="label__title">Page views by region</span>-->
          <!--            </label>-->
          <!--            <div class="country-values">-->
          <!--              <div-->
          <!--                v-for="[name, count] in analytics.formattedData.value.viewsByCountry.data"-->
          <!--                :key="name"-->
          <!--                class="country-value"-->
          <!--              >-->
          <!--                <div class="country-flag-container">-->
          <!--                  <template v-if="name.toLowerCase() === 'xx' || !name">-->
          <!--                    <img-->
          <!--                      src="https://cdn.bbsmc.net/raw/placeholder-banner.svg"-->
          <!--                      alt="Placeholder flag"-->
          <!--                      class="country-flag"-->
          <!--                    />-->
          <!--                  </template>-->
          <!--                  <template v-else>-->
          <!--                    <img-->
          <!--                      :src="countryCodeToFlag(name)"-->
          <!--                      :alt="`${countryCodeToName(name)}'s flag`"-->
          <!--                      class="country-flag"-->
          <!--                    />-->
          <!--                  </template>-->
          <!--                </div>-->
          <!--                <div class="country-text">-->
          <!--                  <strong class="country-name">-->
          <!--                    <template v-if="name.toLowerCase() === 'xx' || !name">Hidden</template>-->
          <!--                    <template v-else>{{ countryCodeToName(name) }}</template>-->
          <!--                  </strong>-->
          <!--                  <span class="data-point">{{ formatNumber(count) }}</span>-->
          <!--                </div>-->
          <!--                <div-->
          <!--                  v-tooltip="-->
          <!--                    `${-->
          <!--                      Math.round(-->
          <!--                        (count / analytics.formattedData.value.viewsByCountry.sum) * 10000,-->
          <!--                      ) / 100-->
          <!--                    }%`-->
          <!--                  "-->
          <!--                  class="percentage-bar"-->
          <!--                >-->
          <!--                  <span-->
          <!--                    :style="{-->
          <!--                      width: `${(count / analytics.formattedData.value.viewsByCountry.sum) * 100}%`,-->
          <!--                      backgroundColor: 'var(&#45;&#45;color-blue)',-->
          <!--                    }"-->
          <!--                  ></span>-->
          <!--                </div>-->
          <!--              </div>-->
          <!--            </div>-->
          <!--          </Card>-->
        </div>
      </div>
    </div>

    <!-- 导出统计表：选择统计粒度 -->
    <NewModal ref="exportModal" header="导出统计表">
      <div class="export-modal">
        <p class="export-modal__summary">
          {{ formatCategoryHeader(selectedChart) }} ·
          {{ formatDateTime(startDate, "YYYY-MM-DD") }} 至
          {{ formatDateTime(endDate, "YYYY-MM-DD") }}
        </p>
        <div class="export-modal__options">
          <button
            v-for="option in exportGranularityOptions"
            :key="option.value"
            type="button"
            class="export-modal__option button-base btn-transparent"
            :class="{ 'export-modal__option--selected': exportGranularity === option.value }"
            @click="exportGranularity = option.value"
          >
            <RadioButtonChecked
              v-if="exportGranularity === option.value"
              class="shrink-0"
              aria-hidden="true"
            />
            <RadioButtonIcon v-else class="shrink-0" aria-hidden="true" />
            <span class="flex min-w-0 flex-col items-start">
              <span>{{ option.label }}</span>
              <span class="export-modal__option-desc">{{ option.description }}</span>
            </span>
          </button>
        </div>
        <div class="modal-actions">
          <ButtonStyled color="brand">
            <button :disabled="exporting" type="button" @click="confirmExport">
              <DownloadIcon aria-hidden="true" />
              {{ exporting ? "导出中..." : "导出 CSV" }}
            </button>
          </ButtonStyled>
          <ButtonStyled>
            <button type="button" @click="exportModal?.hide()">取消</button>
          </ButtonStyled>
        </div>
      </div>
    </NewModal>
  </div>
</template>

<script setup lang="ts">
import { Button, ButtonStyled, Card, DropdownSelect, NewModal } from "@modrinth/ui";
import { formatMoney, formatNumber, formatCategoryHeader, formatDateTime } from "@modrinth/utils";
import { UpdatedIcon, DownloadIcon, RadioButtonChecked, RadioButtonIcon } from "@modrinth/assets";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";
dayjs.locale("zh-cn");

import { computed, onMounted } from "vue";

import { aggregateAnalyticsByUnit, analyticsSetToCSVString, intToRgba } from "~/utils/analytics.js";

import {
  UiChartsCompactChart as CompactChart,
  UiChartsChart as Chart,
  UiChartsDatePicker as DatePicker,
} from "#components";

import PaletteIcon from "~/assets/icons/palette.svg?component";

const router = useNativeRouter();
const theme = useTheme();

const props = withDefaults(
  defineProps<{
    projects?: any[];
    /**
     * @deprecated Use `ranges` instead
     */
    resoloutions?: Record<string, number>;
    ranges?: Record<number, [string, number] | string>;
    personal?: boolean;
  }>(),
  {
    projects: undefined,
    resoloutions: () => defaultResoloutions,
    ranges: () => defaultRanges,
    personal: false,
  },
);

const projects = ref(props.projects || []);

const selectableRanges = Object.entries(props.ranges).map(([duration, extra]) => ({
  label: typeof extra === "string" ? extra : extra[0],
  value: Number(duration),
  res: typeof extra === "string" ? Number(duration) : extra[1],
}));

// const selectedChart = ref('downloads')
const selectedChart = computed({
  get: () => {
    const id = (router.currentRoute.value.query?.chart as string | undefined) || "downloads";
    // if the id is anything but the 3 charts we have or undefined, throw an error
    if (!["downloads", "views", "revenue"].includes(id)) {
      throw new Error(`Unknown chart ${id}`);
    }
    return id;
  },
  set: (chart) => {
    router.push({
      query: {
        ...router.currentRoute.value.query,
        chart,
      },
    });
  },
});

// Chart refs
const downloadsChart = ref();
const viewsChart = ref();
const revenueChart = ref();
const tinyDownloadChart = ref();
const tinyViewChart = ref();
const tinyRevenueChart = ref();

const selectedDisplayProjects = ref(props.projects || []);

const removeProjectFromDisplay = (id: string) => {
  selectedDisplayProjects.value = selectedDisplayProjects.value.filter((p) => p.id !== id);
};

const addProjectToDisplay = (id: string) => {
  selectedDisplayProjects.value = [
    ...selectedDisplayProjects.value,
    props.projects?.find((p) => p.id === id),
  ].filter(Boolean);
};

const projectIsOnDisplay = (id: string) => {
  return selectedDisplayProjects.value?.some((p) => p.id === id) ?? false;
};

const resetCharts = () => {
  downloadsChart.value?.resetChart();
  viewsChart.value?.resetChart();
  revenueChart.value?.resetChart();

  tinyDownloadChart.value?.resetChart();
  tinyViewChart.value?.resetChart();
  tinyRevenueChart.value?.resetChart();
};

const isUsingProjectColors = computed({
  get: () => {
    return (
      router.currentRoute.value.query?.colors === "true" ||
      router.currentRoute.value.query?.colors === undefined
    );
  },
  set: (newValue) => {
    router.push({
      query: {
        ...router.currentRoute.value.query,
        colors: newValue ? "true" : "false",
      },
    });
  },
});

const analytics = useFetchAllAnalytics(
  resetCharts,
  projects,
  selectedDisplayProjects,
  props.personal,
);

const { startDate, endDate, timeRange, timeResolution, isInitialized, chartUnit } = analytics;

onMounted(() => {
  isInitialized.value = true;
});

const selectedRange = computed({
  get: () => {
    return (
      selectableRanges.find((option) => option.value === timeRange.value) || {
        label: "自定义",
        value: timeRange.value,
      }
    );
  },
  set: (newRange: { label: string; value: number; res?: number }) => {
    timeRange.value = newRange.value;
    startDate.value = Date.now() - timeRange.value * 60 * 1000;
    endDate.value = Date.now();

    if (newRange?.res) {
      timeResolution.value = newRange.res;
    }
  },
});

// 日期区间筛选：两个独立日期选择器，与预设区间联动（手动选择后预设显示为「自定义」）
const applyRangeValues = (startMs: number, endMs: number) => {
  const now = Date.now();
  startDate.value = Math.min(startMs, now);
  endDate.value = Math.min(endMs, now);
  if (startDate.value > endDate.value) {
    startDate.value = endDate.value;
  }

  timeRange.value = 0; // 0 不匹配任何预设，下拉框显示「自定义」
  timeResolution.value = autoResolutionForRange(startDate.value, endDate.value);
};

const onStartPick = (dayStart: number) => {
  // 起点晚于当前终点时，终点同步挪到同一天
  applyRangeValues(dayStart, Math.max(endDate.value, dayStart + 86_399_999));
};

const onEndPick = (dayStart: number) => {
  // 终点早于当前起点时，起点同步挪到同一天
  applyRangeValues(Math.min(startDate.value, dayStart), dayStart + 86_399_999);
};

// 根据区间跨度自动选择拉取粒度（分钟）；更长的区间固定按天拉取，图表端聚合为自然周/自然月
const autoResolutionForRange = (startMs: number, endMs: number) => {
  const days = (endMs - startMs) / 86_400_000;
  if (days <= 1) return 60; // 每小时
  if (days <= 7) return 720; // 每 12 小时
  return 1440; // 每天
};

// 当前图表每个数据点的粒度文案（筛选行提示 + 导出弹窗「按当前显示」共用）
const currentBucketLabel = computed(() => {
  if (chartUnit.value === "week") {
    return "自然周（周一 ~ 周日）";
  }
  if (chartUnit.value === "month") {
    return "自然月";
  }
  return resolutionLabel(timeResolution.value);
});

// 图表 tooltip 的桶标签：长区间显示桶范围，与数据桶边界对齐
const chartFormatLabels = (label: string) => {
  if (chartUnit.value === "week") {
    const start = dayjs(label);
    return `${start.format("MM/DD")} ~ ${start.add(6, "day").format("MM/DD")}`;
  }
  if (chartUnit.value === "month") {
    return dayjs(label).format("YYYY年M月");
  }
  return dayjs(label).format("MM/DD");
};

const selectedDataSet = computed(() => {
  switch (selectedChart.value) {
    case "downloads":
      return analytics.totalData.value.downloads;
    case "views":
      return analytics.totalData.value.views;
    case "revenue":
      return analytics.totalData.value.revenue;
    default:
      throw new Error(`Unknown chart ${selectedChart.value}`);
  }
});
const selectedDataSetProjects = computed(() => {
  return selectedDataSet.value.projectIds
    .map((id) => props.projects?.find((p) => p?.id === id))
    .filter(Boolean);
});

const saveCsvFile = (csv: string, filename: string) => {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });

  const link = document.createElement("a");
  const url = URL.createObjectURL(blob);
  link.setAttribute("href", url);
  link.setAttribute("download", filename);
  link.style.visibility = "hidden";
  document.body.appendChild(link);

  link.click();
  link.remove();
  URL.revokeObjectURL(url);
};

// 导出统计表：选择统计粒度
type ExportGranularity = "current" | "day" | "week" | "month";

const exportModal = ref();
const exportGranularity = ref<ExportGranularity>("current");
const exporting = ref(false);
const app = useNuxtApp();

const resolutionLabel = (minutes: number) => {
  if (minutes >= 1440) {
    const d = minutes / 1440;
    return d === 1 ? "每天" : `每 ${d} 天`;
  }
  if (minutes >= 60) {
    const h = minutes / 60;
    return h === 1 ? "每小时" : `每 ${h} 小时`;
  }
  return minutes === 1 ? "每分钟" : `每 ${minutes} 分钟`;
};

const exportGranularityOptions = computed(() => [
  {
    value: "current" as const,
    label: "按当前显示",
    description: `与图表一致（${currentBucketLabel.value}）`,
  },
  { value: "day" as const, label: "按日汇总", description: "每天一行数据" },
  { value: "week" as const, label: "按周汇总", description: "每周一行数据，周一为一周起点" },
  { value: "month" as const, label: "按月汇总", description: "每月一行数据" },
]);

const openExportModal = () => {
  exportGranularity.value = "current";
  exportModal.value?.show();
};

const rawAnalyticsByChart = {
  downloads: analytics.downloadData,
  views: analytics.viewData,
  revenue: analytics.revenueData,
} as const;

const downloadAsCSVWithGranularity = (unit: "day" | "week" | "month") => {
  const kind = selectedChart.value;
  if (kind !== "downloads" && kind !== "views" && kind !== "revenue") {
    throw new Error(`Unknown chart ${kind}`);
  }

  // 图表数据最细为按天（小时粒度也能无损并入更大的桶），前端聚合即可保证准确
  const exportSet = aggregateAnalyticsByUnit(
    rawAnalyticsByChart[kind].value,
    unit,
    kind === "revenue",
    projects.value,
  );
  saveCsvFile(analyticsSetToCSVString(exportSet), `${kind}-data-by-${unit}.csv`);
};

const confirmExport = () => {
  if (exporting.value) {
    return;
  }

  exporting.value = true;
  try {
    if (exportGranularity.value === "current") {
      const csv = analyticsSetToCSVString(selectedDataSet.value);
      saveCsvFile(csv, `${selectedChart.value}-data.csv`);
    } else {
      downloadAsCSVWithGranularity(exportGranularity.value);
    }
    exportModal.value?.hide();
  } catch (e) {
    app.$notify({
      group: "main",
      title: "导出失败",
      text: e instanceof Error ? e.message : "请稍后重试",
      type: "error",
    });
  } finally {
    exporting.value = false;
  }
};

const onToggleColors = () => {
  isUsingProjectColors.value = !isUsingProjectColors.value;
};
</script>

<script lang="ts">
const defaultResoloutions: Record<string, number> = {
  "5 分钟": 5,
  "30 分钟": 30,
  "1 小时": 60,
  "12 小时": 720,
  一天: 1440,
  一周: 10080,
};

const defaultRanges: Record<number, [string, number] | string> = {
  30: ["最近 30 分钟", 1],
  60: ["最近 1 小时", 5],
  720: ["最近 12 小时", 15],
  1440: ["最近 1 天", 60],
  10080: ["最近 1 周", 720],
  43200: ["最近 1 月", 1440],
  129600: ["最近 3 个月", 10080],
  525600: ["最近 1 年", 20160],
  1051200: ["最近 2 年", 40320],
};
</script>

<style scoped lang="scss">
.chart-controls {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--gap-md);

  .chart-controls__buttons {
    display: flex;
    flex-direction: row;
    gap: var(--gap-xs);

    * {
      width: auto;
      min-height: auto;
    }
  }
}

.chart-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--gap-xs);
  margin-top: var(--gap-md);

  &__range {
    width: 10.5rem;
  }

  &__dash {
    color: var(--color-secondary);
    font-size: var(--font-size-sm);
    margin: 0 var(--gap-xs);
  }

  &__hint {
    margin-left: var(--gap-xs);
    color: var(--color-secondary);
    font-size: var(--font-size-xs);
  }
}

.export-modal {
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);

  &__summary {
    margin: 0;
    color: var(--color-secondary);
    font-size: var(--font-size-sm);
  }

  &__options {
    display: flex;
    flex-direction: column;
    gap: var(--gap-xs);
  }

  &__option {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: var(--gap-sm) var(--gap-md);
    border-radius: var(--radius-md);
    width: 100%;
    text-align: left;
    font-size: var(--font-size-nm);

    svg {
      color: var(--color-secondary);
    }

    &--selected {
      background-color: var(--color-brand-highlight);
      box-shadow:
        inset 0 0 0 transparent,
        0 0 0 2px var(--color-brand);

      svg {
        color: var(--color-brand);
      }
    }
  }

  &__option-desc {
    font-size: var(--font-size-xs);
    font-weight: normal;
    color: var(--color-secondary);
  }
}

.modal-actions {
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  gap: var(--gap-sm);
}

.chart-area {
  display: flex;
  flex-direction: row;
  gap: var(--gap-md);

  height: 100%;

  .chart {
    flex-grow: 1;
    flex-shrink: 1;

    display: flex;
    flex-direction: column;
    gap: var(--gap-md);
  }

  .legend {
    margin-top: 24px;
    overflow: hidden;

    max-width: 26ch;
    width: fit-content;

    .legend__items {
      display: flex;
      flex-direction: column;
      gap: var(--gap-xs);

      .legend__item {
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: var(--gap-xs);
        font-size: var(--font-size-sm);
        width: 100%;

        .legend__item__text {
          white-space: nowrap;
          text-overflow: ellipsis;
        }

        .legend__item__color {
          height: var(--font-size-xs);
          width: var(--font-size-xs);
          border-radius: var(--radius-sm);
          background-color: var(--color-brand);

          flex-grow: 0;
          flex-shrink: 0;
        }
      }
    }
  }
}

.btn-transparent {
  background-color: transparent;
  border: none;
  cursor: pointer;

  color: var(--text-color);
  font-weight: var(--font-weight-regular);
}

.btn-dimmed {
  opacity: 0.5;
}

.chart-button-base {
  overflow: hidden;
}

.chart-button-base__selected {
  color: var(--color-contrast);
  background-color: var(--color-brand-highlight);
  box-shadow:
    inset 0 0 0 transparent,
    0 0 0 2px var(--color-brand);

  &:hover {
    background-color: var(--color-brand-highlight);
  }
}

.graphs {
  // Pages clip so we need to add a margin
  margin-left: 0.25rem;
  margin-top: 0.25rem;

  display: flex;
  flex-direction: column;

  .graphs__vertical-bar {
    flex-grow: 0;
    flex-shrink: 0;
    gap: 0.75rem;
    display: flex;
    margin-right: 0.1rem;
  }
}

.country-flag-container {
  width: 40px;
  height: 27px;

  display: flex;
  justify-content: center;
  align-items: center;

  overflow: hidden;

  border: 1px solid var(--color-divider);
  border-radius: var(--radius-xs);
}

.country-flag {
  object-fit: cover;

  min-width: 100%;
  min-height: 100%;
}

.spark-data {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: var(--gap-md);
}

.country-data {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--gap-md);
}

.country-values {
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg);
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-button-bg);
  gap: var(--gap-md);
  padding: var(--gap-md);
  margin-top: var(--gap-md);
  overflow-y: auto;
  max-height: 24rem;
}

.country-value {
  display: grid;
  grid-template-areas: "flag text bar";
  grid-template-columns: auto 1fr 10rem;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: var(--gap-sm);

  .country-text {
    grid-area: text;
    display: flex;
    flex-direction: column;
    gap: var(--gap-xs);
  }

  .percentage-bar {
    grid-area: bar;
    width: 100%;
    height: 1rem;
    background-color: var(--color-raised-bg);
    border: 1px solid var(--color-button-bg);
    border-radius: var(--radius-sm);
    overflow: hidden;

    span {
      display: block;
      height: 100%;
    }
  }
}

@media (max-width: 768px) {
  .chart-area {
    flex-direction: column;
    gap: var(--gap-md);
  }

  .chart-controls {
    flex-direction: column;
    gap: var(--gap-md);
  }

  .chart-filters {
    &__range {
      width: 100%;
    }
  }

  .chart {
    flex-direction: column;
    gap: var(--gap-md);
  }

  .legend {
    margin-top: 0px;
    width: 100%;
    max-width: 100%;
  }

  .graphs {
    margin-left: 0px;
    margin-top: 0px;

    .graphs__vertical-bar {
      flex-direction: column;
      gap: 0;
      margin-right: 0px;
    }
  }

  .country-data {
    display: block;
  }

  .country-value {
    grid-template-columns: auto 1fr 5rem;
  }
}
</style>
