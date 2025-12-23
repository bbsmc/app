<template>
  <span class="tag-item" :class="[sizeClass, variantClass]">
    <slot />
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    size?: 'sm' | 'md' | 'lg'
    variant?: 'default' | 'primary' | 'secondary'
  }>(),
  {
    size: 'md',
    variant: 'default',
  },
)

const sizeClass = computed(() => `tag-item--${props.size}`)
const variantClass = computed(() => `tag-item--${props.variant}`)
</script>

<style lang="scss" scoped>
.tag-item {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-weight: 500;
  white-space: nowrap;
  background-color: var(--color-button-bg);
  color: var(--color-text);

  &--sm {
    padding: 0.0625rem 0.375rem;
    font-size: 0.625rem;
  }

  &--md {
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
  }

  &--lg {
    padding: 0.25rem 0.625rem;
    font-size: 0.875rem;
  }

  &--default {
    background-color: var(--color-button-bg);
    color: var(--color-text);
  }

  &--primary {
    background-color: var(--color-brand);
    color: var(--color-accent-contrast);
  }

  &--secondary {
    background-color: var(--color-bg);
    color: var(--color-text-secondary);
  }
}
</style>
