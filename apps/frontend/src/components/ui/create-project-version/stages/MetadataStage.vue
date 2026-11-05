<template>
  <div class="flex flex-col gap-4">
    <div v-if="!editingVersion" class="text-sm font-semibold text-contrast">确认信息</div>
    <div v-if="!editingVersion" class="text-sm text-secondary">
      请检查所有信息无误后，点击下方"创建版本"按钮完成发布。
    </div>

    <!-- 资源类型（创建时显示；编辑模式不允许修改资源类型） -->
    <div v-if="!editingVersion || draftVersion.type" class="flex flex-col gap-1">
      <span class="text-sm font-semibold text-contrast">资源类型</span>
      <div class="rounded-xl bg-button-bg p-3 text-sm">
        <span v-if="draftVersion.type === 'software'">软件资源</span>
        <span v-else-if="draftVersion.type === 'minecraft'">Minecraft 资源</span>
        <span v-else-if="draftVersion.type === 'language'">汉化包</span>
        <span v-else-if="draftVersion.type === 'map'">地图</span>
        <span v-else class="text-secondary">未选择</span>
      </div>
    </div>

    <!-- 加载器 -->
    <SectionRow
      v-if="showLoaders"
      label="加载器"
      :editable="editingVersion && allowEditLoaders"
      @edit="goto('from-details-loaders')"
    >
      <span v-if="draftVersion.loaders?.length">{{ draftVersion.loaders.join(", ") }}</span>
      <span v-else class="text-secondary">未选择</span>
    </SectionRow>

    <!-- 游戏版本 -->
    <SectionRow
      v-if="showGameVersions"
      label="游戏版本"
      :editable="editingVersion"
      @edit="goto('from-details-mc-versions')"
    >
      <span v-if="draftVersion.game_versions?.length">{{
        draftVersion.game_versions.join(", ")
      }}</span>
      <span v-else class="text-secondary">未选择</span>
    </SectionRow>

    <!-- 环境 -->
    <SectionRow
      v-if="showEnvironment"
      label="环境"
      :editable="editingVersion"
      @edit="goto('from-details-environment')"
    >
      <template v-if="envCopy">
        <div class="font-semibold">{{ envCopy.title }}</div>
        <div class="text-xs text-secondary">{{ envCopy.description }}</div>
      </template>
      <span v-else class="text-secondary">未设置</span>
    </SectionRow>

    <!-- 文件 -->
    <SectionRow label="已选文件" :editable="editingVersion" @edit="goto('from-details-files')">
      <div
        v-if="filesToAdd.length || draftVersion.existing_files?.length"
        class="flex flex-col gap-1"
      >
        <div v-for="(f, idx) in filesToAdd" :key="`add-${idx}`" class="text-sm">
          <span v-if="idx === 0" class="mr-2 font-semibold text-green">主文件</span>
          {{ f.file.name }}
        </div>
        <div v-for="(f, idx) in draftVersion.existing_files" :key="`exist-${idx}`" class="text-sm">
          <span v-if="idx === 0 && !filesToAdd.length" class="mr-2 font-semibold text-green">
            主文件
          </span>
          {{ f.filename }}
        </div>
      </div>
      <span v-else class="text-secondary">无</span>
    </SectionRow>

    <!-- 网盘链接 -->
    <SectionRow
      v-if="diskList.length || (editingVersion && needsDiskInputs)"
      label="网盘链接"
      :editable="editingVersion"
      @edit="goto('from-details-files')"
    >
      <div v-if="diskList.length" class="flex flex-col gap-1">
        <div v-for="(d, idx) in diskList" :key="`disk-${idx}`" class="text-sm">
          <span class="mr-2 font-semibold">{{ diskLabel(d.platform) }}</span>
          <span class="break-all text-secondary">{{ d.url }}</span>
        </div>
      </div>
      <span v-else class="text-secondary">无</span>
    </SectionRow>

    <!-- 依赖 / 翻译链接 -->
    <SectionRow
      v-if="!noDependenciesProject"
      :label="isLanguageVersion ? '翻译链接' : '依赖'"
      :editable="editingVersion"
      @edit="goto('from-details-dependencies')"
    >
      <template v-if="isLanguageVersion">
        <div v-if="draftVersion.version_links?.length" class="flex flex-col gap-1">
          <div
            v-for="(link, idx) in draftVersion.version_links"
            :key="`link-${idx}`"
            class="text-sm"
          >
            <span class="mr-2 font-semibold">{{ langLabel(link.language_code) }}</span>
            <span class="break-all text-secondary">{{ link.description || "" }}</span>
          </div>
        </div>
        <span v-else class="text-secondary">无翻译链接</span>
      </template>
      <template v-else>
        <div v-if="draftVersion.dependencies?.length" class="flex flex-col gap-1">
          <div v-for="(dep, idx) in draftVersion.dependencies" :key="`dep-${idx}`" class="text-sm">
            {{ dep.dependency_type }} ·
            {{ dep.project_id || dep.version_id || dep.file_name || "" }}
          </div>
        </div>
        <span v-else class="text-secondary">无</span>
      </template>
    </SectionRow>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h } from "vue";

import { EditIcon } from "@modrinth/assets";
import { ENVIRONMENTS_COPY } from "@modrinth/ui";

import {
  injectManageVersionContext,
  TRANSLATION_LANGUAGE_OPTIONS,
} from "~/providers/version/manage-version-modal";

const {
  draftVersion,
  filesToAdd,
  isLanguageVersion,
  editingVersion,
  modal,
  projectType,
  noLoadersProject,
  noEnvironmentProject,
  noDependenciesProject,
  needsDiskInputs,
} = injectManageVersionContext();

const envCopy = computed(() => {
  const env = draftVersion.value.environment;
  if (!env) return null;
  return (ENVIRONMENTS_COPY as Record<string, { title: string; description: string }>)[env];
});

const diskList = computed(() => {
  const d = draftVersion.value;
  const list: Array<{ platform: string; url: string }> = [];
  const push = (platform: string, url?: string) => {
    if (url && url.trim()) list.push({ platform, url });
  };
  push("quark", d.quark_disk);
  push("baidu", d.baidu_disk);
  push("curseforge", d.curseforge);
  push("modrinth", d.modrinth);
  push("xunlei", d.xunlei_disk);
  if (!list.length && d.disk_urls?.length) {
    return d.disk_urls.map((it: { platform: string; url: string }) => ({
      platform: it.platform,
      url: it.url,
    }));
  }
  return list;
});

const diskLabelMap: Record<string, string> = {
  quark: "夸克网盘",
  xunlei: "迅雷网盘",
  baidu: "百度网盘",
  modrinth: "Modrinth 转载",
  curseforge: "CurseForge 转载",
};
function diskLabel(p: string) {
  return diskLabelMap[p] || p;
}

const langLabelMap: Record<string, string> = TRANSLATION_LANGUAGE_OPTIONS.reduce(
  (acc, opt) => {
    acc[opt.value] = opt.label;
    return acc;
  },
  {} as Record<string, string>,
);
function langLabel(code: string) {
  return langLabelMap[code] || code;
}

// 资源包 / 数据包：loader 由 project_type 锁死，不允许编辑（modpack 例外，可改 mrpack_loaders）
const isModpack = computed(() => projectType.value === "modpack");
const allowEditLoaders = computed(
  () => !isLanguageVersion.value && (!noLoadersProject.value || isModpack.value),
);
// 加载器 / 游戏版本：language 不显示；其他都显示（含 modpack，让用户看到 mrpack_loaders）
const showLoaders = computed(() => !isLanguageVersion.value);
const showGameVersions = computed(
  () => !isLanguageVersion.value && draftVersion.value.type !== "software",
);
const showEnvironment = computed(
  () =>
    !isLanguageVersion.value &&
    draftVersion.value.type !== "software" &&
    !noEnvironmentProject.value,
);

function goto(stageId: string) {
  modal.value?.setStage(stageId);
}

// 行级展示组件：左 label + 右内容 + 编辑按钮（编辑模式下显示）
const SectionRow = defineComponent({
  props: {
    label: { type: String, required: true },
    editable: { type: Boolean, default: false },
  },
  emits: ["edit"],
  setup(props, { emit, slots }) {
    return () =>
      h("div", { class: "flex flex-col gap-1" }, [
        h("div", { class: "flex items-center justify-between" }, [
          h("span", { class: "text-sm font-semibold text-contrast" }, props.label),
          props.editable
            ? h(
                "button",
                {
                  type: "button",
                  class:
                    "flex items-center gap-1 rounded-lg bg-button-bg px-2 py-1 text-xs text-contrast hover:bg-button-bg-hover",
                  onClick: () => emit("edit"),
                },
                [h(EditIcon, { class: "h-3.5 w-3.5" }), "编辑"],
              )
            : null,
        ]),
        h("div", { class: "rounded-xl bg-button-bg p-3 text-sm" }, slots.default?.()),
      ]);
  },
});
</script>
