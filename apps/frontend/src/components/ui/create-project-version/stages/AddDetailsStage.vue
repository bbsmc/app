<template>
  <div class="flex flex-col gap-6">
    <div class="flex flex-col gap-2">
      <span class="font-semibold text-contrast"> 版本类型 <span class="text-red">*</span> </span>
      <Chips
        v-model="draftVersion.version_type"
        :items="['release', 'alpha', 'beta']"
        :never-empty="true"
        :capitalize="true"
      />
    </div>
    <div class="flex flex-col gap-2">
      <span class="font-semibold text-contrast"> 版本号 <span class="text-red">*</span> </span>
      <input
        id="version-number"
        v-model="draftVersion.version_number"
        placeholder="输入版本号，例如 1.2.3-alpha.1"
        type="text"
        autocomplete="off"
        maxlength="32"
      />
      <span> 版本号用于区分此版本与其他版本。 </span>
    </div>
    <div class="flex flex-col gap-2">
      <span class="font-semibold text-contrast"> 版本副标题 </span>
      <input
        id="version-number"
        v-model="draftVersion.name"
        placeholder="输入副标题..."
        type="text"
        autocomplete="off"
        maxlength="32"
      />
    </div>

    <template v-if="!noLoadersProject && (inferredVersionData?.loaders?.length || editingVersion)">
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between">
          <span class="font-semibold text-contrast">
            {{ usingDetectedLoaders ? "检测到的加载器" : "加载器" }}
          </span>

          <ButtonStyled type="transparent" size="standard">
            <button
              v-tooltip="isModpack ? 'Modpack versions cannot be edited' : undefined"
              :disabled="isModpack"
              @click="editLoaders"
            >
              <EditIcon />
              编辑
            </button>
          </ButtonStyled>
        </div>

        <div
          class="border-surface-5 flex flex-col gap-1.5 gap-y-4 rounded-xl border border-solid p-3 py-4"
        >
          <div class="flex flex-wrap gap-2">
            <template
              v-for="loader in draftVersion.loaders.map((selectedLoader) =>
                loaders.find((loader) => selectedLoader === loader.name),
              )"
            >
              <TagItem
                v-if="loader"
                :key="`loader-${loader.name}`"
                class="border-surface-5 border !border-solid hover:no-underline"
                :style="`--_color: var(--color-platform-${loader.name})`"
              >
                <div v-html="loader.icon"></div>
                {{ formatCategory(loader.name) }}
              </TagItem>
            </template>

            <span v-if="!draftVersion.loaders.length">未选择加载器。</span>
          </div>
        </div>
      </div>
    </template>

    <template v-if="inferredVersionData?.game_versions?.length || editingVersion">
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between">
          <span class="font-semibold text-contrast">
            {{ usingDetectedVersions ? "检测到的版本" : "版本" }}
          </span>

          <ButtonStyled type="transparent" size="standard">
            <button
              v-tooltip="isModpack ? 'Modpack versions cannot be edited' : undefined"
              :disabled="isModpack"
              @click="editVersions"
            >
              <EditIcon />
              编辑
            </button>
          </ButtonStyled>
        </div>

        <div
          class="border-surface-5 flex flex-col gap-1.5 gap-y-4 rounded-xl border border-solid p-3 py-4"
        >
          <div class="flex flex-wrap gap-2">
            <TagItem
              v-for="version in draftVersion.game_versions"
              :key="version"
              class="border-surface-5 border !border-solid hover:no-underline"
            >
              {{ version }}
            </TagItem>

            <span v-if="!draftVersion.game_versions.length">未选择版本。</span>
          </div>
        </div>
      </div>
    </template>

    <template
      v-if="
        !noEnvironmentProject &&
        ((!editingVersion && inferredVersionData?.environment) ||
          (editingVersion && draftVersion.environment))
      "
    >
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between">
          <span class="font-semibold text-contrast"> 运行环境 </span>

          <ButtonStyled type="transparent" size="standard">
            <button @click="editEnvironment">
              <EditIcon />
              编辑
            </button>
          </ButtonStyled>
        </div>

        <div class="bg-surface-2 flex flex-col gap-1.5 gap-y-4 rounded-xl p-3 py-4">
          <div v-if="draftVersion.environment" class="flex flex-col gap-1">
            <div class="font-semibold text-contrast">
              {{ environmentCopy.title }}
            </div>
            <div class="text-sm font-medium">{{ environmentCopy.description }}</div>
          </div>

          <span v-else class="text-sm font-medium">尚未设置运行环境。</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { EditIcon } from "@modrinth/assets";
import { ButtonStyled, Chips, TagItem } from "@modrinth/ui";
import { formatCategory } from "@modrinth/utils";

import { useTags } from "~/composables/tag";
import { injectManageVersionContext } from "~/providers/version/manage-version-modal";

const {
  draftVersion,
  inferredVersionData,
  projectType,
  editingVersion,
  noLoadersProject,
  noEnvironmentProject,
  modal,
} = injectManageVersionContext();

const tags = useTags();
const loaders = computed(() => tags.value.loaders);
const isModpack = computed(() => projectType.value === "modpack");

const editLoaders = () => {
  modal.value?.setStage("from-details-loaders");
};
const editVersions = () => {
  modal.value?.setStage("from-details-mc-versions");
};
const editEnvironment = () => {
  modal.value?.setStage("from-details-environment");
};

const usingDetectedVersions = computed(() => {
  if (!inferredVersionData.value?.game_versions) return false;

  const versionsMatch =
    draftVersion.value.game_versions.length === inferredVersionData.value.game_versions.length &&
    draftVersion.value.game_versions.every((version) =>
      inferredVersionData.value?.game_versions?.includes(version),
    );

  return versionsMatch;
});

const usingDetectedLoaders = computed(() => {
  if (!inferredVersionData.value?.loaders) return false;

  const loadersMatch =
    draftVersion.value.loaders.length === inferredVersionData.value.loaders.length &&
    draftVersion.value.loaders.every((loader) =>
      inferredVersionData.value?.loaders?.includes(loader),
    );

  return loadersMatch;
});

const environmentCopy = computed(() => {
  const emptyMessage = {
    title: "未设置运行环境",
    description: "此版本的运行环境尚未指定。",
  };
  if (!draftVersion.value.environment) return emptyMessage;

  const envCopy: Record<string, { title: string; description: string }> = {
    client_only: {
      title: "仅客户端",
      description: "所有功能均在客户端运行，兼容原版服务端。",
    },
    server_only: {
      title: "仅服务端",
      description: "所有功能均在服务端运行，兼容原版客户端。",
    },
    singleplayer_only: {
      title: "仅单人游戏",
      description: "仅在单人游戏或未连接多人服务器时有效。",
    },
    dedicated_server_only: {
      title: "仅服务端",
      description: "所有功能均在服务端运行，兼容原版客户端。",
    },
    client_and_server: {
      title: "客户端和服务端",
      description: "在客户端和服务端均有部分或全部功能。",
    },
    client_only_server_optional: {
      title: "客户端和服务端",
      description: "在客户端和服务端均有部分或全部功能。",
    },
    server_only_client_optional: {
      title: "客户端和服务端",
      description: "在客户端和服务端均有部分或全部功能。",
    },
    client_or_server: {
      title: "客户端和服务端",
      description: "在客户端和服务端均有部分或全部功能。",
    },
    client_or_server_prefers_both: {
      title: "客户端和服务端",
      description: "在客户端和服务端均有部分或全部功能。",
    },
    unknown: {
      title: "未知环境",
      description: "无法确定此版本的运行环境。",
    },
  };

  return (
    envCopy[draftVersion.value.environment] || {
      title: "未知环境",
      description: `无法识别的运行环境："${draftVersion.value.environment}"。`,
    }
  );
});
</script>
