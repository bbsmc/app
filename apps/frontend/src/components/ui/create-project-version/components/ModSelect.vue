<template>
  <Combobox
    v-model="projectId"
    placeholder="选择项目"
    :options="options"
    :searchable="true"
    search-placeholder="按名称、slug 或粘贴 ID 搜索..."
    :no-options-message="searchLoading ? '加载中...' : '未找到结果'"
    @search-input="(query) => handleSearch(query)"
  />
</template>

<script lang="ts" setup>
import { Combobox, injectModrinthClient, injectNotificationManager } from "@modrinth/ui";
import type { DropdownOption } from "@modrinth/ui";
import { useDebounceFn } from "@vueuse/core";
import { defineAsyncComponent, h } from "vue";

const { addNotification } = injectNotificationManager();
const projectId = defineModel<string>();

const searchLoading = ref(false);
const options = ref<DropdownOption<string>[]>([]);

const { labrinth } = injectModrinthClient();

const search = async (query: string) => {
  if (!query.trim()) {
    searchLoading.value = false;
    return;
  }

  try {
    // 不过滤 project_type：后端对依赖无类型限制（可跨类型依赖，
    // 如 modpack→mod、mod→resourcepack），与上游行为一致。
    // 若加 facets [["project_type:mod"]]，v2 兼容层会将其展开为
    // mod/plugin/datapack，导致 resourcepack 等类型搜不到。
    const results = await labrinth.projects_v2.search({
      query: query,
      limit: 20,
    });

    options.value = results.hits.map((hit) => ({
      label: hit.title,
      value: hit.project_id,
      icon: defineAsyncComponent(() =>
        Promise.resolve({
          setup: () => () =>
            h("img", {
              src: hit.icon_url,
              alt: hit.title,
              class: "h-5 w-5 rounded",
            }),
        }),
      ),
    }));
  } catch (error: any) {
    addNotification({
      title: "发生错误",
      text: error.data ? error.data.description : error,
      type: "error",
    });
  }
  searchLoading.value = false;
};

const throttledSearch = useDebounceFn(search, 500);

const handleSearch = async (query: string) => {
  searchLoading.value = true;
  await throttledSearch(query);
};
</script>
