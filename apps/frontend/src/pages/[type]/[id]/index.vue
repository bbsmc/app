<template>
  <section class="normal-page__content">
    <!-- Gallery Carousel - 暂时禁用 -->
    <div
      v-if="false && showGalleryCarousel && sortedGallery.length > 0"
      class="gallery-carousel card"
    >
      <!-- 主图区域 -->
      <div class="carousel-main" @mouseenter="pauseAutoPlay" @mouseleave="resumeAutoPlay">
        <img
          :src="currentImage.url"
          :alt="currentImage.title || '渲染图'"
          class="carousel-main-image"
          @click="openExpanded"
        />
        <!-- 左箭头 -->
        <button
          v-if="sortedGallery.length > 1"
          class="carousel-arrow carousel-arrow--left"
          @click.stop="prevSlide"
        >
          <LeftArrowIcon aria-hidden="true" />
        </button>
        <!-- 右箭头 -->
        <button
          v-if="sortedGallery.length > 1"
          class="carousel-arrow carousel-arrow--right"
          @click.stop="nextSlide"
        >
          <RightArrowIcon aria-hidden="true" />
        </button>
        <!-- 图片标题浮层 -->
        <div v-if="currentImage.title" class="carousel-caption">
          {{ currentImage.title }}
        </div>
        <!-- 计数指示 -->
        <div class="carousel-counter">{{ currentIndex + 1 }} / {{ sortedGallery.length }}</div>
      </div>

      <!-- 点状导航 -->
      <div v-if="sortedGallery.length > 1" class="carousel-dots">
        <button
          v-for="(_, index) in sortedGallery"
          :key="index"
          :class="['carousel-dot', { active: index === currentIndex }]"
          @click="goToSlide(index)"
        />
      </div>
    </div>

    <!-- 全屏展开 Modal -->
    <div v-if="expandedItem != null" class="expanded-image-modal" @click="expandedItem = null">
      <div class="content">
        <img
          class="image"
          :class="{ 'zoomed-in': zoomedIn }"
          :src="expandedItem.raw_url ? expandedItem.raw_url : expandedItem.url"
          :alt="expandedItem.title || '渲染图'"
          @click.stop
        />

        <div class="floating" @click.stop>
          <div class="text">
            <h2 v-if="expandedItem.title">
              {{ expandedItem.title }}
            </h2>
            <p v-if="expandedItem.description">
              {{ expandedItem.description }}
            </p>
          </div>
          <div class="controls">
            <div class="buttons">
              <button class="close circle-button" @click="expandedItem = null">
                <XIcon aria-hidden="true" />
              </button>
              <a
                class="open circle-button"
                target="_blank"
                :href="expandedItem.raw_url || expandedItem.url"
              >
                <ExternalIcon aria-hidden="true" />
              </a>
              <button class="circle-button" @click="zoomedIn = !zoomedIn">
                <ExpandIcon v-if="!zoomedIn" aria-hidden="true" />
                <ContractIcon v-else aria-hidden="true" />
              </button>
              <button
                v-if="sortedGallery.length > 1"
                class="previous circle-button"
                @click="expandedPrev"
              >
                <LeftArrowIcon aria-hidden="true" />
              </button>
              <button
                v-if="sortedGallery.length > 1"
                class="next circle-button"
                @click="expandedNext"
              >
                <RightArrowIcon aria-hidden="true" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 原有的 markdown body -->
    <div
      v-if="project.body"
      class="markdown-body card"
      v-html="renderHighlightedString(project.body || '')"
    />
  </section>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import {
  LeftArrowIcon,
  RightArrowIcon,
  XIcon,
  ExternalIcon,
  ExpandIcon,
  ContractIcon,
} from "@modrinth/assets";
import { renderHighlightedString } from "~/helpers/highlight.js";

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
  organization: {
    type: Object,
    default() {
      return {};
    },
  },
});

// 是否显示轮播
const showGalleryCarousel = computed(() =>
  ["language", "shader"].includes(props.project.project_type),
);

// 按 ordering 排序的 gallery 图片
const sortedGallery = computed(() => {
  if (!props.project.gallery?.length) return [];
  return [...props.project.gallery].sort((a, b) => (a.ordering ?? 0) - (b.ordering ?? 0));
});

// 轮播状态
const currentIndex = ref(0);
const expandedItem = ref(null);
const zoomedIn = ref(false);
let autoPlayTimer = null;

const currentImage = computed(() => sortedGallery.value[currentIndex.value] || {});

// gallery 变化时重置 index
watch(sortedGallery, () => {
  if (currentIndex.value >= sortedGallery.value.length) {
    currentIndex.value = 0;
  }
});

// 导航
function prevSlide() {
  currentIndex.value =
    (currentIndex.value - 1 + sortedGallery.value.length) % sortedGallery.value.length;
  restartAutoPlay();
}

function nextSlide() {
  currentIndex.value = (currentIndex.value + 1) % sortedGallery.value.length;
  restartAutoPlay();
}

function goToSlide(index) {
  currentIndex.value = index;
  restartAutoPlay();
}

// 自动轮播 (5秒)
function startAutoPlay() {
  stopAutoPlay();
  if (sortedGallery.value.length <= 1) return;
  autoPlayTimer = setInterval(() => {
    currentIndex.value = (currentIndex.value + 1) % sortedGallery.value.length;
  }, 5000);
}

function stopAutoPlay() {
  if (autoPlayTimer) {
    clearInterval(autoPlayTimer);
    autoPlayTimer = null;
  }
}

function restartAutoPlay() {
  stopAutoPlay();
  startAutoPlay();
}

function pauseAutoPlay() {
  stopAutoPlay();
}

function resumeAutoPlay() {
  startAutoPlay();
}

// 展开模态框
function openExpanded() {
  expandedItem.value = currentImage.value;
  zoomedIn.value = false;
  stopAutoPlay();
}

function expandedNext() {
  const idx = sortedGallery.value.indexOf(expandedItem.value);
  const nextIdx = (idx + 1) % sortedGallery.value.length;
  expandedItem.value = sortedGallery.value[nextIdx];
  currentIndex.value = nextIdx;
  zoomedIn.value = false;
}

function expandedPrev() {
  const idx = sortedGallery.value.indexOf(expandedItem.value);
  const prevIdx = (idx - 1 + sortedGallery.value.length) % sortedGallery.value.length;
  expandedItem.value = sortedGallery.value[prevIdx];
  currentIndex.value = prevIdx;
  zoomedIn.value = false;
}

// 键盘支持
function handleKeydown(e) {
  if (expandedItem.value == null) return;
  if (e.key === "Escape") {
    expandedItem.value = null;
    startAutoPlay();
  } else if (e.key === "ArrowLeft") {
    expandedPrev();
  } else if (e.key === "ArrowRight") {
    expandedNext();
  }
}

onMounted(() => {
  document.addEventListener("keydown", handleKeydown);
  if (showGalleryCarousel.value && sortedGallery.value.length > 1) {
    startAutoPlay();
  }
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
  stopAutoPlay();
});
</script>

<style lang="scss" scoped>
.gallery-carousel {
  margin-bottom: var(--spacing-card-md);
  overflow: hidden;
  border-radius: var(--size-rounded-card);
}

.carousel-main {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 9;
  background: #000;
  cursor: pointer;
  overflow: hidden;
}

.carousel-main-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  transition: opacity 0.3s ease;
}

.carousel-arrow {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  border: none;
  border-radius: 50%;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s;
  z-index: 2;

  svg {
    width: 1rem;
    height: 1rem;
  }

  .carousel-main:hover & {
    opacity: 1;
  }

  &:hover {
    background: rgba(0, 0, 0, 0.7);
  }

  &--left {
    left: 12px;
  }

  &--right {
    right: 12px;
  }
}

.carousel-caption {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 12px 16px;
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
  color: #fff;
  font-weight: 600;
  pointer-events: none;
}

.carousel-counter {
  position: absolute;
  top: 12px;
  right: 12px;
  padding: 4px 10px;
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  border-radius: 12px;
  font-size: 0.8rem;
  pointer-events: none;
}

.carousel-dots {
  display: flex;
  justify-content: center;
  gap: 8px;
  padding: 10px;
  background: var(--color-raised-bg);
}

.carousel-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: none;
  padding: 0;
  background: var(--color-button-bg);
  cursor: pointer;
  opacity: 0.5;
  transition: all 0.2s;

  &.active {
    opacity: 1;
    background: var(--color-brand);
    transform: scale(1.25);
  }

  &:hover {
    opacity: 1;
  }
}

/* 全屏展开 Modal - 复用 gallery.vue 样式 */
.expanded-image-modal {
  position: fixed;
  z-index: 20;
  overflow: auto;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.7);
  display: flex;
  justify-content: center;
  align-items: center;

  .content {
    position: relative;
    width: calc(100vw - 2 * var(--spacing-card-lg));
    height: calc(100vh - 2 * var(--spacing-card-lg));

    .circle-button {
      padding: 0.5rem;
      line-height: 1;
      display: flex;
      max-width: 2rem;
      color: var(--color-button-text);
      background-color: var(--color-button-bg);
      border-radius: var(--size-rounded-max);
      margin: 0;
      box-shadow: inset 0px -1px 1px rgb(17 24 39 / 10%);

      &:not(:last-child) {
        margin-right: 0.5rem;
      }

      &:hover {
        background-color: var(--color-button-bg-hover) !important;

        svg {
          color: var(--color-button-text-hover) !important;
        }
      }

      &:active {
        background-color: var(--color-button-bg-active) !important;

        svg {
          color: var(--color-button-text-active) !important;
        }
      }

      svg {
        height: 1rem;
        width: 1rem;
      }
    }

    .image {
      position: absolute;
      left: 50%;
      top: 50%;
      transform: translate(-50%, -50%);
      max-width: calc(100vw - 2 * var(--spacing-card-lg));
      max-height: calc(100vh - 2 * var(--spacing-card-lg));
      border-radius: var(--size-rounded-card);

      &.zoomed-in {
        object-fit: cover;
        width: auto;
        height: calc(100vh - 2 * var(--spacing-card-lg));
        max-width: calc(100vw - 2 * var(--spacing-card-lg));
      }
    }

    .floating {
      position: absolute;
      left: 50%;
      transform: translateX(-50%);
      bottom: var(--spacing-card-md);
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--spacing-card-sm);
      transition: opacity 0.25s ease-in-out;
      opacity: 1;
      padding: 2rem 2rem 0 2rem;

      &:not(&:hover) {
        opacity: 0.4;

        .text {
          transform: translateY(2.5rem) scale(0.8);
          opacity: 0;
        }

        .controls {
          transform: translateY(0.25rem) scale(0.9);
        }
      }

      .text {
        display: flex;
        flex-direction: column;
        max-width: 40rem;
        transition:
          opacity 0.25s ease-in-out,
          transform 0.25s ease-in-out;
        text-shadow: 1px 1px 10px #000000d4;
        margin-bottom: 0.25rem;
        gap: 0.5rem;

        h2 {
          color: var(--dark-color-text-dark);
          font-size: 1.25rem;
          text-align: center;
          margin: 0;
        }

        p {
          color: var(--dark-color-text);
          margin: 0;
        }
      }

      .controls {
        background-color: var(--color-raised-bg);
        padding: var(--spacing-card-md);
        border-radius: var(--size-rounded-card);
        transition:
          opacity 0.25s ease-in-out,
          transform 0.25s ease-in-out;
      }
    }
  }
}

.buttons {
  display: flex;

  button {
    margin-right: 0.5rem;
  }
}

/* 移动端适配 */
@media (max-width: 768px) {
  .carousel-arrow {
    width: 32px;
    height: 32px;
  }
}
</style>
