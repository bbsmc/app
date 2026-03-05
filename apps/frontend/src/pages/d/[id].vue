<template>
  <ConfirmModal
    ref="modal_confirm_delete"
    title="你确定要将该帖子删除吗?"
    description="删除后不可撤回，数据将会被从服务器中删除"
    proceed-label="确认"
    @proceed="deleteForm"
  />

  <div :style="themeVars" class="normal-page">
    <div class="normal-page__sidebar">
      <div class="universal-card">
        <h2>综合交流</h2>
        <NavStack>
          <NavStackItem link="/forums/chat" label="矿工茶馆" />
          <NavStackItem link="/forums/project" label="资源讨论" />
          <NavStackItem link="/forums/article" label="专栏" />
        </NavStack>
        <h2>论坛事务</h2>
        <NavStack>
          <NavStackItem link="/forums/notice" label="公告" />
        </NavStack>
      </div>
    </div>
    <div class="normal-page__content">
      <div class="card">
        <h1 v-if="!isEdit" class="forum-title">{{ forum.title }}</h1>
        <input
          v-else
          id="name"
          v-model="forum.title"
          type="text"
          maxlength="64"
          placeholder="帖子标题"
          autocomplete="off"
        />
        <!-- 分割线 -->
        <div class="divider"></div>
        <div class="user-info">
          <a :href="`/user/${forum.user_name}`" target="_blank">
            <img :src="forum.avatar" alt="user-avatar" class="user-avatar" />
          </a>

          <a :href="`/user/${forum.user_name}`" target="_blank">
            <span class="user-name">{{ forum.user_name }}</span>
          </a>
        </div>

        <div v-if="!isEdit" class="markdown-body" v-html="renderHighlightedString(forum.content)" />
        <MarkdownEditor v-else v-model="forum.content" :on-image-upload="onUploadHandler" />
        <!-- 添加底部操作区 -->
        <div v-if="auth.user && isCreate" class="d-footer">
          <div v-if="isEdit" class="flex gap-2">
            <ButtonStyled color="red" class="mr-2 mt-3">
              <button @click="isEdit = false">取消</button>
            </ButtonStyled>
            <ButtonStyled color="green">
              <button @click="saveChanges">保存</button>
            </ButtonStyled>
          </div>
          <div v-else>
            <button
              v-if="['article', 'notice'].includes(forum.category)"
              class="delete-button"
              @click="isEdit = true"
            >
              编辑
            </button>
            <button class="delete-button" @click="$refs.modal_confirm_delete.show()">删除</button>
          </div>
        </div>
      </div>
      <ForumModal :discussion-id="forumId" :is-project="!!forum.project_id" style="padding: 10px" />
    </div>
  </div>
</template>

<script setup>
import { ButtonStyled, ConfirmModal, MarkdownEditor } from "@modrinth/ui";
import { computed } from "vue";
import ForumModal from "~/components/ui/ForumModal.vue";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";
import { renderHighlightedString } from "~/helpers/highlight.js";
import { useImageUpload } from "~/composables/image-upload.ts";
import { isDarkTheme } from "~/plugins/theme/themes.ts";

const router = useNativeRouter();
const route = useNativeRoute();
const data = useNuxtApp();
const auth = await useAuth();

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--divider-color": "rgba(71, 75, 84, 0.6)",
      "--hover-bg": "rgba(255, 255, 255, 0.1)",
    };
  } else {
    return {
      "--divider-color": "rgba(200, 200, 200, 0.8)",
      "--hover-bg": "rgba(0, 0, 0, 0.05)",
    };
  }
});

const forumId = route.params.id;
const forum = ref(null);
const isEdit = ref(false);

forum.value = await useBaseFetch(`forum/${forumId}`, {
  apiVersion: "3",
});

const isCreate = computed(() => {
  return auth.value.user.role === "admin" || auth.value.user.username === forum.value.user_name;
});

const deleteForm = async () => {
  try {
    await useBaseFetch(`forum/${forumId}`, {
      method: "DELETE",
      apiVersion: "3",
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "已删除",
      type: "success",
    });
    await router.push("/forums/" + forum.value.category);
  } catch (e) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: e.data.description,
      type: "error",
    });
  }
};

const saveChanges = async () => {
  try {
    await useBaseFetch(`forum/${forumId}`, {
      method: "PATCH",
      apiVersion: "3",
      body: {
        title: forum.value.title,
        content: forum.value.content,
      },
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "已保存",
      type: "success",
    });
    isEdit.value = false;
  } catch (e) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: e.data.description,
      type: "error",
    });
  }
};

async function onUploadHandler(file) {
  try {
    // await useBaseFetch(`image/can_upload`, {
    //     apiVersion: "3",
    //     method: "GET",
    // });

    const response = await useImageUpload(file, {
      context: "user",
      userID: auth.value.user.id,
    });
    return response.url;
  } catch (e) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: e.data.description,
      type: "error",
    });
    return "";
  }
}
</script>

<style scoped>
.normal-page__content {
  padding: 20px;
  z-index: 2;
}

.normal-page__sidebar {
  padding: 20px;
  z-index: 2;
}

.user-info {
  display: flex;
  align-items: center;
  margin-bottom: 10px;
}

.user-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  margin-right: 10px;
}

.user-name {
  font-size: 1.2em;
  font-weight: bold;
  color: var(--color-text-dark);
}

/* 添加 universal-card 的悬停效果 */
.universal-card {
  transition:
    box-shadow 0.3s,
    transform 0.3s;
  background-color: var(--color-raised-bg);
}

.universal-card:hover {
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  transform: scale(1.02);
}

.divider {
  width: 100%;
  height: 1px;
  background-color: var(--divider-color);
  margin: 20px 0;
}

.d-footer {
  padding: 4px 16px;
  display: flex;
  justify-content: flex-end;
  margin-top: -8px;
  opacity: 0;
  /* 默认隐藏 */
  transition: opacity 0.2s ease;
}

/* 当鼠标悬停在帖子卡片上时显示回复按钮 */
.card:hover .d-footer {
  opacity: 1;
}

.delete-button {
  background: transparent;
  border: none;
  color: var(--color-secondary);
  padding: 2px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  transition: color 0.2s ease;
}

.delete-button:hover {
  color: var(--color-text-dark);
  background-color: var(--hover-bg);
}

.forum-title {
  font-size: 1.5em;
  margin: 0 0 0.5em 0;
}
</style>
