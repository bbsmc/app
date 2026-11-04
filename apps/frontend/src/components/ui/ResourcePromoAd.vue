<template>
  <div v-if="!dismissed" class="resource-promo-ad">
    <NuxtLink
      :to="target"
      target="_blank"
      :rel="rel"
      class="resource-promo-ad__link"
      :aria-label="label"
    >
      <img :src="image" :alt="label" loading="lazy" />
    </NuxtLink>
    <button
      v-tooltip="'关闭，24 小时内不再显示'"
      type="button"
      class="resource-promo-ad__close"
      aria-label="关闭广告，24 小时内不再显示"
      @click="close"
    >
      <XIcon aria-hidden="true" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { XIcon } from "@modrinth/assets";

type PromoVariant = "server" | "incentive";

const props = withDefaults(
  defineProps<{
    variant?: PromoVariant;
    affiliateKey?: string | null;
  }>(),
  {
    variant: "server",
    affiliateKey: null,
  },
);

const emit = defineEmits<{ dismiss: [] }>();

const { dismissed, dismiss } = usePromoAdDismissal();

function close() {
  dismiss();
  emit("dismiss");
}

const serverTarget = computed(() => {
  const affiliateKey = props.affiliateKey || "LaotouY";

  if (affiliateKey === "pcl") {
    return "/pcl";
  }

  return `/server?aff=${encodeURIComponent(affiliateKey)}`;
});

const target = computed(() =>
  props.variant === "incentive" ? "/legal/incentive-info" : serverTarget.value,
);

const label = computed(() =>
  props.variant === "incentive" ? "BBSMC 创作者激励广告" : "昱通云联联机服务器广告",
);

const image = computed(() =>
  props.variant === "incentive" ? "/bbsmc-incentive-ad.png" : "/yutong-yunlian-ad.png",
);

const rel = computed(() => (props.variant === "server" ? "noopener sponsored" : "noopener"));
</script>

<style scoped lang="scss">
.resource-promo-ad {
  position: relative;
  width: 100%;
  overflow: hidden;
  border: 1px solid var(--color-divider);
  border-radius: 16px;
  background: var(--color-raised-bg);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.08);
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s ease;

  &:hover {
    border-color: rgba(241, 100, 54, 0.32);
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.12);
    transform: translateY(-1px);
  }
}

.resource-promo-ad__link {
  display: block;

  img {
    display: block;
    width: 100%;
    aspect-ratio: 3 / 2;
    object-fit: cover;
  }
}

// 叠在图片右上角：小号浅色磨砂圆底 + 浅灰 ×，平时尽量不抢广告画面
.resource-promo-ad__close {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.55);
  color: rgba(0, 0, 0, 0.32);
  backdrop-filter: blur(4px);
  cursor: pointer;
  transition:
    background-color 0.15s ease,
    color 0.15s ease;

  svg {
    width: 14px;
    height: 14px;
  }

  // 悬停广告时稍微显眼一点，悬停按钮本身时再加深（两条选择器权重相同，后者覆盖前者）
  .resource-promo-ad:hover & {
    background: rgba(255, 255, 255, 0.82);
    color: rgba(0, 0, 0, 0.5);
  }

  .resource-promo-ad &:hover,
  .resource-promo-ad &:focus-visible {
    background: rgba(255, 255, 255, 0.95);
    color: rgba(0, 0, 0, 0.75);
  }

  &:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
  }
}
</style>
