<template>
  <div class="issue-container" :style="themeVars">
    <!-- 加载状态 -->
    <div v-if="pending" class="loading-state">
      <LoadingSpinner />
      <p>加载中...</p>
    </div>

    <!-- Issue 详情 -->
    <div v-else-if="issue" class="issue-detail-wrapper">
      <!-- Issue 头部 -->
      <div class="issue-header card">
        <div class="issue-title-section">
          <h1 class="issue-title">{{ issue.title }}</h1>
          <div class="issue-meta-line">
            <div class="issue-status" :class="{ closed: issue.state === 'closed' }">
              <span v-if="issue.state === 'open'" class="status-icon open">●</span>
              <span v-else class="status-icon closed">✓</span>
              <span>{{ issue.state === "open" ? "开放" : "已关闭" }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Issue 标签和操作按钮 -->
      <div class="labels-actions-section">
        <div class="labels-container">
          <span
            v-for="label in issue.labels"
            :key="label.id"
            class="issue-label"
            :style="{ backgroundColor: label.color }"
          >
            {{ label.name }}
          </span>
        </div>

        <!-- 操作按钮 -->
        <div class="issue-actions">
          <!-- 关闭问题按钮：管理员或创建者 -->
          <button
            v-if="issue.state === 'open' && canCloseIssue"
            class="iconified-button"
            :disabled="isUpdatingState"
            @click="closeIssueModal.show()"
          >
            <LockIcon />
            关闭问题
          </button>

          <!-- 重新打开问题按钮：只有管理员 -->
          <button
            v-if="issue.state === 'closed' && canReopenIssue"
            class="iconified-button"
            :disabled="isUpdatingState"
            @click="reopenIssueModal.show()"
          >
            <PlayIcon />
            重新打开问题
          </button>

          <!-- 管理标签按钮：需要编辑权限 -->
          <button v-if="canEdit" class="iconified-button" @click="openLabelModal()">
            <TagIcon />
            管理标签
          </button>

          <!-- <button class="iconified-button" @click="editIssue()">
            <EditIcon />
            编辑问题
          </button> -->

          <!-- 所有用户都可以使用的按钮 -->
          <button class="iconified-button" @click="copyIssueLink()">
            <ClipboardCopyIcon />
            复制链接
          </button>
        </div>
      </div>

      <!-- Issue 作者信息 -->
      <div class="issue-author-card card">
        <div class="author-header">
          <div class="author-info">
            <img
              v-if="issue.author.avatar"
              :src="issue.author.avatar"
              :alt="issue.author.username"
              class="author-avatar"
            />
            <div class="author-details">
              <NuxtLink :to="`/user/${issue.author.username}`" class="author-name">
                {{ issue.author.username }}
              </NuxtLink>
              <span class="author-time">于 {{ formatRelativeTime(issue.created_at) }} 创建</span>
            </div>
          </div>

          <div v-if="issue.assignees && issue.assignees.length > 0" class="assignees">
            <span class="assignees-label">分配给：</span>
            <div class="assignees-list">
              <span v-for="assignee in issue.assignees" :key="assignee.user_id" class="assignee">
                <NuxtLink :to="`/user/${assignee.user_name}`">{{ assignee.user_name }}</NuxtLink>
              </span>
            </div>
          </div>
        </div>

        <!-- Issue 内容 -->
        <div class="issue-body markdown-body" v-html="renderHighlightedString(issue.body)"></div>
      </div>

      <!-- 回复区 -->
      <div class="comments-section">
        <div class="comments-header">
          <h3>回复 ({{ comments.length }})</h3>
        </div>

        <!-- 回复列表 -->
        <div v-if="comments.length > 0" class="comments-list">
          <div
            v-for="comment in comments"
            :id="`comment-${comment.floor_number}`"
            :key="comment.id"
            class="card comment-card"
          >
            <div class="comment-header">
              <div class="comment-author-info">
                <img
                  v-if="comment.author.avatar"
                  :src="comment.author.avatar"
                  :alt="comment.author.username"
                  class="comment-avatar"
                />
                <div class="comment-author-details">
                  <NuxtLink :to="`/user/${comment.author.username}`" class="comment-author-name">
                    {{ comment.author.username }}
                  </NuxtLink>
                  <span class="comment-floor">#{{ comment.floor_number }}</span>
                  <span class="comment-time">{{ formatRelativeTime(comment.created_at) }}</span>
                </div>
              </div>

              <div v-if="canEditComment(comment) && !comment.deleted" class="comment-actions">
                <button class="action-btn" @click="editComment(comment)">编辑</button>
                <button class="action-btn delete" @click="deleteComment(comment)">删除</button>
              </div>
            </div>

            <!-- 如果是回复，显示回复引用 -->
            <div
              v-if="comment.reply_to_floor"
              class="reply-reference"
              :class="{ 'deleted-reply': isRepliedCommentDeleted(comment) }"
              @click="!isRepliedCommentDeleted(comment) && scrollToComment(comment.reply_to_floor)"
            >
              <div class="reply-info">
                <span class="reply-text">回复</span>
                <span v-if="!isRepliedCommentDeleted(comment)" class="reply-floor"
                  >#{{ comment.reply_to_floor }}</span
                >
                <span v-else class="reply-deleted">已被删除的回复</span>
              </div>
            </div>

            <div class="comment-body markdown-body">
              <div v-if="comment.deleted" class="deleted-comment">
                <span class="deleted-text">该回复已被删除</span>
              </div>
              <div v-else v-html="renderHighlightedString(comment.body)"></div>
            </div>

            <div v-if="!comment.deleted" class="comment-footer">
              <button class="reply-btn" @click="replyToComment(comment)">
                回复 #{{ comment.floor_number }}
              </button>
            </div>
          </div>
        </div>

        <!-- 创建回复表单 -->
        <div v-if="isAuth && issue.state === 'open'" class="create-comment-section">
          <div v-if="replyingTo" class="replying-info card">
            <span>回复 #{{ replyingTo.floor_number }}</span>
            <button class="cancel-reply" @click="cancelReply">取消</button>
          </div>

          <div class="comment-form card">
            <h4>{{ replyingTo ? `回复 #${replyingTo.floor_number}` : "添加回复" }}</h4>
            <MarkdownEditor
              v-model="newComment.body"
              :on-image-upload="onUploadHandler"
              placeholder="写下你的回复..."
            />

            <div class="comment-form-actions">
              <button-styled
                color="green"
                :disabled="!newComment.body.trim() || isCommenting"
                @click="createComment"
              >
                {{ isCommenting ? "发送中..." : "发送回复" }}
              </button-styled>
            </div>
          </div>
        </div>

        <!-- 未登录提示 -->
        <div v-else-if="!isAuth" class="login-prompt card">
          请 <NuxtLink to="/auth/sign-in" class="login-link">登录</NuxtLink> 后发表回复
        </div>

        <!-- Issue 已关闭提示 -->
        <div v-else-if="issue.state === 'closed'" class="closed-prompt card">
          此问题已关闭，无法添加新回复
        </div>
      </div>
    </div>

    <!-- 未找到 -->
    <div v-else class="not-found">
      <div class="not-found-content">
        <h2>问题不存在</h2>
        <p>请检查链接是否正确</p>
        <NuxtLink :to="`/${route.params.type}/${route.params.id}/issues`">
          <button-styled>返回问题列表</button-styled>
        </NuxtLink>
      </div>
    </div>

    <!-- 标签管理模态框 -->
    <div v-if="showLabelModal" class="label-modal-overlay" @click.self="closeLabelModal">
      <div class="label-modal">
        <div class="modal-header">
          <span>管理标签</span>
          <button class="close-button" @click="closeLabelModal">×</button>
        </div>

        <div class="modal-content">
          <div class="current-labels">
            <h4>当前标签</h4>
            <div v-if="issue.labels && issue.labels.length > 0" class="labels-list">
              <span
                v-for="label in issue.labels"
                :key="label.id"
                class="current-label"
                :style="{ backgroundColor: label.color }"
              >
                {{ label.name }}
                <button class="remove-label-btn" @click="removeLabel(label)">×</button>
              </span>
            </div>
            <div v-else class="no-labels">暂无标签</div>
          </div>

          <div class="available-labels">
            <h4>可用标签</h4>
            <div v-if="availableLabels.length > 0" class="labels-list">
              <button
                v-for="label in availableLabels"
                :key="label.id"
                :disabled="isLabelSelected(label)"
                class="available-label"
                :class="{ disabled: isLabelSelected(label) }"
                :style="{ backgroundColor: label.color }"
                @click="addLabel(label)"
              >
                {{ label.name }}
              </button>
            </div>
            <div v-else class="no-labels">暂无可用标签</div>
          </div>
        </div>

        <div class="modal-actions">
          <button-styled @click="closeLabelModal"> 完成 </button-styled>
        </div>
      </div>
    </div>

    <!-- 关闭Issue确认模态框 -->
    <ConfirmModal
      ref="closeIssueModal"
      title="关闭问题"
      description="关闭后，此问题将被标记为已解决，不再接受新的回复。"
      proceed-label="确认关闭"
      @proceed="confirmCloseIssue"
    />

    <!-- 重新打开Issue确认模态框 -->
    <ConfirmModal2
      ref="reopenIssueModal"
      title="重新打开问题"
      description="重新打开后，此问题将重新接受回复和讨论。"
      proceed-label="确认打开"
      @proceed="confirmReopenIssue"
    />

    <!-- 删除回复确认模态框 -->
    <ConfirmModal
      ref="deleteCommentModal"
      title="删除回复"
      description="删除后无法恢复，确定要删除这条回复吗？"
      proceed-label="确认删除"
      @proceed="confirmDeleteComment"
    />
  </div>
</template>

<script setup>
import { ButtonStyled, ConfirmModal, MarkdownEditor } from "@modrinth/ui";

import ConfirmModal2 from "@modrinth/ui/src/components/modal/ConfirmModal2.vue";
import dayjs from "dayjs";
import { TagIcon, LockIcon, PlayIcon, ClipboardCopyIcon } from "@modrinth/assets";
import { renderHighlightedString } from "~/helpers/highlight.js";
import { isDarkTheme } from "~/plugins/theme/themes";

const data = useNuxtApp();
const route = useNativeRoute();
const auth = await useAuth();

const props = defineProps({
  project: {
    type: Object,
    default: () => ({}),
  },
  currentMember: {
    type: Object,
    default() {
      return null;
    },
  },
});

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--color-text-secondary": "#8f9ba8",
      "--color-text-primary": "#edeff1",
      "--color-bg-card": "var(--color-raised-bg)",
      "--color-bg-secondary": "#2d3139",
      "--color-bg-hover": "#363b44",
      "--color-border": "#363b44",
      "--color-highlight": "#007bff",
      "--color-success": "#28a745",
      "--color-closed": "#6f42c1",
      "--color-reply-bg": "rgba(255, 255, 255, 0.12)",
      "--color-reply-bg-hover": "rgba(255, 255, 255, 0.2)",
    };
  } else {
    return {
      "--color-text-secondary": "#666",
      "--color-text-primary": "var(--color-text-dark)",
      "--color-bg-card": "var(--color-raised-bg)",
      "--color-bg-secondary": "#f0f2f5",
      "--color-bg-hover": "#e6e8eb",
      "--color-border": "#dfe1e5",
      "--color-highlight": "#1a73e8",
      "--color-success": "#28a745",
      "--color-closed": "#6f42c1",
      "--color-reply-bg": "rgba(0, 0, 0, 0.05)",
      "--color-reply-bg-hover": "rgba(0, 0, 0, 0.08)",
    };
  }
});

// 响应式数据
const issue = ref(null);
const comments = ref([]);
const pending = ref(true);
const isCommenting = ref(false);
const replyingTo = ref(null);
const showLabelModal = ref(false);
const availableLabels = ref([]);
const isUpdatingLabels = ref(false);
const isUpdatingState = ref(false);
const commentToDelete = ref(null);

// Modal refs
const closeIssueModal = ref();
const reopenIssueModal = ref();
const deleteCommentModal = ref();

const newComment = ref({
  body: "",
  reply_to_id: null,
});

const isAuth = computed(() => {
  return !!auth.value.user;
});

// 计算属性
const hasPermission = computed(() => {
  if (!props.currentMember) return false;
  const EDIT_BODY = 1 << 3;
  return (props.currentMember.permissions & EDIT_BODY) === EDIT_BODY;
});

const canEdit = computed(() => {
  if (!props.currentMember || !issue.value) return false;
  return hasPermission.value;
});

// 关闭问题权限：管理员或问题创建者
const canCloseIssue = computed(() => {
  if (!issue.value || !auth.value.user) return false;
  return (
    (props.currentMember && hasPermission.value) || issue.value.author.id === auth.value.user.id
  );
});

// 重新打开问题权限：只有管理员
const canReopenIssue = computed(() => {
  if (!props.currentMember) return false;
  return hasPermission.value;
});

const canEditComment = (comment) => {
  if (!auth.value.user) return false;
  return hasPermission.value || comment.author.id === auth.value.user.id;
};

// 检查被回复的回复是否被删除（使用后端返回的字段）
const isRepliedCommentDeleted = (comment) => {
  return comment.reply_to_deleted || false;
};

// 加载Issue详情
async function loadIssue() {
  pending.value = true;
  try {
    const response = await useBaseFetch(`issues/${route.params.issue}`, {
      apiVersion: 3,
    });

    issue.value = response;
    await loadComments();
  } catch (err) {
    console.error("加载Issue失败:", err);
    data.$notify({
      group: "main",
      title: "加载失败",
      text: err.data?.description || "无法加载Issue详情",
      type: "error",
    });
  } finally {
    pending.value = false;
  }
}

// 加载回复
async function loadComments() {
  try {
    const response = await useBaseFetch(`issues/${route.params.issue}/comments`, {
      apiVersion: 3,
    });

    comments.value = response.comments || [];
  } catch (err) {
    console.error("加载回复失败:", err);
  }
}

// 创建回复
async function createComment() {
  if (!newComment.value.body.trim()) return;

  isCommenting.value = true;
  try {
    await useBaseFetch(`issues/${route.params.issue}/comments`, {
      apiVersion: 3,
      method: "POST",
      body: {
        body: newComment.value.body.trim(),
        reply_to_id: replyingTo.value?.id || null,
      },
    });

    data.$notify({
      group: "main",
      title: "回复成功",
      text: "回复已发布",
      type: "success",
    });

    newComment.value.body = "";
    replyingTo.value = null;
    await loadComments();
  } catch (err) {
    console.error("创建回复失败:", err);
    data.$notify({
      group: "main",
      title: "回复失败",
      text: err.data?.description || "无法发布回复",
      type: "error",
    });
  } finally {
    isCommenting.value = false;
  }
}

// 回复回复
function replyToComment(comment) {
  replyingTo.value = comment;
  newComment.value.reply_to_id = comment.id;

  // 滚动到回复表单
  nextTick(() => {
    const commentForm = document.querySelector(".comment-form");
    if (commentForm) {
      commentForm.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  });
}

// 取消回复
function cancelReply() {
  replyingTo.value = null;
  newComment.value.reply_to_id = null;
}

// 滚动到回复
function scrollToComment(floorNumber) {
  const element = document.getElementById(`comment-${floorNumber}`);
  if (element) {
    element.scrollIntoView({ behavior: "smooth", block: "center" });
    element.classList.add("comment-highlight");
    setTimeout(() => {
      element.classList.remove("comment-highlight");
    }, 3000);
  }
}

// 确认关闭Issue
async function confirmCloseIssue() {
  isUpdatingState.value = true;
  try {
    await useBaseFetch(`issues/${route.params.issue}`, {
      apiVersion: 3,
      method: "PATCH",
      body: { state: "closed" },
    });

    data.$notify({
      group: "main",
      title: "操作成功",
      text: "Issue已关闭",
      type: "success",
    });

    issue.value.state = "closed";
  } catch (err) {
    console.error("关闭Issue失败:", err);
    data.$notify({
      group: "main",
      title: "操作失败",
      text: err.data?.description || "无法关闭Issue",
      type: "error",
    });
  } finally {
    isUpdatingState.value = false;
  }
}

// 确认重新打开Issue
async function confirmReopenIssue() {
  isUpdatingState.value = true;
  try {
    await useBaseFetch(`issues/${route.params.issue}`, {
      apiVersion: 3,
      method: "PATCH",
      body: { state: "open" },
    });

    data.$notify({
      group: "main",
      title: "操作成功",
      text: "Issue已重新打开",
      type: "success",
    });

    issue.value.state = "open";
  } catch (err) {
    console.error("重新打开Issue失败:", err);
    data.$notify({
      group: "main",
      title: "操作失败",
      text: err.data?.description || "无法重新打开Issue",
      type: "error",
    });
  } finally {
    isUpdatingState.value = false;
  }
}

// 打开标签管理模态框
async function openLabelModal() {
  try {
    const response = await useBaseFetch("issues/labels", {
      apiVersion: 3,
    });
    availableLabels.value = response.labels || [];
    showLabelModal.value = true;
  } catch (err) {
    console.error("加载标签失败:", err);
    data.$notify({
      group: "main",
      title: "加载失败",
      text: "无法加载可用标签",
      type: "error",
    });
  }
}

// 关闭标签管理模态框
function closeLabelModal() {
  showLabelModal.value = false;
}

// 检查标签是否已选中
function isLabelSelected(label) {
  return issue.value.labels?.some((l) => l.id === label.id) || false;
}

// 添加标签
async function addLabel(label) {
  if (isLabelSelected(label)) return;

  isUpdatingLabels.value = true;
  try {
    // 确保 labels 数组存在
    if (!issue.value.labels) {
      issue.value.labels = [];
    }

    const newLabels = [...issue.value.labels, label];
    const labelIds = newLabels.map((l) => l.id);

    await useBaseFetch(`issues/${route.params.issue}`, {
      apiVersion: 3,
      method: "PATCH",
      body: { labels: labelIds },
    });

    issue.value.labels = newLabels;

    data.$notify({
      group: "main",
      title: "操作成功",
      text: `已添加标签 "${label.name}"`,
      type: "success",
    });
  } catch (err) {
    console.error("添加标签失败:", err);
    data.$notify({
      group: "main",
      title: "操作失败",
      text: err.data?.description || "无法添加标签",
      type: "error",
    });
  } finally {
    isUpdatingLabels.value = false;
  }
}

// 移除标签
async function removeLabel(label) {
  isUpdatingLabels.value = true;
  try {
    const newLabels = (issue.value.labels || []).filter((l) => l.id !== label.id);
    const labelIds = newLabels.map((l) => l.id);

    await useBaseFetch(`issues/${route.params.issue}`, {
      apiVersion: 3,
      method: "PATCH",
      body: { labels: labelIds },
    });

    issue.value.labels = newLabels;

    data.$notify({
      group: "main",
      title: "操作成功",
      text: `已移除标签 "${label.name}"`,
      type: "success",
    });
  } catch (err) {
    console.error("移除标签失败:", err);
    data.$notify({
      group: "main",
      title: "操作失败",
      text: err.data?.description || "无法移除标签",
      type: "error",
    });
  } finally {
    isUpdatingLabels.value = false;
  }
}

// 编辑Issue (占位符)
// function editIssue() {
//   // TODO: 实现Issue编辑功能
//   data.$notify({
//     group: "main",
//     title: "功能开发中",
//     text: "Issue编辑功能正在开发中",
//     type: "info",
//   });
// }

// 编辑回复 (占位符)
function editComment(comment) {
  // TODO: 实现回复编辑功能
  data.$notify({
    group: "main",
    title: "功能开发中",
    text: "回复编辑功能正在开发中",
    type: "info",
  });
}

// 删除回复
function deleteComment(comment) {
  commentToDelete.value = comment;
  deleteCommentModal.value.show();
}

// 复制Issue链接
async function copyIssueLink() {
  try {
    const currentUrl = window.location.href;
    await navigator.clipboard.writeText(currentUrl);

    data.$notify({
      group: "main",
      title: "复制成功",
      text: "链接已复制到剪贴板",
      type: "success",
    });
  } catch (err) {
    console.error("复制链接失败:", err);
    data.$notify({
      group: "main",
      title: "复制失败",
      text: "无法复制链接到剪贴板",
      type: "error",
    });
  }
}

// 确认删除回复
async function confirmDeleteComment() {
  if (!commentToDelete.value) return;

  try {
    await useBaseFetch(`issues/comments/${commentToDelete.value.id}`, {
      apiVersion: 3,
      method: "DELETE",
    });

    data.$notify({
      group: "main",
      title: "删除成功",
      text: "回复已删除",
      type: "success",
    });

    await loadComments();
  } catch (err) {
    console.error("删除回复失败:", err);
    data.$notify({
      group: "main",
      title: "删除失败",
      text: err.data?.description || "无法删除回复",
      type: "error",
    });
  } finally {
    commentToDelete.value = null;
  }
}

// 图片上传处理函数
const onUploadHandler = async (file) => {
  const response = await useImageUpload(file, {
    context: "project",
    projectID: props.project.id,
  });
  return response.url;
};

// 格式化相对时间
const formatRelativeTime = (dateString) => {
  return dayjs(dateString).fromNow();
};

// 初始化
onMounted(() => {
  loadIssue();
});
</script>

<style scoped>
.issue-container {
  min-height: 100vh;
}

.loading-state {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  min-height: 400px;
  color: var(--color-text-secondary);
  gap: 16px;
}

.issue-detail-wrapper {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.issue-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 24px;
  gap: 20px;
  flex-wrap: wrap;
}

.issue-title-section {
  flex: 1;
  min-width: 0;
}

.issue-title {
  font-size: 2.5rem;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 12px 0;
  word-break: break-word;
  line-height: 1.2;
}

.issue-meta-line {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.issue-number {
  font-size: 1.25rem;
  color: var(--color-text-secondary);
  font-weight: 500;
}

.issue-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 20px;
  background-color: var(--color-success);
  color: white;
  font-weight: 500;
  font-size: 14px;
}

.issue-status.closed {
  background-color: var(--color-closed);
}

.status-icon {
  font-size: 16px;
  font-weight: bold;
}

.issue-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  flex-shrink: 0;
}

.labels-actions-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  padding: 0 4px;
  flex-wrap: wrap;
}

.labels-container {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  flex: 1;
  min-width: 0;
}

.issue-label {
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.issue-author-card {
  padding: 20px;
}

.author-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 20px;
  flex-wrap: wrap;
  gap: 16px;
}

.author-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.author-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
}

.author-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.author-name {
  color: var(--color-text-primary);
  text-decoration: none;
  font-weight: 600;
  font-size: 16px;
}

.author-name:hover {
  color: var(--color-highlight);
}

.author-time {
  color: var(--color-text-secondary);
  font-size: 14px;
}

.assignees {
  color: var(--color-text-secondary);
  font-size: 14px;
}

.assignees-label {
  margin-right: 8px;
}

.assignees-list {
  display: inline-flex;
  gap: 8px;
}

.assignee:not(:last-child)::after {
  content: ", ";
}

.assignee a {
  color: var(--color-highlight);
  text-decoration: none;
}

.issue-body {
  line-height: 1.6;
  color: var(--color-text-primary);
}

.comments-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.comments-header h3 {
  color: var(--color-text-primary);
  border-bottom: 1px solid var(--color-border);
  padding-bottom: 12px;
  margin: 0;
}

.comments-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.comment-card {
  padding: 0;
  overflow: hidden;
}

.comment-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  background: var(--color-bg-secondary);
  border-bottom: 1px solid var(--color-border);
}

.comment-author-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.comment-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
}

.comment-author-details {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.comment-author-name {
  color: var(--color-text-primary);
  text-decoration: none;
  font-weight: 600;
  font-size: 14px;
}

.comment-author-name:hover {
  color: var(--color-highlight);
}

.comment-floor {
  color: var(--color-text-secondary);
  font-size: 14px;
  font-weight: 500;
}

.comment-time {
  color: var(--color-text-secondary);
  font-size: 14px;
}

.comment-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: color 0.2s ease;
}

.action-btn:hover {
  color: var(--color-text-primary);
}

.action-btn.delete:hover {
  color: #dc3545;
}

.reply-reference {
  margin: 12px 20px;
  padding: 8px 12px;
  background: var(--color-reply-bg);
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.2s ease;
}

.reply-reference:hover {
  background: var(--color-reply-bg-hover);
}

.reply-info {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 14px;
  color: var(--color-text-secondary);
}

.reply-floor {
  font-weight: 500;
  color: var(--color-highlight);
}

.reply-deleted {
  font-weight: 500;
  color: var(--color-text-secondary);
  font-style: italic;
}

.deleted-reply {
  opacity: 0.7;
  cursor: default !important;
}

.deleted-reply:hover {
  background: var(--color-reply-bg) !important;
}

.comment-body {
  padding: 20px;
  line-height: 1.6;
  color: var(--color-text-primary);
}

.deleted-comment {
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

.comment-footer {
  padding: 12px 20px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.comment-card:hover .comment-footer {
  opacity: 1;
}

.reply-btn {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: color 0.2s ease;
}

.reply-btn:hover {
  color: var(--color-text-primary);
}

.create-comment-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.replying-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: var(--color-bg-secondary);
  color: var(--color-text-secondary);
  font-size: 14px;
}

.cancel-reply {
  background: transparent;
  border: none;
  color: var(--color-highlight);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background-color 0.2s ease;
}

.cancel-reply:hover {
  background: var(--color-bg-hover);
}

.comment-form {
  padding: 20px;
}

.comment-form h4 {
  margin: 0 0 16px 0;
  color: var(--color-text-primary);
}

.comment-form-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.login-prompt,
.closed-prompt {
  text-align: center;
  padding: 20px;
  color: var(--color-text-secondary);
}

.login-link {
  color: var(--color-highlight);
  text-decoration: none;
}

.login-link:hover {
  text-decoration: underline;
}

.not-found {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 400px;
}

.not-found-content {
  text-align: center;
  color: var(--color-text-secondary);
}

.not-found-content h2 {
  color: var(--color-text-primary);
  margin-bottom: 16px;
}

.comment-highlight {
  border-left: 3px solid var(--color-highlight);
  animation: highlightFade 3s ease-out;
}

@keyframes highlightFade {
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

/* 标签管理模态框样式 */
.label-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.label-modal {
  background: var(--color-bg-card);
  border-radius: 8px;
  width: 100%;
  max-width: 600px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-primary);
  font-size: 18px;
  font-weight: 600;
}

.close-button {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  font-size: 24px;
  cursor: pointer;
  padding: 0 8px;
  transition: color 0.2s ease;
}

.close-button:hover {
  color: var(--color-text-primary);
}

.modal-content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.current-labels h4,
.available-labels h4 {
  margin: 0 0 12px 0;
  color: var(--color-text-primary);
  font-size: 16px;
  font-weight: 600;
}

.labels-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.current-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.remove-label-btn {
  background: transparent;
  border: none;
  color: white;
  font-size: 16px;
  cursor: pointer;
  padding: 0;
  margin-left: 4px;
  opacity: 0.8;
  transition: opacity 0.2s ease;
}

.remove-label-btn:hover {
  opacity: 1;
}

.available-label {
  padding: 6px 12px;
  border: none;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  cursor: pointer;
  transition:
    opacity 0.2s ease,
    transform 0.1s ease;
}

.available-label:hover:not(.disabled) {
  transform: translateY(-1px);
  opacity: 0.9;
}

.available-label.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.no-labels {
  color: var(--color-text-secondary);
  font-style: italic;
  padding: 12px 0;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 20px;
  border-top: 1px solid var(--color-border);
}

@media (max-width: 768px) {
  .issue-detail-wrapper {
    padding: 0 10px;
  }

  .issue-header {
    flex-direction: column;
    align-items: stretch;
    padding: 16px;
  }

  .issue-title {
    font-size: 2rem;
  }

  .issue-actions {
    justify-content: flex-start;
  }

  .author-header {
    flex-direction: column;
    align-items: stretch;
  }

  .comment-header {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }

  .comment-author-details {
    flex-wrap: wrap;
  }
}
</style>
