<template>
  <div>
    <ModalConfirm
      ref="modal_confirm"
      title="你确定要删除该资源吗?"
      description="如果删除,所有版本和所有附加数据都将从我们的服务器中删除.这可能会破坏其他资源,所以请慎重."
      :has-to-type="true"
      :confirmation-text="project.title"
      proceed-label="确认删除"
      @proceed="deleteProject"
    />
    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">资源信息</span>
        </h3>
      </div>
      <label for="project-icon">
        <span class="label__title">图标</span>
      </label>
      <div class="input-group">
        <Avatar
          :src="deletedIcon ? null : previewImage ? previewImage : project.icon_url"
          :alt="project.title"
          size="md"
          class="project__icon"
        />
        <div class="input-stack">
          <FileInput
            id="project-icon"
            :max-size="262144"
            :show-icon="true"
            accept="image/png,image/jpeg,image/gif,image/webp"
            class="choose-image iconified-button"
            prompt="上传"
            aria-label="上传"
            :disabled="!hasPermission"
            @change="showPreviewImage"
          >
            <UploadIcon aria-hidden="true" />
          </FileInput>
          <button
            v-if="!deletedIcon && (previewImage || project.icon_url)"
            class="iconified-button"
            :disabled="!hasPermission"
            @click="markIconForDeletion"
          >
            <TrashIcon aria-hidden="true" />
            移除
          </button>
        </div>
      </div>

      <label for="project-name">
        <span class="label__title">资源名称</span>
      </label>
      <input
        id="project-name"
        v-model="name"
        maxlength="2048"
        type="text"
        :disabled="!hasPermission"
      />

      <label for="project-slug">
        <span class="label__title">URL</span>
      </label>
      <div class="text-input-wrapper">
        <div class="text-input-wrapper__before">
          https://bbsmc.net/{{ $getProjectTypeForUrl(project.project_type, project.loaders) }}/
        </div>
        <input
          id="project-slug"
          v-model="slug"
          type="text"
          maxlength="64"
          autocomplete="off"
          :disabled="!hasPermission"
        />
      </div>

      <label for="project-summary">
        <span class="label__title">简短介绍</span>
      </label>
      <div class="textarea-wrapper summary-input">
        <textarea
          id="project-summary"
          v-model="summary"
          maxlength="256"
          :disabled="!hasPermission"
        />
      </div>
      <div class="adjacent-input">
        <label for="project-wiki-open">
          <span class="label__title">百科</span>
          <span class="label__description">
            选择是否开启百科的公共编辑，开启后任何人都可以编辑百科页面，并且创建和修改的页面需要经过你审核后才能展示出来
          </span>
        </label>
        <input
          id="advanced-rendering"
          v-model="wikiOpen"
          :disabled="!hasPermission"
          class="switch stylized-toggle"
          type="checkbox"
        />
      </div>
      <div v-if="project.versions.length === 0" class="adjacent-input">
        <label for="project-wiki-open">
          <span class="label__title">资源类型</span>
          <span class="label__description">
            搬运资源,并且没有任何版本上传请在这里设置，设置后前端将同步为该类型的URL,非搬运资源无需设置，因为上传版本以后会自动识别为对应类型
          </span>
        </label>
        <Multiselect
          id="project-default_type"
          v-model="defaultType"
          class="small-multiselect"
          placeholder="选择"
          :options="['mod', 'project', 'plugin', 'datapack', 'resourcepack', 'shader', 'modpack']"
          :searchable="false"
          :close-on-select="true"
          :show-labels="false"
          :allow-empty="false"
          :disabled="!hasPermission"
        />
      </div>
      <div v-if="project.versions.length === 0" class="adjacent-input">
        <label for="project-wiki-open">
          <span class="label__title">游戏版本</span>
          <span class="label__description">
            搬运资源,并且没有任何版本上传请在这里设置，设置后前端将同步为该类型的URL,非搬运资源无需设置，因为上传版本以后会自动识别为对应类型
          </span>
        </label>
        <multiselect
          v-model="defaultGameVersion"
          class="small-multiselect"
          :options="
            tags.gameVersions.filter((it) => it.version_type === 'release').map((x) => x.version)
          "
          :loading="tags.gameVersions.length === 0"
          :disabled="!hasPermission"
          :multiple="true"
          :searchable="true"
          :show-no-results="false"
          :close-on-select="false"
          :clear-on-select="false"
          :show-labels="false"
          :hide-selected="true"
          placeholder="选择支持的MC版本"
        />
      </div>
      <div
        v-if="project.versions.length === 0 && project.project_type !== 'resourcepack'"
        class="adjacent-input"
      >
        <label for="project-wiki-open">
          <span class="label__title">运行平台</span>
          <span class="label__description">
            搬运资源,并且没有任何版本上传请在这里设置，非搬运资源无需设置，因为上传版本以后会自动识别为对应类型
          </span>
        </label>

        <multiselect
          v-model="defaultGameLoaders"
          class="small-multiselect"
          :options="tags.loaderData.allLoaders"
          :disabled="!hasPermission"
          :multiple="true"
          :searchable="true"
          :show-no-results="false"
          :close-on-select="false"
          :clear-on-select="false"
          :show-labels="false"
          :hide-selected="true"
          placeholder="选择运行平台"
        />
      </div>
      <template
        v-if="
          project.versions.length !== 0 &&
          project.project_type !== 'resourcepack' &&
          project.project_type !== 'plugin' &&
          project.project_type !== 'shader' &&
          project.project_type !== 'datapack'
        "
      >
        <div class="adjacent-input">
          <label for="project-env-client">
            <span class="label__title">客户端</span>
            <span class="label__description"> 请选择资源对客户端的支持程度 </span>
          </label>
          <Multiselect
            id="project-env-client"
            v-model="clientSide"
            class="small-multiselect"
            placeholder="Select one"
            :options="sideTypes"
            :custom-label="
              (value) => {
                switch (value) {
                  case 'required':
                    return '必备';
                  case 'optional':
                    return '可选';
                  case 'unsupported':
                    return '不支持';
                  default:
                    return '未知';
                }
              }
            "
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
            :disabled="!hasPermission"
          />
        </div>
        <div class="adjacent-input">
          <label for="project-env-server">
            <span class="label__title">服务端</span>
            <span class="label__description">
              选择该资源在服务端上是否支持,请注意 单人模式 拥有内置服务端
            </span>
          </label>
          <Multiselect
            id="project-env-server"
            v-model="serverSide"
            class="small-multiselect"
            placeholder="Select one"
            :options="sideTypes"
            :custom-label="
              (value) => {
                switch (value) {
                  case 'required':
                    return '必备';
                  case 'optional':
                    return '可选';
                  case 'unsupported':
                    return '不支持';
                  default:
                    return '未知';
                }
              }
            "
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
            :disabled="!hasPermission"
          />
        </div>
      </template>
      <div class="adjacent-input">
        <label for="project-visibility">
          <span class="label__title">可见度</span>
          <div class="label__description">
            公开和私有的资源可在搜索中查看，未公开的资源已发布，但在搜索或用户个人资料中不可见。私人资源仅供资源成员访问。

            <p>已审核项目:</p>
            <ul class="visibility-info">
              <li>
                <CheckIcon
                  v-if="visibility === 'approved' || visibility === 'archived'"
                  class="good"
                />
                <ExitIcon v-else class="bad" />
                {{ hasModifiedVisibility() ? "未" : "" }}允许被搜索
              </li>
              <li>
                <ExitIcon
                  v-if="visibility === 'unlisted' || visibility === 'private'"
                  class="bad"
                />
                <CheckIcon v-else class="good" />
                {{ hasModifiedVisibility() ? "未" : "" }}允许显示在个人资料
              </li>
              <li>
                <CheckIcon v-if="visibility !== 'private'" class="good" />
                <IssuesIcon
                  v-else
                  v-tooltip="{
                    content: visibility === 'private' ? '只有会员才可以查看该项目。' : '',
                  }"
                  class="warn"
                />
                {{ hasModifiedVisibility() ? "未" : "" }}被允许使用URL访问
              </li>
            </ul>
          </div>
        </label>
        <Multiselect
          id="project-visibility"
          v-model="visibility"
          class="small-multiselect"
          placeholder="选择"
          :options="tags.approvedStatuses"
          :custom-label="(value) => formatProjectStatus(value)"
          :searchable="false"
          :close-on-select="true"
          :show-labels="false"
          :allow-empty="false"
          :disabled="!hasPermission"
        />
      </div>
      <div class="button-group">
        <button
          type="button"
          class="iconified-button brand-button"
          :disabled="!hasChanges"
          @click="saveChanges()"
        >
          <SaveIcon aria-hidden="true" />
          保存
        </button>
      </div>
    </section>

    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">删除资源</span>
        </h3>
      </div>
      <p>从 BBSMC 的服务器和搜索中删除您的资源.单击此按钮将删除您的资源,请慎重点击!</p>
      <button
        type="button"
        class="iconified-button danger-button"
        :disabled="!hasDeletePermission"
        @click="$refs.modal_confirm.show()"
      >
        <TrashIcon aria-hidden="true" />
        删除
      </button>
    </section>
  </div>
</template>

<script setup>
import { Multiselect } from "vue-multiselect";

import { formatProjectStatus } from "@modrinth/utils";
import Avatar from "~/components/ui/Avatar.vue";
import ModalConfirm from "~/components/ui/ModalConfirm.vue";
import FileInput from "~/components/ui/FileInput.vue";

import UploadIcon from "~/assets/images/utils/upload.svg?component";
import SaveIcon from "~/assets/images/utils/save.svg?component";
import TrashIcon from "~/assets/images/utils/trash.svg?component";
import ExitIcon from "~/assets/images/utils/x.svg?component";
import IssuesIcon from "~/assets/images/utils/issues.svg?component";
import CheckIcon from "~/assets/images/utils/check.svg?component";

const props = defineProps({
  project: {
    type: Object,
    required: true,
    default: () => ({}),
  },
  currentMember: {
    type: Object,
    required: true,
    default: () => ({}),
  },
  patchProject: {
    type: Function,
    required: true,
    default: () => {},
  },
  patchIcon: {
    type: Function,
    required: true,
    default: () => {},
  },
  resetProject: {
    type: Function,
    required: true,
    default: () => {},
  },
});

const tags = useTags();
const router = useNativeRouter();

const name = ref(props.project.title);
const slug = ref(props.project.slug);
const wikiOpen = ref(props.project.wiki_open);
const defaultType = ref(props.project.default_type);
const defaultGameVersion = ref(props.project.default_game_version);
const defaultGameLoaders = ref(props.project.default_game_loaders);
if (defaultGameLoaders.value === null) {
  defaultGameLoaders.value = [];
}
const summary = ref(props.project.description);
const icon = ref(null);
const previewImage = ref(null);
const clientSide = ref(props.project.client_side);
const serverSide = ref(props.project.server_side);
const deletedIcon = ref(false);
const visibility = ref(
  tags.value.approvedStatuses.includes(props.project.status)
    ? props.project.status
    : props.project.requested_status,
);

const hasPermission = computed(() => {
  const EDIT_DETAILS = 1 << 2;
  return (props.currentMember.permissions & EDIT_DETAILS) === EDIT_DETAILS;
});

const hasDeletePermission = computed(() => {
  const DELETE_PROJECT = 1 << 7;
  return (props.currentMember.permissions & DELETE_PROJECT) === DELETE_PROJECT;
});

const sideTypes = ["required", "optional", "unsupported"];

const patchData = computed(() => {
  const data = {};

  if (name.value !== props.project.title) {
    data.title = name.value.trim();
  }
  if (slug.value !== props.project.slug) {
    data.slug = slug.value.trim();
  }
  if (summary.value !== props.project.description) {
    data.description = summary.value.trim();
  }
  if (wikiOpen.value !== props.project.wiki_open) {
    data.wiki_open = wikiOpen.value;
  }
  if (defaultType.value !== props.project.default_type) {
    data.default_type = defaultType.value;
  }
  if (defaultGameVersion.value !== props.project.default_game_version) {
    data.default_game_version = defaultGameVersion.value;
  }
  if (defaultGameLoaders.value !== props.project.default_game_loaders) {
    data.default_game_loaders = defaultGameLoaders.value;
  }
  if (clientSide.value !== props.project.client_side) {
    data.client_side = clientSide.value;
  }
  if (serverSide.value !== props.project.server_side) {
    data.server_side = serverSide.value;
  }
  if (tags.value.approvedStatuses.includes(props.project.status)) {
    if (visibility.value !== props.project.status) {
      data.status = visibility.value;
    }
  } else if (visibility.value !== props.project.requested_status) {
    data.requested_status = visibility.value;
  }

  return data;
});

const hasChanges = computed(() => {
  return Object.keys(patchData.value).length > 0 || deletedIcon.value || icon.value;
});

const hasModifiedVisibility = () => {
  const originalVisibility = tags.value.approvedStatuses.includes(props.project.status)
    ? props.project.status
    : props.project.requested_status;

  return originalVisibility !== visibility.value;
};

const saveChanges = async () => {
  if (hasChanges.value) {
    await props.patchProject(patchData.value);
  }

  if (deletedIcon.value) {
    await deleteIcon();
    deletedIcon.value = false;
  } else if (icon.value) {
    await props.patchIcon(icon.value);
    icon.value = null;
  }
};

const showPreviewImage = (files) => {
  const reader = new FileReader();
  icon.value = files[0];
  deletedIcon.value = false;
  reader.readAsDataURL(icon.value);
  reader.onload = (event) => {
    previewImage.value = event.target.result;
  };
};

const deleteProject = async () => {
  await useBaseFetch(`project/${props.project.id}`, {
    method: "DELETE",
  });
  await initUserProjects();
  await router.push("/dashboard/projects");
  addNotification({
    group: "main",
    title: "资源已删除",
    text: "您的资源已删除.",
    type: "success",
  });
};

const markIconForDeletion = () => {
  deletedIcon.value = true;
  icon.value = null;
  previewImage.value = null;
};

const deleteIcon = async () => {
  await useBaseFetch(`project/${props.project.id}/icon`, {
    method: "DELETE",
  });
  await props.resetProject();
  addNotification({
    group: "main",
    title: "图片已移除",
    text: "您的资源图标已被移除",
    type: "success",
  });
};
</script>
<style lang="scss" scoped>
.visibility-info {
  padding: 0;
  list-style: none;

  li {
    display: flex;
    align-items: center;
    gap: var(--spacing-card-xs);
  }
}

svg {
  &.good {
    color: var(--color-green);
  }

  &.bad {
    color: var(--color-red);
  }

  &.warn {
    color: var(--color-orange);
  }
}

.summary-input {
  min-height: 8rem;
  max-width: 24rem;
}

.small-multiselect {
  max-width: 15rem;
}

.button-group {
  justify-content: flex-start;
}
</style>
