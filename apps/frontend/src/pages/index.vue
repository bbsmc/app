<template>
  <div>
    <div class="game-header">
      <div class="hero-container">
        <img src="https://cdn.bbsmc.net/raw/top.jpeg" alt="header" />
        <div class="desktop-only"></div>
      </div>
    </div>
    <div class="game-page container">
      <div class="game-description">
        <div class="game-title">
          <h1 class="section-title">BBSMC</h1>
          <span class="num-projects">Minecraft资源社区</span>
        </div>
      </div>
      <div class="game-carousel" @mouseenter="handleMouseEnter" @mouseleave="handleMouseLeave">
        <ul class="carousel-items">
          <li v-for="(item, index) in carouselItems" :key="index" :class="[
            'carousel-item',
            {
              previous:
                currentSlide === 0
                  ? index === carouselItems.length - 1
                  : index === currentSlide - 1,
              current: index === currentSlide,
              next:
                currentSlide === carouselItems.length - 1
                  ? index === 0
                  : index === currentSlide + 1,
            },
          ]">
            <div class="carousel-slide">
              <div class="carousel-image-container">
                <a v-if="index === currentSlide" :href="item.slug" target="_blank">
                  <img :src="item.image" :alt="item.title" />
                </a>
                <img v-else :src="item.image" :alt="item.title" @click="goToSlide(index)" />
              </div>
              <div v-if="index === currentSlide" class="carousel-bottom-container">
                <div class="carousel-item-title">{{ item.title }}</div>
                <div class="carousel-item-content">
                  <div class="carousel-item-description">{{ item.description }}</div>
                </div>
              </div>
            </div>
          </li>
        </ul>
        <!-- <div class="carousel-buttons">
          <button class="btn-prev" @click="prevSlide">←</button>
          <button class="btn-next" @click="nextSlide">→</button>
        </div> -->
        <div class="carousel-dots">
          <span v-for="(_, index) in carouselItems" :key="index" class="dot" :class="{ active: currentSlide === index }"
            @click="goToSlide(index)">
          </span>
        </div>
      </div>

      <div>
        <h1 class="section-title">
          矿工茶馆
          <a href="/forums/chat" target="_blank" class="link-btn btn-secondary">查看更多</a>
        </h1>
        <div class="forum-list">
          <div v-for="forum in forums" :key="forum.id" class="forum-item">
            <h5 class="section-title">
              <a v-if="forum.project_id" :href="`/project/${forum.project_id}/forum`">{{
                forum.title
              }}</a>
              <a v-else :href="`/d/${forum.id}`">{{ forum.title }}</a>
              <span>{{ fromNow(forum.last_post_time) }}</span>
            </h5>
          </div>
        </div>
      </div>

      <div>
        <h1 class="section-title">
          热门整合包
          <a href="/modpacks" target="_blank" class="link-btn btn-secondary">查看更多</a>
        </h1>
        <div class="modpacks-grid">
          <div v-for="project in modpacks" :key="project.project_id" class="modpack-card">
            <a :href="`/modpack/${project.slug}`" class="modpack-link" target="_blank">
              <div class="card-content">
                <img :src="project.featured_gallery" :alt="project.title" class="modpack-image" />
                <div class="modpack-basic-info">
                  <div class="modpack-info-top">
                    <h3 class="modpack-title">{{ project.title }}</h3>
                    <div class="modpack-author">By {{ project.author }}</div>
                  </div>
                </div>
                <div class="modpack-footer">
                  <div class="modpack-stats">
                    <span class="download-count">
                      <span class="icon">↓</span> {{ $formatNumber(project.downloads) }}
                    </span>
                    <span class="category">整合包</span>
                  </div>
                </div>
              </div>
            </a>
          </div>
        </div>
      </div>
      <div>
        <h1 class="section-title">
          最新整合包
          <a href="/modpacks?s=newest" target="_blank" class="link-btn btn-secondary">查看更多</a>
        </h1>
        <div class="modpacks-grid">
          <div v-for="project in newModpacks" :key="project.project_id" class="modpack-card">
            <a :href="`/modpack/${project.slug}`" class="modpack-link" target="_blank">
              <div class="card-content">
                <img :src="project.featured_gallery" :alt="project.title" class="modpack-image" />
                <div class="modpack-basic-info">
                  <div class="modpack-info-top">
                    <h3 class="modpack-title">{{ project.title }}</h3>
                    <div class="modpack-author">By {{ project.author }}</div>
                  </div>
                </div>
                <div class="modpack-footer">
                  <div class="modpack-stats">
                    <span class="download-count">
                      <span class="icon">↓</span> {{ project.downloads }}
                    </span>
                    <span class="category">整合包</span>
                  </div>
                </div>
              </div>
            </a>
          </div>
        </div>
      </div>
      <div>
        <h1 class="section-title">
          热门模组
          <a href="/mods" target="_blank" class="link-btn btn-secondary">查看更多</a>
        </h1>
        <div class="modpacks-grid">
          <div v-for="project in mods" :key="project.project_id" class="modpack-card">
            <a :href="`/mod/${project.slug}`" class="modpack-link" target="_blank">
              <div class="card-content">
                <img :src="project.featured_gallery" :alt="project.title" class="modpack-image" />
                <div class="modpack-basic-info">
                  <div class="modpack-info-top">
                    <h3 class="modpack-title">{{ project.title }}</h3>
                    <div class="modpack-author">By {{ project.author }}</div>
                  </div>
                </div>
                <div class="modpack-footer">
                  <div class="modpack-stats">
                    <span class="download-count">
                      <span class="icon">↓</span> {{ project.downloads }}
                    </span>
                    <span class="category">Mod</span>
                  </div>
                </div>
              </div>
            </a>
          </div>
        </div>
      </div>
      <div>
        <h1 class="section-title">
          热门插件
          <a href="/plugins" target="_blank" class="link-btn btn-secondary">查看更多</a>
        </h1>
        <div class="modpacks-grid">
          <div v-for="project in plugins" :key="project.project_id" class="modpack-card">
            <a :href="`/plugin/${project.slug}`" class="modpack-link" target="_blank">
              <div class="card-content">
                <img :src="project.featured_gallery" :alt="project.title" class="modpack-image" />
                <div class="modpack-basic-info">
                  <div class="modpack-info-top">
                    <h3 class="modpack-title">{{ project.title }}</h3>
                    <div class="modpack-author">By {{ project.author }}</div>
                  </div>
                </div>
                <div class="modpack-footer">
                  <div class="modpack-stats">
                    <span class="download-count">
                      <span class="icon">↓</span> {{ project.downloads }}
                    </span>
                    <span class="category">服务端插件</span>
                  </div>
                </div>
              </div>
            </a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
// import { homePageSearch } from "~/generated/state.json";

// const searchQuery = ref("");
// const sortType = ref("relevance");

// const searchProjects = ref(homePageSearch.hits ?? []);

// async function updateSearchProjects() {
//   const res = await useBaseFetch(
//     `search?limit=3&query=${searchQuery.value}&index=${sortType.value}`,
//   );

//   searchProjects.value = res.hits ?? [];
// }
import dayjs from "dayjs";
const modpacks = ref([]);
const newModpacks = ref([]);
const mods = ref([]);
const plugins = ref([]);
const forums = ref([]);
async function getProjects() {
  const [modpacksResponse, newModpacksResponse, modResponse, pluginsResponse, forumsResponse] =
    await Promise.all([
      useBaseFetch(`search?limit=6&index=relevance&facets=[["project_type:modpack"]]`),
      useBaseFetch(`search?limit=6&index=newest&facets=[["project_type:modpack"]]`),
      useBaseFetch(`search?limit=6&index=relevance&facets=[["project_type:mod"]]`),
      useBaseFetch(`search?limit=6&index=relevance&facets=[["project_type:plugin"]]`),
      useBaseFetch(`forum`, { apiVersion: 3 }),
    ]);

  modpacks.value =
    modpacksResponse.hits?.map((modpack) => ({
      ...modpack,
      slug: modpack.slug || modpack.project_id,
      featured_gallery:
        modpack.featured_gallery ||
        (modpack.gallery?.length > 0 ? modpack.gallery[0] : modpack.icon_url),
    })) ?? [];

  newModpacks.value =
    newModpacksResponse.hits?.map((modpack) => ({
      ...modpack,
      slug: modpack.slug || modpack.project_id,
      featured_gallery:
        modpack.featured_gallery ||
        (modpack.gallery?.length > 0 ? modpack.gallery[0] : modpack.icon_url),
    })) ?? [];

  mods.value =
    modResponse.hits?.map((mod) => ({
      ...mod,
      slug: mod.slug || mod.project_id,
      featured_gallery:
        mod.featured_gallery || (mod.gallery?.length > 0 ? mod.gallery[0] : mod.icon_url),
    })) ?? [];
  plugins.value =
    pluginsResponse.hits?.map((plugin) => ({
      ...plugin,
      slug: plugin.slug || plugin.project_id,
      featured_gallery:
        plugin.featured_gallery ||
        (plugin.gallery?.length > 0 ? plugin.gallery[0] : plugin.icon_url),
    })) ?? [];

  forums.value = forumsResponse.forums ?? [];
}
await getProjects();

// 初始化的时候就打乱carouselItems的顺序

const carouselItems = ref([
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/1p2TFl6X/images/73cc070ff496b26f2674eb5928b021cb2ef93426.jpeg",
    title: "乌托邦探险之旅",
    description: "乌托邦探险之旅",
    slug: "/modpack/utopia-journey",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/NxtrWNas/images/329b6261d797271622386b146078d7130a5438c0.jpeg",
    title: "探索自然2",
    description: "通过探索，种田来发展经济，提升实力，面临不断增强的怪物",
    slug: "/modpack/tansuoziran2",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/dL0Tbr7N/images/19f25c62f6bcc1d068c9b35e4e603e81991754f9.jpeg",
    title: "脆骨症：黯光",
    description: "脆骨症的维度分支，引入了大量的新维度作为内容的补充。",
    slug: "/modpack/no-flesh-within-chest-dim",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/uXcveaXY/images/3e6d50d5bc617f730cddff4c93407272443c911c.gif",
    title: "TrMenu",
    description: "社区维护TrMenu3.0",
    slug: "/plugin/trmenu",
  },
]);

const currentSlide = ref(0);

const fromNow = (date) => {
  const currentDate = useCurrentDate();
  return dayjs(date).from(currentDate.value);
};

// const currentSlide = ref(0);

const autoPlayInterval = ref(null);
const autoPlayDelay = 5000;

// 添加一个标志来判断是否在客户端
const isClient = ref(false);

// 开始自动播放
const startAutoPlay = () => {
  // 只在客户端执行
  if (!isClient.value) return;

  stopAutoPlay();
  autoPlayInterval.value = setInterval(() => {
    nextSlide();
  }, autoPlayDelay);
};

// 停止自动播放
const stopAutoPlay = () => {
  if (autoPlayInterval.value) {
    clearInterval(autoPlayInterval.value);
    autoPlayInterval.value = null;
  }
};

// 修改 nextSlide 和 prevSlide 函数，添加重置自动播放
const nextSlide = () => {
  currentSlide.value = (currentSlide.value + 1) % carouselItems.value.length;
  startAutoPlay(); // 重置自动播放计时器
};

// const prevSlide = () => {
//   currentSlide.value =
//     currentSlide.value === 0 ? carouselItems.value.length - 1 : currentSlide.value - 1;
//   startAutoPlay(); // 重置自动播放计时器
// };

const goToSlide = (index) => {
  if (index === currentSlide.value) {
    // 打开链接
    window.open(`${carouselItems.value[index].slug}`, "_blank");
    return;
  }

  currentSlide.value = index;
  startAutoPlay(); // 重置自动播放计时器
};

// 在组件挂载时启动自动播放
onMounted(() => {
  isClient.value = true;
  currentSlide.value = Math.floor(Math.random() * carouselItems.value.length);
  startAutoPlay();
});

// 确保在组件卸载时清除定时器
onUnmounted(() => {
  stopAutoPlay();
  isClient.value = false;
});

// 鼠标事件处理
const handleMouseEnter = () => {
  if (!isClient.value) return;
  stopAutoPlay();
};

const handleMouseLeave = () => {
  if (!isClient.value) return;
  startAutoPlay();
};
</script>

<style scoped>
.resource-list {
  display: grid;
  grid-auto-flow: column;
  grid-gap: 24px;
  gap: 24px;
}

h1 {
  font-size: 18px;
  display: flex;
  justify-content: space-between;
  margin-bottom: 24px;
  color: #e5e5e5;
}

h2,
p {
  line-height: 1.45;
  color: #e5e5e5;
}

.hero-container {
  width: 100%;
  height: 144px;
  position: relative;
}

.game-header img {
  width: 100%;
  height: 144px;
}

.game-header .hero-container {
  height: 144px;
  z-index: 1;
}

.game-header .hero-container img {
  width: 100%;
  height: 144px;
  display: block;
}

body:has(.game-page) .game-header {
  margin-bottom: -110px;
  background-repeat: no-repeat;
}

body:has(.game-page) .game-header .hero-container:after {
  background: linear-gradient(hsla(0, 0%, 5%, 0.5), var(--color-background, #0d0d0d) 100%);
}

.game-header .hero-container:afterfont-weight {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: -1px;
  background: linear-gradient(0deg, #0d0d0d, transparent);
}

.game-page {
  position: relative;
  z-index: 2;
}

.game-description h1 {
  line-height: 48px;
  font-family: var(--montserrat-font);
  font-weight: 700;
}

.game-description .game-title {
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  white-space: nowrap;
}

.container,
.element-container {
  max-width: 1224px;
  margin: auto;
}

.container {
  min-height: 724px;
}

.game-description .num-projects {
  --space: 16px;
  position: relative;
  color: #e5e5e5;
  padding-left: var(--space);
  margin-left: var(--space);
  flex-shrink: 10;
  height: 48px;
  line-height: 48px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.game-description .num-projects:before {
  position: absolute;
  margin-top: auto;
  margin-bottom: auto;
  top: 0;
  bottom: 0;
  left: 0;
  content: "";
  width: 1px;
  height: 28px;
  background: #4d4d4d;
}

.game-description .expandable-html-block {
  display: flex;
  gap: 8px;
}

.game-carousel {
  position: relative;
  width: 100%;
  height: 400px;
  overflow: hidden;
}

.carousel-items {
  position: relative;
  height: 100%;
  margin: 0;
  padding: 0;
  list-style: none;
}

.carousel-item {
  position: absolute;
  width: 80%;
  height: 100%;
  left: 50%;
  transition: all 0.5s ease;
  visibility: hidden;
}

.carousel-item.current {
  transform: translateX(-50%) scale(1);
  opacity: 1;
  z-index: 2;
  visibility: visible;
}

.carousel-item.previous {
  transform: translateX(-125%) scale(0.8);
  opacity: 0.6;
  z-index: 1;
  visibility: visible;
}

.carousel-item.next {
  transform: translateX(25%) scale(0.8);
  opacity: 0.6;
  z-index: 1;
  visibility: visible;
}

.carousel-image-container {
  width: 100%;
  height: 300px;
  overflow: hidden;
  border-radius: 8px;
}

.carousel-image-container img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.carousel-bottom-container {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 20px;
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.8));
  color: #e5e5e5;
}

.carousel-item-title {
  font-size: 24px;
  font-weight: bold;
  margin-bottom: 10px;
}

.carousel-item-description {
  margin-bottom: 15px;
}

.carousel-buttons {
  position: absolute;
  top: 50%;
  left: 20px;
  right: 20px;
  transform: translateY(-50%);
  display: flex;
  justify-content: space-between;
  z-index: 3;
}

.carousel-buttons button {
  background: rgba(0, 0, 0, 0.5);
  border: none;
  color: #e5e5e5;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  transition: background-color 0.3s;
}

.carousel-buttons button:hover {
  background: rgba(0, 0, 0, 0.8);
}

.carousel-dots {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 8px;
  z-index: 2;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.5);
  cursor: pointer;
}

.dot.active {
  background: #f1f1f1;
}

.modpacks-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 16px;
  padding: 16px;
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .modpacks-grid {
    grid-template-columns: repeat(2, 1fr);
    /* 两列布局 */
  }
}

/* 媒体查询：当屏幕宽度小于 480px 时 */
@media (max-width: 480px) {
  .modpacks-grid {
    grid-template-columns: 1fr;
    /* 单列布局 */
  }
}

.modpack-card {
  background: rgb(16, 16, 19);
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.2s ease;
  display: flex;
  flex-direction: column;
}

.modpack-card:hover {
  transform: scale(1.05);
  z-index: 1;
}

.card-content {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.modpack-image {
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
}

.modpack-basic-info {
  padding: 12px;
  margin-top: 12px;
  flex-grow: 1;
}

.modpack-info-top {
  flex-grow: 1;
}

.modpack-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 4px 0;
  color: #e5e5e5;
}

.modpack-author {
  font-size: 14px;
  color: #888;
  margin-bottom: 8px;
}

.modpack-footer {
  padding: 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  background: rgb(16, 16, 19);
}

.modpack-stats {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
  color: #888;
}

.download-count {
  display: flex;
  align-items: center;
  gap: 4px;
}

.category {
  padding: 2px 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

.modpack-link {
  text-decoration: none;
  color: inherit;
  display: block;
  height: 100%;
}

.section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .section-title {
    padding: 0 24px;
    /* 添加左右间距 */
  }
}

/* 媒体查询：当屏幕宽度小于 480px 时 */
@media (max-width: 480px) {
  .section-title {
    padding: 0 8px;
    /* 更小的左右间距 */
  }
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .game-carousel {
    height: 300px;
    /* 手机端高度设置为 300px */
  }

  .carousel-image-container {
    height: 200px;
    /* 手机端高度设置为 200px */
  }
}
</style>
