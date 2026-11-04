import dayjs from "dayjs";

import { ref, watch, computed } from "vue";

// note: build step can miss unix import for some reason, so
// we have to import it like this

const { unix } = dayjs; // eslint-disable-line import/no-named-as-default-member

export function useCountryNames(style = "long") {
  const formattingOptions = { type: "region", style };
  const { formats } = useVIntl();
  return function formatCountryName(code) {
    return formats.displayName(code, formattingOptions);
  };
}

export const countryCodeToName = (code) => {
  const formatCountryName = useCountryNames();

  return formatCountryName(code);
};

export const countryCodeToFlag = (code) => {
  if (code === "XX") {
    return undefined;
  }
  return `https://flagcdn.com/h240/${code.toLowerCase()}.png`;
};

export const formatTimestamp = (timestamp) => {
  return unix(timestamp).format();
};

export const formatPercent = (value, sum) => {
  return `${((value / sum) * 100).toFixed(2)}%`;
};

const hashProjectId = (projectId) => {
  return projectId.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0) % 30;
};

export const defaultColors = [
  "#ff496e", // Original: Bright pink
  "#ffa347", // Original: Bright orange
  "#1bd96a", // Original: Bright green
  "#4f9cff", // Original: Bright blue
  "#c78aff", // Original: Bright purple
  "#ffeb3b", // Added: Bright yellow
  "#00bcd4", // Added: Bright cyan
  "#ff5722", // Added: Bright red-orange
  "#9c27b0", // Added: Bright deep purple
  "#3f51b5", // Added: Bright indigo
  "#009688", // Added: Bright teal
  "#cddc39", // Added: Bright lime
  "#795548", // Added: Bright brown
  "#607d8b", // Added: Bright blue-grey
];

/**
 * @param {string | number} value
 * @returns {string} color
 */
export const getDefaultColor = (value) => {
  if (typeof value === "string") {
    value = hashProjectId(value);
  }
  return defaultColors[value % defaultColors.length];
};

export const intToRgba = (color, projectId = "Unknown", theme = "dark", alpha = "1") => {
  const hash = hashProjectId(projectId);

  if (!color || color === 0) {
    return getDefaultColor(hash);
  }

  // if color is a string, return that instead
  if (typeof color === "string") {
    return color;
  }

  // Extract RGB values
  let r = (color >> 16) & 255;
  let g = (color >> 8) & 255;
  let b = color & 255;

  // Hash function to alter color slightly based on project_id
  r = (r + hash) % 256;
  g = (g + hash) % 256;
  b = (b + hash) % 256;

  // Adjust brightness for theme
  const brightness = r * 0.299 + g * 0.587 + b * 0.114;
  const threshold = theme === "dark" ? 50 : 200;
  if (theme === "dark" && brightness < threshold) {
    // Increase brightness for dark theme
    r += threshold / 2;
    g += threshold / 2;
    b += threshold / 2;
  } else if (theme === "light" && brightness > threshold) {
    // Decrease brightness for light theme
    r -= threshold / 4;
    g -= threshold / 4;
    b -= threshold / 4;
  }

  // Ensure RGB values are within 0-255
  r = Math.min(255, Math.max(0, r));
  g = Math.min(255, Math.max(0, g));
  b = Math.min(255, Math.max(0, b));

  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
};

const emptyAnalytics = {
  sum: 0,
  len: 0,
  chart: {
    labels: [],
    data: [],
    sumData: [
      {
        name: "",
        data: [],
      },
    ],
    colors: [],
    defaultColors: [],
  },
  projectIds: [],
};

export const analyticsSetToCSVString = (analytics) => {
  if (!analytics) {
    return "";
  }

  const newline = "\n";
  const labels = analytics.chart.labels;
  const projects = analytics.chart.data;

  const projectNames = projects.map((p) => p.name);

  const header = ["Date", ...projectNames].join(",");

  const data = labels.map((label, i) => {
    const values = projects.map((p) => p.data?.[i] || "");
    return [label, ...values].join(",");
  });

  // \uFEFF BOM，避免 Excel 打开 UTF-8 中文乱码
  return "\uFEFF" + [header, ...data].join(newline);
};

/**
 * 桶起点（毫秒）：day=当天零点；week=显式周一对齐（不依赖 dayjs locale 的 weekStart）；month=自然月 1 号。
 */
const bucketStartMs = (day, unit) => {
  if (unit === "week") {
    return day.subtract((day.day() + 6) % 7, "day").valueOf();
  }
  if (unit === "month") {
    return day.startOf("month").valueOf();
  }
  return day.valueOf(); // day
};

/**
 * 将原始分析数据按日/周/月聚合（同桶内求和）。
 * 返回结构兼容 analyticsSetToCSVString。
 */
export const aggregateAnalyticsByUnit = (rawData, unit, isMoney, projects) => {
  if (!rawData || !projects) {
    return { sum: 0, chart: { labels: [], data: [] } };
  }

  const projectById = new Map(projects.map((p) => [p.id, p]));

  // 桶起点(ms) -> { projectId -> 累计值 }
  const buckets = new Map();
  Object.entries(rawData).forEach(([id, series]) => {
    if (!projectById.has(id)) {
      return;
    }
    Object.entries(series).forEach(([ts, value]) => {
      const key = bucketStartMs(unix(Number(ts)).startOf("day"), unit);
      let bucket = buckets.get(key);
      if (!bucket) {
        bucket = {};
        buckets.set(key, bucket);
      }
      bucket[id] = (bucket[id] || 0) + (Number(value) || 0);
    });
  });

  const keys = Array.from(buckets.keys()).sort((a, b) => a - b);
  const round = (v) => (isMoney ? Math.round(v * 100) / 100 : v);

  const chartData = Object.keys(rawData)
    .filter((id) => projectById.has(id))
    .map((id) => ({
      name: projectById.get(id).title,
      data: keys.map((k) => round(buckets.get(k)[id] || 0)),
    }))
    .sort(
      (a, b) =>
        b.data.reduce((acc, cur) => acc + cur, 0) - a.data.reduce((acc, cur) => acc + cur, 0),
    );

  const labelFormat = unit === "month" ? "YYYY-MM" : "YYYY-MM-DD";

  return {
    sum: chartData.reduce((acc, p) => acc + p.data.reduce((a, c) => a + c, 0), 0),
    chart: {
      labels: keys.map((k) => dayjs(k).format(labelFormat)),
      data: chartData,
    },
  };
};

/**
 * 将原始分析数据（{projectId: {unix秒: 值}}）聚合到更大的时间桶。
 * 桶起点同样以 unix 秒为键，可直接交给 processAnalytics 渲染。
 */
export const aggregateRawAnalytics = (rawData, unit) => {
  if (!rawData) {
    return rawData;
  }
  const out = {};
  Object.entries(rawData).forEach(([id, series]) => {
    const buckets = {};
    Object.entries(series).forEach(([ts, value]) => {
      const bucketTs = bucketStartMs(unix(Number(ts)).startOf("day"), unit) / 1000;
      buckets[bucketTs] = (buckets[bucketTs] || 0) + (Number(value) || 0);
    });
    out[id] = buckets;
  });
  return out;
};

export const processAnalytics = (category, projects, labelFn, sortFn, mapFn, chartName, theme) => {
  if (!category || !projects) {
    return emptyAnalytics;
  }

  // Get an intersection of category keys and project ids
  const projectIds = projects.map((p) => p.id);
  const loadedProjectIds = Object.keys(category).filter((id) => projectIds.includes(id));

  if (!loadedProjectIds?.length) {
    return emptyAnalytics;
  }

  const loadedProjectData = loadedProjectIds.map((id) => category[id]);

  // Convert each project's data into a list of [unix_ts_str, number] pairs
  const projectData = loadedProjectData
    .map((data) => Object.entries(data))
    .map((data) => data.sort(sortFn))
    .map((data) => (mapFn ? data.map(mapFn) : data));

  // Each project may not include the same timestamps, so we should use the union of all timestamps
  const timestamps = Array.from(
    new Set(projectData.flatMap((data) => data.map(([ts]) => ts))),
  ).sort();

  const chartData = projectData
    .map((data, i) => {
      const project = projects.find((p) => p.id === loadedProjectIds[i]);
      if (!project) {
        throw new Error(`Project ${loadedProjectIds[i]} not found`);
      }

      return {
        name: `${project.title}`,
        data: timestamps.map((ts) => {
          const entry = data.find(([ets]) => ets === ts);
          return entry ? entry[1] : 0;
        }),
        id: project.id,
        color: project.color,
      };
    })
    .sort(
      (a, b) =>
        b.data.reduce((acc, cur) => acc + cur, 0) - a.data.reduce((acc, cur) => acc + cur, 0),
    );

  const projectIdsSortedBySum = chartData.map((p) => p.id);

  return {
    // The total count of all the values across all projects
    sum: projectData.reduce((acc, cur) => acc + cur.reduce((a, c) => a + c[1], 0), 0),
    len: timestamps.length,
    chart: {
      labels: timestamps.map(labelFn),
      data: chartData.map((x) => ({ name: x.name, data: x.data })),
      sumData: [
        {
          name: chartName,
          data: timestamps.map((ts) => {
            const entries = projectData.flat().filter(([ets]) => ets === ts);
            return entries.reduce((acc, cur) => acc + cur[1], 0);
          }),
        },
      ],
      colors: projectData.map((_, i) => {
        const project = chartData[i];

        return intToRgba(project.color, project.id, theme);
      }),
      defaultColors: projectData.map((_, i) => {
        const project = chartData[i];
        return getDefaultColor(project.id);
      }),
    },
    projectIds: projectIdsSortedBySum,
  };
};

export const processAnalyticsByCountry = (category, projects, sortFn) => {
  if (!category || !projects) {
    return {
      sum: 0,
      len: 0,
      data: [],
    };
  }

  // Get an intersection of category keys and project ids
  const projectIds = projects.map((p) => p.id);
  const loadedProjectIds = Object.keys(category).filter((id) => projectIds.includes(id));

  if (!loadedProjectIds?.length) {
    return {
      sum: 0,
      len: 0,
      data: [],
    };
  }

  const loadedProjectData = loadedProjectIds.map((id) => category[id]);

  // Convert each project's data into a list of [countrycode, number] pairs
  // Fold into a single list with summed values for each country over all projects

  const countrySums = new Map();

  loadedProjectData.forEach((data) => {
    Object.entries(data).forEach(([country, value]) => {
      const current = countrySums.get(country) || 0;
      countrySums.set(country, current + value);
    });
  });

  const entries = Array.from(countrySums.entries());

  return {
    sum: entries.reduce((acc, cur) => acc + cur[1], 0),
    len: entries.length,
    data: entries.sort(sortFn),
  };
};

const sortCount = ([_a, a], [_b, b]) => b - a;
const sortTimestamp = ([a], [b]) => a - b;
const roundValue = ([ts, value]) => [ts, Math.round(parseFloat(value) * 100) / 100];
const MAX_ANALYTICS_PROJECT_IDS = 100;

const processCountryAnalytics = (c, projects) => processAnalyticsByCountry(c, projects, sortCount);
const processNumberAnalytics = (c, projects, theme) =>
  processAnalytics(c, projects, formatTimestamp, sortTimestamp, null, "Downloads", theme);
const processRevAnalytics = (c, projects, theme) =>
  processAnalytics(c, projects, formatTimestamp, sortTimestamp, roundValue, "Revenue", theme);

const useFetchAnalytics = (
  url,
  baseOptions = {
    apiVersion: 3,
  },
) => {
  return useBaseFetch(url, baseOptions);
};

/**
 * @param {Ref<any[]>} projects
 * @param {undefined | () => any} onDataRefresh
 */
export const useFetchAllAnalytics = (
  onDataRefresh,
  projects,
  selectedProjects,
  personalRevenue = false,
) => {
  const timeResolution = ref(1440); // 1 day
  const timeRange = ref(43200); // 30 days

  const startDate = ref(Date.now() - timeRange.value * 60 * 1000);
  const endDate = ref(Date.now());

  const downloadData = ref(null);
  const viewData = ref(null);
  const revenueData = ref(null);
  const downloadsByCountry = ref(null);
  const viewsByCountry = ref(null);
  const loading = ref(true);
  const error = ref(null);
  const isInitialized = ref(false);

  // 图表聚合单位：31 天内按拉取粒度直出；更长区间按天拉取后聚合为自然周（周一起始）/ 自然月，
  // 保证数据桶与日历对齐（后端的固定分钟窗口会与自然周/月错位）
  const chartUnit = computed(() => {
    const days = (endDate.value - startDate.value) / 86_400_000;
    if (days <= 31) {
      return null;
    }
    return days <= 180 ? "week" : "month";
  });

  const bucketize = (data) =>
    chartUnit.value ? aggregateRawAnalytics(data, chartUnit.value) : data;

  const formattedData = computed(() => ({
    downloads: processNumberAnalytics(bucketize(downloadData.value), selectedProjects.value),
    views: processNumberAnalytics(bucketize(viewData.value), selectedProjects.value),
    revenue: processRevAnalytics(bucketize(revenueData.value), selectedProjects.value),
    downloadsByCountry: processCountryAnalytics(downloadsByCountry.value, selectedProjects.value),
    viewsByCountry: processCountryAnalytics(viewsByCountry.value, selectedProjects.value),
  }));

  const theme = useTheme();

  const totalData = computed(() => ({
    downloads: processNumberAnalytics(downloadData.value, projects.value, theme.active),
    views: processNumberAnalytics(viewData.value, projects.value, theme.active),
    revenue: processRevAnalytics(revenueData.value, projects.value, theme.active),
  }));

  const fetchData = async (query) => {
    const normalQuery = new URLSearchParams(query);
    const revenueQuery = new URLSearchParams(query);

    if (personalRevenue) {
      revenueQuery.delete("project_ids");
    }

    const qs = normalQuery.toString();
    const revenueQs = revenueQuery.toString();

    try {
      loading.value = true;
      error.value = null;

      const responses = await Promise.all([
        useFetchAnalytics(`analytics/downloads?${qs}`),
        useFetchAnalytics(`analytics/views?${qs}`),
        useFetchAnalytics(`analytics/revenue?${revenueQs}`),
        useFetchAnalytics(`analytics/countries/downloads?${qs}`),
        useFetchAnalytics(`analytics/countries/views?${qs}`),
      ]);

      // collect project ids from projects.value into a set
      const projectIds = new Set();
      if (projects.value) {
        projects.value.forEach((p) => projectIds.add(p.id));
      } else {
        // if projects.value is not set, we assume that we want all project ids
        Object.keys(responses[0] || {}).forEach((id) => projectIds.add(id));
      }

      const filterProjectIds = (data) => {
        const filtered = {};
        Object.entries(data).forEach(([id, values]) => {
          if (projectIds.has(id)) {
            filtered[id] = values;
          }
        });
        return filtered;
      };

      downloadData.value = filterProjectIds(responses[0] || {});
      viewData.value = filterProjectIds(responses[1] || {});
      revenueData.value = filterProjectIds(responses[2] || {});

      downloadsByCountry.value = responses[3] || {};
      viewsByCountry.value = responses[4] || {};
    } catch (e) {
      error.value = e;
    } finally {
      loading.value = false;
    }
  };

  watch(
    [
      () => startDate.value,
      () => endDate.value,
      () => timeResolution.value,
      () => projects.value,
      () => isInitialized.value,
    ],
    async () => {
      if (!isInitialized.value) {
        return;
      }

      const spanDays = (endDate.value - startDate.value) / 86_400_000;
      // 超过 31 天的区间固定按天拉取，图表端聚合为自然周/自然月
      const resolutionMinutes = spanDays > 31 ? 1440 : timeResolution.value;
      const q = {
        start_date: dayjs(startDate.value).toISOString(),
        end_date: dayjs(endDate.value).toISOString(),
        resolution_minutes: resolutionMinutes,
      };

      if (projects.value?.length && projects.value.length <= MAX_ANALYTICS_PROJECT_IDS) {
        q.project_ids = JSON.stringify(projects.value.map((p) => p.id));
      }

      await fetchData(q);

      if (onDataRefresh) {
        onDataRefresh();
      }
    },
    {
      immediate: true,
    },
  );

  const validProjectIds = computed(() => {
    const ids = new Set();

    if (downloadData.value) {
      Object.keys(downloadData.value).forEach((id) => ids.add(id));
    }

    if (viewData.value) {
      Object.keys(viewData.value).forEach((id) => ids.add(id));
    }

    if (revenueData.value) {
      // revenue will always have all project ids, but the ids may have an empty object or a ton of keys below a cent (0.00...) as values. We want to filter those out
      Object.entries(revenueData.value).forEach(([id, data]) => {
        if (Object.keys(data).length) {
          if (Object.values(data).some((v) => v >= 0.01)) {
            ids.add(id);
          }
        }
      });
    }

    return Array.from(ids);
  });

  return {
    // Configuration
    timeResolution,
    timeRange,

    startDate,
    endDate,

    // 图表聚合单位：null = 按拉取粒度直出，week/month = 按天数据聚合
    chartUnit,

    // Data
    downloadData,
    viewData,
    revenueData,
    downloadsByCountry,
    viewsByCountry,

    // Computed state
    validProjectIds,
    formattedData,
    totalData,
    loading,
    error,
    isInitialized,
  };
};
