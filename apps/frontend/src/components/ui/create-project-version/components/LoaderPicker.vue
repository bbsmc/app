<template>
  <div class="flex flex-col gap-2.5">
    <span class="font-semibold text-contrast">加载器 <span class="text-red">*</span></span>

    <Chips
      v-if="visibleGroups.length > 1"
      v-model="loaderGroup"
      :items="visibleGroups"
      :format-label="(g: GroupLabels) => GROUP_LABEL_TEXT[g]"
      :never-empty="false"
      :capitalize="false"
      size="small"
    />

    <div
      class="border-surface-5 flex min-h-[150px] flex-1 flex-col gap-4 overflow-y-auto rounded-xl border border-solid p-3"
    >
      <div v-if="groupedLoaders[loaderGroup].length" class="flex flex-col gap-1.5">
        <div class="flex flex-wrap gap-2">
          <button
            v-for="loader in groupedLoaders[loaderGroup]"
            :key="`loader-${loader.name}`"
            type="button"
            class="inline-flex items-center gap-1.5 rounded-full border-2 border-solid px-3 py-1.5 text-sm font-medium transition-all hover:opacity-80"
            :class="
              selectedLoaders.includes(loader.name)
                ? 'border-brand bg-highlight-green text-brand'
                : 'border-surface-5 bg-button-bg text-contrast'
            "
            :style="`--_color: var(--color-platform-${loader.name})`"
            @click="toggleLoader(loader.name)"
          >
            <span v-html="loader.icon" class="inline-flex h-5 w-5 items-center justify-center" />
            {{ formatCategory(loader.name) }}
          </button>
        </div>
      </div>
    </div>

    <span>选择此版本支持的加载器。</span>
  </div>
</template>

<script lang="ts" setup>
import type { Labrinth } from "@modrinth/api-client";
import { Chips } from "@modrinth/ui";
import { formatCategory } from "@modrinth/utils";

const selectedLoaders = defineModel<string[]>({ default: [] });

const { loaders, versionType } = defineProps<{
  loaders: Labrinth.Tags.v2.Loader[];
  toggleLoader: (loader: string) => void;
  /** BBSMC 资源类型，决定显示哪些 group（software/language/minecraft） */
  versionType?: "software" | "language" | "minecraft" | null;
}>();

type GroupLabels = "mods" | "plugins" | "packs" | "shaders" | "software" | "language" | "other";

const GROUP_LABEL_TEXT: Record<GroupLabels, string> = {
  mods: "模组",
  plugins: "插件",
  packs: "包",
  shaders: "光影",
  software: "软件",
  language: "汉化",
  other: "其他",
};

function groupLoaders(loaders: Labrinth.Tags.v2.Loader[]) {
  const groups: Record<GroupLabels, Labrinth.Tags.v2.Loader[]> = {
    mods: [],
    plugins: [],
    packs: [],
    shaders: [],
    software: [],
    language: [],
    other: [],
  };

  const MOD_SORT = [
    "fabric",
    "neoforge",
    "forge",
    "quilt",
    "liteloader",
    "rift",
    "ornithe",
    "nilloader",
    "risugami",
    "legacy-fabric",
    "bta-babric",
    "babric",
    "modloader",
    "java-agent",
  ];

  const PLUGIN_SORT = [
    "paper",
    "purpur",
    "spigot",
    "bukkit",
    "sponge",
    "folia",
    "bungeecord",
    "velocity",
    "waterfall",
    "geyser",
  ];

  const SHADER_SORT = ["optifine", "iris", "canvas", "vanilla"];
  const PACKS_SORT = ["minecraft", "datapack"];
  const SOFTWARE_SORT = ["windows", "macos", "linux"];
  const LANGUAGE_SORT = ["language"];

  for (const loader of loaders) {
    const name = loader.name.toLowerCase();
    // 优先按 supported_project_types 决定（来自后端）
    const supported = (loader as any).supported_project_types as string[] | undefined;

    if (LANGUAGE_SORT.includes(name) || supported?.includes("language")) {
      groups.language.push(loader);
    } else if (SOFTWARE_SORT.includes(name) || supported?.includes("software")) {
      groups.software.push(loader);
    } else if (PACKS_SORT.includes(name)) groups.packs.push(loader);
    else if (SHADER_SORT.includes(name)) groups.shaders.push(loader);
    else if (PLUGIN_SORT.includes(name)) groups.plugins.push(loader);
    else if (MOD_SORT.includes(name)) groups.mods.push(loader);
    else groups.other.push(loader);
  }

  const sortByOrder = (arr: any[], order: string[]) =>
    arr.sort((a, b) => order.indexOf(a.name) - order.indexOf(b.name));

  sortByOrder(groups.mods, MOD_SORT);
  sortByOrder(groups.plugins, PLUGIN_SORT);
  sortByOrder(groups.shaders, SHADER_SORT);
  sortByOrder(groups.software, SOFTWARE_SORT);

  return groups;
}

const groupedLoaders = computed(() => groupLoaders(loaders));

// 根据 versionType 决定显示哪些 group + 默认选中哪个
const visibleGroups = computed<GroupLabels[]>(() => {
  if (versionType === "software") return ["software"];
  if (versionType === "language") return ["language"];
  // minecraft 或未指定：显示原有 4 组
  return ["mods", "plugins", "packs", "shaders"];
});

const loaderGroup = ref<GroupLabels>(visibleGroups.value[0] ?? "mods");

// versionType 变化时切换默认 group
watch(
  visibleGroups,
  (groups) => {
    if (!groups.includes(loaderGroup.value)) {
      loaderGroup.value = groups[0] ?? "mods";
    }
  },
  { immediate: true },
);
</script>
