<template>
  <div class="admonition" :class="[`admonition--${type}`]">
    <div v-if="showIcon" class="admonition__icon">
      <component :is="iconComponent" />
    </div>
    <div class="admonition__content">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { InfoIcon, IssuesIcon, XIcon, CheckIcon } from '@modrinth/assets'

const props = withDefaults(
  defineProps<{
    type?: 'info' | 'warning' | 'error' | 'success'
    showIcon?: boolean
  }>(),
  {
    type: 'info',
    showIcon: true,
  },
)

const iconComponent = computed(() => {
  switch (props.type) {
    case 'warning':
      return IssuesIcon
    case 'error':
      return XIcon
    case 'success':
      return CheckIcon
    case 'info':
    default:
      return InfoIcon
  }
})
</script>

<style lang="scss" scoped>
.admonition {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  line-height: 1.5;

  &__icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;

    svg {
      width: 1.25rem;
      height: 1.25rem;
    }
  }

  &__content {
    flex: 1;
  }

  &--info {
    background-color: rgba(var(--color-blue-rgb, 59, 130, 246), 0.1);
    border: 1px solid rgba(var(--color-blue-rgb, 59, 130, 246), 0.3);
    color: var(--color-blue, #3b82f6);

    .admonition__icon {
      color: var(--color-blue, #3b82f6);
    }
  }

  &--warning {
    background-color: rgba(var(--color-orange-rgb, 249, 115, 22), 0.1);
    border: 1px solid rgba(var(--color-orange-rgb, 249, 115, 22), 0.3);
    color: var(--color-orange, #f97316);

    .admonition__icon {
      color: var(--color-orange, #f97316);
    }
  }

  &--error {
    background-color: rgba(var(--color-red-rgb, 239, 68, 68), 0.1);
    border: 1px solid rgba(var(--color-red-rgb, 239, 68, 68), 0.3);
    color: var(--color-red, #ef4444);

    .admonition__icon {
      color: var(--color-red, #ef4444);
    }
  }

  &--success {
    background-color: rgba(var(--color-green-rgb, 34, 197, 94), 0.1);
    border: 1px solid rgba(var(--color-green-rgb, 34, 197, 94), 0.3);
    color: var(--color-green, #22c55e);

    .admonition__icon {
      color: var(--color-green, #22c55e);
    }
  }
}
</style>
