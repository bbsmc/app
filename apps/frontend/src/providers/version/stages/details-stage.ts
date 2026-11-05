import { LeftArrowIcon, PlusIcon, SaveIcon, UpdatedIcon, XIcon } from "@modrinth/assets";
import type { StageConfigInput } from "@modrinth/ui";
import { markRaw } from "vue";

import type { ManageVersionContextValue } from "../manage-version-modal";
import AddDetailsStage from "~/components/ui/create-project-version/stages/AddDetailsStage.vue";

/**
 * 详情 stage：用户填写版本号、版本类型、changelog、featured 等
 *
 * 创建模式：作为最后一步，触发 handleCreateVersion
 * 编辑模式：作为单独编辑入口（nonProgressStage），触发 handleSaveVersionEdits
 */
export const stageConfig: StageConfigInput<ManageVersionContextValue> = {
  id: "add-details",
  stageContent: markRaw(AddDetailsStage),
  title: (ctx) => (ctx.editingVersion.value ? "编辑版本" : "详情"),
  nonProgressStage: (ctx) => ctx.editingVersion.value,
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
          disabled: ctx.isUploading.value,
          onClick: () => ctx.modal.value?.prevStage(),
        },
  rightButtonConfig: (ctx) => ({
    label: ctx.editingVersion.value
      ? "保存修改"
      : ctx.isUploading.value
        ? ctx.uploadProgress.value.progress >= 1
          ? "正在创建版本"
          : `上传中 ${Math.round(ctx.uploadProgress.value.progress * 100)}%`
        : "创建版本",
    icon: ctx.isSubmitting.value ? UpdatedIcon : ctx.editingVersion.value ? SaveIcon : PlusIcon,
    iconPosition: "before",
    iconClass: ctx.isSubmitting.value ? "animate-spin" : undefined,
    color: "green",
    disabled: ctx.isSubmitting.value || ctx.draftVersion.value.version_number.trim().length === 0,
    onClick: () =>
      ctx.editingVersion.value ? ctx.handleSaveVersionEdits() : ctx.handleCreateVersion(),
  }),
};
