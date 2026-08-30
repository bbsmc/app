<template>
  <div>
    <section class="universal-card">
      <div class="header-section">
        <h2>信息统计</h2>
        <p class="description">查看站点资源、用户与下载概况</p>
      </div>

      <div class="tabs-wrapper">
        <NavTabs :links="tabLinks" query="tab" />
      </div>

      <div v-if="!currentTab">
        <div class="grid-display">
          <div class="grid-display__item">
            <div class="label">资源</div>
            <div class="value">
              {{ formatNumber(stats?.projects, false) }}
            </div>
          </div>
          <div class="grid-display__item">
            <div class="label">版本</div>
            <div class="value">
              {{ formatNumber(stats?.versions, false) }}
            </div>
          </div>
          <div class="grid-display__item">
            <div class="label">文件</div>
            <div class="value">
              {{ formatNumber(stats?.files, false) }}
            </div>
          </div>
          <div class="grid-display__item">
            <div class="label">创作者</div>
            <div class="value">
              {{ formatNumber(stats?.authors, false) }}
            </div>
          </div>
          <div class="grid-display__item">
            <div class="label">注册用户</div>
            <div class="value">
              {{ formatNumber(stats?.users, false) }}
            </div>
          </div>
        </div>
      </div>

      <RegistrationsPanel v-else-if="currentTab === 'registrations'" />
      <DownloadsPanel v-else-if="currentTab === 'downloads'" />
      <TopProjectsPanel v-else-if="currentTab === 'top-projects'" />
    </section>
  </div>
</template>

<script setup>
import { computed } from "vue";
import { formatNumber } from "~/plugins/shorthands.js";
import NavTabs from "~/components/ui/NavTabs.vue";
import RegistrationsPanel from "~/components/ui/moderation-stats/RegistrationsPanel.vue";
import DownloadsPanel from "~/components/ui/moderation-stats/DownloadsPanel.vue";
import TopProjectsPanel from "~/components/ui/moderation-stats/TopProjectsPanel.vue";

useHead({
  title: "信息统计 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const route = useNativeRoute();

const currentTab = computed(() => {
  const tab = route.query.tab;
  if (tab === "registrations" || tab === "downloads" || tab === "top-projects") {
    return tab;
  }
  return "";
});

const tabLinks = computed(() => [
  { label: "概览", href: "" },
  { label: "注册趋势", href: "registrations" },
  { label: "下载分析", href: "downloads" },
  { label: "资源排行", href: "top-projects" },
]);

const { data: stats } = await useAsyncData("statistics", () => useBaseFetch("statistics"));
</script>

<style scoped lang="scss">
.header-section {
  margin-bottom: var(--gap-md);

  h2 {
    margin: 0;
  }

  .description {
    margin: var(--gap-xs) 0 0;
    color: var(--color-secondary);
  }
}

.tabs-wrapper {
  margin-bottom: var(--gap-lg);
}
</style>
