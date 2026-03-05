<template>
  <!-- <NewModal ref="createForumModel" style="height: auto; width: 800px;">
        <template #title>
            <div class="truncate text-lg font-extrabold text-contrast"">发表新帖</div>
        </template>
<div class=" mt-5 flex gap-2 " style=" justify-content: flex-end">
  <MarkdownEditor v-model="forumContent" :on-image-upload="onUploadHandler" />
</div>
</NewModal> -->
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
      <div style="display: flex; justify-content: space-between; align-items: center">
        <h1>{{ title }}</h1>
        <div
          v-if="type === 'chat' || type === 'article' || (auth.user && auth.user.role === 'admin')"
        >
          <ButtonStyled v-if="!createForumModel" color="green">
            <button @click="createForum">发帖</button>
          </ButtonStyled>
          <ButtonStyled v-else color="red">
            <button @click="createForumModel = false">取消发帖</button>
          </ButtonStyled>
        </div>
      </div>

      <div v-if="createForumModel">
        <div class="universal-card">
          <div class="flex flex-col gap-2" style="margin-bottom: 30px">
            <label for="name">
              <span class="text-lg font-semibold text-contrast">
                帖子标题
                <span class="text-brand-red">*</span>
              </span>
            </label>
            <input
              id="name"
              v-model="forumTitle"
              type="text"
              maxlength="64"
              placeholder="帖子标题"
              autocomplete="off"
            />
          </div>
          <div class="flex flex-col gap-2" style="margin-bottom: 10px">
            <label for="name">
              <span class="text-lg font-semibold text-contrast">
                帖子内容
                <span class="text-brand-red">*</span>
              </span>
            </label>
          </div>

          <MarkdownEditor v-model="forumContent" :on-image-upload="onUploadHandler" />

          <div style="display: flex; justify-content: space-between; align-items: center">
            <span
              >使用
              <a href="http://commonmark.org/help/" target="_blank">Markdown</a> 语法编辑文本</span
            >
            <div
              v-if="
                type === 'chat' || type === 'article' || (auth.user && auth.user.role === 'admin')
              "
            >
              <ButtonStyled color="green">
                <button @click="submitForum">发布</button>
              </ButtonStyled>
            </div>
          </div>
        </div>
      </div>

      <div v-else>
        <div v-for="forum in forums" :key="forum.id" class="universal-card tf">
          <div class="forum-header">
            <a :href="`/user/${forum.user_name}`" target="_blank">
              <img :src="forum.avatar" alt="User Avatar" class="user-avatar" />
            </a>
            <div class="forum-info">
              <a v-if="forum.project_id" :href="`/project/${forum.project_id}/forum`">
                <h2 class="forum-title">{{ forum.title }}</h2>
              </a>
              <a v-else :href="`/d/${forum.id}`">
                <h2 class="forum-title">{{ forum.title }}</h2>
              </a>
              <p class="forum-meta">
                <a
                  v-if="forum.organization_id"
                  :href="`/organization/${forum.organization_id}`"
                  target="_blank"
                  class="forum-user"
                  >{{ forum.organization }}</a
                >
                <a v-else :href="`/user/${forum.user_name}`" target="_blank" class="forum-user">{{
                  forum.user_name
                }}</a>
                <span class="forum-date" style="margin-left: 4px">{{
                  formatDateTime(forum.created_at)
                }}</span>
              </p>
            </div>
          </div>
          <!-- <p class="forum-content">{{ forum.content }}</p> -->
          <div class="forum-footer">
            <span class="forum-state">回复: {{ forum.replies }}</span>
            <span class="forum-last-post">最后回复: {{ fromNow(forum.last_post_time) }}</span>
          </div>
        </div>
        <div>
          <pagination
            :page="currentPage"
            :count="pageCount"
            :link-function="(x) => getSearchUrl(x <= 1 ? 0 : (x - 1) * maxResults)"
            class="justify-end"
            @switch-page="onSearchChangeToTop"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import dayjs from "dayjs";
import { formatDateTime } from "@modrinth/utils";
import { Pagination, ButtonStyled, MarkdownEditor } from "@modrinth/ui";
import { computed } from "vue";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";
import { isDarkTheme } from "~/plugins/theme/themes.ts";
import { useImageUpload } from "~/composables/image-upload.ts";

const data = useNuxtApp();
const route = useNativeRoute();
const createForumModel = ref(false);

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--meta-color": "var(--color-secondary)",
      "--forum-hover-bg": "rgba(255, 255, 255, 0.05)",
    };
  } else {
    return {
      "--meta-color": "#666",
      "--forum-hover-bg": "rgba(0, 0, 0, 0.03)",
    };
  }
});

const auth = await useAuth();
const forums = ref([]);
const currentPage = ref(1);
const pageCount = ref(1);
const maxResults = ref(20);
const router = useNativeRouter();
const noLoad = ref(false);
const query = ref("");
const type = ref(route.params.forum);
const title = ref(
  type.value === "chat"
    ? "矿工茶馆"
    : type.value === "notice"
      ? "公告"
      : type.value === "project"
        ? "资源讨论"
        : type.value === "article"
          ? "专栏"
          : "论坛",
);

const forumContent = ref("");
const forumTitle = ref("");

const ogTitle = computed(
  () => `${title.value} - BBSMC 我的世界社区论坛${query.value ? " | " + query.value : ""}`,
);
const description = computed(
  () =>
    `在 BBSMC 我的世界社区浏览和参与${title.value}讨论，与其他 Minecraft 玩家交流经验、分享心得，获取最新资讯和攻略。`,
);
if (route.query.o) {
  currentPage.value = Math.ceil(route.query.o / maxResults.value) + 1;
}

useSeoMeta({
  title: ogTitle,
  description,
  ogTitle,
  ogDescription: description,
  ogImage: "https://cdn.bbsmc.net/raw/bbsmc-logo.png",
});

// 假设这里有一个方法来获取论坛数据
async function fetchForums() {
  noLoad.value = true;
  const res = await useBaseFetch(`forum/${route.params.forum}/lists`, {
    apiVersion: "3",
    query: {
      page: currentPage.value,
      page_size: maxResults.value,
    },
  });
  forums.value = res.forums;
  pageCount.value = Math.ceil(res.pagination.total / maxResults.value);
  noLoad.value = false;
}

const fromNow = (date) => {
  const currentDate = useCurrentDate();
  return dayjs(date).from(currentDate.value);
};

function onSearchChangeToTop(newPageNumber) {
  if (import.meta.client) {
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  onSearchChange(newPageNumber);
}

function getSearchUrl(offset, useObj) {
  const queryItems = [];
  const obj = {};

  if (offset > 0) {
    queryItems.push(`o=${offset}`);
    obj.o = offset;
  }

  let url = `${route.path}`;

  if (queryItems.length > 0) {
    url += `?${queryItems[0]}`;

    for (let i = 1; i < queryItems.length; i++) {
      url += `&${queryItems[i]}`;
    }
  }

  return useObj ? obj : url;
}
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
function onSearchChange(newPageNumber) {
  noLoad.value = true;

  currentPage.value = newPageNumber;

  if (query.value === null) {
    return;
  }

  fetchForums();

  if (import.meta.client) {
    const obj = getSearchUrl((currentPage.value - 1) * maxResults.value, true);
    router.replace({ path: route.path, query: obj });
  }
}

function createForum() {
  if (!auth.value.user) {
    router.push("/auth/sign-in");

    data.$notify({
      group: "main",
      title: "未登录",
      text: "</br>请先登录或创建账号",
      type: "error",
    });

    return;
  }

  if (!auth.value.user.has_phonenumber) {
    router.push("/settings/account");
    data.$notify({
      group: "main",
      title: "未绑定手机号",
      text: "</br>根据《互联网论坛社区服务管理规定》第八条，您需要绑定手机号后才可以发布信息",
      type: "error",
    });
    return;
  }

  createForumModel.value = true;
}
async function submitForum() {
  if (!auth.value.user) {
    router.push("/auth/sign-in");

    data.$notify({
      group: "main",
      title: "未登录",
      text: "</br>请先登录或创建账号",
      type: "error",
    });

    return;
  }

  if (!auth.value.user.has_phonenumber) {
    router.push("/settings/account");
    data.$notify({
      group: "main",
      title: "未绑定手机号",
      text: "</br>根据《互联网论坛社区服务管理规定》第八条，您需要绑定手机号后才可以发布信息",
      type: "error",
    });
    return;
  }

  if (!forumTitle.value) {
    data.$notify({
      group: "main",
      title: "未填写标题",
      text: "</br>请填写标题",
      type: "error",
    });
    return;
  }
  if (!forumContent.value) {
    data.$notify({
      group: "main",
      title: "未填写内容",
      text: "</br>请填写内容",
      type: "error",
    });
    return;
  }

  try {
    const res = await useBaseFetch(`forum`, {
      apiVersion: "3",
      method: "POST",
      body: {
        title: forumTitle.value,
        content: forumContent.value,
        forum_type: type.value,
      },
    });

    data.$notify({
      group: "main",
      title: "成功",
      text: "帖子发布成功",
      type: "success",
    });
    // console.log(res);

    router.push(`/d/${res.discussion}`);
  } catch (e) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: e.data.description,
      type: "error",
    });
  }
}

// 在组件挂载时获取论坛数据
await fetchForums();
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

.forum-header {
  display: flex;
  align-items: center;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  margin-right: 15px;
}

.forum-info {
  flex-grow: 1;
}

.forum-title {
  font-size: 1em;
  margin: 0;
  color: var(--color-text-dark);
}

.forum-meta {
  font-size: 0.8em;
  color: var(--meta-color);
}

.forum-content {
  margin: 10px 0;
  font-size: 0.8em;
  line-height: 1.5;
  color: var(--color-text);
}

.forum-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.8em;
  color: var(--color-secondary);
}

.forum-state {
  padding: 2px 8px;
  border-radius: 5px;
}

.tf {
  transition:
    box-shadow 0.3s,
    transform 0.3s;
  /* 添加过渡效果 */
  background-color: var(--color-raised-bg);
}

.tf:hover {
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
  transform: scale(1.02);
  /* 鼠标悬停时放大 */
  background-color: var(--forum-hover-bg);
}

.universal-card {
  background-color: var(--color-raised-bg);
}

.forum-user {
  color: var(--color-text-primary);
  text-decoration: none;
}

.forum-date {
  color: var(--color-secondary);
}
</style>
