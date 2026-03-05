<template>
  <div>
    <!-- 审核弹窗 -->
    <NewModal ref="reviewModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">审核申请</div>
      </template>
      <div class="review-content">
        <div v-if="currentApplication" class="application-summary">
          <div class="summary-row">
            <span class="label">申请用户：</span>
            <span>{{ currentApplication.username }}</span>
          </div>
          <div class="summary-row">
            <span class="label">真实姓名：</span>
            <span>{{ currentApplication.real_name }}</span>
          </div>
          <div class="summary-row">
            <span class="label">联系方式：</span>
            <span>{{ currentApplication.contact_info }}</span>
          </div>
          <div v-if="currentApplication.application_reason" class="summary-row">
            <span class="label">申请理由：</span>
            <span>{{ currentApplication.application_reason }}</span>
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
              批准申请
            </button>
            <button
              class="decision-btn reject"
              :class="{ active: reviewDecision === 'rejected' }"
              @click="reviewDecision = 'rejected'"
            >
              <CrossIcon aria-hidden="true" />
              拒绝申请
            </button>
          </div>

          <label class="form-label">
            <span>审核备注</span>
            <span class="optional">（可选，将发送到申请线程）</span>
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
        <h2>高级创作者申请管理</h2>
        <p class="description">管理用户提交的高级创作者申请</p>
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

      <!-- 申请列表 -->
      <div v-else-if="applications.length > 0" class="applications-container">
        <div class="applications-list">
          <div v-for="appItem in applications" :key="appItem.id" class="application-item">
            <div class="application-main">
              <!-- 用户信息 -->
              <div class="application-header">
                <div class="user-info">
                  <nuxt-link :to="`/user/${appItem.username}`" class="user-link">
                    <span class="username">{{ appItem.username }}</span>
                  </nuxt-link>
                </div>
                <div class="status-info">
                  <span class="status-badge" :class="`status-${appItem.status}`">
                    {{ getStatusLabel(appItem.status) }}
                  </span>
                </div>
              </div>

              <!-- 申请信息 -->
              <div class="info-grid">
                <div class="info-item">
                  <span class="label">真实姓名：</span>
                  <span>{{ appItem.real_name }}</span>
                </div>
                <div class="info-item">
                  <span class="label">联系方式：</span>
                  <span>{{ appItem.contact_info }}</span>
                </div>
                <div v-if="appItem.id_card_number" class="info-item">
                  <span class="label">身份证号：</span>
                  <span class="id-card">{{ maskIdCard(appItem.id_card_number) }}</span>
                </div>
                <div v-if="appItem.portfolio_links" class="info-item full-width">
                  <span class="label">作品链接：</span>
                  <a :href="appItem.portfolio_links" target="_blank" class="portfolio-link">
                    {{ appItem.portfolio_links }}
                  </a>
                </div>
              </div>

              <!-- 申请理由 -->
              <div v-if="appItem.application_reason" class="application-reason">
                <span class="label">申请理由：</span>
                <span>{{ appItem.application_reason }}</span>
              </div>

              <!-- 元信息 -->
              <div class="application-meta">
                <div class="meta-item">
                  <span class="meta-label">申请时间：</span>
                  <span>{{ formatDateTime(appItem.created_at) }}</span>
                </div>
                <div v-if="appItem.reviewed_at" class="meta-item">
                  <span class="meta-label">审核时间：</span>
                  <span>{{ formatDateTime(appItem.reviewed_at) }}</span>
                </div>
              </div>

              <!-- 审核备注 -->
              <div v-if="appItem.review_note" class="review-notes">
                <span class="label">审核备注：</span>
                <span>{{ appItem.review_note }}</span>
              </div>

              <!-- 对话线程 -->
              <div v-if="appItem.thread_id" class="thread-section">
                <button class="btn btn-secondary thread-toggle" @click="toggleThread(appItem)">
                  <MessageIcon aria-hidden="true" />
                  {{ expandedThreads[appItem.id] ? "收起对话" : "查看对话" }}
                </button>

                <div
                  v-if="expandedThreads[appItem.id] && applicationThreads[appItem.id]"
                  class="thread-container"
                >
                  <ConversationThread
                    :thread="applicationThreads[appItem.id]"
                    :auth="auth"
                    :current-member="null"
                    :project="null"
                    :report="null"
                    @update-thread="refreshThread(appItem)"
                  />
                </div>
              </div>
            </div>

            <!-- 操作按钮 -->
            <div v-if="appItem.status === 'pending'" class="application-actions">
              <button
                class="btn btn-primary"
                :disabled="processingApplications.includes(appItem.id)"
                @click="openReviewModal(appItem)"
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
            statusFilter === "all" ? "暂无申请记录" : `暂无${formatStatusLabel(statusFilter)}的申请`
          }}
        </p>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, watch } from "vue";
import { NewModal, ButtonStyled } from "@modrinth/ui";
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

const auth = await useAuth();
const nuxtApp = useNuxtApp();

// 权限检查 - 只有超级管理员可以访问
if (!auth.value?.user || auth.value.user.role !== "admin") {
  throw createError({
    statusCode: 403,
    statusMessage: "权限不足",
    message: "只有超级管理员可以访问此页面",
  });
}

useHead({
  title: "高级创作者申请管理 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

// 常量
const ITEMS_PER_PAGE = 20;

// 响应式状态
const currentPage = ref(1);
const statusFilter = ref("pending");
const statusOptions = ["all", "pending", "approved", "rejected"];
const processingApplications = ref([]);

// 对话线程
const expandedThreads = ref({});
const applicationThreads = ref({});

// 审核相关
const reviewModal = ref(null);
const currentApplication = ref(null);
const reviewDecision = ref("");
const reviewNotes = ref("");
const submittingReview = ref(false);

// SSR 数据获取 - 使用 useAsyncData 避免数据滞后
const {
  data: applicationsData,
  pending: loading,
  refresh: refreshApplications,
} = await useAsyncData(
  "creator-applications",
  async () => {
    const query = {
      limit: ITEMS_PER_PAGE,
      offset: (currentPage.value - 1) * ITEMS_PER_PAGE,
      status: statusFilter.value !== "all" ? statusFilter.value : undefined,
    };
    return await useBaseFetch("creator/applications", { method: "GET", query });
  },
  { watch: [currentPage, statusFilter] },
);

// 计算属性
const applications = computed(() => applicationsData.value?.applications || []);
const totalCount = computed(() => applicationsData.value?.total || 0);
const totalPages = computed(() => Math.ceil(totalCount.value / ITEMS_PER_PAGE));

const displayedPages = computed(() => {
  const pages = [];
  const total = totalPages.value;
  const current = currentPage.value;
  if (total <= 7) {
    for (let i = 1; i <= total; i++) pages.push(i);
  } else if (current <= 3) {
    for (let i = 1; i <= 5; i++) pages.push(i);
    pages.push("...");
    pages.push(total);
  } else if (current >= total - 2) {
    pages.push(1);
    pages.push("...");
    for (let i = total - 4; i <= total; i++) pages.push(i);
  } else {
    pages.push(1);
    pages.push("...");
    for (let i = current - 1; i <= current + 1; i++) pages.push(i);
    pages.push("...");
    pages.push(total);
  }
  return pages;
});

// 辅助函数
const formatStatusLabel = (status) => {
  const labels = { all: "全部", pending: "待审核", approved: "已批准", rejected: "已拒绝" };
  return labels[status] || status;
};

const getStatusLabel = (status) => {
  const labels = { pending: "待审核", approved: "已批准", rejected: "已拒绝" };
  return labels[status] || status;
};

const formatDateTime = (date) => nuxtApp.$dayjs(date).format("YYYY-MM-DD HH:mm");

const maskIdCard = (idCard) => {
  if (!idCard || idCard.length < 8) return idCard;
  return idCard.substring(0, 4) + "**********" + idCard.substring(idCard.length - 4);
};

// 切换页码
const changePage = (page) => {
  if (page < 1 || page > totalPages.value) return;
  currentPage.value = page;
  // useAsyncData 会自动响应 currentPage 变化
  if (import.meta.client) window.scrollTo({ top: 0, behavior: "smooth" });
};

// 切换对话线程显示
const toggleThread = async (appItem) => {
  const appId = appItem.id;
  if (expandedThreads.value[appId]) {
    expandedThreads.value[appId] = false;
  } else {
    expandedThreads.value[appId] = true;
    if (!applicationThreads.value[appId]) {
      try {
        const thread = await useBaseFetch(`thread/${appItem.thread_id}`);
        applicationThreads.value[appId] = thread;
      } catch (err) {
        console.error("加载对话失败:", err);
        nuxtApp.$notify({ group: "main", title: "错误", text: "加载对话失败", type: "error" });
        expandedThreads.value[appId] = false;
      }
    }
  }
};

// 刷新对话
const refreshThread = async (appItem) => {
  if (appItem.thread_id) {
    try {
      const thread = await useBaseFetch(`thread/${appItem.thread_id}`);
      applicationThreads.value[appItem.id] = thread;
    } catch (err) {
      console.error("刷新对话失败:", err);
    }
  }
};

// 打开审核弹窗
const openReviewModal = (appItem) => {
  currentApplication.value = appItem;
  reviewDecision.value = "";
  reviewNotes.value = "";
  reviewModal.value?.show();
};

// 提交审核
const submitReview = async () => {
  if (!currentApplication.value || !reviewDecision.value || submittingReview.value) return;
  submittingReview.value = true;
  processingApplications.value.push(currentApplication.value.id);
  try {
    const action = reviewDecision.value === "approved" ? "approve" : "reject";
    await useBaseFetch(`creator/applications/${currentApplication.value.id}/${action}`, {
      method: "POST",
      body: { review_note: reviewNotes.value || undefined },
    });
    reviewModal.value?.hide();
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: reviewDecision.value === "approved" ? "已批准申请" : "已拒绝申请",
      type: "success",
    });
    await refreshApplications();
  } catch (error) {
    console.error("审核失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "审核失败",
      type: "error",
    });
  } finally {
    submittingReview.value = false;
    processingApplications.value = processingApplications.value.filter(
      (id) => id !== currentApplication.value?.id,
    );
    currentApplication.value = null;
  }
};

// 监听筛选条件变化 - useAsyncData 已通过 watch 选项自动处理
watch(statusFilter, () => {
  currentPage.value = 1;
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

.applications-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
.applications-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.application-item {
  padding: 1.25rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-divider);
  transition: all 0.2s;
  &:hover {
    border-color: var(--color-primary);
  }
}

.application-main {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.application-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 1rem;
}

.user-info .user-link {
  text-decoration: none;
  color: var(--color-text);
  font-weight: 600;
  &:hover {
    color: var(--color-primary);
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

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 0.75rem;
  .info-item {
    font-size: 0.9rem;
    .label {
      font-weight: 500;
      color: var(--color-text-secondary);
      margin-right: 0.25rem;
    }
    &.full-width {
      grid-column: 1 / -1;
    }
  }
  .id-card {
    font-family: monospace;
  }
  .portfolio-link {
    color: var(--color-primary);
    text-decoration: none;
    &:hover {
      text-decoration: underline;
    }
  }
}

.application-reason,
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

.application-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  align-items: center;
  .meta-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.9rem;
    .meta-label {
      color: var(--color-text-secondary);
    }
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

.application-actions {
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
    background: var(--color-brand);
    color: var(--color-brand-inverted);
    &:hover:not(:disabled) {
      filter: brightness(0.85);
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

.review-content {
  padding: 1rem;
}

.application-summary {
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
  color: var(--color-text);
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
