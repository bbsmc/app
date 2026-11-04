<template>
  <NewModal
    ref="modal"
    :scrollable="true"
    max-content-height="72vh"
    :on-hide="onModalHide"
    :closable="true"
    :close-on-click-outside="false"
  >
    <template #title>
      <div
        v-if="breadcrumbs && !nonProgressStage"
        class="relative w-full"
      >
        <div
          class="pointer-events-none absolute left-0 top-0 bottom-0 w-8 bg-gradient-to-r from-bg-raised to-transparent z-10 transition-opacity duration-200"
          :class="showLeftShadow ? 'opacity-100' : 'opacity-0'"
        />
        <div
          ref="breadcrumbScroller"
          class="flex w-full overflow-x-auto overflow-y-hidden scrollbar-hide pr-6"
          @wheel.prevent="onBreadcrumbWheel"
          @scroll="updateScrollShadows"
        >
          <template v-for="(stage, index) in breadcrumbStages" :key="stage.id">
            <div
              :ref="(el) => setBreadcrumbRef(stage.id, el as HTMLElement | null)"
              class="flex w-max items-center"
            >
              <button
                class="bg-transparent active:scale-95 font-bold text-secondary p-0 w-max py-3 px-1"
                :class="{
                  '!text-contrast font-bold': currentStage?.id === stage.id,
                  'font-bold': currentStage?.id !== stage.id,
                  'opacity-50 cursor-not-allowed': cannotNavigateToStage(index),
                }"
                :disabled="cannotNavigateToStage(index)"
                @click="setStage(stage.id)"
              >
                {{ resolveCtxFn(stage.title, context) }}
              </button>
              <ChevronRightIcon
                v-if="index < breadcrumbStages.length - 1"
                class="h-5 w-5 text-secondary"
                stroke-width="3"
              />
            </div>
          </template>
        </div>
        <div
          class="pointer-events-none absolute right-0 top-0 bottom-0 w-8 bg-gradient-to-l from-bg-raised to-transparent z-10 transition-opacity duration-200"
          :class="showRightShadow ? 'opacity-100' : 'opacity-0'"
        />
      </div>
      <span v-else class="text-lg font-bold text-contrast sm:text-xl">{{ resolvedTitle }}</span>
    </template>

    <progress
      v-if="!nonProgressStage && !disableProgress"
      :value="progressValue"
      max="100"
      class="w-full h-1 appearance-none border-none absolute top-0 left-0"
    ></progress>

    <!-- 内容宽度基准：NewModal 的 modal-body 是 width:fit-content，
         窄内容 stage（如元数据汇总列表）会把弹窗收缩到 ~250px，
         故在此固定内容宽度；小屏由 max-w-full 兜底 -->
    <div class="w-[48rem] max-w-full">
      <component :is="currentStage?.stageContent" />
    </div>

    <template #actions>
      <div
        class="flex flex-col justify-end gap-2 sm:flex-row"
        :class="leftButtonConfig || rightButtonConfig ? 'mt-4' : ''"
      >
        <ButtonStyled v-if="leftButtonConfig" type="outlined">
          <button
            class="!border-surface-5 !shadow-none"
            :class="leftButtonConfig.buttonClass"
            :disabled="leftButtonConfig.disabled"
            @click="leftButtonConfig.onClick"
          >
            <component :is="leftButtonConfig.icon" />
            {{ leftButtonConfig.label }}
          </button>
        </ButtonStyled>
        <ButtonStyled v-if="rightButtonConfig" :color="rightButtonConfig.color">
          <button
            class="!shadow-none"
            :class="rightButtonConfig.buttonClass"
            :disabled="rightButtonConfig.disabled || rightButtonConfig.loading"
            @click="rightButtonConfig.onClick"
          >
            <SpinnerIcon
              v-if="rightButtonConfig.loading && rightButtonConfig.iconPosition === 'before'"
              class="animate-spin"
            />
            <component
              :is="rightButtonConfig.icon"
              v-else-if="rightButtonConfig.iconPosition === 'before'"
              :class="rightButtonConfig.iconClass"
            />
            {{ rightButtonConfig.label }}
            <SpinnerIcon
              v-if="rightButtonConfig.loading && rightButtonConfig.iconPosition === 'after'"
              class="animate-spin"
            />
            <component
              :is="rightButtonConfig.icon"
              v-else-if="rightButtonConfig.iconPosition === 'after'"
              :class="rightButtonConfig.iconClass"
            />
          </button>
        </ButtonStyled>
      </div>
    </template>
  </NewModal>
</template>

<script lang="ts">
import { ChevronRightIcon, UpdatedIcon as SpinnerIcon } from '@modrinth/assets'
import type { Component } from 'vue'

import NewModal from '../modal/NewModal.vue'
import ButtonStyled from './ButtonStyled.vue'
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'

export interface StageButtonConfig {
  label?: string
  icon?: Component | null
  iconPosition?: 'before' | 'after'
  color?: InstanceType<typeof ButtonStyled>['$props']['color']
  disabled?: boolean
  loading?: boolean
  iconClass?: string | null
  buttonClass?: string | null
  onClick?: () => void
}

export type MaybeCtxFn<T, R> = R | ((ctx: T) => R)

export interface StageConfigInput<T> {
  id: string
  stageContent: Component
  title: MaybeCtxFn<T, string>
  skip?: MaybeCtxFn<T, boolean>
  hideStageInBreadcrumb?: MaybeCtxFn<T, boolean>
  /** 该 stage 是否不算入进度（不在面包屑里 / 不显示进度条） */
  nonProgressStage?: MaybeCtxFn<T, boolean>
  /** 当前 stage 是否不允许向前导航（用于面包屑禁用 + next 按钮 disable） */
  cannotNavigateForward?: MaybeCtxFn<T, boolean>
  leftButtonConfig: MaybeCtxFn<T, StageButtonConfig | null>
  rightButtonConfig: MaybeCtxFn<T, StageButtonConfig | null>
}

export function resolveCtxFn<T, R>(value: MaybeCtxFn<T, R>, ctx: T): R {
  return typeof value === 'function' ? (value as (ctx: T) => R)(ctx) : value
}
</script>

<script setup lang="ts" generic="T">
const props = withDefaults(
  defineProps<{
    stages: StageConfigInput<T>[]
    context: T
    /** 顶部是否显示面包屑导航（图1 那条 `Files > Loaders > ...`） */
    breadcrumbs?: boolean
    /** 是否禁用顶部进度条 */
    disableProgress?: boolean
  }>(),
  {
    breadcrumbs: false,
    disableProgress: false,
  },
)

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const currentStageIndex = ref<number>(0)

function show() {
  modal.value?.show()
}

function hide() {
  modal.value?.hide()
}

const setStage = (indexOrId: number | string) => {
  let index: number = 0
  if (typeof indexOrId === 'number') {
    index = indexOrId
    if (index < 0 || index >= props.stages.length) return
  } else {
    index = props.stages.findIndex((stage) => stage.id === indexOrId)
    if (index === -1) return
  }
  while (index < props.stages.length) {
    const skip = props.stages[index]?.skip
    if (!skip || !resolveCtxFn(skip, props.context)) break
    index++
  }
  if (index < props.stages.length) {
    currentStageIndex.value = index
  }
}

const nextStage = () => {
  if (currentStageIndex.value === -1) return
  if (currentStageIndex.value >= props.stages.length - 1) return
  let nextIndex = currentStageIndex.value + 1
  while (nextIndex < props.stages.length) {
    const skip = props.stages[nextIndex]?.skip
    if (!skip || !resolveCtxFn(skip, props.context)) break
    nextIndex++
  }
  if (nextIndex < props.stages.length) {
    currentStageIndex.value = nextIndex
  }
}

const prevStage = () => {
  if (currentStageIndex.value <= 0) return
  let prevIndex = currentStageIndex.value - 1
  while (prevIndex >= 0) {
    const skip = props.stages[prevIndex]?.skip
    if (!skip || !resolveCtxFn(skip, props.context)) break
    prevIndex--
  }
  if (prevIndex >= 0) {
    currentStageIndex.value = prevIndex
  }
}

const currentStage = computed(() => props.stages[currentStageIndex.value])
const context = computed(() => props.context)

const resolvedTitle = computed(() => {
  const stage = currentStage.value
  if (!stage) return ''
  return resolveCtxFn(stage.title, props.context)
})

const leftButtonConfig = computed(() => {
  const stage = currentStage.value
  if (!stage) return null
  return resolveCtxFn(stage.leftButtonConfig, props.context)
})

const rightButtonConfig = computed(() => {
  const stage = currentStage.value
  if (!stage) return null
  return resolveCtxFn(stage.rightButtonConfig, props.context)
})

const nonProgressStage = computed(() => {
  const stage = currentStage.value
  if (!stage) return false
  return resolveCtxFn(stage.nonProgressStage, props.context)
})

const progressValue = computed(() => {
  const isProgressStage = (stage: StageConfigInput<T>) => {
    if (resolveCtxFn(stage.nonProgressStage, props.context)) return false
    const skip = stage.skip ? resolveCtxFn(stage.skip, props.context) : false
    return !skip
  }

  const completedCount = props.stages
    .slice(0, currentStageIndex.value + 1)
    .filter(isProgressStage).length
  const totalCount = props.stages.filter(isProgressStage).length

  return totalCount > 0 ? (completedCount / totalCount) * 100 : 0
})

// === 面包屑导航实现 ===
const breadcrumbScroller = ref<HTMLElement | null>(null)
const breadcrumbRefs = ref<Map<string, HTMLElement>>(new Map())
const showLeftShadow = ref(false)
const showRightShadow = ref(false)

function setBreadcrumbRef(stageId: string, el: HTMLElement | null) {
  if (el) breadcrumbRefs.value.set(stageId, el)
  else breadcrumbRefs.value.delete(stageId)
}

function scrollToCurrentBreadcrumb() {
  const stage = currentStage.value
  if (!stage || !breadcrumbScroller.value) return

  const el = breadcrumbRefs.value.get(stage.id)
  if (!el) return

  nextTick(() => {
    breadcrumbScroller.value?.scrollTo({
      left: el.offsetLeft - 50,
      behavior: 'smooth',
    })
  })
}

function updateScrollShadows() {
  const el = breadcrumbScroller.value
  if (!el) {
    showLeftShadow.value = false
    showRightShadow.value = false
    return
  }

  showLeftShadow.value = el.scrollLeft > 0
  showRightShadow.value = el.scrollLeft < el.scrollWidth - el.clientWidth - 1
}

function onBreadcrumbWheel(e: WheelEvent) {
  if (!breadcrumbScroller.value) return

  const el = breadcrumbScroller.value
  const canScrollHorizontally = el.scrollWidth > el.clientWidth

  if (canScrollHorizontally) {
    const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY
    el.scrollLeft += delta
  }
}

const breadcrumbStages = computed(() => {
  return props.stages.filter((stage) => {
    const visibleStep =
      !resolveCtxFn(stage.skip, props.context) &&
      !resolveCtxFn(stage.nonProgressStage, props.context) &&
      !resolveCtxFn(stage.hideStageInBreadcrumb, props.context)
    return visibleStep
  })
})

function cannotNavigateToStage(breadcrumbIndex: number): boolean {
  const targetStage = breadcrumbStages.value[breadcrumbIndex]
  if (!targetStage) return false

  const targetStageIndex = props.stages.findIndex((s) => s.id === targetStage.id)
  if (targetStageIndex === -1) return false

  // 向后导航总是允许
  if (targetStageIndex <= currentStageIndex.value) return false

  // 向前导航：检查中间所有 stage 都允许 next
  for (let i = currentStageIndex.value; i < targetStageIndex; i++) {
    const stage = props.stages[i]
    if (stage.skip && resolveCtxFn(stage.skip, props.context)) continue
    if (resolveCtxFn(stage.cannotNavigateForward, props.context)) {
      return true
    }
  }

  return false
}

watch([breadcrumbStages, currentStageIndex], () => nextTick(() => updateScrollShadows()), {
  immediate: true,
})

watch(currentStageIndex, () => {
  scrollToCurrentBreadcrumb()
})

const emit = defineEmits<{
  (e: 'refresh-data' | 'hide'): void
}>()

function onModalHide() {
  emit('hide')
}

defineExpose({
  show,
  hide,
  setStage,
  nextStage,
  prevStage,
  currentStageIndex,
})
</script>

<style scoped>
progress {
  background-color: var(--color-button-bg, rgb(30, 30, 30));
}

progress::-webkit-progress-bar {
  background-color: var(--color-button-bg, rgb(30, 30, 30));
}

progress::-webkit-progress-value {
  background-color: var(--color-contrast, rgb(255, 255, 255));
}

progress::-moz-progress-bar {
  background-color: var(--color-contrast, rgb(255, 255, 255));
}

.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
</style>
