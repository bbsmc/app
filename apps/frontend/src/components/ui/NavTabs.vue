<template>
  <nav
    ref="scrollContainer"
    class="experimental-styles-within nav-tabs-underline relative flex w-fit overflow-x-auto border-b border-divider text-sm font-medium"
  >
    <NuxtLink
      v-for="(link, index) in filteredLinks"
      v-show="link.shown === undefined ? true : link.shown"
      :key="index"
      ref="tabLinkElements"
      :to="query ? (link.href ? `?${query}=${link.href}` : '?') : link.href"
      class="nav-tab-link z-[1] flex flex-row items-center gap-2 px-5 py-3 transition-colors duration-200"
      :class="{
        'font-semibold text-brand': activeIndex === index && !subpageSelected,
        'text-secondary': activeIndex === index && subpageSelected,
        'text-secondary hover:text-contrast': activeIndex !== index,
      }"
    >
      <component :is="link.icon" v-if="link.icon" class="size-5" />
      <span class="text-nowrap">{{ link.label }}</span>
    </NuxtLink>
    <!-- Underline indicator -->
    <div
      v-show="sliderReady"
      class="navtabs-underline-indicator pointer-events-none absolute bottom-0 h-[2px]"
      :class="subpageSelected ? 'bg-secondary' : 'bg-brand'"
      :style="{
        left: sliderLeftPx,
        width: sliderWidthPx,
      }"
      aria-hidden="true"
    ></div>
  </nav>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from "vue";

const route = useNativeRoute();

interface Tab {
  label: string;
  href: string;
  shown?: boolean;
  icon?: string;
  subpages?: string[];
}

const props = defineProps<{
  links: Tab[];
  query?: string;
}>();

const scrollContainer = ref<HTMLElement | null>(null);

const sliderLeft = ref(0);
const sliderWidth = ref(0);
const sliderReady = ref(false);
const activeIndex = ref(-1);
const subpageSelected = ref(false);

const filteredLinks = computed(() =>
  props.links.filter((x) => (x.shown === undefined ? true : x.shown)),
);
const sliderLeftPx = computed(() => `${sliderLeft.value}px`);
const sliderWidthPx = computed(() => `${sliderWidth.value}px`);

const tabLinkElements = ref();

function pickLink() {
  let index = -1;
  subpageSelected.value = false;

  if (props.query) {
    const currentQueryValue = route.query[props.query] as string | undefined;
    for (let i = 0; i < filteredLinks.value.length; i++) {
      const link = filteredLinks.value[i];
      if (link.href === "" && (!currentQueryValue || currentQueryValue === "")) {
        index = i;
        break;
      } else if (link.href && currentQueryValue === link.href) {
        index = i;
        break;
      }
    }
  } else {
    for (let i = filteredLinks.value.length - 1; i >= 0; i--) {
      const link = filteredLinks.value[i];
      if (decodeURIComponent(route.path) === link.href) {
        index = i;
        break;
      } else if (
        decodeURIComponent(route.path).includes(link.href) ||
        (link.subpages &&
          link.subpages.some((subpage) => decodeURIComponent(route.path).includes(subpage)))
      ) {
        index = i;
        subpageSelected.value = true;
        break;
      }
    }
  }

  activeIndex.value = index;

  if (activeIndex.value !== -1) {
    nextTick(() => {
      startAnimation();
    });
  } else {
    sliderReady.value = false;
  }
}

function startAnimation() {
  const el = tabLinkElements.value?.[activeIndex.value]?.$el;

  if (!el || !el.offsetParent) {
    sliderReady.value = false;
    return;
  }

  const elWidth = el.offsetWidth;

  if (elWidth === 0) {
    sliderReady.value = false;
    return;
  }

  const newLeft = el.offsetLeft;
  const newWidth = elWidth;

  if (newLeft < 0) {
    sliderReady.value = false;
    return;
  }

  if (!sliderReady.value) {
    sliderLeft.value = newLeft;
    sliderWidth.value = newWidth;
    sliderReady.value = true;
  } else {
    // Smooth transition for underline movement
    sliderLeft.value = newLeft;
    sliderWidth.value = newWidth;
  }
}

onMounted(() => {
  // 延迟执行确保字体和样式完全加载
  setTimeout(() => {
    pickLink();
  }, 50);
});

watch(
  () => route.path,
  () => pickLink(),
);

watch(
  () => route.query,
  () => {
    if (props.query) {
      pickLink();
    }
  },
  { deep: true },
);
</script>

<style scoped>
.navtabs-underline-indicator {
  transition:
    left 250ms cubic-bezier(0.16, 1, 0.3, 1),
    width 250ms cubic-bezier(0.16, 1, 0.3, 1),
    opacity 150ms ease;
  border-radius: 1px 1px 0 0;
}

.nav-tab-link {
  position: relative;
}
</style>
