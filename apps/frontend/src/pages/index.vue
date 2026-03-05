<template>
  <div class="home-page">
    <!-- Hero Section -->
    <section class="hero">
      <div class="hero-content">
        <h1 class="hero-title">
          <span class="hero-title-line">发现最好的</span>
          <span class="hero-title-line"><span class="hero-title-highlight">Minecraft</span></span>
          <span class="hero-title-line">中文资源</span>
        </h1>

        <p class="hero-desc">
          探索数十万个模组、整合包、光影和资源包。下载、分享、与百万玩家共建最活跃的中文 MC 社区。
        </p>

        <div class="hero-actions">
          <NuxtLink to="/mods" class="hero-btn hero-btn-primary">开始探索</NuxtLink>
          <NuxtLink to="/dashboard/projects" class="hero-btn hero-btn-secondary"
            >创作者入驻</NuxtLink
          >
        </div>

        <div class="hero-stats">
          <div class="hero-stat">
            <div class="hero-stat-value">{{ formatStatNumber(stats.projects) }}+</div>
            <div class="hero-stat-label">资源总数</div>
          </div>
          <div class="hero-stat">
            <div class="hero-stat-value">{{ formatStatNumber(stats.downloads) }}+</div>
            <div class="hero-stat-label">累计下载</div>
          </div>
          <div class="hero-stat">
            <div class="hero-stat-value">{{ formatStatNumber(stats.users) }}</div>
            <div class="hero-stat-label">注册用户</div>
          </div>
        </div>
      </div>

      <div class="hero-visual">
        <div
          class="hero-banner"
          :class="isDragging ? 'cursor-grabbing' : 'cursor-grab'"
          @mouseenter="handleMouseEnter"
          @mouseleave="handleMouseLeave"
          @mousedown="handleDragStart"
          @touchstart="handleDragStart"
          @touchmove="handleDragMove"
          @touchend="handleDragEnd"
        >
          <div
            v-for="(item, index) in heroBanners"
            :key="index"
            :class="[
              'hero-slide',
              {
                active: index === currentHeroSlide,
              },
            ]"
            @click="handleBannerClick($event, index)"
          >
            <img
              :src="item.image"
              :alt="item.title"
              class="hero-slide-image"
              :class="{ 'scale-effect': !isDragging }"
              draggable="false"
              width="800"
              height="400"
              loading="eager"
              fetchpriority="high"
            />
            <div class="hero-slide-overlay">
              <span class="hero-slide-badge">{{ item.badge }}</span>
              <h3 class="hero-slide-title">{{ item.title }}</h3>
              <p class="hero-slide-desc">{{ item.description }}</p>
            </div>
          </div>
          <div class="hero-banner-nav">
            <button
              v-for="(_, index) in heroBanners"
              :key="index"
              :class="['hero-nav-dot', { active: currentHeroSlide === index }]"
              @click.stop="goToHeroSlide(index)"
            ></button>
          </div>
        </div>
      </div>
    </section>

    <!-- Categories -->
    <section class="categories">
      <div class="categories-track">
        <NuxtLink to="/" class="cat-chip active"><StarIcon class="cat-icon" /> 热门推荐</NuxtLink>
        <NuxtLink to="/mods" class="cat-chip"><BoxIcon class="cat-icon" /> 模组</NuxtLink>
        <NuxtLink to="/plugins" class="cat-chip"><PlugIcon class="cat-icon" /> 插件</NuxtLink>
        <NuxtLink to="/modpacks" class="cat-chip"
          ><PackageClosedIcon class="cat-icon" /> 整合包</NuxtLink
        >
        <NuxtLink to="/shaders" class="cat-chip"><GlassesIcon class="cat-icon" /> 光影</NuxtLink>
        <NuxtLink to="/resourcepacks" class="cat-chip"
          ><PaintBrushIcon class="cat-icon" /> 资源包</NuxtLink
        >
        <NuxtLink to="/languages" class="cat-chip"
          ><LanguagesIcon class="cat-icon" /> 汉化</NuxtLink
        >
        <NuxtLink to="/datapacks" class="cat-chip"><BracesIcon class="cat-icon" /> 数据包</NuxtLink>
        <NuxtLink to="/softwares" class="cat-chip"><WrenchIcon class="cat-icon" /> 工具</NuxtLink>
      </div>
    </section>

    <!-- Main Content -->
    <main class="main-content">
      <div class="content-area">
        <!-- Hot Resources -->
        <section class="section">
          <header class="section-header">
            <div class="section-title-group">
              <span class="section-label">{{ searchQuery ? "SEARCH" : "TRENDING" }}</span>
              <h2 class="section-title">{{ searchQuery ? "搜索结果" : "热门资源" }}</h2>
            </div>
            <div class="section-header-right">
              <div class="home-search">
                <SearchIcon class="home-search-icon" aria-hidden="true" />
                <input
                  v-model="searchQuery"
                  type="search"
                  placeholder="搜索资源..."
                  class="home-search-input"
                  autocomplete="off"
                />
                <button v-if="searchQuery" class="home-search-clear" @click="searchQuery = ''">
                  <XIcon />
                </button>
              </div>
              <NuxtLink v-if="!searchQuery" to="/mods" class="section-more">浏览全部 →</NuxtLink>
            </div>
          </header>

          <!-- 搜索加载状态 -->
          <div v-if="searchLoading" class="search-loading">
            <span class="search-loading-spinner" />
            <span>搜索中...</span>
          </div>

          <!-- 搜索无结果 -->
          <div v-else-if="searchQuery && displayProjects.length === 0" class="search-empty">
            <SearchIcon class="search-empty-icon" />
            <p>未找到与「{{ searchQuery }}」相关的资源</p>
            <button class="search-empty-btn" @click="searchQuery = ''">清除搜索</button>
          </div>

          <div v-else class="resource-grid">
            <NuxtLink
              v-for="project in displayProjects"
              :key="project.project_id"
              :to="getProjectLink(project)"
              class="resource-card"
            >
              <!-- Gallery Image -->
              <div class="resource-gallery" :style="getGalleryStyle(project)">
                <div class="resource-type-badge">
                  {{ getProjectTypeLabel(project.project_type) }}
                </div>
              </div>

              <!-- Card Body -->
              <div class="resource-body">
                <!-- Header: Icon + Title -->
                <div class="resource-header">
                  <img
                    v-if="project.icon_url"
                    :src="project.icon_url"
                    :alt="project.title"
                    class="resource-icon"
                    loading="lazy"
                    width="48"
                    height="48"
                  />
                  <span v-else class="resource-icon-placeholder"><PackageClosedIcon /></span>
                  <div class="resource-title-wrap">
                    <div class="resource-name">{{ project.title }}</div>
                    <div class="resource-author">by {{ project.author }}</div>
                  </div>
                </div>

                <!-- Description -->
                <p class="resource-desc">{{ project.description }}</p>

                <!-- Tags: Loaders + Version -->
                <div class="resource-tags">
                  <span
                    v-for="loader in (project.loaders || []).slice(0, 3)"
                    :key="loader"
                    class="tag loader"
                  >
                    {{ formatLoader(loader) }}
                  </span>
                  <span v-if="project.versions?.[0]" class="tag version">{{
                    project.versions[0]
                  }}</span>
                </div>

                <!-- Footer Stats -->
                <div class="resource-footer">
                  <div class="resource-stat">
                    <DownloadIcon class="stat-icon" />
                    <span>{{ formatNumber(project.downloads) }}</span>
                  </div>
                  <div class="resource-stat">
                    <HeartIcon class="stat-icon" />
                    <span>{{ formatNumber(project.follows) }}</span>
                  </div>
                  <div class="resource-stat update">
                    <UpdatedIcon class="stat-icon" />
                    <span>{{ formatDate(project.date_modified) }}</span>
                  </div>
                </div>
              </div>
            </NuxtLink>
          </div>
        </section>

        <!-- 热门汉化包 -->
        <section v-if="latestModpackTranslations.length > 0" class="section">
          <header class="section-header">
            <div class="section-title-group">
              <span class="section-label">HOT</span>
              <h2 class="section-title">热门汉化包</h2>
            </div>
            <NuxtLink to="/languages" class="section-more">浏览全部 →</NuxtLink>
          </header>

          <div class="resource-grid">
            <NuxtLink
              v-for="project in latestModpackTranslations"
              :key="project.project_id"
              :to="getProjectLink(project)"
              class="resource-card"
            >
              <!-- Gallery Image -->
              <div class="resource-gallery" :style="getGalleryStyle(project)">
                <div class="resource-type-badge">汉化</div>
              </div>

              <!-- Card Body -->
              <div class="resource-body">
                <!-- Header: Icon + Title -->
                <div class="resource-header">
                  <img
                    v-if="project.icon_url"
                    :src="project.icon_url"
                    :alt="project.title"
                    class="resource-icon"
                    loading="lazy"
                    width="48"
                    height="48"
                  />
                  <span v-else class="resource-icon-placeholder"><PackageClosedIcon /></span>
                  <div class="resource-title-wrap">
                    <div class="resource-name">{{ project.title }}</div>
                    <div class="resource-author">by {{ project.author }}</div>
                  </div>
                </div>

                <!-- Description -->
                <p class="resource-desc">{{ project.description }}</p>

                <!-- Tags: Loaders + Version -->
                <div class="resource-tags">
                  <span
                    v-for="loader in (project.loaders || []).slice(0, 3)"
                    :key="loader"
                    class="tag loader"
                  >
                    {{ formatLoader(loader) }}
                  </span>
                  <span v-if="project.versions?.[0]" class="tag version">{{
                    project.versions[0]
                  }}</span>
                </div>

                <!-- Footer Stats -->
                <div class="resource-footer">
                  <div class="resource-stat">
                    <DownloadIcon class="stat-icon" />
                    <span>{{ formatNumber(project.downloads) }}</span>
                  </div>
                  <div class="resource-stat">
                    <HeartIcon class="stat-icon" />
                    <span>{{ formatNumber(project.follows) }}</span>
                  </div>
                  <div class="resource-stat update">
                    <UpdatedIcon class="stat-icon" />
                    <span>{{ formatDate(project.date_modified) }}</span>
                  </div>
                </div>
              </div>
            </NuxtLink>
          </div>
        </section>

        <!-- 社区资讯 - 暂时隐藏
        <section v-if="notices.length > 0" class="section">
          <header class="section-header">
            <div class="section-title-group">
              <span class="section-label">NEWS</span>
              <h2 class="section-title">社区资讯</h2>
            </div>
            <NuxtLink to="/forums/notice" class="section-more">查看全部 →</NuxtLink>
          </header>

          <div class="notice-list">
            <NuxtLink
              v-for="notice in notices"
              :key="notice.id"
              :to="`/d/${notice.id}`"
              class="notice-item"
            >
              <img :src="notice.avatar" :alt="notice.user_name" class="notice-avatar" />
              <div class="notice-content">
                <div class="notice-title">{{ notice.title }}</div>
                <div class="notice-meta">
                  <span class="notice-author">{{ notice.user_name }}</span>
                  <span class="notice-time">{{ fromNow(notice.last_post_time) }}</span>
                </div>
              </div>
            </NuxtLink>
          </div>
        </section>
        -->

        <!-- 热门讨论 - 暂时隐藏
        <section class="section">
          <header class="section-header">
            <div class="section-title-group">
              <span class="section-label">COMMUNITY</span>
              <h2 class="section-title">热门讨论</h2>
            </div>
            <NuxtLink to="/forums/chat" class="section-more">进入论坛 →</NuxtLink>
          </header>

          <div class="discussion-list">
            <NuxtLink
              v-for="forum in forums"
              :key="forum.id"
              :to="forum.project_id ? `/project/${forum.project_id}/forum` : `/d/${forum.id}`"
              class="discussion-item"
            >
              <img :src="forum.avatar" :alt="forum.user_name" class="discussion-avatar" />
              <div class="discussion-content">
                <div class="discussion-header">
                  <span :class="['discussion-category', `cat-${forum.category}`]">
                    {{ getCategoryLabel(forum.category) }}
                  </span>
                  <span class="discussion-time">{{ fromNow(forum.last_post_time) }}</span>
                </div>
                <div class="discussion-title">{{ forum.title }}</div>
                <div class="discussion-meta">
                  <span class="discussion-author">{{ forum.user_name }}</span>
                  <span class="discussion-replies"
                    ><MessageIcon class="stat-icon" /> {{ forum.replies }} 回复</span
                  >
                </div>
              </div>
            </NuxtLink>
          </div>
        </section>
        -->
      </div>

      <!-- Sidebar -->
      <aside class="sidebar">
        <!-- 汉化包推荐 -->
        <div v-if="translations.length > 0" class="sidebar-card">
          <div class="sidebar-header">
            <h3 class="sidebar-title"><LanguagesIcon class="sidebar-icon" /> 最近更新汉化</h3>
            <NuxtLink to="/languages" class="sidebar-more">更多 →</NuxtLink>
          </div>
          <div class="sidebar-body">
            <div class="translation-list">
              <NuxtLink
                v-for="item in translations"
                :key="item.project_id"
                :to="`/language/${item.slug}`"
                class="translation-item"
                :title="item.title"
              >
                <img
                  :src="item.icon_url"
                  :alt="item.title"
                  class="translation-icon"
                  loading="lazy"
                  width="40"
                  height="40"
                />
                <div class="translation-info">
                  <div class="translation-title">{{ item.title }}</div>
                  <div class="translation-meta">
                    <span class="meta-item">
                      <DownloadIcon class="stat-icon" /> {{ formatNumber(item.downloads) }}
                    </span>
                    <span class="meta-item">
                      <UpdatedIcon class="stat-icon" /> {{ fromNow(item.date_modified) }}
                    </span>
                  </div>
                </div>
              </NuxtLink>
            </div>
          </div>
        </div>

        <!-- 测评专栏 - 暂时隐藏
        <div class="sidebar-card">
          <div class="sidebar-header">
            <h3 class="sidebar-title"><StarIcon class="sidebar-icon" /> 测评专栏</h3>
            <NuxtLink to="/forums/article" class="sidebar-more">更多 →</NuxtLink>
          </div>
          <div class="sidebar-body">
            <div class="article-list">
              <NuxtLink
                v-for="article in articles"
                :key="article.id"
                :to="`/d/${article.id}`"
                class="article-item"
              >
                <img :src="article.avatar" :alt="article.user_name" class="article-avatar" />
                <div class="article-info">
                  <div class="article-title">{{ article.title }}</div>
                  <div class="article-meta">
                    {{ article.user_name }} · {{ fromNow(article.last_post_time) }}
                  </div>
                </div>
              </NuxtLink>
            </div>
          </div>
        </div>
        -->

        <!-- 服务器插件推荐 -->
        <div v-if="latestPlugins.length > 0" class="sidebar-card plugin-recommend-card">
          <div class="sidebar-header plugin-header">
            <h3 class="sidebar-title"><PlugIcon class="sidebar-icon" /> 服务器插件</h3>
            <NuxtLink to="/plugins" class="sidebar-more">更多 →</NuxtLink>
          </div>
          <div class="mini-resource-list">
            <NuxtLink
              v-for="plugin in latestPlugins"
              :key="plugin.project_id"
              :to="getProjectLink(plugin)"
              class="mini-resource-item"
            >
              <img
                v-if="plugin.icon_url"
                :src="plugin.icon_url"
                :alt="plugin.title"
                class="mini-resource-icon"
                loading="lazy"
                width="48"
                height="48"
              />
              <span v-else class="mini-resource-icon-placeholder"><PlugIcon /></span>
              <div class="mini-resource-info">
                <div class="mini-resource-title">{{ plugin.title }}</div>
                <div class="mini-resource-author">
                  {{ plugin.author === "BBSMC" ? "社区搬运" : `by ${plugin.author}` }}
                </div>
                <div class="mini-resource-desc">{{ plugin.description }}</div>
                <div class="mini-resource-tags">
                  <span
                    v-for="loader in (plugin.loaders || []).slice(0, 2)"
                    :key="loader"
                    class="mini-tag"
                  >
                    {{ formatLoader(loader) }}
                  </span>
                  <span v-if="plugin.versions?.[0]" class="mini-tag version-tag">
                    {{ plugin.versions[0] }}
                  </span>
                </div>
                <div class="mini-resource-stats">
                  <span class="mini-stat-item">
                    <DownloadIcon class="mini-stat-icon" />
                    {{ formatNumber(plugin.downloads) }}
                  </span>
                  <span class="mini-stat-item">
                    <HeartIcon class="mini-stat-icon" />
                    {{ formatNumber(plugin.follows) }}
                  </span>
                  <span class="mini-stat-item update-time">
                    <UpdatedIcon class="mini-stat-icon" />
                    {{ formatDate(plugin.date_modified) }}
                  </span>
                </div>
              </div>
            </NuxtLink>
          </div>
        </div>

        <!-- 友情链接 -->
        <div class="sidebar-card">
          <div class="sidebar-header">
            <h3 class="sidebar-title"><LinkIcon class="sidebar-icon" /> 友情链接</h3>
          </div>
          <div class="quick-links">
            <a
              href="https://www.mcmod.cn/"
              target="_blank"
              rel="nofollow noopener"
              class="quick-link"
            >
              <img
                src="https://www.mcmod.cn/images/links/mcmod.gif"
                alt="MC百科"
                class="friend-link-logo"
                loading="lazy"
                width="88"
                height="31"
              />
              <span class="quick-link-text">MC百科</span>
            </a>
            <a
              href="https://bbs.mc9y.net/"
              target="_blank"
              rel="nofollow noopener"
              class="quick-link"
            >
              <img
                src="https://bbs.mc9y.net/styles/io_dark/io/images/logo.png"
                alt="九域资源社区"
                class="friend-link-logo"
                loading="lazy"
                width="88"
                height="31"
              />
              <span class="quick-link-text">九域资源社区</span>
            </a>
          </div>
        </div>
      </aside>
    </main>
  </div>
</template>

<script setup>
import dayjs from "dayjs";
import {
  BoxIcon,
  PackageClosedIcon,
  LanguagesIcon,
  StarIcon,
  PlugIcon,
  GlassesIcon,
  PaintBrushIcon,
  BracesIcon,
  WrenchIcon,
  LinkIcon,
  HeartIcon,
  DownloadIcon,
  UpdatedIcon,
  SearchIcon,
  XIcon,
} from "@modrinth/assets";

useSeoMeta({
  title: "BBSMC - 我的世界中文资源社区 | Minecraft 模组、整合包、光影下载",
  description:
    "BBSMC 是最活跃的 Minecraft 我的世界中文资源社区。提供海量模组、整合包、光影、资源包、数据包和插件的免费下载，支持 Forge、Fabric、NeoForge 等主流加载器。加入百万玩家共同打造最好的中文 Minecraft 内容平台。",
  ogTitle: "BBSMC - 我的世界中文资源社区 | Minecraft 模组、整合包、光影下载",
  ogDescription:
    "BBSMC 是最活跃的 Minecraft 我的世界中文资源社区。提供海量模组、整合包、光影、资源包、数据包和插件的免费下载，支持 Forge、Fabric、NeoForge 等主流加载器。加入百万玩家共同打造最好的中文 Minecraft 内容平台。",
  ogImage: "https://cdn.bbsmc.net/raw/bbsmc-logo.png",
});

useHead({
  script: [
    {
      type: "application/ld+json",
      children: JSON.stringify({
        "@context": "https://schema.org",
        "@type": "WebSite",
        name: "BBSMC",
        alternateName: "BBSMC 我的世界资源社区",
        url: "https://bbsmc.net",
        description:
          "BBSMC 是最活跃的 Minecraft 我的世界中文资源社区，提供模组、整合包、光影、资源包和插件下载。",
        potentialAction: {
          "@type": "SearchAction",
          target: "https://bbsmc.net/mods?q={search_term_string}",
          "query-input": "required name=search_term_string",
        },
      }),
    },
  ],
});

// 使用 useAsyncData 获取所有主页数据，支持 SSR
const { data: pageData } = await useAsyncData("homepage-data", async () => {
  const [
    projectsResponse,
    forumsResponse,
    noticesResponse,
    articlesResponse,
    translationsResponse,
    latestModpackTranslationsResponse,
    latestPluginsResponse,
  ] = await Promise.all([
    useBaseFetch(`search?limit=6&index=relevance`),
    useBaseFetch(`forum`, { apiVersion: 3 }),
    useBaseFetch(`forum/notice/lists`, { apiVersion: 3 }),
    useBaseFetch(`forum/article/lists`, { apiVersion: 3 }),
    useBaseFetch(`search?limit=5&index=updated&facets=[["project_type:language"]]`),
    useBaseFetch(`search?limit=6&index=relevance&facets=[["project_type:language"]]`),
    useBaseFetch(`search?limit=5&index=updated&facets=[["project_type:plugin"]]`),
  ]);

  return {
    hotProjects: projectsResponse.hits ?? [],
    forums: (forumsResponse.forums ?? []).slice(0, 5),
    notices: (noticesResponse.forums ?? []).slice(0, 5),
    articles: (articlesResponse.forums ?? []).slice(0, 3),
    translations: translationsResponse.hits ?? [],
    latestModpackTranslations: latestModpackTranslationsResponse.hits ?? [],
    latestPlugins: (latestPluginsResponse.hits ?? []).map((plugin) => ({
      ...plugin,
      project_type: plugin.project_type || "plugin", // 确保插件类型正确
    })),
  };
});

// 从 pageData 中提取数据
const hotProjects = computed(() => pageData.value?.hotProjects ?? []);
// forums, notices, articles available from pageData if needed
const translations = computed(() => pageData.value?.translations ?? []);
const latestModpackTranslations = computed(() => pageData.value?.latestModpackTranslations ?? []);
const latestPlugins = computed(() => pageData.value?.latestPlugins ?? []);

// 搜索功能
const searchQuery = ref("");
const searchResults = ref([]);
const searchLoading = ref(false);
let searchDebounceTimer = null;

const displayProjects = computed(() => {
  if (searchQuery.value && searchResults.value.length > 0) {
    return searchResults.value;
  }
  return hotProjects.value;
});

const performSearch = async (query) => {
  if (!query.trim()) {
    searchResults.value = [];
    searchLoading.value = false;
    return;
  }
  searchLoading.value = true;
  try {
    const res = await useBaseFetch(
      `search?query=${encodeURIComponent(query.trim())}&limit=12&index=relevance`,
    );
    searchResults.value = res.hits ?? [];
  } catch {
    searchResults.value = [];
  } finally {
    searchLoading.value = false;
  }
};

watch(searchQuery, (val) => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  if (!val.trim()) {
    searchResults.value = [];
    searchLoading.value = false;
    return;
  }
  searchLoading.value = true;
  searchDebounceTimer = setTimeout(() => performSearch(val), 300);
});

const stats = ref({
  projects: 12580,
  downloads: 1580000,
  users: 89234,
  modpacks: 1247,
  mods: 3842,
  languages: 4521,
  forums: 28000,
});

// Hero Banners - 来自整合包页面的内容
const heroBanners = ref([
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/G23dLUsP/images/e681d996cd07316e12facedd8fb22e9f74ce68a1_350.webp",
    title: "剑与王国",
    description: "围绕模拟殖民地与村民招募玩法的深度魔改整合包",
    badge: "热门整合包",
    slug: "/modpack/snk",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/EIrkPpcm/images/7d43813f0ff22b6c769e7382d36d5059657e8a94_350.webp",
    title: "龙之冒险：新征程",
    description: "面对众多怪物的冒险之旅，你做好准备了吗？",
    badge: "精选整合包",
    slug: "/modpack/lzmx",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/XMUypeti/images/82d38f228afad3b75202eaf8a148c1318a8cea48_350.webp",
    title: "愚者 - The Fool",
    description: "愚弄、伪装、欺诈，屠龙者终成恶龙。",
    badge: "精选整合包",
    slug: "/modpack/the-fool",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/e11vzqXl/images/346fd8930411f592c94acce68b8290a5266843e3_350.webp",
    title: "香草纪元:食旅纪行",
    description: "农夫乐事全附属与异界冒险",
    badge: "热门整合包",
    slug: "/modpack/vefc",
  },
]);

const currentHeroSlide = ref(0);
const autoPlayInterval = ref(null);
const isClient = ref(false);
const isDragging = ref(false);
const dragStartX = ref(0);
const dragCurrentX = ref(0);
const hasDragged = ref(false);

// Time formatting
const fromNow = (date) => {
  const currentDate = useCurrentDate();
  return dayjs(date).from(currentDate.value);
};

const formatDate = (date) => {
  return dayjs(date).format("YYYY-MM-DD");
};

// Number formatting
const formatNumber = (num) => {
  if (!num) return "0";
  if (num >= 10000) {
    return (num / 10000).toFixed(1).replace(/\.0$/, "") + "万";
  }
  return num.toLocaleString();
};

const formatStatNumber = (num) => {
  if (!num) return "0";
  if (num >= 10000) {
    return (num / 10000).toFixed(0) + "万";
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1).replace(/\.0$/, "") + "k";
  }
  return num.toLocaleString();
};

// Project link
const getProjectLink = (project) => {
  const typeMap = {
    mod: "mod",
    modpack: "modpack",
    plugin: "plugin",
    resourcepack: "resourcepack",
    shader: "shader",
    datapack: "datapack",
    software: "software",
    language: "language",
  };

  // 获取项目类型,如果没有则使用 mod 作为默认值
  let projectType = project.project_type;

  // 如果 project_type 不存在,尝试从其他字段推断
  if (!projectType && project.loaders) {
    // 根据 loaders 判断类型
    const loaders = Array.isArray(project.loaders) ? project.loaders : [];
    if (
      loaders.some((l) =>
        ["bukkit", "spigot", "paper", "purpur", "folia"].includes(l.toLowerCase()),
      )
    ) {
      projectType = "plugin";
    }
  }

  const type = typeMap[projectType] || projectType || "mod";
  return `/${type}/${project.slug || project.project_id}`;
};

// Project type label
const getProjectTypeLabel = (type) => {
  const labels = {
    mod: "模组",
    modpack: "整合包",
    plugin: "插件",
    resourcepack: "资源包",
    shader: "光影",
    datapack: "数据包",
    software: "软件",
    language: "汉化",
  };
  return labels[type] || type;
};

// Get gallery image URL (handles different API response formats)
const getGalleryStyle = (project) => {
  // Try different possible gallery formats
  let imageUrl = null;

  // Format 1: gallery is array of strings
  if (project.gallery?.[0] && typeof project.gallery[0] === "string") {
    imageUrl = project.gallery[0];
  }
  // Format 2: gallery is array of objects with url property
  else if (project.gallery?.[0]?.url) {
    imageUrl = project.gallery[0].url;
  }
  // Format 3: featured_gallery field
  else if (project.featured_gallery) {
    imageUrl = project.featured_gallery;
  }
  // Format 4: icon_url as fallback for software
  else if (project.project_type === "software" && project.icon_url) {
    imageUrl = project.icon_url;
  }

  return imageUrl ? `background-image: url(${imageUrl})` : "";
};

// getCategoryLabel available if needed for forum categories

// Loader formatting
const formatLoader = (loader) => {
  const loaderNames = {
    fabric: "Fabric",
    forge: "Forge",
    neoforge: "NeoForge",
    quilt: "Quilt",
    bukkit: "Bukkit",
    spigot: "Spigot",
    paper: "Paper",
    purpur: "Purpur",
    sponge: "Sponge",
    bungeecord: "BungeeCord",
    velocity: "Velocity",
    waterfall: "Waterfall",
    folia: "Folia",
    canvas: "Canvas",
    iris: "Iris",
    optifine: "OptiFine",
    vanilla: "原版",
  };
  return loaderNames[loader] || loader;
};

// Hero Banner controls
const startAutoPlay = () => {
  if (!isClient.value) return;
  stopAutoPlay();
  autoPlayInterval.value = setInterval(() => {
    currentHeroSlide.value = (currentHeroSlide.value + 1) % heroBanners.value.length;
  }, 5000);
};

const stopAutoPlay = () => {
  if (autoPlayInterval.value) {
    clearInterval(autoPlayInterval.value);
    autoPlayInterval.value = null;
  }
};

const goToHeroSlide = (index) => {
  if (index === currentHeroSlide.value) {
    navigateTo(heroBanners.value[index].slug);
    return;
  }
  currentHeroSlide.value = index;
  startAutoPlay();
};

const prevHeroSlide = () => {
  currentHeroSlide.value =
    (currentHeroSlide.value - 1 + heroBanners.value.length) % heroBanners.value.length;
  startAutoPlay();
};

const nextHeroSlide = () => {
  currentHeroSlide.value = (currentHeroSlide.value + 1) % heroBanners.value.length;
  startAutoPlay();
};

// Banner 拖拽处理函数
const handleDragStart = (e) => {
  const isTouchEvent = e.type.includes("touch");
  isDragging.value = true;
  hasDragged.value = false;
  dragStartX.value = isTouchEvent ? e.touches[0].clientX : e.clientX;
  dragCurrentX.value = dragStartX.value;
  stopAutoPlay();

  if (!isTouchEvent) {
    e.preventDefault();
    document.addEventListener("mousemove", handleDragMove);
    document.addEventListener("mouseup", handleDragEnd);
  }
};

const handleDragMove = (e) => {
  if (!isDragging.value) return;
  const isTouchEvent = e.type.includes("touch");
  const currentX = isTouchEvent ? e.touches[0].clientX : e.clientX;
  dragCurrentX.value = currentX;

  const distance = Math.abs(currentX - dragStartX.value);
  if (distance > 5) {
    hasDragged.value = true;
    e.preventDefault();
  }
};

const handleDragEnd = () => {
  if (!isDragging.value) return;

  const dragDistance = dragCurrentX.value - dragStartX.value;
  const threshold = 50;

  if (Math.abs(dragDistance) > threshold) {
    if (dragDistance > 0) {
      prevHeroSlide();
    } else {
      nextHeroSlide();
    }
  }

  isDragging.value = false;
  startAutoPlay();

  document.removeEventListener("mousemove", handleDragMove);
  document.removeEventListener("mouseup", handleDragEnd);

  setTimeout(() => {
    dragStartX.value = 0;
    dragCurrentX.value = 0;
  }, 10);
};

const handleBannerClick = (e, index) => {
  e.preventDefault();
  e.stopPropagation();

  if (hasDragged.value) {
    hasDragged.value = false;
    return;
  }

  navigateTo(heroBanners.value[index].slug);
};

const handleMouseEnter = () => {
  if (!isClient.value) return;
  stopAutoPlay();
};

const handleMouseLeave = () => {
  if (!isClient.value) return;
  startAutoPlay();
};

onMounted(() => {
  isClient.value = true;
  currentHeroSlide.value = Math.floor(Math.random() * heroBanners.value.length);
  startAutoPlay();
});

onUnmounted(() => {
  stopAutoPlay();
  isClient.value = false;
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
});
</script>

<style scoped lang="scss">
.home-page {
  min-height: 100vh;
}

// ==========================================
// HERO SECTION
// ==========================================
.hero {
  min-height: 90vh;
  padding: 120px 40px 80px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 60px;
  align-items: center;
  max-width: 1600px;
  margin: 0 auto;
  position: relative;
}

.hero-content {
  position: relative;
  z-index: 1;
}

.hero-title {
  font-family: var(--font-display);
  font-size: clamp(2.5rem, 5vw, 4rem);
  font-weight: 900;
  line-height: 1.1;
  letter-spacing: -0.02em;
  margin-bottom: 28px;
  color: var(--color-text-dark);
  animation: reveal-up 0.8s var(--ease-out) 0.1s both;
}

.hero-title-line {
  display: block;
}

.hero-title-highlight {
  color: var(--flame, #f16436);
  position: relative;
}

.hero-title-highlight::after {
  content: "";
  position: absolute;
  left: 0;
  bottom: 0.05em;
  width: 100%;
  height: 0.12em;
  background: var(--flame, #f16436);
  opacity: 0.3;
  border-radius: 4px;
}

.hero-desc {
  font-size: 1.15rem;
  color: var(--color-secondary);
  line-height: 1.8;
  max-width: 500px;
  margin-bottom: 40px;
  animation: reveal-up 0.8s var(--ease-out) 0.2s both;
}

.hero-actions {
  display: flex;
  gap: 16px;
  margin-bottom: 60px;
  animation: reveal-up 0.8s var(--ease-out) 0.3s both;
}

.hero-btn {
  font-family: var(--font-display);
  font-size: 1rem;
  font-weight: 700;
  padding: 16px 32px;
  border-radius: 16px;
  border: none;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  transition: all 0.4s var(--ease-spring);
  text-decoration: none;
}

.hero-btn-primary {
  background: var(--flame, #f16436);
  color: #000;

  &:hover {
    transform: translateY(-4px) scale(1.02);
    box-shadow: 0 20px 40px var(--accent-glow, rgba(241, 100, 54, 0.4));
  }
}

.hero-btn-secondary {
  background: var(--color-raised-bg);
  border: 2px solid var(--color-divider);
  color: var(--color-text-dark);

  &:hover {
    border-color: var(--color-divider-dark);
    transform: translateY(-4px);
  }
}

.hero-stats {
  display: flex;
  gap: 48px;
  animation: reveal-up 0.8s var(--ease-out) 0.4s both;
}

.hero-stat {
  position: relative;

  &::after {
    content: "";
    position: absolute;
    right: -24px;
    top: 50%;
    transform: translateY(-50%);
    width: 1px;
    height: 40px;
    background: var(--color-divider);
  }

  &:last-child::after {
    display: none;
  }
}

.hero-stat-value {
  font-family: var(--font-display);
  font-size: 2rem;
  font-weight: 900;
  color: var(--color-text-dark);
  letter-spacing: -0.02em;
}

.hero-stat-label {
  font-size: 0.9rem;
  color: var(--color-secondary);
  margin-top: 4px;
}

// Hero Visual - Banner Carousel
.hero-visual {
  position: relative;
  z-index: 1;
  animation: reveal-scale 1s var(--ease-out) 0.3s both;
}

.hero-banner {
  position: relative;
  width: 100%;
  height: 480px;
  border-radius: 24px;
  overflow: hidden;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
  user-select: none;

  &.cursor-grab {
    cursor: grab;
  }

  &.cursor-grabbing {
    cursor: grabbing;
  }
}

.hero-slide {
  position: absolute;
  inset: 0;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.6s ease,
    transform 0.6s ease;
  transform: scale(1.02);

  &.active {
    opacity: 1;
    transform: scale(1);
    pointer-events: auto;
  }
}

.hero-slide-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  user-select: none;
  pointer-events: none;

  &.scale-effect {
    transition: transform 0.5s ease;

    .hero-banner:hover & {
      transform: scale(1.05);
    }
  }
}

.hero-slide-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 40px 32px;
  background: linear-gradient(
    to top,
    rgba(0, 0, 0, 0.85) 0%,
    rgba(0, 0, 0, 0.4) 60%,
    transparent 100%
  );
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hero-slide-badge {
  display: inline-flex;
  align-self: flex-start;
  padding: 6px 16px;
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  color: #fff;
  font-size: 0.75rem;
  font-weight: 600;
  border-radius: 6px;
  letter-spacing: 0.03em;
  border: 1px solid rgba(255, 255, 255, 0.2);
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.hero-slide-title {
  font-family: var(--font-display);
  font-size: 1.8rem;
  font-weight: 800;
  color: #fff;
  margin: 0;
  text-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.hero-slide-desc {
  font-size: 1rem;
  color: rgba(255, 255, 255, 0.85);
  margin: 0;
  max-width: 400px;
  line-height: 1.5;
}

.hero-slide-link {
  position: absolute;
  inset: 0;
  z-index: 1;
}

.hero-banner-nav {
  position: absolute;
  bottom: 20px;
  right: 32px;
  display: flex;
  gap: 8px;
  z-index: 2;
}

.hero-nav-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.4);
  border: none;
  cursor: pointer;
  transition: all 0.3s ease;
  padding: 0;

  &:hover {
    background: rgba(255, 255, 255, 0.7);
  }

  &.active {
    background: var(--flame, #f16436);
    width: 28px;
    border-radius: 5px;
  }
}

.hero-nav-arrow {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s ease;
  opacity: 0;
  z-index: 2;

  svg {
    width: 20px;
    height: 20px;
  }

  &:hover {
    background: var(--flame, #f16436);
    border-color: var(--flame, #f16436);
  }

  &.prev {
    left: 16px;
  }

  &.next {
    right: 16px;
  }
}

// ==========================================
// CATEGORIES
// ==========================================
.categories {
  max-width: 1600px;
  margin: 0 auto 60px;
  padding: 0 40px;
  overflow: hidden;
}

.categories-track {
  display: flex;
  gap: 12px;
  overflow-x: auto;
  padding: 20px 0;
  scrollbar-width: none;
  -webkit-overflow-scrolling: touch;

  &::-webkit-scrollbar {
    display: none;
  }

  @media (min-width: 1400px) {
    justify-content: center;
    flex-wrap: wrap;
  }
}

.cat-chip {
  flex-shrink: 0;
  font-family: var(--font-display);
  font-size: 0.9rem;
  font-weight: 600;
  padding: 14px 24px;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: 100px;
  color: var(--color-secondary);
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.3s var(--ease-out);
  cursor: pointer;
  text-decoration: none;

  &:hover {
    border-color: var(--flame, #f16436);
    color: var(--color-text-dark);
    transform: translateY(-3px);
  }

  &.active {
    background: var(--flame, #f16436);
    border-color: var(--flame, #f16436);
    color: #000;
  }
}

.cat-icon {
  width: 1.1rem;
  height: 1.1rem;
  flex-shrink: 0;
}

// ==========================================
// MAIN CONTENT
// ==========================================
.main-content {
  max-width: 1600px;
  margin: 0 auto;
  padding: 0 40px 80px;
  display: grid;
  grid-template-columns: 1fr 380px;
  gap: 48px;
}

.content-area {
  min-width: 0;
}

// Section Headers
.section {
  margin-bottom: 48px;
}

.section-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  margin-bottom: 24px;
  padding-bottom: 20px;
  border-bottom: 1px solid var(--color-divider);
}

.section-title-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-label {
  font-family: var(--font-mono);
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--flame, #f16436);
  text-transform: uppercase;
  letter-spacing: 0.15em;
}

.section-title {
  font-family: var(--font-display);
  font-size: 1.5rem;
  font-weight: 800;
  letter-spacing: -0.02em;
  color: var(--color-text-dark);
  margin: 0;
}

.section-more {
  font-family: var(--font-display);
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--flame, #f16436);
  display: flex;
  align-items: center;
  gap: 8px;
  transition: gap 0.3s var(--ease-out);
  text-decoration: none;
  white-space: nowrap;

  &:hover {
    gap: 14px;
  }
}

.section-header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

// 主页搜索框
.home-search {
  position: relative;
  display: flex;
  align-items: center;
}

.home-search-icon {
  position: absolute;
  left: 10px;
  width: 18px;
  height: 18px;
  color: var(--color-text-secondary);
  opacity: 0.5;
  pointer-events: none;
  transition: opacity 0.2s;

  .home-search:focus-within & {
    opacity: 1;
    color: var(--color-brand);
  }
}

.home-search-input {
  width: 220px;
  height: 36px;
  padding: 0 32px 0 34px;
  border: 1px solid var(--color-divider);
  border-radius: 18px;
  background: var(--color-raised-bg);
  color: var(--color-text);
  font-size: 0.85rem;
  outline: none;
  transition:
    border-color 0.2s,
    box-shadow 0.2s,
    width 0.3s;

  &::placeholder {
    color: var(--color-text-secondary);
    opacity: 0.6;
  }

  &:focus {
    width: 280px;
    border-color: var(--color-brand);
    box-shadow: 0 0 0 3px rgba(var(--color-brand-rgb, 241, 100, 54), 0.15);
  }
}

.home-search-clear {
  position: absolute;
  right: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition:
    background 0.2s,
    color 0.2s;

  svg {
    width: 14px;
    height: 14px;
  }

  &:hover {
    background: var(--color-button-bg);
    color: var(--color-text);
  }
}

// 搜索状态
.search-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 60px 20px;
  color: var(--color-text-secondary);
  font-size: 0.9rem;
}

.search-loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--color-divider);
  border-top-color: var(--color-brand);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.search-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--color-text-secondary);
  text-align: center;
}

.search-empty-icon {
  width: 48px;
  height: 48px;
  opacity: 0.3;
  margin-bottom: 12px;
}

.search-empty p {
  margin: 0 0 16px;
  font-size: 0.95rem;
}

.search-empty-btn {
  padding: 8px 20px;
  border: 1px solid var(--color-divider);
  border-radius: 20px;
  background: var(--color-raised-bg);
  color: var(--color-text);
  font-size: 0.85rem;
  cursor: pointer;
  transition:
    background 0.2s,
    border-color 0.2s;

  &:hover {
    background: var(--color-button-bg);
    border-color: var(--color-brand);
  }
}

// Resource Grid - Card Layout
.resource-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;

  @media (max-width: 1200px) {
    grid-template-columns: repeat(2, 1fr);
  }

  @media (max-width: 768px) {
    grid-template-columns: 1fr;
  }
}

.resource-card {
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 16px;
  overflow: hidden;
  transition: all 0.3s var(--ease-out);
  text-decoration: none;
  display: flex;
  flex-direction: column;

  &:hover {
    border-color: var(--flame, #f16436);
    transform: translateY(-6px);
    box-shadow: 0 20px 40px var(--accent-glow, rgba(241, 100, 54, 0.15));

    .resource-gallery {
      &::after {
        opacity: 0.3;
      }
    }
  }
}

.resource-gallery {
  height: 120px;
  background-color: var(--bg-elevated, #1a1d23);
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  position: relative;

  &::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, var(--color-divider) 0%, var(--bg-elevated, #1a1d23) 100%);
    z-index: 0;
  }

  // Hide fallback gradient when image is loaded
  &[style*="background-image"] {
    &::before {
      display: none;
    }
  }

  &::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.6), transparent);
    opacity: 0.5;
    transition: opacity 0.3s;
    z-index: 1;
  }
}

.resource-type-badge {
  position: absolute;
  top: 10px;
  right: 10px;
  padding: 5px 10px;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  color: #fff;
  font-size: 0.7rem;
  font-weight: 600;
  border-radius: 20px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  z-index: 2;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.resource-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
}

.resource-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.resource-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  border: 1px solid var(--color-divider);
}

.resource-icon-placeholder {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  background: var(--bg-elevated, #12151a);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  svg {
    width: 1.5rem;
    height: 1.5rem;
    color: var(--color-secondary);
  }
}

.resource-title-wrap {
  min-width: 0;
  flex: 1;
}

.resource-name {
  font-family: var(--font-display);
  font-size: 1rem;
  font-weight: 700;
  color: var(--color-text-dark);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.resource-author {
  font-size: 0.8rem;
  color: var(--color-secondary);
}

.resource-desc {
  font-size: 0.85rem;
  color: var(--color-secondary);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin: 0;
  flex: 1;
}

.resource-tags {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.tag {
  font-family: var(--font-mono);
  font-size: 0.65rem;
  font-weight: 600;
  padding: 4px 8px;
  background: var(--bg-elevated, #12151a);
  border-radius: 4px;
  color: var(--color-secondary);
  letter-spacing: 0.02em;

  &.loader {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    color: var(--flame, #f16436);
  }

  &.version {
    background: rgba(45, 212, 191, 0.1);
    color: var(--teal, #2dd4bf);
  }
}

.resource-footer {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--color-divider);
  margin-top: auto;
}

.resource-stat {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.8rem;
  color: var(--color-secondary);

  .stat-icon {
    width: 0.9rem;
    height: 0.9rem;
    color: var(--color-secondary);
  }

  &.update {
    margin-left: auto;
    font-size: 0.75rem;
  }
}

// Notice List
.notice-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.notice-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 18px 20px;
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 16px;
  transition: all 0.3s var(--ease-out);
  text-decoration: none;
  position: relative;
  overflow: hidden;

  &::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: linear-gradient(180deg, var(--flame, #f16436) 0%, #ff8a5c 100%);
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  &:hover {
    border-color: var(--flame, #f16436);
    background: var(--accent-muted, rgba(241, 100, 54, 0.06));

    &::before {
      opacity: 1;
    }

    .notice-badge {
      background: var(--flame, #f16436);
      color: #fff;
    }
  }
}

.notice-avatar {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  object-fit: cover;
  flex-shrink: 0;
  transition: transform 0.3s ease;

  .notice-item:hover & {
    transform: scale(1.05);
  }
}

.notice-content {
  flex: 1;
  min-width: 0;
}

.notice-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--color-text-dark);
  margin-bottom: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.notice-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 0.8rem;
  color: var(--color-secondary);
}

.notice-author {
  font-weight: 500;
}

.notice-time {
  opacity: 0.8;
}

// Discussion List
.discussion-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.discussion-item {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 16px;
  transition: all 0.3s var(--ease-out);
  text-decoration: none;

  &:hover {
    border-color: var(--color-divider-dark);
    background: var(--bg-card-hover, var(--color-raised-bg));
    transform: translateX(4px);

    .discussion-avatar {
      transform: scale(1.05);
    }
  }
}

.discussion-avatar {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  object-fit: cover;
  flex-shrink: 0;
  transition: transform 0.3s ease;
}

.discussion-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.discussion-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.discussion-category {
  font-family: var(--font-mono);
  font-size: 0.7rem;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: 6px;
  text-transform: uppercase;
  letter-spacing: 0.03em;

  // Default style
  background: var(--accent-muted, rgba(241, 100, 54, 0.1));
  color: var(--flame, #f16436);

  // Category specific colors
  &.cat-project {
    background: rgba(45, 212, 191, 0.12);
    color: #2dd4bf;
  }

  &.cat-chat {
    background: rgba(139, 92, 246, 0.12);
    color: #8b5cf6;
  }

  &.cat-article {
    background: rgba(59, 130, 246, 0.12);
    color: #3b82f6;
  }

  &.cat-notice {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    color: var(--flame, #f16436);
  }

  &.cat-help {
    background: rgba(255, 179, 71, 0.12);
    color: #ffb347;
  }

  &.cat-share {
    background: rgba(34, 197, 94, 0.12);
    color: #22c55e;
  }
}

.discussion-time {
  font-size: 0.75rem;
  color: var(--color-secondary);
  opacity: 0.7;
}

.discussion-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--color-text-dark);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}

.discussion-meta {
  display: flex;
  align-items: center;
  gap: 16px;
  font-size: 0.8rem;
  color: var(--color-secondary);
}

.discussion-author {
  font-weight: 500;
}

.discussion-replies {
  display: flex;
  align-items: center;
  gap: 5px;
  opacity: 0.8;
}

.stat-icon {
  width: 0.9rem;
  height: 0.9rem;
  color: var(--color-secondary);
}

// ==========================================
// SIDEBAR
// ==========================================
.sidebar {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.sidebar-card {
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 20px;
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.06);
}

.sidebar-header {
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-divider);
}

.sidebar-title {
  font-family: var(--font-display);
  font-size: 1rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--color-text-dark);
  margin: 0;
}

.sidebar-icon {
  width: 1.2rem;
  height: 1.2rem;
  color: var(--flame, #f16436);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sidebar-more {
  font-size: 0.8rem;
  color: var(--flame, #f16436);
  text-decoration: none;
  font-weight: 500;
  transition: gap 0.2s;

  &:hover {
    opacity: 0.8;
  }
}

.sidebar-body {
  padding: 16px 20px;
}

// Article List
.article-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.article-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-elevated, #12151a);
  border-radius: 10px;
  transition: all 0.2s ease;
  text-decoration: none;

  &:hover {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    transform: translateX(4px);
  }
}

.article-avatar {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  object-fit: cover;
  flex-shrink: 0;
}

.article-info {
  flex: 1;
  min-width: 0;
}

.article-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-text-dark);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}

.article-meta {
  font-size: 0.75rem;
  color: var(--color-secondary);
}

// Translation List
.translation-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.translation-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-elevated, #12151a);
  border-radius: 10px;
  transition: all 0.2s ease;
  text-decoration: none;

  &:hover {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    transform: translateX(4px);
  }
}

.translation-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  object-fit: cover;
  flex-shrink: 0;
}

.translation-info {
  flex: 1;
  min-width: 0;
}

.translation-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-text-dark);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}

.translation-meta {
  font-size: 0.75rem;
  color: var(--color-secondary);
  display: flex;
  align-items: center;
  gap: 10px;

  .meta-item {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .stat-icon {
    width: 12px;
    height: 12px;
  }
}

// Mini Resource List (服务器插件推荐) - 丰富内容版样式
.mini-resource-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.mini-resource-item {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 16px;
  background: var(--bg-elevated, #12151a);
  border: 1px solid transparent;
  border-radius: 12px;
  transition: all 0.2s ease;
  text-decoration: none;

  &:hover {
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));
    border-color: var(--color-divider);
    transform: translateX(4px);

    .mini-resource-title {
      color: var(--flame, #f16436);
    }
  }
}

.mini-resource-icon {
  width: 56px;
  height: 56px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--bg-elevated, #12151a);
  border: 1px solid var(--color-divider);
}

.mini-resource-icon-placeholder {
  width: 56px;
  height: 56px;
  border-radius: 10px;
  flex-shrink: 0;
  background: var(--bg-elevated, #12151a);
  border: 1px solid var(--color-divider);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-secondary);

  svg {
    width: 28px;
    height: 28px;
  }
}

.mini-resource-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.mini-resource-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--color-text-dark);
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  transition: color 0.3s ease;
  letter-spacing: -0.01em;
  line-height: 1.3;
}

.mini-resource-author {
  font-size: 0.75rem;
  color: var(--color-secondary);
  font-weight: 500;
  opacity: 0.8;
}

.mini-resource-desc {
  font-size: 0.8rem;
  color: var(--color-text);
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  margin: 2px 0;
}

.mini-resource-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 4px 0;
}

.mini-tag {
  display: inline-flex;
  align-items: center;
  padding: 3px 8px;
  background: var(--accent-muted, rgba(241, 100, 54, 0.15));
  border: 1px solid var(--flame, rgba(241, 100, 54, 0.3));
  border-radius: 6px;
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--flame, #f16436);
  text-transform: capitalize;
  letter-spacing: 0.02em;
}

.version-tag {
  background: var(--bg-elevated, rgba(255, 255, 255, 0.05));
  border-color: var(--color-divider);
  color: var(--color-secondary);
}

.mini-resource-stats {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 4px;
}

.mini-stat-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.75rem;
  color: var(--color-secondary);
  font-weight: 500;

  .mini-stat-icon {
    width: 13px;
    height: 13px;
    color: var(--flame, #f16436);
    opacity: 0.8;
  }

  &.update-time {
    color: var(--color-text-inactive);

    .mini-stat-icon {
      color: var(--color-secondary);
    }
  }
}

// Quick Links
.quick-links {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  padding: 20px;
}

.quick-link {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 20px 16px;
  background: var(--bg-elevated, #12151a);
  border-radius: 12px;
  transition: all 0.3s var(--ease-out);
  text-decoration: none;

  &:hover {
    background: var(--accent-muted, rgba(241, 100, 54, 0.12));
    transform: translateY(-4px);
  }
}

.quick-link-icon {
  font-size: 1.6rem;
  display: flex;
  align-items: center;
  justify-content: center;

  svg {
    width: 1.6rem;
    height: 1.6rem;
    color: var(--flame, #f16436);
  }
}

.quick-link-text {
  font-family: var(--font-display);
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-secondary);
}

// Friend Links
.friend-link-logo {
  width: auto;
  height: 32px;
  max-width: 120px;
  object-fit: contain;
  border-radius: 6px;
}

// ==========================================
// ANIMATIONS
// ==========================================
@keyframes reveal-up {
  from {
    opacity: 0;
    transform: translateY(40px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes reveal-scale {
  from {
    opacity: 0;
    transform: scale(0.9);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes glow {
  0%,
  100% {
    filter: drop-shadow(0 0 10px var(--accent-glow, rgba(241, 100, 54, 0.4)));
  }
  50% {
    filter: drop-shadow(0 0 20px var(--accent-glow, rgba(241, 100, 54, 0.4)));
  }
}

// ==========================================
// RESPONSIVE
// ==========================================
@media (max-width: 1200px) {
  .hero {
    grid-template-columns: 1fr;
    padding: 120px 32px 60px;
    min-height: auto;
  }

  .hero-visual {
    max-width: 100%;
  }

  .hero-banner {
    height: 400px;
  }

  .main-content {
    grid-template-columns: 1fr;
    padding: 0 32px 60px;
  }

  .sidebar {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 20px;
  }
}

@media (max-width: 768px) {
  .hero {
    padding: 100px 24px 40px;
  }

  .hero-title {
    font-size: clamp(2rem, 8vw, 2.5rem);
  }

  .hero-banner {
    height: 320px;
  }

  .hero-slide-title {
    font-size: 1.4rem;
  }

  .hero-slide-desc {
    font-size: 0.9rem;
    display: none;
  }

  .hero-slide-overlay {
    padding: 24px 20px;
  }

  .hero-nav-arrow {
    width: 40px;
    height: 40px;
    opacity: 1;
  }

  .hero-stats {
    flex-wrap: wrap;
    gap: 24px;
  }

  .hero-stat::after {
    display: none;
  }

  .categories {
    padding: 0 24px;
  }

  .main-content {
    padding: 0 24px 40px;
  }

  .resource-item {
    grid-template-columns: 56px 1fr;
  }

  .resource-icon-wrap {
    width: 56px;
    height: 56px;
  }

  .resource-stats {
    display: none;
  }

  .sidebar {
    grid-template-columns: 1fr;
  }

  .section-header {
    flex-wrap: wrap;
    gap: 12px;
  }

  .section-header-right {
    width: 100%;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }

  .home-search-input {
    width: 100%;

    &:focus {
      width: 100%;
    }
  }

  .section-more {
    justify-content: center;
  }
}
</style>
