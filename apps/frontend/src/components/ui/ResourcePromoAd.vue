<template>
  <NuxtLink :to="target" target="_blank" :rel="rel" class="resource-promo-ad" :aria-label="label">
    <img :src="image" :alt="label" loading="lazy" />
  </NuxtLink>
</template>

<script setup lang="ts">
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
  display: block;
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

  img {
    display: block;
    width: 100%;
    aspect-ratio: 3 / 2;
    object-fit: cover;
  }
}
</style>
