<template>
  <img
    v-if="src"
    ref="img"
    :style="`--_size: ${cssSize}`"
    class="`experimental-styles-within avatar"
    :class="{
      circle: circle,
      'no-shadow': noShadow,
      raised: raised,
      pixelated: raised,
    }"
    :src="src"
    :alt="alt"
    :loading="loading"
    @load="updatePixelated"
  />
  <svg
    v-else
    class="`experimental-styles-within avatar"
    :style="`--_size: ${cssSize}`"
    :class="{
      circle: circle,
      'no-shadow': noShadow,
      raised: raised,
    }"
    xml:space="preserve"
    fill-rule="evenodd"
    stroke-linecap="round"
    stroke-linejoin="round"
    stroke-miterlimit="1.5"
    clip-rule="evenodd"
    viewBox="0 0 1024 1024"
    aria-hidden="true"
  >
    <path d="M0 0h1024v1024H0V0z" fill="#83A46E" p-id="5299"></path>
    <path
      d="M341.333333 720.592593h341.333334v75.851851H341.333333v-75.851851z"
      fill="#4C8056"
      p-id="5300"
    ></path>
    <path
      d="M758.518519 0H0v265.481481h113.777778V151.703704h322.37037V94.814815h322.370371v94.814815h265.481481V0H758.518519"
      fill="#4B783F"
      p-id="5301"
    ></path>
    <path
      d="M929.185185 796.444444v75.851852H208.592593v-75.851852H0v227.555556h1024V796.444444h-94.814815"
      fill="#476F37"
      p-id="5302"
    ></path>
    <path
      d="M132.740741 341.333333h341.333333v151.703704H132.740741v-151.703704zM568.888889 341.333333h341.333333v151.703704H568.888889v-151.703704z"
      fill="#222222"
      p-id="5303"
    ></path>
  </svg>
</template>

<script setup>
import { ref, computed } from 'vue'

const pixelated = ref(false)
const img = ref(null)

const props = defineProps({
  src: {
    type: String,
    default: null,
  },
  alt: {
    type: String,
    default: '',
  },
  size: {
    type: String,
    default: '2rem',
  },
  circle: {
    type: Boolean,
    default: false,
  },
  noShadow: {
    type: Boolean,
    default: false,
  },
  loading: {
    type: String,
    default: 'eager',
  },
  raised: {
    type: Boolean,
    default: false,
  },
})

const LEGACY_PRESETS = {
  xxs: '1.25rem',
  xs: '2.5rem',
  sm: '3rem',
  md: '6rem',
  lg: '9rem',
}

const cssSize = computed(() => LEGACY_PRESETS[props.size] ?? props.size)

function updatePixelated() {
  if (img.value && img.value.naturalWidth && img.value.naturalWidth <= 96) {
    pixelated.value = true
  } else {
    pixelated.value = false
  }
}
</script>

<style lang="scss" scoped>
.avatar {
  @apply min-w-[--_size] min-h-[--_size] w-[--_size] h-[--_size];
  --_size: 2rem;

  border: 1px solid var(--color-button-border);
  background-color: var(--color-button-bg);
  object-fit: contain;
  border-radius: calc(16 / 96 * var(--_size));

  &.circle {
    border-radius: 50%;
  }

  &:not(.no-shadow) {
    box-shadow: var(--shadow-inset-lg), var(--shadow-card);
  }

  &.no-shadow {
    box-shadow: none;
  }

  &.pixelated {
    image-rendering: pixelated;
  }

  &.raised {
    background-color: var(--color-raised-bg);
  }
}
</style>
