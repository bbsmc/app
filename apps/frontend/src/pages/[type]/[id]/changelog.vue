<template>
  <div class="content">
    <div class="mb-3 flex">
      <VersionFilterControl :versions="props.versions" @switch-page="switchPage" />
      <Pagination
        :page="currentPage"
        :count="Math.ceil(filteredVersions.length / 20)"
        class="ml-auto mt-auto"
        :link-function="(page) => `?page=${page}`"
        @switch-page="switchPage"
      />
    </div>
    <div class="card changelog-wrapper">
      <SpinnerIcon v-if="!paginatedVersions" class="mx-auto my-4 animate-spin" />
      <div v-for="version in paginatedVersions ?? []" :key="version.id" class="changelog-item">
        <div
          :class="`changelog-bar ${version.version_type} ${version.duplicate ? 'duplicate' : ''}`"
        />
        <div class="version-wrapper">
          <div class="version-header">
            <div class="version-header-text">
              <h2 class="name">
                <nuxt-link
                  :to="`/${props.project.project_type}/${
                    props.project.slug ? props.project.slug : props.project.id
                  }/version/${encodeURI(version.displayUrlEnding)}`"
                >
                  {{ version.name }}
                </nuxt-link>
              </h2>
              <span v-if="version.author">
                by
                <nuxt-link class="text-link" :to="'/user/' + version.author.user.username">{{
                  version.author.user.username
                }}</nuxt-link>
              </span>
              <span>
                on
                {{ $dayjs(version.date_published).format("YYYY MMM DD") }}</span
              >
            </div>
            <a
              :href="version.primaryFile.url"
              class="iconified-button download"
              :title="`Download ${version.name}`"
            >
              <DownloadIcon aria-hidden="true" />
              下载
            </a>
          </div>
          <div
            v-if="version.changelog && !version.duplicate"
            class="markdown-body"
            v-html="renderHighlightedString(version.changelog)"
          />
        </div>
      </div>
    </div>
    <Pagination
      :page="currentPage"
      :count="Math.ceil(filteredVersions.length / 20)"
      class="mb-2 flex justify-end"
      :link-function="(page) => `?page=${page}`"
      @switch-page="switchPage"
    />
  </div>
</template>
<script setup>
import { Pagination } from "@modrinth/ui";
import { DownloadIcon, UpdatedIcon as SpinnerIcon } from "@modrinth/assets";

import { renderHighlightedString } from "~/helpers/highlight.js";
import VersionFilterControl from "~/components/ui/VersionFilterControl.vue";

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {};
    },
  },
  versions: {
    type: Array,
    default() {
      return [];
    },
  },
  members: {
    type: Array,
    default() {
      return [];
    },
  },
});

const title = `${props.project.title} 更新日志 - 我的世界资源版本记录 | BBSMC`;
const versionCount = props.versions.length;
const description =
  versionCount > 0
    ? `查看 ${props.project.title} 的完整更新日志，按时间追踪 ${versionCount} 个版本的新增内容、功能改动、问题修复和兼容性调整，了解每次更新支持的 Minecraft 游戏版本与加载器。在 BBSMC 掌握这款我的世界资源的开发进展和最新变化。`
    : `查看 ${props.project.title} 的更新日志和开发进展，创作者发布新版本后，新增内容、功能改动和问题修复都会在这里按时间记录。在 BBSMC 关注这款 Minecraft 我的世界资源的动态，第一时间获取最新版本信息。`;

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: props.project.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
});

const router = useNativeRouter();
const route = useNativeRoute();

// useNativeRoute 是全局实时路由：离开本页时（目标页 async setup 未完成，本页仍挂着）冻结在本页的 query，
// 免得为正在离开的页面按新路由重新补取、闪 Spinner
const pagePath = route.path;
const pageQuery = computed((previous) => (route.path === pagePath ? route.query : previous));

const currentPage = ref(Number(route.query.page ?? 1));
const filteredVersions = computed(() => {
  const selectedGameVersions = getArrayOrString(pageQuery.value.g) ?? [];
  const selectedLoaders = getArrayOrString(pageQuery.value.l) ?? [];
  const selectedVersionTypes = getArrayOrString(pageQuery.value.c) ?? [];

  return props.versions.filter(
    (projectVersion) =>
      (selectedGameVersions.length === 0 ||
        selectedGameVersions.some((gameVersion) =>
          projectVersion.game_versions.includes(gameVersion),
        )) &&
      (selectedLoaders.length === 0 ||
        selectedLoaders.some((loader) => projectVersion.loaders.includes(loader))) &&
      (selectedVersionTypes.length === 0 ||
        selectedVersionTypes.includes(projectVersion.version_type)),
  );
});

// 筛选变化时在同一拍里回到第 1 页：VersionFilterControl 先改 query（保留 ?page=N）再 emit switch-page(1)，
// 分两拍的话会先按「新筛选 + 旧页码」多补取一次
watch(
  () => JSON.stringify([pageQuery.value.g, pageQuery.value.l, pageQuery.value.c]),
  () => {
    currentPage.value = 1;
  },
  { flush: "sync" },
);

// 上游 4871abfb3 + aec268c6e (#5121/#5146): 列表不再带 changelog，这里按当前页（20 条）
// 补取完整版本（含 changelog）并计算 duplicate，渲染 paginatedVersions。
// BBSMC: 用 useAsyncData 在服务端补取并渲染，更新日志正文对搜索引擎可见；结果随 payload 带到客户端，
// 水合时直接复用、不再请求；翻页/筛选时客户端按新 ids 重新补取。payload 只多当前页 20 条 changelog。
const pageVersions = computed(() =>
  filteredVersions.value.slice((currentPage.value - 1) * 20, currentPage.value * 20),
);
// watch 源用 ids 拼成的字符串：route.query 整体替换（如翻页写 ?page=）会让数组重新计算，
// 字符串不变就不重复请求
const pageVersionIds = computed(() => pageVersions.value.map((v) => v.id).join(","));

const {
  data: pageChangelogs,
  status: pageChangelogsStatus,
  refresh: refreshPageChangelogs,
} = useAsyncData(
  `project/${props.project.id}/changelog`,
  async () => {
    // 非规范 URL 会被 [id].vue 301 到规范地址、这次服务端渲染结果直接丢弃，不必补取（条件与 [id].vue 一致）
    if (
      import.meta.server &&
      (props.project.project_type !== route.params.type || route.params.id !== props.project.slug)
    ) {
      return {};
    }
    const ids = pageVersions.value.map((v) => v.id);
    if (ids.length === 0) {
      return {};
    }
    const fullVersions = await useBaseFetch("versions", {
      apiVersion: 3,
      query: { ids: JSON.stringify(ids) },
    });
    // 只留 id -> changelog，不把整份版本对象重复塞进 payload；接口没返回的版本按空 changelog 处理
    const changelogById = new Map(fullVersions.map((v) => [v.id, v.changelog ?? ""]));
    return Object.fromEntries(ids.map((id) => [id, changelogById.get(id) ?? ""]));
  },
  // lazy：服务端仍经 onServerPrefetch 等数据再渲染（SEO 不受影响）；客户端切到本 tab 不再被补取阻塞，先显示 Spinner
  { lazy: true, watch: [pageVersionIds] },
);

// 服务端补取失败时，错误随 payload 带到客户端、水合不会重试：挂载后补试一次
onMounted(() => {
  if (pageChangelogsStatus.value === "error") {
    refreshPageChangelogs();
  }
});

const paginatedVersions = computed(() => {
  const changelogs = pageChangelogs.value ?? {};
  // 当前页还没补取到（加载中，含失败后的重试）时显示 Spinner；补取已失败则照常列出版本、不带正文
  if (
    pageChangelogsStatus.value !== "error" &&
    !pageVersions.value.every((v) => v.id in changelogs)
  ) {
    return null;
  }
  return pageVersions.value.map((version, index) => {
    const changelog = changelogs[version.id] ?? "";
    const next = pageVersions.value[index + 1];
    return {
      ...version,
      // 与紧邻的下一个（更旧的）版本 changelog 相同才算重复，正文只显示在这一段的最后一条（同 HEAD 的 computeVersions）
      duplicate: !!changelog && !!next && changelogs[next.id] === changelog,
      changelog,
    };
  });
});

function switchPage(page) {
  currentPage.value = page;

  router.replace({
    query: {
      ...route.query,
      page: currentPage.value !== 1 ? currentPage.value : undefined,
    },
  });
}
</script>

<style lang="scss">
.changelog-wrapper {
  padding-bottom: calc(var(--spacing-card-md) + 0.5rem);
}

.changelog-item {
  display: block;
  margin-bottom: 1rem;
  position: relative;
  padding-left: 1.8rem;

  &:last-child {
    .changelog-bar.duplicate {
      height: 100%;
      background: transparent;
    }
  }

  .changelog-bar {
    --color: var(--color-green);

    &.alpha {
      --color: var(--color-red);
    }

    &.release {
      --color: var(--color-green);
    }

    &.beta {
      --color: var(--color-orange);
    }

    left: 0;
    top: 0.5rem;
    width: 0.2rem;
    min-width: 0.2rem;
    position: absolute;
    margin: 0 0.4rem;
    border-radius: var(--size-rounded-max);
    min-height: 100%;
    background-color: var(--color);

    &:before {
      content: "";
      width: 1rem;
      height: 1rem;
      position: absolute;
      top: 0;
      left: -0.4rem;
      border-radius: var(--size-rounded-max);
      background-color: var(--color);
    }

    &.duplicate {
      background: linear-gradient(
        to bottom,
        transparent,
        transparent 30%,
        var(--color) 30%,
        var(--color)
      );
      background-size: 100% 10px;
    }

    &.duplicate {
      height: calc(100% + 1.5rem);
    }
  }

  .markdown-body {
    margin: 0.5rem 0.5rem 0 0;
  }

  .version-header {
    display: flex;
    align-items: center;
    margin-top: 0.2rem;

    .circle {
      min-width: 0.75rem;
      min-height: 0.75rem;
      border-radius: 50%;
      display: inline-block;
      margin-right: 0.25rem;
    }

    .version-header-text {
      display: flex;
      align-items: baseline;
      flex-wrap: wrap;

      h2 {
        margin: 0;
        font-size: var(--font-size-lg);
      }

      h2,
      span {
        padding-right: 0.25rem;
      }
    }

    .download {
      margin-left: auto;
      display: none;

      @media screen and (min-width: 800px) {
        display: flex;
      }
    }
  }
}

.brand-button {
  color: var(--color-accent-contrast);
}
</style>
