<template>
  <div class="flex w-full flex-col gap-5">
    <!-- 资源类型选择（强制必选；进入 modal 第一步） -->
    <div class="flex flex-col gap-2">
      <span class="text-base font-semibold text-contrast">
        资源类型 <span class="text-red">*</span>
      </span>
      <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
        <button
          v-for="opt in versionTypeOptions"
          :key="opt.value"
          type="button"
          class="hover:bg-button-bg-hover rounded-xl border border-solid bg-button-bg px-3 py-2.5 text-left text-contrast transition-colors"
          :class="draftVersion.type === opt.value ? 'border-brand !text-brand' : 'border-surface-5'"
          :disabled="editingVersion"
          @click="handleSelectType(opt.value)"
        >
          <div class="font-semibold">{{ opt.label }}</div>
          <div class="text-xs text-secondary">{{ opt.hint }}</div>
        </button>
      </div>
      <span v-if="!draftVersion.type" class="text-xs text-secondary">
        请先选择资源类型再进入下一步。
      </span>
    </div>

    <Admonition v-if="draftVersion.type === 'language'" type="info">
      汉化包只能上传 .zip 文件，并在「依赖」步骤添加被翻译的源版本链接。
    </Admonition>
  </div>
</template>

<script lang="ts" setup>
import { Admonition, injectProjectPageContext } from "@modrinth/ui";

import { injectManageVersionContext } from "~/providers/version/manage-version-modal";

const { draftVersion, editingVersion } = injectManageVersionContext();
const { projectV2 } = injectProjectPageContext();

const versionTypeOptions = [
  { value: "minecraft" as const, label: "Minecraft 资源", hint: "模组、资源包、整合包等" },
  { value: "software" as const, label: "软件资源", hint: "独立运行的软件" },
  { value: "language" as const, label: "汉化包", hint: "对其他版本的翻译" },
  { value: "map" as const, label: "地图", hint: "完整存档、建筑模板、结构文件" },
];

const { modal } = injectManageVersionContext();

/**
 * 根据当前选中的资源类型 + 项目类型重置 loaders（避免切换类型时遗留旧值）
 * - language → ['language']
 * - minecraft + resourcepack 项目 → ['minecraft']
 * - minecraft + modpack 项目 → ['mrpack']
 * - minecraft + datapack 项目 → ['datapack']
 * - 其它 → 清空让用户在 loaders 步骤选择
 */
function resetLoadersForCurrentType() {
  const pt = projectV2.value?.project_type;
  if (draftVersion.value.type === "language") {
    draftVersion.value.loaders = ["language"];
  } else if (draftVersion.value.type === "map" || pt === "map") {
    draftVersion.value.loaders = ["map"];
  } else if (pt === "resourcepack") {
    draftVersion.value.loaders = ["minecraft"];
  } else if (pt === "modpack") {
    draftVersion.value.loaders = ["mrpack"];
  } else if (pt === "datapack") {
    draftVersion.value.loaders = ["datapack"];
  } else {
    draftVersion.value.loaders = [];
  }
}

function handleSelectType(value: "minecraft" | "software" | "language" | "map") {
  if (editingVersion.value) return;
  const prevType = draftVersion.value.type;
  draftVersion.value.type = value;

  // 切换到不同类型时清空旧类型的 loaders / game_versions / environment 遗留
  // 避免：先选汉化包（自动 ['language']）→ 切回 minecraft 时仍残留 ['language']
  if (prevType !== value) {
    resetLoadersForCurrentType();
    draftVersion.value.game_versions = [];
    draftVersion.value.environment = undefined;
  }

  // 选完后自动跳到下一步（上传方式）
  modal.value?.nextStage();
}
</script>
