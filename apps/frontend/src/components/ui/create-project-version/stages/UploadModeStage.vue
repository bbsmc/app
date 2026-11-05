<template>
  <div class="flex w-full flex-col gap-6">
    <div class="flex flex-col gap-2">
      <span class="text-base font-semibold text-contrast">
        上传方式 <span class="text-red">*</span>
      </span>
      <span class="text-xs text-secondary">
        选择此版本下用户的下载方式。可仅站内、仅网盘，或两者并存。
      </span>
    </div>

    <div class="grid grid-cols-1 gap-3">
      <!-- 选项 1：纯站内文件 -->
      <button
        type="button"
        class="flex flex-col gap-1 rounded-xl border-2 border-solid p-4 text-left transition-all"
        :class="
          uploadMode === 'local'
            ? 'border-brand bg-highlight-green text-brand'
            : 'border-surface-5 hover:bg-button-bg-hover bg-button-bg text-contrast'
        "
        @click="handleSelect('local')"
      >
        <div class="flex items-center gap-2 text-base font-semibold">
          <UploadIcon />
          仅站内文件
          <CheckCircleIcon v-if="uploadMode === 'local'" class="ml-auto" />
        </div>
        <div class="text-xs" :class="uploadMode === 'local' ? 'opacity-80' : 'text-secondary'">
          上传到本站 CDN。用户在版本页可直接下载，体验最佳。
        </div>
      </button>

      <!-- 选项 2：纯网盘 -->
      <button
        type="button"
        class="flex flex-col gap-1 rounded-xl border-2 border-solid p-4 text-left transition-all"
        :class="
          uploadMode === 'disk'
            ? 'border-brand bg-highlight-green text-brand'
            : 'border-surface-5 hover:bg-button-bg-hover bg-button-bg text-contrast'
        "
        @click="handleSelect('disk')"
      >
        <div class="flex items-center gap-2 text-base font-semibold">
          <LinkIcon />
          仅网盘下载
          <CheckCircleIcon v-if="uploadMode === 'disk'" class="ml-auto" />
        </div>
        <div class="text-xs" :class="uploadMode === 'disk' ? 'opacity-80' : 'text-secondary'">
          不上传站内文件，仅提供网盘链接（夸克 / 迅雷 / 百度等）。用于网盘合作或大文件分发。
        </div>
      </button>

      <!-- 选项 3：站内文件 + 网盘 -->
      <button
        type="button"
        class="flex flex-col gap-1 rounded-xl border-2 border-solid p-4 text-left transition-all"
        :class="
          uploadMode === 'both'
            ? 'border-brand bg-highlight-green text-brand'
            : 'border-surface-5 hover:bg-button-bg-hover bg-button-bg text-contrast'
        "
        @click="handleSelect('both')"
      >
        <div class="flex items-center gap-2 text-base font-semibold">
          <PlusIcon />
          站内文件 + 网盘
          <CheckCircleIcon v-if="uploadMode === 'both'" class="ml-auto" />
        </div>
        <div class="text-xs" :class="uploadMode === 'both' ? 'opacity-80' : 'text-secondary'">
          两种方式并存。用户既可以从站内 CDN 下载，也可以选择网盘链接。
        </div>
      </button>
    </div>

    <!-- 网盘模式 + Minecraft 类型下的整合包确认（软件资源 / 汉化包不需要询问） -->
    <div
      v-if="shouldAskModpack"
      class="border-orange-500/60 bg-orange-500/10 flex flex-col gap-3 rounded-xl border-2 border-solid p-4"
    >
      <div class="flex items-center gap-2 font-semibold text-contrast">
        <InfoIcon class="text-orange-500" />
        重要：是否为整合包资源？
      </div>
      <div class="text-sm text-secondary">
        请确认本网盘文件是否为<b>整合包 / 导入包</b
        >类型。整合包标识会影响搜索、下载页展示与统计，不准确会导致用户误下载。
      </div>
      <div class="flex flex-col gap-2 sm:flex-row">
        <button
          type="button"
          class="flex-1 rounded-lg border-2 border-solid px-3 py-3 text-sm font-semibold transition-all"
          :class="
            draftVersion.is_modpack === true
              ? 'border-orange-500 bg-orange-500 text-white shadow-md'
              : 'border-surface-5 hover:bg-button-bg-hover bg-button-bg text-contrast'
          "
          @click="draftVersion.is_modpack = true"
        >
          ✓ 是的，这是整合包资源
        </button>
        <button
          type="button"
          class="flex-1 rounded-lg border-2 border-solid px-3 py-3 text-sm font-semibold transition-all"
          :class="
            draftVersion.is_modpack === false
              ? 'border-brand bg-brand text-white shadow-md'
              : 'border-surface-5 hover:bg-button-bg-hover bg-button-bg text-contrast'
          "
          @click="draftVersion.is_modpack = false"
        >
          ✗ 不是整合包
        </button>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";

import { CheckCircleIcon, InfoIcon, LinkIcon, PlusIcon, UploadIcon } from "@modrinth/assets";

import {
  injectManageVersionContext,
  type UploadMode,
} from "~/providers/version/manage-version-modal";

const { draftVersion, uploadMode, needsDiskInputs, modal } = injectManageVersionContext();

/**
 * 是否需要在本步询问"是否整合包"：
 * - 必须是网盘模式（disk / both）—— 站内文件靠 manifest.json/modrinth.index.json 自动检测
 * - 必须是 Minecraft 资源类型 —— 软件资源 / 汉化包跟整合包概念无关
 */
const shouldAskModpack = computed(
  () => needsDiskInputs.value && draftVersion.value.type === "minecraft",
);

function handleSelect(mode: UploadMode) {
  uploadMode.value = mode;
  // 纯站内模式：跳过整合包询问（is_modpack 由文件检测决定），直接下一步
  if (mode === "local") {
    modal.value?.nextStage();
    return;
  }
  // 非 minecraft 类型选了网盘：也不需要问整合包，直接下一步
  if (draftVersion.value.type !== "minecraft") {
    modal.value?.nextStage();
  }
  // minecraft 类型 + 网盘模式：留在本步等用户回答整合包问题
}
</script>
