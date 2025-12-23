<template>
  <div class="env-selector">
    <div class="env-option">
      <label class="label">
        <span class="label__title">客户端</span>
        <span class="label__description">请选择资源对客户端的支持程度</span>
      </label>
      <DropdownSelect
        v-model="clientSide"
        name="client-side"
        :options="sideTypes"
        :display-name="getSideLabel"
      />
    </div>
    <div class="env-option">
      <label class="label">
        <span class="label__title">服务端</span>
        <span class="label__description">选择该资源在服务端上是否支持</span>
      </label>
      <DropdownSelect
        v-model="serverSide"
        name="server-side"
        :options="sideTypes"
        :display-name="getSideLabel"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import DropdownSelect from './DropdownSelect.vue'

export type Environment = {
  client: 'required' | 'optional' | 'unsupported'
  server: 'required' | 'optional' | 'unsupported'
} | null

const props = defineProps<{
  modelValue?: Environment
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: Environment): void
}>()

const sideTypes = ['required', 'optional', 'unsupported']

const sideLabels: Record<string, string> = {
  required: '必需',
  optional: '可选',
  unsupported: '不支持',
}

const getSideLabel = (value: string) => sideLabels[value] || value

const clientSide = ref(props.modelValue?.client || 'optional')
const serverSide = ref(props.modelValue?.server || 'optional')

watch(
  [clientSide, serverSide],
  ([client, server]) => {
    emit('update:modelValue', { client, server } as Environment)
  },
  { immediate: false },
)

watch(
  () => props.modelValue,
  (newVal) => {
    if (newVal) {
      clientSide.value = newVal.client
      serverSide.value = newVal.server
    }
  },
)
</script>

<style lang="scss" scoped>
.env-selector {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.env-option {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  &__title {
    font-weight: 600;
    color: var(--color-text);
  }

  &__description {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
  }
}
</style>
