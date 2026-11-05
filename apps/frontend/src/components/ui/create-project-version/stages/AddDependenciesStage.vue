<template>
  <div class="flex w-full max-w-full flex-col gap-6">
    <!-- BBSMC: language 类型显示翻译链接管理 UI -->
    <template v-if="isLanguageVersion">
      <div class="flex flex-col gap-4">
        <span class="font-semibold text-contrast">翻译版本链接</span>
        <div class="border-surface-5 flex flex-col gap-3 rounded-2xl border border-solid p-4">
          <div class="grid gap-2.5">
            <span class="font-semibold text-contrast">
              要翻译的源版本 ID <span class="text-red">*</span>
            </span>
            <input
              v-model="linkTargetId"
              type="text"
              class="border-surface-5 rounded-lg border border-solid bg-button-bg px-3 py-2 text-sm"
              placeholder="请输入要翻译的版本 ID"
            />
          </div>
          <div class="grid gap-2.5">
            <span class="font-semibold text-contrast"
              >翻译语言 <span class="text-red">*</span></span
            >
            <Combobox
              v-model="linkLanguageCode"
              placeholder="选择翻译语言"
              :options="languageOptions"
            />
          </div>
          <div class="grid gap-2.5">
            <span class="font-semibold text-contrast">说明（可选）</span>
            <input
              v-model="linkDescription"
              type="text"
              class="border-surface-5 rounded-lg border border-solid bg-button-bg px-3 py-2 text-sm"
              placeholder="翻译说明..."
            />
          </div>
          <ButtonStyled color="brand">
            <button
              class="self-start"
              :disabled="!linkTargetId || !linkLanguageCode"
              @click="handleAddVersionLink"
            >
              添加链接
            </button>
          </ButtonStyled>
        </div>
      </div>

      <div v-if="(draftVersion.version_links?.length ?? 0) > 0" class="flex flex-col gap-4">
        <span class="font-semibold text-contrast">已添加的翻译链接</span>
        <div class="flex flex-col gap-2">
          <div
            v-for="(link, index) in draftVersion.version_links"
            :key="`vl-${index}`"
            class="border-surface-5 flex items-center justify-between gap-2 rounded-xl border border-solid bg-button-bg p-3"
          >
            <div class="flex flex-col">
              <span class="font-semibold">{{ langLabel(link.language_code) }}</span>
              <span class="text-xs text-secondary"> 源版本 ID: {{ link.joining_version_id }} </span>
              <span v-if="link.description" class="text-xs">{{ link.description }}</span>
            </div>
            <ButtonStyled type="transparent" size="standard">
              <button @click="removeVersionLink(index)">
                <XIcon />
              </button>
            </ButtonStyled>
          </div>
        </div>
      </div>
    </template>

    <!-- 非 language 类型：常规依赖编辑 -->
    <template v-else>
      <div class="flex flex-col gap-4">
        <span class="font-semibold text-contrast">添加依赖</span>
        <div class="border-surface-5 flex flex-col gap-3 rounded-2xl border border-solid p-4">
          <div class="grid gap-2.5">
            <span class="font-semibold text-contrast">项目 <span class="text-red">*</span></span>
            <ModSelect v-model="newDependencyProjectId" />
          </div>

          <template v-if="newDependencyProjectId">
            <div class="grid gap-2.5">
              <span class="font-semibold text-contrast"> 版本 </span>
              <Combobox
                v-model="newDependencyVersionId"
                placeholder="选择版本"
                :options="[{ label: '任意版本', value: null }, ...newDependencyVersions]"
                :searchable="true"
              />
            </div>

            <div class="grid gap-2.5">
              <span class="font-semibold text-contrast"> 依赖关系 </span>
              <Combobox
                v-model="newDependencyType"
                placeholder="选择依赖类型"
                :options="[
                  { label: '必需', value: 'required' },
                  { label: '可选', value: 'optional' },
                  { label: '不兼容', value: 'incompatible' },
                  { label: '内嵌', value: 'embedded' },
                ]"
              />
            </div>

            <ButtonStyled>
              <button
                class="self-start"
                :disabled="!newDependencyProjectId"
                @click="
                  () =>
                    addDependency(
                      toRaw({
                        project_id: newDependencyProjectId,
                        version_id: newDependencyVersionId || undefined,
                        dependency_type: newDependencyType,
                      }),
                    )
                "
              >
                添加依赖
              </button>
            </ButtonStyled>
          </template>
        </div>
      </div>

      <SuggestedDependencies
        :suggested-dependencies="suggestedDependencies"
        @on-add-suggestion="handleAddSuggestedDependency"
      />

      <div v-if="addedDependencies.length" class="flex flex-col gap-4">
        <span class="font-semibold text-contrast">已添加的依赖</span>
        <div class="5 flex flex-col gap-2">
          <template v-for="(dependency, index) in addedDependencies">
            <AddedDependencyRow
              v-if="dependency"
              :key="index"
              :project-id="dependency.projectId"
              :name="dependency.name"
              :icon="dependency.icon"
              :dependency-type="dependency.dependencyType"
              :version-name="dependency.versionName"
              @remove="() => removeDependency(index)"
            />
          </template>
          <span v-if="!addedDependencies.length"> 尚未添加依赖。 </span>
        </div>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import type { Labrinth } from "@modrinth/api-client";
import { XIcon } from "@modrinth/assets";
import {
  ButtonStyled,
  Combobox,
  injectModrinthClient,
  injectNotificationManager,
  injectProjectPageContext,
} from "@modrinth/ui";
import type { DropdownOption } from "@modrinth/ui";

import ModSelect from "~/components/ui/create-project-version/components/ModSelect.vue";
import {
  injectManageVersionContext,
  TRANSLATION_LANGUAGE_OPTIONS,
} from "~/providers/version/manage-version-modal";

import AddedDependencyRow from "../components/AddedDependencyRow.vue";
import SuggestedDependencies from "../components/SuggestedDependencies/SuggestedDependencies.vue";

const { addNotification } = injectNotificationManager();
const { labrinth } = injectModrinthClient();

const errorNotification = (err: any) => {
  addNotification({
    title: "发生错误",
    text: err.data ? err.data.description : err,
    type: "error",
  });
};

const newDependencyProjectId = ref<string>();
const newDependencyType = ref<Labrinth.Versions.v2.DependencyType>("required");
const newDependencyVersionId = ref<string | null>(null);

const newDependencyVersions = ref<DropdownOption<string>[]>([]);

const projectsFetchLoading = ref(false);
const suggestedDependencies = ref<
  Array<Labrinth.Versions.v3.Dependency & { name?: string; icon?: string; versionName?: string }>
>([]);

// reset to defaults when select different project
watch(newDependencyProjectId, async () => {
  newDependencyVersionId.value = null;
  newDependencyType.value = "required";

  if (!newDependencyProjectId.value) {
    newDependencyVersions.value = [];
  } else {
    try {
      const versions = await labrinth.versions_v3.getProjectVersions(newDependencyProjectId.value);
      newDependencyVersions.value = versions.map((version) => ({
        label: version.name,
        value: version.id,
      }));
    } catch (error: any) {
      errorNotification(error);
    }
  }
});

const {
  draftVersion,
  dependencyProjects,
  dependencyVersions,
  getProject,
  getVersion,
  isLanguageVersion,
  addVersionLink,
  removeVersionLink,
} = injectManageVersionContext();
const { projectV2: project } = injectProjectPageContext();

// === BBSMC: 翻译链接编辑 state ===
const linkTargetId = ref("");
const linkLanguageCode = ref<string | null>(null);
const linkDescription = ref("");

const languageOptions = TRANSLATION_LANGUAGE_OPTIONS.map((o) => ({
  label: o.label,
  value: o.value,
}));

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

function handleAddVersionLink() {
  if (!linkTargetId.value || !linkLanguageCode.value) return;
  // 重复检查
  const existing = draftVersion.value.version_links ?? [];
  const dup = existing.find(
    (l: any) =>
      l.joining_version_id === linkTargetId.value && l.language_code === linkLanguageCode.value,
  );
  if (dup) {
    addNotification({
      title: "已存在",
      text: "已为该源版本和语言添加过翻译链接。",
      type: "error",
    });
    return;
  }
  addVersionLink({
    joining_version_id: linkTargetId.value,
    link_type: "translation",
    language_code: linkLanguageCode.value,
    description: linkDescription.value || undefined,
  });
  linkTargetId.value = "";
  linkLanguageCode.value = null;
  linkDescription.value = "";
}

const getSuggestedDependencies = async () => {
  try {
    suggestedDependencies.value = [];

    if (!draftVersion.value.game_versions?.length || !draftVersion.value.loaders?.length) {
      return;
    }

    try {
      const versions = await labrinth.versions_v3.getProjectVersions(project.value.id, {
        loaders: draftVersion.value.loaders,
      });

      // Get the most recent matching version and extract its dependencies
      if (versions.length > 0) {
        const mostRecentVersion = versions[0];
        for (const dep of mostRecentVersion.dependencies) {
          suggestedDependencies.value.push({
            project_id: dep.project_id,
            version_id: dep.version_id,
            dependency_type: dep.dependency_type,
            file_name: dep.file_name,
          });
        }
      }
    } catch (error: any) {
      console.error(`Failed to get versions for project ${project.value.id}:`, error);
    }

    for (const dep of suggestedDependencies.value) {
      try {
        if (dep.project_id) {
          const proj = await getProject(dep.project_id);
          dep.name = proj.name;
          dep.icon = proj.icon_url;
        }

        if (dep.version_id) {
          const version = await getVersion(dep.version_id);
          dep.versionName = version.name;
        }
      } catch (error: any) {
        console.error(`Failed to fetch project/version data for dependency:`, error);
      }
    }
  } catch (error: any) {
    errorNotification(error);
  }
};

onMounted(() => {
  getSuggestedDependencies();
});

watch(
  draftVersion,
  async (draftVersion) => {
    const deps = draftVersion.dependencies || [];

    for (const dep of deps) {
      if (dep?.project_id) {
        try {
          await getProject(dep.project_id);
        } catch (error: any) {
          errorNotification(error);
        }
      }

      if (dep?.version_id) {
        try {
          await getVersion(dep.version_id);
        } catch (error: any) {
          errorNotification(error);
        }
      }
    }
    projectsFetchLoading.value = false;
  },
  { immediate: true, deep: true },
);

const addedDependencies = computed(() =>
  (draftVersion.value.dependencies || [])
    .map((dep) => {
      if (!dep.project_id) return null;

      const dependencyProject = dependencyProjects.value[dep.project_id];
      const versionName = dependencyVersions.value[dep.version_id || ""]?.name ?? "";

      if (!dependencyProject && projectsFetchLoading.value) return null;

      return {
        projectId: dep.project_id,
        name: dependencyProject?.name,
        icon: dependencyProject?.icon_url,
        dependencyType: dep.dependency_type,
        versionName,
      };
    })
    .filter(Boolean),
);

const addDependency = (dependency: Labrinth.Versions.v3.Dependency) => {
  if (!draftVersion.value.dependencies) draftVersion.value.dependencies = [];

  // already added
  if (
    draftVersion.value.dependencies.find(
      (d) => d.project_id === dependency.project_id && d.version_id === dependency.version_id,
    )
  ) {
    addNotification({
      title: "依赖已存在",
      text: "不能重复添加相同的依赖。",
      type: "error",
    });
    return;
  }

  projectsFetchLoading.value = true;
  draftVersion.value.dependencies.push(dependency);
  newDependencyProjectId.value = undefined;
};

const removeDependency = (index: number) => {
  if (!draftVersion.value.dependencies) return;
  draftVersion.value.dependencies.splice(index, 1);
};

const handleAddSuggestedDependency = (dependency: Labrinth.Versions.v3.Dependency) => {
  draftVersion.value.dependencies?.push({
    project_id: dependency.project_id,
    version_id: dependency.version_id,
    dependency_type: dependency.dependency_type,
  });
};
</script>
