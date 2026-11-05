import { LeftArrowIcon, RightArrowIcon, XIcon } from "@modrinth/assets";
import type { StageConfigInput } from "@modrinth/ui";
import { markRaw } from "vue";

import type { ManageVersionContextValue } from "../manage-version-modal";
import FilesUploadStage from "~/components/ui/create-project-version/stages/FilesUploadStage.vue";

/**
 * 是否满足"可以下一步"的条件：
 * - 站内模式（local / both）：必须有文件
 * - 网盘模式（disk）：至少一个网盘链接
 * - 共存模式（both）：必须有文件 且 至少一个网盘链接
 */
function canProceed(ctx: ManageVersionContextValue): boolean {
  if (ctx.handlingNewFiles.value) return false;

  const draft = ctx.draftVersion.value;
  const hasFiles = ctx.filesToAdd.value.length > 0 || (draft.existing_files?.length ?? 0) > 0;
  const networks = [
    draft.quark_disk,
    draft.xunlei_disk,
    draft.baidu_disk,
    draft.modrinth,
    draft.curseforge,
  ];
  const hasDisks = networks.some((u) => u && u.trim() !== "");

  if (ctx.uploadMode.value === "local") return hasFiles;
  if (ctx.uploadMode.value === "disk") return hasDisks;
  if (ctx.uploadMode.value === "both") return hasFiles && hasDisks;
  return false;
}

export const stageConfig: StageConfigInput<ManageVersionContextValue> = {
  id: "files-upload",
  stageContent: markRaw(FilesUploadStage),
  title: (ctx) => (ctx.editingVersion.value ? "上传内容" : "上传内容"),
  cannotNavigateForward: (ctx) => !canProceed(ctx),
  leftButtonConfig: (ctx) =>
    ctx.editingVersion.value
      ? {
          label: "取消",
          icon: XIcon,
          onClick: () => ctx.modal.value?.hide(),
        }
      : {
          label: "返回",
          icon: LeftArrowIcon,
          onClick: () => ctx.modal.value?.prevStage(),
        },
  rightButtonConfig: (ctx) => {
    if (ctx.editingVersion.value) {
      return {
        ...ctx.saveButtonConfig(),
        label: "保存",
        disabled: ctx.isSubmitting.value,
      };
    }
    return {
      label: ctx.getNextLabel(),
      icon: RightArrowIcon,
      iconPosition: "after",
      disabled: !canProceed(ctx),
      onClick: () => ctx.modal.value?.nextStage(),
    };
  },
};

/**
 * 编辑模式专用：编辑文件 stage（不要求重选 uploadMode / 资源类型）。
 * 由 metadata 入口的"编辑文件"按钮跳转，复用 FilesUploadStage（uploadMode 在 newDraftVersion
 * 阶段已根据已有 disk_urls / existing_files 反推完成，所以可以直接进入文件编辑界面）。
 */
export const fromDetailsStageConfig: StageConfigInput<ManageVersionContextValue> = {
  id: "from-details-files",
  stageContent: markRaw(FilesUploadStage),
  title: "编辑文件",
  nonProgressStage: true,
  leftButtonConfig: (ctx) => ({
    label: "返回",
    icon: LeftArrowIcon,
    onClick: () => ctx.modal.value?.setStage("metadata"),
  }),
  rightButtonConfig: (ctx) => ({
    ...ctx.saveButtonConfig(),
    label: "保存",
    disabled: ctx.isSubmitting.value,
  }),
};
