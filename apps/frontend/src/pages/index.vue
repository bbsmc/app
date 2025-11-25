<template>
  <div :style="themeVars">
    <!-- <div class="game-header">
      <div class="hero-container">
        <img src="https://cdn.bbsmc.net/raw/top.jpeg" alt="header" />
        <div class="desktop-only"></div>
      </div>
    </div> -->
    <div class="game-page container">
      <!-- <div class="game-description">
        <div class="game-title">
          <h1 class="section-title">BBSMC</h1>
          <span class="num-projects">Minecraft资源社区</span>
        </div>
      </div> -->
      <!-- Banner 轮播区域 -->
      <section
        class="group relative mb-12 h-[450px] select-none overflow-hidden rounded-xl"
        :class="isDragging ? 'cursor-grabbing' : 'cursor-grab'"
        @mouseenter="handleMouseEnter"
        @mouseleave="handleMouseLeave"
        @mousedown="handleDragStart"
        @touchstart="handleDragStart"
        @touchmove="handleDragMove"
        @touchend="handleDragEnd"
      >
        <div
          v-for="(item, index) in carouselItems"
          :key="index"
          :class="[
            'absolute inset-0 h-full w-full select-none transition-opacity duration-500',
            { 'z-10 opacity-100': index === currentSlide, 'z-0 opacity-0': index !== currentSlide },
          ]"
          @click="handleBannerClick($event, item.slug)"
        >
          <img
            :src="item.image"
            :alt="item.title"
            class="user-select-none absolute inset-0 h-full w-full object-cover"
            :class="{ 'transition-transform duration-500 group-hover:scale-105': !isDragging }"
            draggable="false"
          />
          <div class="absolute inset-0 bg-gradient-to-t from-black/80 to-transparent"></div>
          <div class="pointer-events-none absolute bottom-0 left-0 p-8 text-white md:p-12">
            <h2 class="banner-title">{{ item.title }}</h2>
            <p class="banner-description">{{ item.description }}</p>
          </div>
        </div>
        <div class="absolute bottom-6 right-6 z-20 flex space-x-2">
          <button
            v-for="(_, index) in carouselItems"
            :key="index"
            :class="[
              'h-2 w-2 rounded-full transition-all duration-300',
              currentSlide === index ? 'bg-white' : 'bg-white/50 hover:bg-white',
            ]"
            @click="goToSlide(index)"
          ></button>
        </div>
      </section>

      <!-- 搜索框区域 -->
      <section class="mb-12">
        <div class="mx-auto max-w-2xl px-4">
          <form class="relative" @submit.prevent="handleSearch">
            <input
              v-model="searchQuery"
              type="search"
              placeholder="搜索模组、资源包、整合包..."
              class="search-input w-full rounded-full py-4 pl-12 pr-4 outline-none transition-all"
              style="background: var(--color-raised-bg); color: var(--color-text)"
              @input="handleSearchInput"
            />
            <div class="pointer-events-none absolute left-4 top-1/2 -translate-y-1/2">
              <svg
                class="h-5 w-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                style="color: var(--color-text)"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
            </div>
          </form>
        </div>
      </section>

      <section class="mb-12">
        <div class="px-4">
          <h2 class="mb-6 text-2xl font-bold" style="color: var(--color-text-dark)">热门资源</h2>
        </div>
        <div class="grid grid-cols-1 gap-8 px-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          <div
            v-for="project in modpacks"
            :key="project.project_id"
            class="group overflow-hidden rounded-xl transition-all duration-300 hover:-translate-y-2 hover:shadow-lg"
            style="
              background: var(--color-raised-bg);
              box-shadow:
                0 10px 15px -3px rgba(0, 0, 0, 0.05),
                0 4px 6px -2px rgba(0, 0, 0, 0.03);
            "
          >
            <a :href="getProjectLink(project)" target="_blank" class="block">
              <div class="relative h-56">
                <img
                  :src="project.featured_gallery"
                  :alt="project.title"
                  class="h-full w-full object-cover transition-transform duration-500 group-hover:scale-110"
                />
                <div class="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent"></div>

                <!-- 下载量统计 -->
                <div
                  class="absolute right-4 top-4 flex items-center gap-1.5 rounded-full bg-black/50 px-3 py-1.5 backdrop-blur-sm"
                >
                  <svg
                    class="h-4 w-4 text-white"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                    />
                  </svg>
                  <span class="text-sm font-medium text-white">{{
                    formatNumber(project.downloads)
                  }}</span>
                </div>

                <div class="absolute bottom-4 left-4 text-white">
                  <h3 class="card-title">{{ project.title }}</h3>
                  <p v-if="project.author !== 'BBSMC'" class="card-author">
                    By {{ project.author }}
                  </p>
                </div>
              </div>
            </a>
          </div>
        </div>
      </section>

      <!-- 矿工茶馆区域 -->
      <section class="mb-16">
        <div class="px-4">
          <h2 class="mb-6 text-2xl font-bold" style="color: var(--color-text-dark)">矿工茶馆</h2>
          <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
            <a
              v-for="forum in forums"
              :key="forum.id"
              :href="forum.project_id ? `/project/${forum.project_id}/forum` : `/d/${forum.id}`"
              class="forum-card group"
            >
              <div class="flex items-start gap-3">
                <!-- 用户头像 -->
                <img :src="forum.avatar" :alt="forum.user_name" class="forum-avatar" />

                <!-- 中间内容区 -->
                <div class="min-w-0 flex-1">
                  <h3 class="forum-card-title">{{ forum.title }}</h3>
                  <div class="mt-1 flex items-center gap-2">
                    <span class="forum-username">{{ forum.user_name }}</span>
                    <span class="forum-replies">{{ forum.replies }} 回复</span>
                  </div>
                </div>

                <!-- 右侧时间和箭头 -->
                <div class="flex flex-shrink-0 flex-col items-end gap-1">
                  <p class="forum-card-time">{{ fromNow(forum.last_post_time) }}</p>
                  <svg
                    class="h-5 w-5 transform transition-transform group-hover:translate-x-1"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    style="color: var(--color-brand)"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                </div>
              </div>
            </a>
          </div>
        </div>
      </section>
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
import { isDarkTheme } from "~/plugins/theme/themes.ts";
const modpacks = ref([]);
const searchQuery = ref("");
const forums = ref([]);

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--carousel-gradient-end": "rgba(0, 0, 0, 0.8)",
      "--carousel-dot-bg": "rgba(255, 255, 255, 0.5)",
      "--carousel-dot-active": "var(--color-text-dark)",
      "--carousel-text-color": "var(--color-text-dark)",
    };
  } else {
    return {
      "--carousel-gradient-end": "rgba(255, 255, 255, 0.9)",
      "--carousel-dot-bg": "rgba(100, 100, 100, 0.5)",
      "--carousel-dot-active": "var(--color-brand)",
      "--carousel-text-color": "var(--color-text)",
    };
  }
});

async function getProjects() {
  const [modpacksResponse, forumsResponse] = await Promise.all([
    useBaseFetch(`search?limit=8&index=relevance&facets=[["project_type:modpack"]]`),
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

  forums.value = (forumsResponse.forums ?? []).slice(0, 6); // 只显示6个
}
await getProjects();

// 时间格式化
const fromNow = (date) => {
  const currentDate = useCurrentDate();
  return dayjs(date).from(currentDate.value);
};

// 初始化的时候就打乱carouselItems的顺序

const carouselItems = ref([
  {
    image: "https://cdn.bbsmc.net/raw/images/pcl2.jpg",
    title: "PCL2",
    description:
      "Minecraft 启动器：Plain Craft Launcher！简称 PCL！ 超快的下载速度，下载安装 Mod 和整合包，简洁且高度自定义的界面，流畅精细的动画……总之很棒就完事啦！",
    slug: "/software/pcl",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/vC327lbX/images/9b83a4e1111aadfff2e6ca82bec99883bb04bc3f.webp",
    title: "PCL CE",
    description: "基于 PCL 公开源代码二次开发的社区版本，添加了许多实用功能与改进",
    slug: "/software/pcl",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/1p2TFl6X/images/73cc070ff496b26f2674eb5928b021cb2ef93426_350.webp",
    title: "乌托邦探险之旅",
    description: "乌托邦探险之旅",
    slug: "/modpack/utopia-journey",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/NxtrWNas/images/329b6261d797271622386b146078d7130a5438c0_350.webp",
    title: "探索自然2",
    description: "通过探索，种田来发展经济，提升实力，面临不断增强的怪物",
    slug: "/modpack/tansuoziran2",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/dL0Tbr7N/images/19f25c62f6bcc1d068c9b35e4e603e81991754f9_350.webp",
    title: "脆骨症：黯光",
    description: "脆骨症的维度分支，引入了大量的新维度作为内容的补充。",
    slug: "/modpack/no-flesh-within-chest-dim",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/G23dLUsP/images/e681d996cd07316e12facedd8fb22e9f74ce68a1_350.webp",
    title: "剑与王国",
    description: "围绕模拟殖民地与村民招募玩法的深度魔改整合包",
    slug: "/modpack/snk",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/EIrkPpcm/images/7d43813f0ff22b6c769e7382d36d5059657e8a94_350.webp",
    title: "龙之冒险：新征程",
    description: "面对众多怪物的冒险之旅，你做好准备了吗？",
    slug: "/modpack/lzmx",
  },
  {
    image:
      "https://cdn.bbsmc.net/bbsmc/data/OIIWCwpQ/images/fce31aca660ea4b6cf77ce8e51468d4e6585c0d8_350.webp",
    title: "机械动力:齿轮盛宴",
    description:
      "欢迎来到齿轮盛宴的世界,欢迎来到齿轮盛宴的世界,也能体验到沉浸式做菜的欢乐,更能体验到丰富的世界之旅，怪物、美景层出不穷，美不胜收,愿齿轮盛宴能给你带来盛宴一般的感受.",
    slug: "/modpack/create-delight",
  },
]);

const currentSlide = ref(0);

// 拖动相关状态
const isDragging = ref(false);
const dragStartX = ref(0);
const dragCurrentX = ref(0);
const hasDragged = ref(false); // 标记是否发生了拖动

// 处理拖动开始
const handleDragStart = (e) => {
  const isTouchEvent = e.type.includes("touch");

  isDragging.value = true;
  hasDragged.value = false; // 重置拖动标志
  dragStartX.value = isTouchEvent ? e.touches[0].clientX : e.clientX;
  dragCurrentX.value = dragStartX.value;
  stopAutoPlay(); // 停止自动播放

  if (!isTouchEvent) {
    e.preventDefault(); // 阻止默认行为
    // 桌面端：在 document 上监听 mousemove 和 mouseup
    document.addEventListener("mousemove", handleDragMove);
    document.addEventListener("mouseup", handleDragEnd);
  }
};

// 处理拖动中
const handleDragMove = (e) => {
  if (!isDragging.value) return;

  const isTouchEvent = e.type.includes("touch");
  const currentX = isTouchEvent ? e.touches[0].clientX : e.clientX;
  dragCurrentX.value = currentX;

  // 如果拖动距离超过5px，标记为已拖动
  const distance = Math.abs(currentX - dragStartX.value);
  if (distance > 5) {
    hasDragged.value = true;
    e.preventDefault();
  }
};

// 处理拖动结束
const handleDragEnd = (_e) => {
  if (!isDragging.value) return;

  const dragDistance = dragCurrentX.value - dragStartX.value;
  const threshold = 50; // 拖动超过50px才切换

  if (Math.abs(dragDistance) > threshold) {
    if (dragDistance > 0) {
      // 向右拖动，显示上一张
      prevSlide();
    } else {
      // 向左拖动，显示下一张
      nextSlide();
    }
  }

  isDragging.value = false;
  startAutoPlay(); // 恢复自动播放

  // 移除 document 上的监听器
  document.removeEventListener("mousemove", handleDragMove);
  document.removeEventListener("mouseup", handleDragEnd);

  // 延迟重置拖动位置，让 click 事件能正确判断
  setTimeout(() => {
    dragStartX.value = 0;
    dragCurrentX.value = 0;
  }, 10);
};

// 上一张
const prevSlide = () => {
  currentSlide.value =
    currentSlide.value === 0 ? carouselItems.value.length - 1 : currentSlide.value - 1;
  startAutoPlay(); // 重置自动播放计时器
};

// 处理 Banner 点击（防止拖动后触发跳转）
const handleBannerClick = (e, url) => {
  e.preventDefault();
  e.stopPropagation();

  // 如果发生了拖动，不跳转
  if (hasDragged.value) {
    hasDragged.value = false; // 重置标志
    return;
  }

  // 否则正常跳转
  window.open(url, "_blank");
};

// 搜索功能
const handleSearch = async () => {
  const query = searchQuery.value.trim();

  if (query) {
    // 调用搜索API，不限制项目类型
    const searchResponse = await useBaseFetch(
      `search?limit=8&index=relevance&query=${encodeURIComponent(query)}`,
    );

    modpacks.value =
      searchResponse.hits?.map((project) => ({
        ...project,
        slug: project.slug || project.project_id,
        featured_gallery:
          project.featured_gallery ||
          (project.gallery?.length > 0 ? project.gallery[0] : project.icon_url),
      })) ?? [];
  } else {
    // 搜索框为空时，恢复显示热门整合包
    await getProjects();
  }
};

// 防抖定时器
let searchTimeout = null;

// 实时搜索
const handleSearchInput = () => {
  // 清除之前的定时器
  if (searchTimeout) {
    clearTimeout(searchTimeout);
  }

  // 设置新的定时器，300ms 后执行搜索
  searchTimeout = setTimeout(() => {
    handleSearch();
  }, 300);
};

// 格式化数字显示（下载量等）
const formatNumber = (num) => {
  if (!num) return "0";

  if (num >= 10000) {
    return (num / 10000).toFixed(1).replace(/\.0$/, "") + "万";
  }
  return num.toString();
};

// 根据项目类型生成链接
const getProjectLink = (project) => {
  const typeMap = {
    mod: "mod",
    modpack: "modpack",
    plugin: "plugin",
    resourcepack: "resourcepack",
    shader: "shader",
    datapack: "datapack",
  };
  const type = typeMap[project.project_type] || project.project_type;
  return `/${type}/${project.slug}`;
};

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
.container {
  max-width: 1224px;
  margin: auto;
  min-height: 724px;
}

.game-page {
  position: relative;
  z-index: 2;
}

/* Banner 标题样式 */
.banner-title {
  font-size: 3rem;
  font-weight: 700;
  margin-bottom: 1rem;
  line-height: 1.2;
  font-family:
    "Space Grotesk",
    var(--montserrat-font),
    system-ui,
    -apple-system,
    sans-serif;
  color: #ffffff !important;
}

.banner-description {
  font-size: 1.25rem;
  line-height: 1.75rem;
  max-width: 42rem;
  color: #d1d5db !important;
}

/* 卡片标题样式 */
.card-title {
  font-weight: 700;
  font-size: 1.25rem;
  line-height: 1.75rem;
  font-family:
    "Space Grotesk",
    var(--montserrat-font),
    system-ui,
    -apple-system,
    sans-serif;
  color: #ffffff !important;
}

.card-author {
  font-size: 0.875rem;
  line-height: 1.25rem;
  color: #d1d5db !important;
  margin-top: 0.25rem;
}

/* 矿工茶馆卡片样式 */
.forum-card {
  display: block;
  padding: 1rem;
  border-radius: 0.75rem;
  background: var(--color-raised-bg);
  box-shadow:
    0 1px 3px 0 rgba(0, 0, 0, 0.1),
    0 1px 2px 0 rgba(0, 0, 0, 0.06);
  transition: all 0.2s ease;
  text-decoration: none;
  border: 1px solid transparent;
}

.forum-card:hover {
  transform: translateY(-2px);
  box-shadow:
    0 10px 15px -3px rgba(0, 0, 0, 0.1),
    0 4px 6px -2px rgba(0, 0, 0, 0.05);
  border-color: var(--color-brand);
}

/* 用户头像 */
.forum-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
  border: 2px solid var(--color-divider);
  transition: border-color 0.2s;
}

.forum-card:hover .forum-avatar {
  border-color: var(--color-brand);
}

/* 标题 */
.forum-card-title {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--color-text-dark);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  line-height: 1.4;
  transition: color 0.2s;
}

.forum-card:hover .forum-card-title {
  color: var(--color-brand);
}

/* 用户名 */
.forum-username {
  font-size: 0.75rem;
  color: var(--color-text);
  font-weight: 500;
}

/* 回复数 */
.forum-replies {
  font-size: 0.75rem;
  color: var(--color-text);
  opacity: 0.7;
}

/* 时间 */
.forum-card-time {
  font-size: 0.75rem;
  color: var(--color-text);
  margin: 0;
  white-space: nowrap;
  text-align: right;
}

/* 搜索框样式 */
.search-input {
  border: 1px solid rgba(128, 128, 128, 0.3) !important;
  box-shadow: none !important;
}

.search-input:focus {
  border: 1px solid rgba(128, 128, 128, 0.6) !important;
  box-shadow: 0 0 0 3px rgba(128, 128, 128, 0.1) !important;
}

/* Banner 链接样式重置 */
section a:focus {
  outline: none;
  box-shadow: none;
}

section a:active {
  outline: none;
  box-shadow: none;
}

.user-select-none {
  user-select: none;
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
}

/* 响应式调整 */
@media (max-width: 768px) {
  .container {
    padding: 0 8px;
  }

  .banner-title {
    font-size: 2.25rem;
  }

  .banner-description {
    font-size: 1.125rem;
  }
}

@media (min-width: 768px) {
  .banner-title {
    font-size: 3rem;
  }

  .banner-description {
    font-size: 1.25rem;
  }
}
</style>
