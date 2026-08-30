<template>
  <div class="flex w-full flex-col gap-5">
    <!-- 编辑模式专用：上传方式切换（创建模式由 upload-mode-stage 处理） -->
    <div v-if="editingVersion" class="flex flex-col gap-2">
      <span class="text-sm font-semibold text-contrast">上传方式</span>
      <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
        <button
          v-for="opt in uploadModeOptions"
          :key="opt.value"
          type="button"
          class="rounded-lg border-2 border-solid bg-button-bg px-3 py-2 text-sm text-left transition-all"
          :class="
            uploadMode === opt.value
              ? 'border-brand bg-highlight-green text-brand'
              : 'border-surface-5 text-contrast hover:bg-button-bg-hover'
          "
          @click="uploadMode = opt.value"
        >
          <div class="font-semibold">{{ opt.label }}</div>
          <div class="text-xs opacity-80">{{ opt.hint }}</div>
        </button>
      </div>
    </div>

    <!-- 站内文件区（local / both 模式显示） -->
    <div v-if="needsLocalFiles" class="flex flex-col gap-3">
      <div class="flex items-center justify-between">
        <span class="text-base font-semibold text-contrast">站内文件</span>
        <span class="text-xs text-secondary">
          上传到本站 CDN，用户可直接下载
        </span>
      </div>

      <template v-if="!(filesToAdd.length || draftVersion.existing_files?.length)">
        <DropzoneFileInput
          aria-label="Upload file"
          multiple
          :accept="fileAccept"
          :max-size="1073741824"
          @change="handleNewFilesFiltered"
        />
      </template>

      <template v-else>
        <div class="flex flex-col gap-2">
          <span class="text-sm font-semibold text-contrast">主文件</span>
          <div class="flex flex-col gap-2.5">
            <VersionFileRow
              v-if="primaryFile"
              :key="primaryFile.name"
              :name="primaryFile.name"
              :is-primary="true"
              :editing-version="editingVersion"
              :accept-override="fileAccept"
              :on-remove="undefined"
              @set-primary-file="
                (file) => {
                  if (file && !editingVersion && validateFile(file)) replacePrimaryFile(file);
                }
              "
            />
          </div>
          <span class="text-xs text-secondary">
            主文件是用户安装项目时默认下载的文件。
          </span>
        </div>

        <div class="flex flex-col gap-2">
          <Admonition v-if="hasSupplementaryFiles" type="warning">
            附属文件用于支持性资源（如源代码），不应用于替代版本或变体。
          </Admonition>

          <span class="text-sm font-semibold text-contrast">附属文件</span>

          <DropzoneFileInput
            aria-label="Upload additional file"
            multiple
            :accept="fileAccept"
            :max-size="1073741824"
            size="small"
            :primary-prompt="null"
            secondary-prompt="拖放文件或点击浏览"
            @change="handleNewFilesFiltered"
          />

          <div v-if="hasSupplementaryFiles" class="flex flex-col gap-2.5">
            <VersionFileRow
              v-for="versionFile in supplementaryExistingFiles"
              :key="versionFile.filename"
              :name="versionFile.filename"
              :is-primary="false"
              :initial-file-type="versionFile.file_type"
              :editing-version="editingVersion"
              :accept-override="fileAccept"
              :on-remove="() => handleRemoveExistingFile(versionFile.hashes.sha1 || '')"
              @set-file-type="(type) => (versionFile.file_type = type)"
            />
            <VersionFileRow
              v-for="(versionFile, idx) in supplementaryNewFiles"
              :key="versionFile.file.name"
              :name="versionFile.file.name"
              :is-primary="false"
              :initial-file-type="versionFile.fileType"
              :editing-version="editingVersion"
              :accept-override="fileAccept"
              :on-remove="() => handleRemoveFile(idx + (primaryFile?.existing ? 0 : 1))"
              @set-file-type="(type) => (versionFile.fileType = type)"
              @set-primary-file="() => swapPrimaryFile(idx + (primaryFile?.existing ? 0 : 1))"
            />
          </div>
          <span class="text-xs text-secondary">
            可选添加附属文件，例如源代码、文档或必需的资源包。
          </span>
        </div>
      </template>
    </div>

    <!-- 网盘下载区（disk / both 模式显示） -->
    <div
      v-if="needsDiskInputs"
      class="flex flex-col gap-3 rounded-xl border border-solid border-surface-5 p-4"
    >
      <div class="flex items-center justify-between">
        <span class="text-base font-semibold text-contrast">网盘下载链接</span>
        <span class="text-xs text-secondary">
          至少填写一个网盘链接
        </span>
      </div>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div class="flex flex-col gap-1">
          <label class="text-xs font-semibold text-contrast">夸克网盘</label>
          <div class="flex gap-2">
            <input
              v-model="draftVersion.quark_disk"
              type="text"
              class="border-surface-5 min-w-0 flex-1 rounded-lg border border-solid bg-button-bg px-3 py-2 text-sm text-contrast"
              placeholder="直接链接，不要设置访问密码"
            />
            <div class="w-36 shrink-0">
              <DropdownSelect
                v-model="draftVersion.quark_disk_display"
                :options="diskUrlDisplayModes"
                :display-name="diskUrlDisplayLabel"
                style="width: 100%"
                name="夸克网盘展示方式"
              />
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-xs font-semibold text-contrast">迅雷网盘</label>
          <div class="flex gap-2">
            <input
              v-model="draftVersion.xunlei_disk"
              type="text"
              class="border-surface-5 min-w-0 flex-1 rounded-lg border border-solid bg-button-bg px-3 py-2 text-sm text-contrast"
              placeholder="直接链接，不要设置访问密码"
            />
            <div class="w-36 shrink-0">
              <DropdownSelect
                v-model="draftVersion.xunlei_disk_display"
                :options="diskUrlDisplayModes"
                :display-name="diskUrlDisplayLabel"
                style="width: 100%"
                name="迅雷网盘展示方式"
              />
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-xs font-semibold text-contrast">百度网盘</label>
          <div class="flex gap-2">
            <input
              v-model="draftVersion.baidu_disk"
              type="text"
              class="border-surface-5 min-w-0 flex-1 rounded-lg border border-solid bg-button-bg px-3 py-2 text-sm text-contrast"
              placeholder="直接链接，不要设置访问密码"
            />
            <div class="w-36 shrink-0">
              <DropdownSelect
                v-model="draftVersion.baidu_disk_display"
                :options="diskUrlDisplayModes"
                :display-name="diskUrlDisplayLabel"
                style="width: 100%"
                name="百度网盘展示方式"
              />
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-xs font-semibold text-contrast">Modrinth 转载</label>
          <input
            v-model="draftVersion.modrinth"
            type="text"
            class="rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast"
            placeholder="转载资源时填写对应版本子页面"
          />
        </div>
        <div class="flex flex-col gap-1 sm:col-span-2">
          <label class="text-xs font-semibold text-contrast">CurseForge 转载</label>
          <input
            v-model="draftVersion.curseforge"
            type="text"
            class="rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast"
            placeholder="转载资源时填写对应版本子页面"
          />
        </div>
      </div>

      <p class="m-0 text-xs text-secondary">
        展示方式说明：「默认下载」点击后直接跳转网盘；「纯二维码」弹出扫码窗口，用户用手机App扫码转存；「二维码+链接」扫码窗口内同时保留跳转按钮。网盘转存可为作者提供收益。
      </p>

      <!-- 整合包标识提示（已在 upload-mode 步骤设置过，这里只显示状态） -->
      <div
        class="flex items-center gap-2 rounded-lg p-2 text-sm"
        :class="
          draftVersion.is_modpack
            ? 'bg-orange-500/10 text-orange-500'
            : 'bg-button-bg text-secondary'
        "
      >
        <InfoIcon />
        <span v-if="draftVersion.is_modpack">
          已标识为整合包资源（如需修改请返回上一步）
        </span>
        <span v-else>
          已标识为非整合包资源（如需修改请返回上一步）
        </span>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import type { Labrinth } from "@modrinth/api-client";
import { InfoIcon } from "@modrinth/assets";
import {
  Admonition,
  DropzoneFileInput,
  DropdownSelect,
  injectNotificationManager,
  injectProjectPageContext,
} from "@modrinth/ui";
import { acceptFileFromProjectType } from "@modrinth/utils";
import { computed } from "vue";

import { injectManageVersionContext } from "~/providers/version/manage-version-modal";
import { diskUrlDisplayLabel, diskUrlDisplayModes } from "~/utils/disk-urls";

import VersionFileRow from "../components/VersionFileRow.vue";

const { projectV2 } = injectProjectPageContext();
const { addNotification } = injectNotificationManager();

const {
  draftVersion,
  filesToAdd,
  existingFilesToDelete,
  swapPrimaryFile,
  replacePrimaryFile,
  handleNewFiles,
  primaryFile,
  editingVersion,
  needsDiskInputs,
  needsLocalFiles,
  uploadMode,
} = injectManageVersionContext();

const uploadModeOptions = [
  { value: "local" as const, label: "仅站内文件", hint: "上传到本站 CDN" },
  { value: "disk" as const, label: "仅网盘下载", hint: "提供网盘链接" },
  { value: "both" as const, label: "站内 + 网盘", hint: "两者并存" },
];

const fileAccept = computed(() => {
  if (draftVersion.value.type === "language") return ".zip";
  return acceptFileFromProjectType(projectV2.value.project_type);
});

function validateFile(file: File): boolean {
  if (draftVersion.value.type === "language") {
    if (!file.name.toLowerCase().endsWith(".zip")) {
      addNotification({
        title: "文件类型不支持",
        text: `汉化包只能上传 .zip 文件，"${file.name}" 已被忽略。`,
        type: "error",
      });
      return false;
    }
  }
  return true;
}

async function handleNewFilesFiltered(files: File[]) {
  const valid = files.filter((f) => validateFile(f));
  if (valid.length === 0) return;
  await handleNewFiles(valid);
}

function handleRemoveFile(index: number) {
  filesToAdd.value.splice(index, 1);
}

function handleRemoveExistingFile(sha1: string) {
  existingFilesToDelete.value.push(sha1);
  draftVersion.value.existing_files = draftVersion.value.existing_files?.filter(
    (file: Labrinth.Versions.v3.Version["files"][0]) => file.hashes.sha1 !== sha1,
  );
}

const supplementaryNewFiles = computed(() => {
  if (primaryFile.value?.existing) {
    return filesToAdd.value;
  } else {
    return filesToAdd.value.slice(1);
  }
});

const supplementaryExistingFiles = computed(() => {
  if (primaryFile.value?.existing) {
    return draftVersion.value.existing_files?.slice(1);
  } else {
    return draftVersion.value.existing_files;
  }
});

const hasSupplementaryFiles = computed(
  () => filesToAdd.value.length + (draftVersion.value.existing_files?.length || 0) > 1,
);
</script>
