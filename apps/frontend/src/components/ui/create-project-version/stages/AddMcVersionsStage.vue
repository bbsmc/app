<template>
  <div class="flex flex-col gap-6">
    <McVersionPicker v-model="draftVersion.game_versions" :game-versions="gameVersions" />
    <div v-if="draftVersion.game_versions.length" class="space-y-1">
      <div class="flex items-center justify-between">
        <span class="font-semibold text-contrast"> 已添加版本 </span>
        <ButtonStyled type="transparent" size="standard">
          <button @click="clearAllVersions()">清除全部</button>
        </ButtonStyled>
      </div>
      <div
        class="border-surface-5 flex max-h-56 flex-col gap-1.5 gap-y-4 overflow-y-auto rounded-xl border border-solid p-3 py-4"
      >
        <div class="flex flex-wrap gap-2">
          <template v-if="draftVersion.game_versions.length">
            <TagItem
              v-for="version in draftVersion.game_versions"
              :key="version"
              :action="() => toggleVersion(version)"
              class="border-surface-5 border !border-solid !transition-all hover:bg-button-bgHover hover:no-underline"
            >
              {{ version }}
              <XIcon />
            </TagItem>
          </template>
          <template v-else>
            <span>未选择版本。</span>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { XIcon } from "@modrinth/assets";
import { ButtonStyled, TagItem } from "@modrinth/ui";

import { injectManageVersionContext } from "~/providers/version/manage-version-modal";

import McVersionPicker from "../components/McVersionPicker.vue";

const tags = useTags();
const gameVersions = tags.value.gameVersions;

const { draftVersion } = injectManageVersionContext();

const toggleVersion = (version: string) => {
  if (draftVersion.value.game_versions.includes(version)) {
    draftVersion.value.game_versions = draftVersion.value.game_versions.filter(
      (v) => v !== version,
    );
  } else {
    draftVersion.value.game_versions.push(version);
  }
};

const clearAllVersions = () => {
  draftVersion.value.game_versions = [];
};
</script>
