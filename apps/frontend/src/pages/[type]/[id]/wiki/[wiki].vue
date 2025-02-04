<template>
  <ConfirmModal2
    ref="modal_confirmIndex"
    title="你确定要将该页面设置为主页吗?"
    description="设置成主页后访问该资源的 /wikis 页面会直接显示该页面的内容"
    proceed-label="确认"
    @proceed="confirmIndex()"
  />

  <ConfirmModal
    ref="modal_confirm_delete"
    title="你确定要将该页面删除吗?"
    description="若页面为目录并且目录下含有子页面，请先删除子页面后再提交删除"
    proceed-label="确认"
    @proceed="deleteWiki()"
  />

  <NewModal ref="editSortModel">
    <template #title>
      <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
      <div class="truncate text-lg font-extrabold text-contrast">设置页面排序</div>
    </template>
    <div class="flex flex-col gap-3" style="width: 500px">
      <div class="flex flex-col gap-2">
        <div class="flex flex-col gap-2">
          <label for="name">
            <span class="text-lg font-semibold text-contrast">
              权重
              <span class="text-brand-red">*</span>
              <br /><span style="font-size: 14px"
                >数字越小，越靠前 (默认为0)，取值范围 0 - 1000</span
              >
            </span>
          </label>
          <input
            id="name"
            v-model="wiki.sort_order"
            type="number"
            maxlength="64"
            placeholder="数字越小越靠前"
            autocomplete="off"
          />
        </div>
      </div>
      <div class="mt-5 flex gap-2">
        <ButtonStyled color="brand">
          <button @click="saveChanges">
            <SunriseIcon aria-hidden="true" />
            保存修改
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="$refs.editSortModel.hide()">
            <XIcon aria-hidden="true" />
            取消
          </button>
        </ButtonStyled>
      </div>
    </div>
  </NewModal>
  <section class="normal-page__content">
    <div
      v-if="
        props.wikis.is_editor && props.wikis.is_editor_user && props.wikis.cache.status === 'draft'
      "
      class="universal-card"
    >
      <div class="markdown-disclaimer">
        <h2>正在编辑 [ {{ wiki.title }} ]</h2>
        <span class="label__description">
          请尽量单个页面只包含一个内容，以便于查找和编辑以及分类，
          <br /><br />
          请注意，编辑后的内容会被保存为草稿，若要提交发布请点击目录页的提交审核<br /><br />
        </span>
      </div>
      <MarkdownEditor v-model="wiki.body" :on-image-upload="onUploadHandler" />
      <div class="input-group markdown-disclaimer">
        <button
          type="button"
          class="iconified-button brand-button"
          :disabled="check()"
          @click="saveChanges()"
        >
          <SaveIcon />
          保存草稿
        </button>

        <button
          v-if="wiki.featured === false"
          type="button"
          class="iconified-button brand-button"
          style="margin-left: auto"
          @click="$refs.modal_confirmIndex.show()"
        >
          <StarIcon />
          设置主页
        </button>
        <button
          type="button"
          class="iconified-button brand-button"
          style="margin-left: auto"
          @click="$refs.editSortModel.show()"
        >
          <SunriseIcon />
          设置页面顺序
        </button>

        <button
          type="button"
          class="iconified-button brand-button"
          style="background-color: rgb(255, 73, 110); margin-left: auto"
          @click="$refs.modal_confirm_delete.show()"
        >
          <TrashIcon />
          删除该页面
        </button>
      </div>
    </div>

    <div v-else>
      <div
        v-if="wiki.body"
        class="markdown-body card"
        v-html="renderHighlightedString(wiki.body || '')"
      />
    </div>
  </section>
</template>

<script setup>
import { Avatar, ButtonStyled, ConfirmModal, MarkdownEditor, NewModal } from "@modrinth/ui";
import ConfirmModal2 from "@modrinth/ui/src/components/modal/ConfirmModal2.vue";
import { XIcon } from "@modrinth/assets";
import { renderHighlightedString } from "~/helpers/highlight.js";
import SaveIcon from "assets/images/utils/save.svg";
import TrashIcon from "assets/images/utils/trash.svg";
import StarIcon from "assets/images/utils/star.svg";
import SunriseIcon from "assets/images/utils/sunrise.svg";
import { useImageUpload } from "~/composables/image-upload.ts";

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {};
    },
  },
  wikis: {
    type: Object,
    default() {
      return {};
    },
  },
});
const title = `${props.project.title} - WIKI`;
const description = `浏览 ${props.project.title} 个图片的WIKI页面`;
let wiki = ref(null);
const route = useNativeRoute();
const router = useNativeRouter();
const data = useNuxtApp();
let wikiBodyCache = ref("");
let wikiSort = ref(0);
const editSortModel = ref();

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
});

// console.log('wikis', props.wikis);
if (
  props.wikis.is_editor === true &&
  props.wikis.is_editor_user &&
  props.wikis.cache.status === "draft"
) {
  props.wikis.cache.cache.forEach((wiki_) => {
    if (wiki_.child) {
      wiki_.child.forEach((wiki__) => {
        if (route.params.wiki === wiki__.slug) {
          wiki = wiki__;
        }
      });
    }
    if (route.params.wiki === wiki_.slug) {
      wiki = wiki_;
    }
  });
} else {
  props.wikis.wikis.forEach((wiki_) => {
    if (wiki_.child) {
      wiki_.child.forEach((wiki__) => {
        if (route.params.wiki === wiki__.slug) {
          wiki = wiki__;
        }
      });
    }
    if (route.params.wiki === wiki_.slug) {
      wiki = wiki_;
    }
  });
}

wikiBodyCache = wiki.body;
wikiSort = wiki.sort_order;

async function saveChanges() {
  if (wikiBodyCache === wiki.body && wikiSort === wiki.sort_order) {
    data.$notify({
      group: "main",
      title: "错误",
      text: "</br>未修改任何内容",
      type: "error",
    });
    return;
  }

  const resData = {
    id: wiki.id,
    body: wiki.body,
    sort_order: wiki.sort_order,
  };

  try {
    await useBaseFetch(`project/${route.params.id}/wiki_edit`, {
      apiVersion: 3,
      method: "POST",
      body: resData,
    });
    wikiBodyCache = wiki.body;
    data.$notify({
      group: "main",
      title: "成功",
      text: "</br>草稿保存成功,若要提交发布请前往到WIKI页面的主页提交",
      type: "success",
    });
    router.push(`/project/${route.params.id}/wiki/${wiki.slug}`);
    editSortModel.value.hide();
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}

function check() {
  if (wikiBodyCache === wiki.body) {
    return true;
  }
  return false;
}

async function confirmIndex() {
  // const resData = {
  //   id: wiki.id,
  //   featured: true
  // }

  await useAsyncData(`project/${route.params.id}/wiki_edit_star`, () =>
    useBaseFetch(`project/${route.params.id}/wiki_edit_star`, {
      apiVersion: 3,
      method: "POST",
      body: { id: wiki.id },
    }),
  );

  data.$notify({
    group: "main",
    title: "成功",
    text: "</br>已成功设置为主页",
    type: "success",
  });
  router.push(`/project/${route.params.id}/wikis`);
}

async function deleteWiki() {
  if (wiki.child && wiki.child.length > 0) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "</br>当前为目录,请先删除目录下所有页面后再删除该页面",
      type: "error",
    });
    return;
  }

  await useAsyncData(`project/${route.params.id}/wiki_create`, () =>
    useBaseFetch(`project/${route.params.id}/wiki_delete`, {
      apiVersion: 3,
      method: "DELETE",
      body: { id: wiki.id },
    }),
  );

  // router.push("/dashboard/notifications");
  router.push(`/project/${route.params.id}/wikis`);
}

async function onUploadHandler(file) {
  const response = await useImageUpload(file, {
    context: "project",
    projectID: props.project.id,
  });
  return response.url;
}
</script>

<style scoped lang="scss"></style>
