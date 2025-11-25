<template>
  <div class="forum-container" :style="themeVars">
    <!-- 顶部操作栏 -->
    <div class="forum-header">
      <div class="forum-controls">
        <div class="sort-controls">
          <label>排序方式：</label>
          <select v-model="sortType" @change="changeSortAndReload" class="sort-select">
            <option value="floor_asc">楼层 ↑</option>
            <option value="floor_desc">楼层 ↓</option>
          </select>
        </div>
        <div class="reply-controls">
          <button-styled color="green" @click="showNewReply">发表回复</button-styled>
        </div>
      </div>
    </div>

    <div v-if="forum.length == 0" style="text-align: center; margin-top: 20px">
      还没有任何回复，快来回复吧！
    </div>

    <!-- 帖子列表 -->
    <div class="posts-wrapper">
      <div
        v-for="post in forum"
        :id="`post-${post.floor_number}`"
        :key="post.floor_number"
        ref="postRefs"
        :data-floor-number="post.floor_number"
        class="card markdown-body"
      >
        <div class="post-header">
          <!-- 用户信息区 -->
          <div class="user-info">
            <a :href="`/user/${post.user_name}`" target="_blank" class="user-avatar-link">
              <img
                :src="
                  post.user_avatar === ''
                    ? 'https://cdn.bbsmc.net/raw/bbsmc-logo.png'
                    : post.user_avatar
                "
                :alt="post.user_name"
                class="avatar"
              />
            </a>
            <a :href="`/user/${post.user_name}`" target="_blank" class="username">
              {{ post.user_name }}
            </a>
            <span class="post-time">{{ formatRelativeTime(post.created_at) }}</span>
          </div>
          <div class="post-actions">
            <div class="post-id" :title="'点击复制链接'" @click="copyPostUrl(post.floor_number)">
              #{{ post.floor_number }}
            </div>
          </div>
        </div>

        <!-- 如果是回复帖子，显示回复引用 -->
        <div
          v-if="post.reply_content && !isRepliedToDeleted(post)"
          class="reply-reference"
          @click="scrollToPost(post.replied_to)"
        >
          <div class="reply-info">
            <img
              :src="
                post.reply_content.user_avatar === ''
                  ? 'https://cdn.bbsmc.net/raw/bbsmc-logo.png'
                  : post.reply_content.user_avatar
              "
              :alt="post.reply_content.user_name"
              class="reply-avatar"
            />
            <div class="reply-user-info">
              <span class="reply-username">{{ post.reply_content.user_name }}</span>
              <span class="reply-post-id">#{{ post.replied_to }}</span>
            </div>
          </div>
          <div
            class="reply-quote"
            v-html="renderHighlightedString(post.reply_content.content)"
          ></div>
        </div>

        <!-- 如果回复的帖子被删除了，显示删除提示（保留头像和楼层号） -->
        <div
          v-if="post.reply_content && isRepliedToDeleted(post)"
          class="reply-reference deleted-reply"
          @click="scrollToPost(post.replied_to)"
        >
          <div class="reply-info">
            <img
              :src="
                post.reply_content.user_avatar === ''
                  ? 'https://cdn.bbsmc.net/raw/bbsmc-logo.png'
                  : post.reply_content.user_avatar
              "
              :alt="post.reply_content.user_name"
              class="reply-avatar"
            />
            <div class="reply-user-info">
              <span class="reply-username">{{ post.reply_content.user_name }}</span>
              <span class="reply-post-id">#{{ post.replied_to }}</span>
            </div>
          </div>
          <div class="reply-quote reply-deleted-text">该回复已被删除</div>
        </div>

        <!-- 帖子内容 -->
        <div class="post-content">
          <div v-if="post.deleted" class="deleted-post">
            <span class="deleted-text">该回复已被删除</span>
          </div>
          <div v-else class="markdown-body" v-html="renderHighlightedString(post.content)" />
        </div>

        <!-- 添加底部操作区 -->
        <div v-if="!post.deleted" class="post-footer">
          <!-- 删除按钮：只有发布者和管理员能看到 -->
          <button
            v-if="canDeletePost(post)"
            class="delete-button"
            @click="deletePost(post)"
            title="删除回复"
          >
            删除
          </button>
          <button class="reply-button" @click="showReplyForm(post)">回复</button>
        </div>

        <!-- 如果有回复，显示回复链接 -->
        <div v-if="post.replies.length > 0" class="replies-info">
          <span
            v-for="reply in post.replies"
            :key="reply.floor_number"
            class="reply-link"
            @click="scrollToPost(reply.floor_number)"
            @mouseenter="showReplyPreview(reply)"
            @mouseleave="hideReplyPreview"
          >
            #{{ reply.floor_number }}
          </span>
        </div>
      </div>
    </div>

    <!-- 翻页控制 -->
    <div v-if="totalPages > 1" class="pagination-controls">
      <div class="pagination-nav">
        <!-- Previous按钮 -->
        <button
          class="nav-button prev-next"
          :disabled="currentPage <= 1"
          @click="loadPage(currentPage - 1)"
        >
          ← 上一页
        </button>

        <!-- 页码按钮 -->
        <div class="page-numbers">
          <!-- 第一页 -->
          <button
            v-if="showFirstPage"
            class="page-button"
            :class="{ active: currentPage === 1 }"
            @click="loadPage(1)"
          >
            1
          </button>

          <!-- 前省略号 -->
          <span v-if="showStartEllipsis" class="ellipsis">...</span>

          <!-- 中间页码 -->
          <button
            v-for="page in visiblePages"
            :key="page"
            class="page-button"
            :class="{ active: currentPage === page }"
            @click="loadPage(page)"
          >
            {{ page }}
          </button>

          <!-- 后省略号 -->
          <span v-if="showEndEllipsis" class="ellipsis">...</span>

          <!-- 最后一页 -->
          <button
            v-if="showLastPage"
            class="page-button"
            :class="{ active: currentPage === totalPages }"
            @click="loadPage(totalPages)"
          >
            {{ totalPages }}
          </button>
        </div>

        <!-- Next按钮 -->
        <button
          class="nav-button prev-next"
          :disabled="currentPage >= totalPages"
          @click="loadPage(currentPage + 1)"
        >
          下一页 →
        </button>
      </div>

      <!-- 简化的信息显示 -->
      <div class="pagination-info">共 {{ totalPosts }} 条回复</div>
    </div>

    <!-- 添加悬浮提示框 -->
    <div v-if="previewPost" class="reply-preview" :style="previewPosition">
      <div class="preview-header">
        <img
          :src="
            previewPost.user_avatar === ''
              ? 'https://cdn.bbsmc.net/raw/bbsmc-logo.png'
              : previewPost.user_avatar
          "
          :alt="previewPost.user_name"
          class="preview-avatar"
        />
        <div class="preview-user-info">
          <span class="preview-username">{{ previewPost.user_name }}</span>
        </div>
      </div>
      <div v-if="previewPost.deleted" class="preview-content preview-deleted">该回复已被删除</div>
      <div
        v-else
        class="preview-content"
        v-html="renderHighlightedString(previewPost.content)"
      ></div>
    </div>

    <!-- 修改回复表单为固定定位的底部弹出框 -->
    <div v-if="replyingTo" class="reply-form-overlay" @click.self="cancelReply">
      <div class="reply-form-modal">
        <div class="reply-form-header">
          <span>{{
            replyingTo.id === "new" ? "发表新帖" : `回复 #${replyingTo.floor_number}`
          }}</span>
          <button class="close-button" @click="cancelReply">×</button>
        </div>
        <div class="reply-form-content">
          <MarkdownEditor
            v-model="replyContent"
            :on-image-upload="onUploadHandler"
            placeholder="输入回复内容..."
          />
        </div>
        <div class="reply-form-actions">
          <button class="submit-button" :disabled="!replyContent.trim()" @click="submitReply">
            发送回复
          </button>
        </div>
      </div>
    </div>

    <!-- 删除确认模态框 -->
    <ConfirmModal
      ref="deletePostModal"
      title="删除回复"
      description="删除后无法恢复，确定要删除这条回复吗？"
      proceed-label="确认删除"
      @proceed="confirmDeletePost"
    />
  </div>
</template>

<script setup>
import { onMounted, ref, computed, nextTick, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import dayjs from "dayjs";
import { MarkdownEditor, ConfirmModal } from "@modrinth/ui";
import { renderHighlightedString } from "~/helpers/highlight.js";
import { isDarkTheme } from "~/plugins/theme/themes";

const data = useNuxtApp();
const route = useRoute();
const router = useRouter();
const auth = await useAuth();

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// Props
const props = defineProps({
  discussionId: {
    type: String,
    default: () => null,
  },
});

// 响应式数据
const displayedPosts = ref([]);
const totalPosts = ref(0);
const currentPage = ref(1);
const pageSize = 20;
const sortType = ref("floor_asc");

const postToDelete = ref(null);

// Modal refs
const deletePostModal = ref();

// 添加回复相关的状态
const replyingTo = ref(null);
const replyContent = ref("");

// 添加预览相关的状态
const previewPost = ref(null);
const previewPosition = ref({
  top: "0px",
  left: "0px",
});

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--color-text-secondary": "#8f9ba8",
      "--color-text-primary": "#edeff1",
      "--color-bg-card": "rgba(45, 49, 57, 0.8)",
      "--color-bg-secondary": "rgba(54, 59, 68, 0.6)",
      "--color-bg-hover": "rgba(255, 255, 255, 0.08)",
      "--color-border": "#363b44",
      "--color-timeline": "#2d3139",
      "--color-highlight": "#007aff",
      "--color-highlight-reply": "#ffd700",
      "--color-reply-bg": "rgba(255, 255, 255, 0.12)",
      "--color-reply-bg-hover": "rgba(255, 255, 255, 0.2)",
      "--color-submit-button": "#007aff",
      "--color-submit-button-hover": "#0056cc",
      "--color-submit-disabled": "#363b44",
      "--color-overlay": "rgba(0, 0, 0, 0.5)",
      "--color-modal-bg": "#26292f",
      "--color-scrollbar-track": "#363b44",
      "--color-scrollbar-thumb": "#8f9ba8",
      "--color-delete-hover": "#ff3b30",
    };
  } else {
    return {
      "--color-text-secondary": "#8e8e93",
      "--color-text-primary": "var(--color-text-dark)",
      "--color-bg-card": "rgba(255, 255, 255, 0.8)",
      "--color-bg-secondary": "rgba(242, 242, 247, 0.8)",
      "--color-bg-hover": "rgba(0, 0, 0, 0.04)",
      "--color-border": "#d1d1d6",
      "--color-timeline": "#e0e0e0",
      "--color-highlight": "#007aff",
      "--color-highlight-reply": "#ff9500",
      "--color-reply-bg": "rgba(0, 0, 0, 0.05)",
      "--color-reply-bg-hover": "rgba(0, 0, 0, 0.08)",
      "--color-submit-button": "#007aff",
      "--color-submit-button-hover": "#0056cc",
      "--color-submit-disabled": "#d1d1d6",
      "--color-overlay": "rgba(0, 0, 0, 0.3)",
      "--color-modal-bg": "#ffffff",
      "--color-scrollbar-track": "#f2f2f7",
      "--color-scrollbar-thumb": "#c7c7cc",
      "--color-delete-hover": "#ff3b30",
    };
  }
});

// 计算属性
const forum = computed(() => displayedPosts.value);

const totalPages = computed(() => {
  return Math.ceil(totalPosts.value / pageSize);
});

// 计算可见的页码
const visiblePages = computed(() => {
  const current = currentPage.value;
  const total = totalPages.value;
  const delta = 2; // 当前页左右显示的页数

  let start = Math.max(1, current - delta);
  let end = Math.min(total, current + delta);

  // 调整范围以确保显示足够的页码
  if (end - start < delta * 2) {
    if (start === 1) {
      end = Math.min(total, start + delta * 2);
    } else if (end === total) {
      start = Math.max(1, end - delta * 2);
    }
  }

  const pages = [];
  for (let i = start; i <= end; i++) {
    pages.push(i);
  }
  return pages;
});

// 是否显示第一页
const showFirstPage = computed(() => {
  return !visiblePages.value.includes(1);
});

// 是否显示最后一页
const showLastPage = computed(() => {
  return !visiblePages.value.includes(totalPages.value);
});

// 是否显示开始省略号
const showStartEllipsis = computed(() => {
  return visiblePages.value[0] > 2;
});

// 是否显示结束省略号
const showEndEllipsis = computed(() => {
  return visiblePages.value[visiblePages.value.length - 1] < totalPages.value - 1;
});

// 检查是否有删除权限
const canDeletePost = (post) => {
  if (!auth.value.user) return false;
  return auth.value.user.role === "admin" || post.user_name === auth.value.user.username;
};

// 检查被回复的帖子是否被删除（使用后端返回的字段）
const isRepliedToDeleted = (post) => {
  return post.reply_to_deleted || false;
};

// 获取帖子数据
async function fetchPosts(page = 1, sort = "floor_asc") {
  try {
    if (!props.discussionId) {
      return null;
    }

    const params = new URLSearchParams({
      page_size: pageSize.toString(),
      page: page.toString(),
      sort: sort,
    });

    const data = await useBaseFetch(`forum/${props.discussionId}/posts?${params}`, {
      apiVersion: 3,
      method: "GET",
    });

    return data;
  } catch (error) {
    console.error("Failed to fetch posts:", error);
    return null;
  }
}

// 加载指定页的数据
async function loadPage(page) {
  if (page < 1 || (page > totalPages.value && totalPages.value > 0)) {
    return;
  }

  const data = await fetchPosts(page, sortType.value);

  if (data) {
    displayedPosts.value = data.posts || [];
    totalPosts.value = data.pagination ? data.pagination.total : 0;
    currentPage.value = page;

    // 滚动到顶部
    window.scrollTo({ top: 0, behavior: "smooth" });
  }
}

// 改变排序并重新加载
async function changeSortAndReload() {
  await loadPage(1);
}

// 初始化加载
async function initLoad() {
  const targetId = route.query.id;
  let targetPage = 1;

  if (targetId) {
    // 如果有目标楼层号，计算对应的页码
    targetPage = Math.ceil(parseInt(targetId) / pageSize);
  }

  await loadPage(targetPage);

  if (targetId) {
    // 滚动到目标楼层并高亮
    nextTick(() => {
      const element = document.querySelector(`[data-floor-number="${targetId}"]`);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "center" });
        highlightPost(targetId);
      }
    });
  }
}

// 删除帖子
function deletePost(post) {
  postToDelete.value = post;
  deletePostModal.value.show();
}

// 确认删除帖子
async function confirmDeletePost() {
  if (!postToDelete.value) return;

  try {
    await useBaseFetch(`forum/posts/${postToDelete.value.post_id}`, {
      apiVersion: 3,
      method: "DELETE",
    });

    data.$notify({
      group: "main",
      title: "删除成功",
      text: "回复已删除",
      type: "success",
    });

    // 前端直接更新：将该回复标记为已删除
    const postIndex = displayedPosts.value.findIndex(
      (post) => post.post_id === postToDelete.value.post_id,
    );

    if (postIndex !== -1) {
      // 直接修改当前显示的帖子列表
      displayedPosts.value[postIndex] = {
        ...displayedPosts.value[postIndex],
        deleted: true,
        content: "", // 清空内容
      };
    }
  } catch (err) {
    console.error("删除回复失败:", err);
    data.$notify({
      group: "main",
      title: "删除失败",
      text: err.data?.description || "无法删除回复",
      type: "error",
    });
  } finally {
    postToDelete.value = null;
  }
}

// 复制帖子链接
const copyPostUrl = (floorNumber) => {
  const currentUrl = new URL(window.location.href);
  currentUrl.searchParams.set("id", floorNumber);
  const url = currentUrl.toString();

  navigator.clipboard
    .writeText(url)
    .then(() => {
      data.$notify({
        group: "main",
        title: "成功",
        text: "链接已复制到剪贴板",
        type: "success",
      });
    })
    .catch((err) => {
      console.error("复制失败:", err);
    });
};

// 滚动到指定帖子
async function scrollToPost(floorNumber) {
  hideReplyPreview();

  // 查找帖子是否在当前页
  const targetPost = displayedPosts.value.find((p) => p.floor_number === floorNumber);

  if (targetPost) {
    // 在当前页，直接滚动
    const element = document.getElementById(`post-${floorNumber}`);
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "center" });
      highlightPost(floorNumber);
    }
  } else {
    // 不在当前页，计算页码并跳转
    const targetPage = Math.ceil(floorNumber / pageSize);
    await loadPage(targetPage);

    nextTick(() => {
      const element = document.getElementById(`post-${floorNumber}`);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "center" });
        highlightPost(floorNumber);
      }
    });
  }
}

// 高亮帖子
function highlightPost(floorNumber) {
  const element = document.getElementById(`post-${floorNumber}`);
  if (element) {
    element.classList.add("post-highlight");
    setTimeout(() => {
      element.classList.remove("post-highlight");
    }, 3000);
  }
}

// 显示回复预览
const showReplyPreview = (reply) => {
  previewPost.value = reply;
  nextTick(() => {
    const target = event.target;
    const rect = target.getBoundingClientRect();
    const preview = document.querySelector(".reply-preview");
    if (preview) {
      const previewRect = preview.getBoundingClientRect();
      const left = Math.min(rect.left, window.innerWidth - previewRect.width - 20);
      const top = rect.bottom + window.scrollY + 10;
      previewPosition.value = {
        top: `${top}px`,
        left: `${left}px`,
      };
    }
  });
};

// 隐藏回复预览
const hideReplyPreview = () => {
  previewPost.value = null;
};

// 显示回复表单
const showReplyForm = (post) => {
  if (!auth.value.user) {
    data.$notify({
      group: "main",
      title: "未登录",
      text: "请先登录或创建账号",
      type: "error",
    });
    router.push(`/auth/sign-in`);
    return;
  }

  if (!auth.value.user.has_phonenumber) {
    data.$notify({
      group: "main",
      title: "未绑定手机号",
      text: "根据《互联网论坛社区服务管理规定》第八条，您需要绑定手机号后才可以发布信息",
      type: "error",
    });
    router.push(`/settings/account`);
    return;
  }

  replyingTo.value = post;
  replyContent.value = "";
};

// 取消回复
const cancelReply = () => {
  replyingTo.value = null;
  replyContent.value = "";
};

// 发表新回复
const showNewReply = () => {
  if (!auth.value.user) {
    data.$notify({
      group: "main",
      title: "未登录",
      text: "请先登录或创建账号",
      type: "error",
    });
    router.push(`/auth/sign-in`);
    return;
  }

  if (!auth.value.user.has_phonenumber) {
    data.$notify({
      group: "main",
      title: "未绑定手机号",
      text: "根据《互联网论坛社区服务管理规定》第八条，您需要绑定手机号后才可以发布信息",
      type: "error",
    });
    router.push(`/settings/account`);
    return;
  }

  replyingTo.value = { id: "new" };
  replyContent.value = "";
};

// 提交回复
const submitReply = async () => {
  if (!replyContent.value.trim()) return;
  if (!auth.value.user) {
    data.$notify({
      group: "main",
      title: "未登录",
      text: "请先登录或创建账号",
      type: "error",
    });
    router.push(`/auth/sign-in`);
    return;
  }

  try {
    const res = await useBaseFetch(`forum/${props.discussionId}/post`, {
      apiVersion: 3,
      method: "POST",
      body: {
        content: replyContent.value,
        replied_to: replyingTo.value.id === "new" ? null : replyingTo.value.post_id,
      },
    });

    data.$notify({
      group: "main",
      title: "回复成功",
      text: "回复已发布",
      type: "success",
    });

    cancelReply();

    // 根据新回复的楼层号和排序方式，计算其所在页面
    const newPost = res.post;
    if (newPost) {
      let targetPage;

      if (sortType.value === "floor_desc") {
        // 楼层倒序：新回复（楼层号最大）在第一页
        targetPage = 1;
      } else {
        // 楼层正序：新回复在最后一页
        targetPage = Math.ceil(newPost.floor_number / pageSize);
      }

      // 跳转到新回复所在页面
      await loadPage(targetPage);

      // 滚动到新回复
      nextTick(() => {
        const element = document.getElementById(`post-${newPost.floor_number}`);
        if (element) {
          element.scrollIntoView({ behavior: "smooth", block: "center" });
          highlightPost(newPost.floor_number);
        }
      });
    }
  } catch (err) {
    console.log(err);
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data?.description || "回复发送失败",
      type: "error",
    });
  }
};

// 图片上传处理函数
const onUploadHandler = async (file) => {
  const response = await useImageUpload(file, {
    context: "project",
    projectID: props.discussionId,
  });
  return response.url;
};

// 格式化相对时间
const formatRelativeTime = (dateString) => {
  return dayjs(dateString).fromNow();
};

// 初始化
onMounted(() => {
  initLoad();
});

// 监听 discussionId 变化
watch(
  () => props.discussionId,
  (newId, oldId) => {
    if (newId && newId !== oldId) {
      initLoad();
    }
  },
);
</script>

<style scoped>
.forum-container {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* 顶部操作栏样式 */
.forum-header {
  background: var(--color-bg-card);
  border-radius: 8px;
  padding: 16px;
  border: 1px solid var(--color-border);
}

.forum-controls {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.sort-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sort-controls label {
  color: var(--color-text-primary);
  font-size: 14px;
  font-weight: 500;
}

.sort-select {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 4px;
  padding: 6px 12px;
  color: var(--color-text-primary);
  font-size: 14px;
  min-width: 120px;
  cursor: pointer;
}

.sort-select:focus {
  outline: none;
  border-color: var(--color-highlight);
}

.reply-controls {
  flex-shrink: 0;
}

.reply-controls button-styled {
  cursor: pointer;
}

.posts-wrapper {
  flex: 1;
}

/* 帖子样式 */
.post-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
}

.user-info {
  display: flex;
  align-items: center;
  margin-bottom: 0;
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  margin-right: 10px;
}

.username {
  font-weight: 500;
  margin-right: 10px;
  color: var(--color-text-primary);
  text-decoration: none;
  cursor: pointer;
  transition: color 0.2s ease;
}

.username:hover {
  color: var(--color-highlight);
}

.post-time {
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.post-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.post-id {
  color: var(--color-text-secondary);
  font-size: 0.9em;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: color 0.2s ease;
}

.post-id:hover {
  color: var(--color-text-primary);
}

/* 回复引用样式 */
.reply-reference {
  margin: 8px 16px;
  padding: 8px 12px;
  background: var(--color-reply-bg);
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s ease;
}

.reply-reference:hover {
  background: var(--color-reply-bg-hover);
}

.reply-reference.deleted-reply {
  opacity: 0.7;
  cursor: default !important;
}

.reply-reference.deleted-reply:hover {
  background: var(--color-reply-bg) !important;
}

.reply-info {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.reply-avatar {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  margin-right: 8px;
}

.reply-user-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.reply-username {
  color: var(--color-text-primary);
  font-size: 0.9em;
  font-weight: 500;
}

.reply-post-id {
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.reply-text {
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.reply-deleted {
  color: var(--color-text-secondary);
  font-size: 0.9em;
  font-style: italic;
}

.reply-deleted-text {
  color: var(--color-text-secondary);
  font-style: italic;
}

.reply-quote {
  color: var(--color-text-primary);
  font-size: 0.95em;
  opacity: 0.8;
  overflow: hidden;
  max-height: 100px;
  margin-left: 32px;
}

/* 帖子内容样式 */
.post-content {
  padding: 16px;
}

.deleted-post {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  background: var(--color-bg-secondary);
  border-radius: 8px;
  border: 1px dashed var(--color-border);
}

.deleted-text {
  color: var(--color-text-secondary);
  font-style: italic;
  font-size: 14px;
}

.markdown-body {
  line-height: 1.6;
  color: var(--color-text-primary);
}

/* 帖子底部操作 */
.post-footer {
  padding: 4px 16px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: -8px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.card:hover .post-footer {
  opacity: 1;
}

.reply-button {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  padding: 2px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  transition: color 0.2s ease;
}

.reply-button:hover {
  color: var(--color-text-primary);
}

.delete-button {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  padding: 2px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  transition: color 0.2s ease;
}

.delete-button:hover {
  color: var(--color-delete-hover);
}

/* 回复信息 */
.replies-info {
  margin: 8px 16px;
  padding-top: 8px;
  border-top: 1px solid var(--color-border);
  color: var(--color-text-secondary);
  font-size: 0.9em;
}

.reply-link {
  cursor: pointer;
  margin-right: 8px;
  transition: color 0.2s ease;
  position: relative;
}

.reply-link:hover {
  color: var(--color-text-primary);
}

/* 翻页控制样式 - 宽布局风格 */
.pagination-controls {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: var(--color-bg-card);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  width: 100%;
}

.pagination-nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: 12px;
}

.prev-next {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  color: var(--color-text-secondary);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
  min-width: 100px;
}

.prev-next:hover:not(:disabled) {
  background: var(--color-bg-hover);
  color: var(--color-text-primary);
  border-color: var(--color-highlight);
}

.prev-next:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  color: var(--color-text-secondary);
}

.page-numbers {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  flex: 1;
}

.page-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  color: var(--color-text-secondary);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
}

.page-button:hover {
  background: var(--color-bg-hover);
  color: var(--color-text-primary);
  border-color: var(--color-highlight);
}

.page-button.active {
  background: var(--color-highlight);
  color: white;
  border-color: var(--color-highlight);
  font-weight: 600;
}

.page-button.active:hover {
  background: var(--color-highlight);
  color: white;
}

.ellipsis {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  color: var(--color-text-secondary);
  font-size: 14px;
  font-weight: 500;
  user-select: none;
}

.pagination-info {
  color: var(--color-text-secondary);
  font-size: 13px;
  text-align: center;
  opacity: 0.8;
  font-weight: 400;
}

/* 高亮效果 */
.post-highlight {
  animation: popEffect 3s ease-out;
  border-left: 3px solid var(--color-highlight);
}

@keyframes popEffect {
  0% {
    transform: scale(1);
    box-shadow: 0 0 20px rgba(26, 115, 232, 0.3);
  }

  50% {
    transform: scale(1.01);
  }

  100% {
    transform: scale(1);
    box-shadow: none;
  }
}

/* 预览样式 */
.reply-preview {
  position: absolute;
  z-index: 1000;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-button-bg);
  border-radius: 6px;
  padding: 12px;
  min-width: 300px;
  max-width: 500px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.preview-header {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.preview-avatar {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  margin-right: 8px;
}

.preview-user-info {
  display: flex;
  flex-direction: column;
}

.preview-username {
  color: var(--color-text-primary);
  font-size: 0.9em;
  font-weight: 500;
}

.preview-content {
  color: var(--color-text-primary);
  font-size: 0.95em;
  line-height: 1.5;
  max-height: 200px;
  overflow-y: auto;
}

.preview-deleted {
  color: var(--color-text-secondary);
  font-style: italic;
}

.preview-content::-webkit-scrollbar {
  width: 4px;
}

.preview-content::-webkit-scrollbar-track {
  background: var(--color-scrollbar-track);
}

.preview-content::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar-thumb);
  border-radius: 2px;
}

/* 回复表单样式 */
.reply-form-overlay {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  top: 0;
  background: var(--color-overlay);
  z-index: 1000;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  animation: fadeIn 0.2s ease;
}

.reply-form-modal {
  background: var(--color-modal-bg);
  width: 800px;
  max-width: 90%;
  margin: 0 auto;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  animation: slideUp 0.3s ease;
  border-top-left-radius: 8px;
  border-top-right-radius: 8px;
}

.reply-form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-primary);
  font-size: 1em;
}

.close-button {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  font-size: 1.5em;
  cursor: pointer;
  padding: 0 8px;
  transition: color 0.2s ease;
}

.close-button:hover {
  color: var(--color-text-primary);
}

.reply-form-content {
  flex: 1;
  min-height: 200px;
  max-height: calc(80vh - 120px);
  overflow-y: auto;
  padding: 16px;
}

.reply-form-actions {
  padding: 16px;
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid var(--color-border);
}

.submit-button {
  background: var(--color-submit-button);
  border: none;
  color: white;
  padding: 8px 24px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.95em;
  transition: background-color 0.2s ease;
}

.submit-button:hover:not(:disabled) {
  background: var(--color-submit-button-hover);
}

.submit-button:disabled {
  background: var(--color-submit-disabled);
  cursor: not-allowed;
  opacity: 0.7;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }

  to {
    opacity: 1;
  }
}

@keyframes slideUp {
  from {
    transform: translateY(100%);
  }

  to {
    transform: translateY(0);
  }
}

/* 响应式样式 */
@media (max-width: 768px) {
  .forum-controls {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }

  .sort-controls {
    justify-content: center;
  }

  .reply-controls {
    display: flex;
    justify-content: center;
  }

  .reply-controls button-styled {
    cursor: pointer !important;
    display: inline-flex !important;
  }

  .pagination-controls {
    padding: 16px;
    gap: 12px;
  }

  .pagination-nav {
    flex-direction: column;
    gap: 12px;
  }

  .prev-next {
    width: 100%;
    min-width: auto;
    font-size: 13px;
    padding: 10px 16px;
  }

  .page-numbers {
    justify-content: center;
    gap: 2px;
    flex-wrap: wrap;
  }

  .page-button {
    width: 36px;
    height: 36px;
    font-size: 13px;
  }

  .ellipsis {
    width: 36px;
    height: 36px;
    font-size: 13px;
  }

  .pagination-info {
    font-size: 12px;
  }

  .reply-form-modal {
    width: 100%;
    max-width: 100%;
    height: 100vh;
    max-height: 100vh;
    border-radius: 0;
  }

  .post-actions {
    flex-wrap: wrap;
  }
}

.user-avatar-link {
  display: block;
  text-decoration: none;
  transition: opacity 0.2s ease;
}

.user-avatar-link:hover {
  opacity: 0.8;
}
</style>
