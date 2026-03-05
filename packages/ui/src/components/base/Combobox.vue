<template>
  <div class="combobox" ref="comboboxRef">
    <div
      class="combobox-trigger"
      :class="{ open: isOpen, disabled }"
      @click="toggleDropdown"
    >
      <span v-if="selectedLabel" class="selected-label">{{ selectedLabel }}</span>
      <span v-else class="placeholder">{{ placeholder }}</span>
      <DropdownIcon class="arrow" :class="{ rotate: isOpen }" />
    </div>

    <Transition name="dropdown">
      <div v-if="isOpen" class="combobox-dropdown">
        <div v-if="searchable" class="search-container">
          <input
            ref="searchInput"
            v-model="searchQuery"
            type="text"
            class="search-input"
            :placeholder="searchPlaceholder"
            @input="handleSearchInput"
            @click.stop
          />
        </div>
        <div class="options-list">
          <div
            v-for="option in filteredOptions"
            :key="getOptionValue(option)"
            class="option"
            :class="{ selected: isSelected(option) }"
            @click.stop="selectOption(option)"
          >
            <component
              v-if="option.icon"
              :is="option.icon"
              class="option-icon"
            />
            <span>{{ getOptionLabel(option) }}</span>
          </div>
          <div v-if="filteredOptions.length === 0" class="no-options">
            {{ noOptionsMessage }}
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { DropdownIcon } from '@modrinth/assets'
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'

export interface DropdownOption<T = string> {
  label: string
  value: T
  icon?: any
}

const props = withDefaults(
  defineProps<{
    modelValue?: string | null
    options: DropdownOption<string | null>[]
    placeholder?: string
    searchable?: boolean
    searchPlaceholder?: string
    noOptionsMessage?: string
    disabled?: boolean
  }>(),
  {
    placeholder: '请选择',
    searchable: false,
    searchPlaceholder: '搜索...',
    noOptionsMessage: '未找到选项',
    disabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | null): void
  (e: 'searchInput', query: string): void
}>()

const isOpen = ref(false)
const searchQuery = ref('')
const comboboxRef = ref<HTMLElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)

const selectedLabel = computed(() => {
  const selected = props.options.find(
    (opt) => getOptionValue(opt) === props.modelValue,
  )
  return selected ? getOptionLabel(selected) : null
})

const filteredOptions = computed(() => {
  if (!searchQuery.value) return props.options
  const query = searchQuery.value.toLowerCase()
  return props.options.filter((opt) =>
    getOptionLabel(opt).toLowerCase().includes(query),
  )
})

const getOptionLabel = (option: DropdownOption<string | null>): string => {
  return option.label
}

const getOptionValue = (option: DropdownOption<string | null>): string | null => {
  return option.value
}

const isSelected = (option: DropdownOption<string | null>): boolean => {
  return getOptionValue(option) === props.modelValue
}

const toggleDropdown = () => {
  if (props.disabled) return
  isOpen.value = !isOpen.value
  if (isOpen.value && props.searchable) {
    nextTick(() => {
      searchInput.value?.focus()
    })
  }
}

const selectOption = (option: DropdownOption<string | null>) => {
  emit('update:modelValue', getOptionValue(option))
  isOpen.value = false
  searchQuery.value = ''
}

const handleSearchInput = () => {
  emit('searchInput', searchQuery.value)
}

const handleClickOutside = (event: MouseEvent) => {
  if (comboboxRef.value && !comboboxRef.value.contains(event.target as Node)) {
    isOpen.value = false
    searchQuery.value = ''
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

watch(
  () => props.modelValue,
  () => {
    searchQuery.value = ''
  },
)
</script>

<style lang="scss" scoped>
.combobox {
  position: relative;
  width: 100%;
}

.combobox-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--gap-sm) var(--gap-lg);
  background-color: var(--color-button-bg);
  border-radius: var(--radius-md);
  cursor: pointer;
  user-select: none;
  box-shadow: var(--shadow-inset-sm);
  transition: all 0.2s ease;

  &:hover:not(.disabled) {
    filter: brightness(1.1);
  }

  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &.open {
    border-radius: var(--radius-md) var(--radius-md) 0 0;
  }

  .placeholder {
    color: var(--color-text-secondary);
  }

  .arrow {
    transition: transform 0.2s ease;

    &.rotate {
      transform: rotate(180deg);
    }
  }
}

.combobox-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  z-index: 100;
  background-color: var(--color-button-bg);
  border-radius: 0 0 var(--radius-md) var(--radius-md);
  box-shadow: var(--shadow-card);
  max-height: 300px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.search-container {
  padding: var(--gap-sm);
  border-bottom: 1px solid var(--color-divider);
}

.search-input {
  width: 100%;
  padding: var(--gap-sm) var(--gap-md);
  background-color: var(--color-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  outline: none;

  &:focus {
    border-color: var(--color-brand);
  }
}

.options-list {
  overflow-y: auto;
  max-height: 250px;
}

.option {
  display: flex;
  align-items: center;
  gap: var(--gap-sm);
  padding: var(--gap-md);
  cursor: pointer;
  transition: background-color 0.1s ease;

  &:hover {
    background-color: var(--color-bg);
  }

  &.selected {
    background-color: var(--color-brand);
    color: var(--color-accent-contrast);
  }

  .option-icon {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
  }
}

.no-options {
  padding: var(--gap-md);
  text-align: center;
  color: var(--color-text-secondary);
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
