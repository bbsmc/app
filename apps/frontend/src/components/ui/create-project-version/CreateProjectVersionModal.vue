<template>
  <MultiStageModal
    ref="modal"
    :stages="ctx.stageConfigs"
    :context="ctx"
    :breadcrumbs="!editingVersion"
  />
</template>

<script setup lang="ts">
import type { Labrinth } from "@modrinth/api-client";
import {
  injectModrinthClient,
  injectNotificationManager,
  injectProjectPageContext,
  MultiStageModal,
} from "@modrinth/ui";
import type { ComponentExposed } from "vue-component-type-helpers";

import {
  createManageVersionContext,
  provideManageVersionContext,
} from "~/providers/version/manage-version-modal";

const emit = defineEmits<{
  (e: "save"): void;
}>();

const modal = useTemplateRef<ComponentExposed<typeof MultiStageModal>>("modal");

const ctx = createManageVersionContext(modal, () => emit("save"));
provideManageVersionContext(ctx);

const { newDraftVersion, editingVersion, handleNewFiles } = ctx;

const { projectV2 } = injectProjectPageContext();
const { addNotification } = injectNotificationManager();
const { labrinth } = injectModrinthClient();

async function openEditVersionModal(versionId: string, projectId: string, stageId?: string | null) {
  try {
    const versionData = await labrinth.versions_v3.getVersion(versionId);

    // BBSMC: 从已有 disk_urls 反推 5 个 input 临时字段，让 FilesUploadStage 能直接展示编辑
    const diskByPlatform = (versionData.disk_urls ?? []).reduce(
      (acc, d) => {
        if (d?.platform && d?.url)
          acc[d.platform] = {
            url: d.url,
            display: (d.display as Labrinth.Versions.v3.DiskDisplayMode) ?? "default",
          };
        return acc;
      },
      {} as Record<string, { url: string; display: Labrinth.Versions.v3.DiskDisplayMode }>,
    );

    const draftVersionData: Labrinth.Versions.v3.DraftVersion = {
      project_id: projectId,
      version_id: versionId,
      name: versionData.name ?? "",
      version_number: versionData.version_number ?? "",
      changelog: versionData.changelog ?? "",
      game_versions: versionData.game_versions ?? [],
      version_type: versionData.version_type ?? "release",
      loaders: versionData.loaders ?? [],
      dependencies: versionData.dependencies ?? [],
      existing_files: versionData.files ?? [],
      environment: versionData.environment,
      featured: versionData.featured ?? false,
      // BBSMC 自定义字段（必须传入，否则编辑模式下显示"未选择"且字段丢失）
      type: versionData.type ?? null,
      disk_only: versionData.disk_only ?? false,
      disk_urls: versionData.disk_urls ?? [],
      version_links: versionData.version_links ?? [],
      is_modpack: versionData.is_modpack,
      quark_disk: diskByPlatform.quark?.url ?? "",
      xunlei_disk: diskByPlatform.xunlei?.url ?? "",
      baidu_disk: diskByPlatform.baidu?.url ?? "",
      modrinth: diskByPlatform.modrinth?.url ?? "",
      curseforge: diskByPlatform.curseforge?.url ?? "",
      quark_disk_display: diskByPlatform.quark?.display ?? "default",
      xunlei_disk_display: diskByPlatform.xunlei?.display ?? "default",
      baidu_disk_display: diskByPlatform.baidu?.display ?? "default",
    };

    if (projectV2.value.project_type === "modpack" && draftVersionData.loaders.includes("mrpack")) {
      draftVersionData.loaders.push(...(versionData.mrpack_loaders ?? []));
    }

    openCreateVersionModal(draftVersionData, stageId);
  } catch (err: any) {
    addNotification({
      title: "An error occurred",
      text: err.data ? err.data.description : err,
      type: "error",
    });
  }
}

/**
 * BBSMC: 复用已有版本的信息创建新版本（复刻）。
 * 预填 loaders / 游戏版本 / 资源类型 / 环境 / 依赖 / 网盘链接 / 翻译链接 / changelog 等；
 * 不带 version_id（进入新建流程）与 existing_files（文件有独立 hash 校验，不复用）。
 */
let duplicatingVersionId: string | null = null;

async function openDuplicateVersionModal(versionId: string) {
  // 防重入：await getVersion 期间再次触发会竞态覆盖已展示的 draft
  if (duplicatingVersionId) return;
  duplicatingVersionId = versionId;
  try {
    const versionData = await labrinth.versions_v3.getVersion(versionId);

    // BBSMC: 从已有 disk_urls 反推 5 个 input 临时字段，让 FilesUploadStage 能直接展示
    const diskByPlatform = (versionData.disk_urls ?? []).reduce(
      (acc, d) => {
        if (d?.platform && d?.url)
          acc[d.platform] = {
            url: d.url,
            display: (d.display as Labrinth.Versions.v3.DiskDisplayMode) ?? "default",
          };
        return acc;
      },
      {} as Record<string, { url: string; display: Labrinth.Versions.v3.DiskDisplayMode }>,
    );

    const draftVersionData: Labrinth.Versions.v3.DraftVersion = {
      project_id: projectV2.value.id,
      name: versionData.name ?? "",
      version_number: versionData.version_number ?? "",
      changelog: versionData.changelog ?? "",
      game_versions: versionData.game_versions ?? [],
      version_type: versionData.version_type ?? "release",
      loaders: versionData.loaders ?? [],
      dependencies: versionData.dependencies ?? [],
      // 'unknown' 是哨兵值，常规创建会强制用户重选，这里同样归一为未设置
      environment: versionData.environment === "unknown" ? undefined : versionData.environment,
      featured: versionData.featured ?? false,
      // 复刻保留源版本的可见性（创建接口默认 Listed，unlisted 源直接公开会意外泄露；
      // Scheduled 无法被创建接口请求，退化为 unlisted）
      status:
        versionData.status === "listed" ||
        versionData.status === "unlisted" ||
        versionData.status === "archived" ||
        versionData.status === "draft"
          ? versionData.status
          : "unlisted",
      type: versionData.type ?? null,
      disk_only: versionData.disk_only ?? false,
      // 只预填 5 个 input 字段、不携带旧 disk_urls 数组：
      // createVersion 的 aggregateDiskUrls 优先读 draft.disk_urls，若带旧数组，
      // 用户在向导中对网盘链接 / 展示方式的修改会在提交时被静默丢弃
      disk_urls: [],
      version_links: versionData.version_links ?? [],
      is_modpack: versionData.is_modpack,
      quark_disk: diskByPlatform.quark?.url ?? "",
      xunlei_disk: diskByPlatform.xunlei?.url ?? "",
      baidu_disk: diskByPlatform.baidu?.url ?? "",
      modrinth: diskByPlatform.modrinth?.url ?? "",
      curseforge: diskByPlatform.curseforge?.url ?? "",
      quark_disk_display: diskByPlatform.quark?.display ?? "default",
      xunlei_disk_display: diskByPlatform.xunlei?.display ?? "default",
      baidu_disk_display: diskByPlatform.baidu?.display ?? "default",
    };

    if (projectV2.value.project_type === "modpack" && draftVersionData.loaders.includes("mrpack")) {
      draftVersionData.loaders.push(...(versionData.mrpack_loaders ?? []));
    }

    // 老数据 type 可能为 null：第 0 步会没有预选中，用户重选类型会触发
    // AddFilesStage 清空 loaders / game_versions / environment，预填就全丢了。
    // 这里按项目类型 / loaders 反推 type，让向导从头到尾都保持预填状态。
    // 注意 software 版本的 GET loaders 是实际平台（如 ['windows']）不含 'software' 标记，
    // 必须靠 project_type 判断
    if (!draftVersionData.type) {
      const dupLoaders = draftVersionData.loaders ?? [];
      if (projectV2.value.project_type === "software") draftVersionData.type = "software";
      else if (dupLoaders.includes("language")) draftVersionData.type = "language";
      else if (dupLoaders.includes("map") || projectV2.value.project_type === "map")
        draftVersionData.type = "map";
      else draftVersionData.type = "minecraft";
    }

    // modpack 项目的版本必为整合包，直接预置，避免用户被整合包确认卡住
    if (projectV2.value.project_type === "modpack" && draftVersionData.is_modpack == null) {
      draftVersionData.is_modpack = true;
    }

    newDraftVersion(projectV2.value.id, draftVersionData);

    // newDraftVersion 会按 existing_files（复刻时无）+ 网盘字段把 uploadMode 推断成 disk，
    // 这里按源版本的真实形态修正。不能用 GET 返回的 disk_only 判断——后端它是
    // disk_only = !disk_urls.is_empty() 的派生值（"有没有网盘链接"）而非创建模式，
    // both 版本（站内文件 + 网盘）也会返回 true；真实站内文件以「带 sha1 hash」为准
    // （files 数组可能含无 hash 的转载记录 / disk_only 时代残留的文件记录）
    const hasDiskUrls =
      !!draftVersionData.quark_disk ||
      !!draftVersionData.xunlei_disk ||
      !!draftVersionData.baidu_disk ||
      !!draftVersionData.modrinth ||
      !!draftVersionData.curseforge;
    const hadLocalFiles = (versionData.files ?? []).some((file: any) => file.hashes?.sha1);
    if (hadLocalFiles && hasDiskUrls) ctx.uploadMode.value = "both";
    else if (hadLocalFiles) ctx.uploadMode.value = "local";
    else if (hasDiskUrls) ctx.uploadMode.value = "disk";

    modal.value?.setStage(0);
    modal.value?.show();
  } catch (err: any) {
    addNotification({
      title: "获取版本信息失败",
      text: err.data ? err.data.description : err,
      type: "error",
    });
  } finally {
    duplicatingVersionId = null;
  }
}

function openCreateVersionModal(
  version: Labrinth.Versions.v3.DraftVersion | null = null,
  stageId: string | null = null,
) {
  newDraftVersion(projectV2.value.id, version);
  modal.value?.setStage(stageId ?? 0);
  modal.value?.show();
}

/**
 * 从外部（如 versions.vue 的 DropArea / FileInput）传入文件直接打开新建版本 modal。
 * 用户可在 modal 内继续添加文件 / 网盘 / 翻译链接。
 */
async function handleDropFiles(files: File[]) {
  newDraftVersion(projectV2.value.id, null);
  modal.value?.setStage(0);
  await handleNewFiles(files);
  modal.value?.show();
}

defineExpose({
  openEditVersionModal,
  openCreateVersionModal,
  openDuplicateVersionModal,
  handleDropFiles,
});
</script>
