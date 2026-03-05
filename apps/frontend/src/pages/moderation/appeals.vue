<template>
  <div>
    <!-- 审核弹窗 -->
    <NewModal ref="reviewModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">审核申诉</div>
      </template>
      <div class="review-content">
        <div v-if="currentAppeal" class="appeal-summary">
          <div class="summary-row">
            <span class="label">申诉用户：</span>
            <span>{{ getUserName(currentAppeal.user_id) }}</span>
          </div>
          <div class="summary-row">
            <span class="label">封禁类型：</span>
            <span
              class="ban-type-badge"
              :class="`type-${getBanInfo(currentAppeal.ban_id)?.ban_type}`"
            >
              {{ getBanTypeName(getBanInfo(currentAppeal.ban_id)?.ban_type) }}
            </span>
          </div>
          <div class="summary-row">
            <span class="label">申诉理由：</span>
            <span>{{ currentAppeal.reason }}</span>
          </div>
        </div>

        <div class="review-form">
          <label class="form-label">
            <span>审核决定</span>
            <span class="required">*</span>
          </label>
          <div class="decision-buttons">
            <button
              class="decision-btn approve"
              :class="{ active: reviewDecision === 'approved' }"
              @click="reviewDecision = 'approved'"
            >
              <CheckIcon aria-hidden="true" />
              批准申诉（解除封禁）
            </button>
            <button
              class="decision-btn reject"
              :class="{ active: reviewDecision === 'rejected' }"
              @click="reviewDecision = 'rejected'"
            >
              <CrossIcon aria-hidden="true" />
              拒绝申诉
            </button>
          </div>

          <label class="form-label">
            <span>审核备注</span>
            <span class="optional">（可选，将发送到申诉线程）</span>
          </label>
          <textarea
            v-model="reviewNotes"
            class="review-textarea"
            placeholder="请输入审核备注..."
            rows="4"
          ></textarea>
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled :color="reviewDecision === 'approved' ? 'primary' : 'danger'">
          <button :disabled="!reviewDecision || submittingReview" @click="submitReview">
            <CheckIcon v-if="reviewDecision === 'approved'" aria-hidden="true" />
            <CrossIcon v-else aria-hidden="true" />
            {{ reviewDecision === "approved" ? "确认批准" : "确认拒绝" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="reviewModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="header-section">
        <h2>封禁申诉管理</h2>
        <p class="description">管理用户提交的封禁申诉请求</p>
      </div>

      <!-- 筛选栏 -->
      <div class="filter-section">
        <Chips v-model="statusFilter" :items="statusOptions" :format-label="formatStatusLabel" />
        <div class="filter-info">
          <span class="filter-status-label" :class="`filter-${statusFilter}`">
            {{ formatStatusLabel(statusFilter) }}: {{ totalCount }} 个
          </span>
        </div>
      </div>

      <!-- 加载中 -->
      <div v-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>

      <!-- 申诉列表 -->
      <div v-else-if="appeals.length > 0" class="appeals-container">
        <div class="appeals-list">
          <div v-for="appeal in appeals" :key="appeal.id" class="appeal-item">
            <div class="appeal-main">
              <!-- 用户和封禁信息 -->
              <div class="appeal-header">
                <div class="user-info">
                  <nuxt-link :to="`/user/${getUserName(appeal.user_id)}`" class="user-link">
                    <Avatar
                      :src="getUserAvatar(appeal.user_id)"
                      :alt="getUserName(appeal.user_id)"
                      size="sm"
                      circle
                    />
                    <span class="username">{{ getUserName(appeal.user_id) }}</span>
                  </nuxt-link>
                </div>
                <div class="ban-info">
                  <span
                    class="ban-type-badge"
                    :class="`type-${getBanInfo(appeal.ban_id)?.ban_type}`"
                  >
                    {{ getBanTypeName(getBanInfo(appeal.ban_id)?.ban_type) }}
                  </span>
                  <span class="status-badge" :class="`status-${appeal.status}`">
                    {{ getStatusLabel(appeal.status) }}
                  </span>
                </div>
              </div>

              <!-- 封禁原因 -->
              <div v-if="getBanInfo(appeal.ban_id)" class="ban-reason">
                <span class="label">封禁原因：</span>
                <span>{{ getBanInfo(appeal.ban_id)?.reason }}</span>
              </div>

              <!-- 申诉理由 -->
              <div class="appeal-reason">
                <span class="label">申诉理由：</span>
                <span>{{ appeal.reason }}</span>
              </div>

              <!-- 元信息 -->
              <div class="appeal-meta">
                <div class="meta-item">
                  <span class="meta-label">申诉时间：</span>
                  <span>{{ formatDateTime(appeal.created_at) }}</span>
                </div>
                <div v-if="appeal.reviewed_by_username" class="meta-item">
                  <span class="meta-label">审核人：</span>
                  <span>{{ appeal.reviewed_by_username }}</span>
                </div>
                <div v-if="appeal.reviewed_at" class="meta-item">
                  <span class="meta-label">审核时间：</span>
                  <span>{{ formatDateTime(appeal.reviewed_at) }}</span>
                </div>
              </div>

              <!-- 审核备注 -->
              <div v-if="appeal.review_notes" class="review-notes">
                <span class="label">审核备注：</span>
                <span>{{ appeal.review_notes }}</span>
              </div>

              <!-- 对话线程 -->
              <div v-if="appeal.thread_id" class="thread-section">
                <button class="btn btn-secondary thread-toggle" @click="toggleThread(appeal)">
                  <MessageIcon aria-hidden="true" />
                  {{ expandedThreads[appeal.id] ? "收起对话" : "查看对话" }}
                </button>

                <div
                  v-if="expandedThreads[appeal.id] && appealThreads[appeal.id]"
                  class="thread-container"
                >
                  <ConversationThread
                    :thread="appealThreads[appeal.id]"
                    :auth="auth"
                    :current-member="null"
                    :project="null"
                    :report="null"
                    @update-thread="refreshThread(appeal)"
                  />
                </div>
              </div>
            </div>

            <!-- 操作按钮 -->
            <div v-if="appeal.status === 'pending'" class="appeal-actions">
              <button
                class="btn btn-primary"
                :disabled="processingAppeals.includes(appeal.id)"
                @click="openReviewModal(appeal)"
              >
                <EditIcon aria-hidden="true" />
                审核
              </button>
            </div>
          </div>
        </div>

        <!-- 分页 -->
        <div v-if="totalPages > 1" class="pagination">
          <button
            class="page-btn"
            :disabled="currentPage === 1"
            @click="changePage(currentPage - 1)"
          >
            <ChevronLeftIcon aria-hidden="true" />
            上一页
          </button>

          <div class="page-numbers">
            <button
              v-for="page in displayedPages"
              :key="page"
              class="page-number"
              :class="{ active: page === currentPage }"
              :disabled="page === '...'"
              @click="page !== '...' && changePage(page)"
            >
              {{ page }}
            </button>
          </div>

          <button
            class="page-btn"
            :disabled="currentPage === totalPages"
            @click="changePage(currentPage + 1)"
          >
            下一页
            <ChevronRightIcon aria-hidden="true" />
          </button>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            statusFilter === "all" ? "暂无申诉记录" : `暂无${formatStatusLabel(statusFilter)}的申诉`
          }}
        </p>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from "vue";
import { useRouter } from "vue-router";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import Avatar from "~/components/ui/Avatar.vue";
import Chips from "~/components/ui/Chips.vue";
import ConversationThread from "~/components/ui/thread/ConversationThread.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import EditIcon from "~/assets/images/utils/edit.svg?component";
import MessageIcon from "~/assets/images/utils/message.svg?component";
import ChevronLeftIcon from "~/assets/images/utils/chevron-left.svg?component";
import ChevronRightIcon from "~/assets/images/utils/chevron-right.svg?component";

const router = useRouter();
const auth = await useAuth();
const app = useNuxtApp();

useHead({
  title: "封禁申诉管理 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

// 常量
const ITEMS_PER_PAGE = 20;

// 响应式状态
const loading = ref(true);
const appeals = ref([]);
const processingAppeals = ref([]);
const totalCount = ref(0);
const currentPage = ref(1);
const statusFilter = ref("pending");
const statusOptions = ["all", "pending", "approved", "rejected"];

// 缓存用户和封禁信息
const usersCache = ref({});
const bansCache = ref({});

// 对话线程
const expandedThreads = ref({});
const appealThreads = ref({});

// 审核相关
const reviewModal = ref(null);
const currentAppeal = ref(null);
const reviewDecision = ref("");
const reviewNotes = ref("");
const submittingReview = ref(false);

// 计算属性
const totalPages = computed(() => Math.ceil(totalCount.value / ITEMS_PER_PAGE));

const displayedPages = computed(() => {
  const pages = [];
  const total = totalPages.value;
  const current = currentPage.value;

  if (total <= 7) {
    for (let i = 1; i <= total; i++) {
      pages.push(i);
    }
  } else if (current <= 3) {
    for (let i = 1; i <= 5; i++) {
      pages.push(i);
    }
    pages.push("...");
    pages.push(total);
  } else if (current >= total - 2) {
    pages.push(1);
    pages.push("...");
    for (let i = total - 4; i <= total; i++) {
      pages.push(i);
    }
  } else {
    pages.push(1);
    pages.push("...");
    for (let i = current - 1; i <= current + 1; i++) {
      pages.push(i);
    }
    pages.push("...");
    pages.push(total);
  }

  return pages;
});

// 辅助函数
const formatStatusLabel = (status) => {
  const labels = {
    all: "全部",
    pending: "待审核",
    approved: "已批准",
    rejected: "已拒绝",
  };
  return labels[status] || status;
};

const getStatusLabel = (status) => {
  const labels = {
    pending: "待审核",
    approved: "已批准",
    rejected: "已拒绝",
  };
  return labels[status] || status;
};

const getBanTypeName = (type) => {
  const types = {
    global: "全局封禁",
    resource: "资源封禁",
    forum: "论坛封禁",
  };
  return types[type] || type || "未知";
};

const formatDateTime = (date) => {
  return app.$dayjs(date).format("YYYY-MM-DD HH:mm");
};

const getUserName = (userId) => {
  return usersCache.value[userId]?.username || `用户#${userId}`;
};

const getUserAvatar = (userId) => {
  return usersCache.value[userId]?.avatar_url;
};

const getBanInfo = (banId) => {
  return bansCache.value[banId];
};

// 获取申诉列表
const fetchAppeals = async () => {
  loading.value = true;
  try {
    const params = {
      page: currentPage.value,
      limit: ITEMS_PER_PAGE,
    };

    if (statusFilter.value !== "all") {
      params.status = statusFilter.value;
    }

    const response = await useBaseFetch("bans/appeals", {
      method: "GET",
      params,
    });

    if (response) {
      appeals.value = response.appeals || [];
      totalCount.value = response.total || 0;

      // 加载用户和封禁信息
      await loadRelatedData(appeals.value);
    }
  } catch (error) {
    console.error("获取申诉列表失败:", error);
    app.$notify({
      group: "main",
      title: "错误",
      text: "获取申诉列表失败",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

// 加载关联数据（用户信息和封禁信息）
const loadRelatedData = async (appealsList) => {
  const userIds = [...new Set(appealsList.map((a) => a.user_id))];
  const banIds = [...new Set(appealsList.map((a) => a.ban_id))];

  // 批量获取用户信息
  const missingUserIds = userIds.filter((id) => !usersCache.value[id]);
  if (missingUserIds.length > 0) {
    try {
      const users = await useBaseFetch("users", {
        method: "GET",
        params: { ids: JSON.stringify(missingUserIds) },
      });
      if (users) {
        for (const user of users) {
          usersCache.value[user.id] = user;
        }
      }
    } catch (err) {
      console.error("获取用户信息失败:", err);
    }
  }

  // 批量获取封禁信息，避免 N+1 请求
  const missingBanIds = banIds.filter((id) => !bansCache.value[id]);
  if (missingBanIds.length > 0) {
    try {
      const bans = await useBaseFetch("bans/batch", {
        params: { ids: JSON.stringify(missingBanIds) },
      });
      if (bans && Array.isArray(bans)) {
        for (const ban of bans) {
          bansCache.value[ban.id] = ban;
        }
      }
    } catch (err) {
      console.error("批量获取封禁信息失败:", err);
    }
  }
};

// 切换页码
const changePage = (page) => {
  if (page < 1 || page > totalPages.value) return;
  currentPage.value = page;
  fetchAppeals();
  if (import.meta.client) window.scrollTo({ top: 0, behavior: "smooth" });
};

// 切换对话线程显示
const toggleThread = async (appeal) => {
  const appealId = appeal.id;

  if (expandedThreads.value[appealId]) {
    expandedThreads.value[appealId] = false;
  } else {
    expandedThreads.value[appealId] = true;

    if (!appealThreads.value[appealId]) {
      try {
        const thread = await useBaseFetch(`thread/${appeal.thread_id}`);
        appealThreads.value[appealId] = thread;
      } catch (err) {
        console.error("加载对话失败:", err);
        app.$notify({
          group: "main",
          title: "错误",
          text: "加载对话失败",
          type: "error",
        });
        expandedThreads.value[appealId] = false;
      }
    }
  }
};

// 刷新对话
const refreshThread = async (appeal) => {
  if (appeal.thread_id) {
    try {
      const thread = await useBaseFetch(`thread/${appeal.thread_id}`);
      appealThreads.value[appeal.id] = thread;
    } catch (err) {
      console.error("刷新对话失败:", err);
    }
  }
};

// 打开审核弹窗
const openReviewModal = (appeal) => {
  currentAppeal.value = appeal;
  reviewDecision.value = "";
  reviewNotes.value = "";
  reviewModal.value?.show();
};

// 提交审核
const submitReview = async () => {
  if (!currentAppeal.value || !reviewDecision.value || submittingReview.value) return;

  submittingReview.value = true;
  processingAppeals.value.push(currentAppeal.value.id);

  try {
    await useBaseFetch(`bans/appeals/${currentAppeal.value.id}`, {
      method: "PATCH",
      body: {
        status: reviewDecision.value,
        review_notes: reviewNotes.value || undefined,
      },
    });

    reviewModal.value?.hide();

    app.$notify({
      group: "main",
      title: "成功",
      text: reviewDecision.value === "approved" ? "已批准申诉，封禁已解除" : "已拒绝申诉",
      type: "success",
    });

    // 刷新列表
    await fetchAppeals();
  } catch (error) {
    console.error("审核失败:", error);
    app.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "审核失败",
      type: "error",
    });
  } finally {
    submittingReview.value = false;
    processingAppeals.value = processingAppeals.value.filter(
      (id) => id !== currentAppeal.value?.id,
    );
    currentAppeal.value = null;
  }
};

// 监听筛选条件变化
watch(statusFilter, () => {
  currentPage.value = 1;
  fetchAppeals();
});

// 检查权限并加载数据
onMounted(() => {
  if (
    !auth.value?.user ||
    (auth.value.user.role !== "admin" && auth.value.user.role !== "moderator")
  ) {
    router.push("/");
    app.$notify({
      group: "main",
      title: "权限不足",
      text: "您没有权限访问此页面",
      type: "error",
    });
  } else {
    fetchAppeals();
  }
});
</script>

<style scoped lang="scss">
.header-section {
  margin-bottom: 1.5rem;

  h2 {
    margin: 0 0 0.5rem 0;
    color: var(--color-heading);
  }

  .description {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: 0.95rem;
  }
}

.filter-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.5rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--color-divider);

  .filter-info {
    color: var(--color-text-secondary);
    font-size: 0.9rem;

    .filter-status-label {
      font-weight: 600;

      &.filter-all {
        color: var(--color-text);
      }

      &.filter-pending {
        color: rgb(217, 119, 6);
      }

      &.filter-approved {
        color: rgb(21, 128, 61);
      }

      &.filter-rejected {
        color: rgb(185, 28, 28);
      }
    }
  }
}

.loading-section,
.empty-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem;
  gap: 1rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
  }

  p {
    margin: 0;
    font-size: 0.95rem;
  }
}

.appeals-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.appeals-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.appeal-item {
  padding: 1.25rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-divider);
  transition: all 0.2s;

  &:hover {
    border-color: var(--color-primary);
  }
}

.appeal-main {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.appeal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 1rem;
}

.user-info {
  .user-link {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-decoration: none;
    color: var(--color-text);

    &:hover {
      color: var(--color-primary);
    }

    .username {
      font-weight: 600;
    }
  }
}

.ban-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.ban-type-badge {
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
  font-weight: 500;

  &.type-global {
    background: rgba(239, 68, 68, 0.15);
    color: rgb(185, 28, 28);
  }

  &.type-resource {
    background: rgba(251, 191, 36, 0.15);
    color: rgb(217, 119, 6);
  }

  &.type-forum {
    background: rgba(59, 130, 246, 0.15);
    color: rgb(37, 99, 235);
  }
}

.status-badge {
  padding: 0.25rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  font-weight: 600;

  &.status-pending {
    background: rgba(251, 191, 36, 0.15);
    color: rgb(217, 119, 6);
    border: 1px solid rgba(251, 191, 36, 0.3);
  }

  &.status-approved {
    background: rgba(34, 197, 94, 0.15);
    color: rgb(21, 128, 61);
    border: 1px solid rgba(34, 197, 94, 0.3);
  }

  &.status-rejected {
    background: rgba(239, 68, 68, 0.15);
    color: rgb(185, 28, 28);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }
}

.ban-reason,
.appeal-reason,
.review-notes {
  padding: 0.75rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  font-size: 0.9rem;

  .label {
    font-weight: 500;
    margin-right: 0.5rem;
    color: var(--color-text-secondary);
  }
}

.appeal-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  align-items: center;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.9rem;

  .meta-label {
    color: var(--color-text-secondary);
  }
}

.thread-section {
  margin-top: 0.5rem;

  .thread-toggle {
    margin-bottom: 1rem;
  }

  .thread-container {
    padding: 1rem;
    background: var(--color-bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-divider);
  }
}

.appeal-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: var(--radius-md);
  border: none;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  transition: all 0.2s;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &.btn-primary {
    background: var(--color-primary);
    color: white;

    &:hover:not(:disabled) {
      background: var(--color-primary-dark);
    }
  }

  &.btn-secondary {
    background: var(--color-button-bg);
    color: var(--color-text);

    &:hover:not(:disabled) {
      background: var(--color-raised-bg);
    }
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

// 分页样式
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--color-divider);
}

.page-btn {
  padding: 0.5rem 1rem;
  background: var(--color-button-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  color: var(--color-text);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: var(--color-raised-bg);
    border-color: var(--color-primary);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

.page-numbers {
  display: flex;
  gap: 0.5rem;
}

.page-number {
  min-width: 2.5rem;
  padding: 0.5rem;
  background: var(--color-button-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
  text-align: center;

  &:hover:not(:disabled):not(.active) {
    background: var(--color-raised-bg);
    border-color: var(--color-primary);
  }

  &.active {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  &:disabled {
    cursor: default;
    background: transparent;
    border: none;
  }
}

// 审核弹窗样式
.review-content {
  padding: 1rem;
}

.appeal-summary {
  padding: 1rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  margin-bottom: 1.5rem;

  .summary-row {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.5rem;

    &:last-child {
      margin-bottom: 0;
    }

    .label {
      font-weight: 500;
      color: var(--color-text-secondary);
      min-width: 80px;
    }
  }
}

.review-form {
  .form-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
    font-weight: 500;

    .required {
      color: var(--color-danger);
    }

    .optional {
      font-weight: normal;
      color: var(--color-text-secondary);
      font-size: 0.85rem;
    }
  }
}

.decision-buttons {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.decision-btn {
  flex: 1;
  padding: 0.75rem 1rem;
  border: 2px solid var(--color-divider);
  border-radius: var(--radius-md);
  background: var(--color-raised-bg);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  transition: all 0.2s;

  &.approve {
    &:hover,
    &.active {
      border-color: rgb(21, 128, 61);
      background: rgba(34, 197, 94, 0.1);
      color: rgb(21, 128, 61);
    }
  }

  &.reject {
    &:hover,
    &.active {
      border-color: rgb(185, 28, 28);
      background: rgba(239, 68, 68, 0.1);
      color: rgb(185, 28, 28);
    }
  }

  svg {
    width: 1.25rem;
    height: 1.25rem;
  }
}

.review-textarea {
  width: 100%;
  min-height: 100px;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  font-size: 0.95rem;
  font-family: inherit;
  resize: vertical;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  padding: 1rem;
  padding-top: 0;

  button {
    display: flex;
    align-items: center;
    gap: 0.25rem;

    svg {
      width: 1rem;
      height: 1rem;
    }
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.animate-spin {
  animation: spin 1s linear infinite;
}
</style>
