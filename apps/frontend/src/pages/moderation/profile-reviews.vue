<template>
  <div>
    <ProfileReviewModal ref="reviewModal" @reviewed="fetchReviews" />

    <!-- 一键通过确认弹窗 -->
    <NewModal ref="approveAllModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">确认批量审批</div>
      </template>
      <div class="approve-all-content">
        <p>
          确定要一键通过全部 <strong>{{ pendingCount }}</strong> 条待审核记录吗？此操作不可撤销。
        </p>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="primary">
          <button :disabled="approveAllLoading" @click="doApproveAll">
            <CheckIcon aria-hidden="true" />
            {{ approveAllLoading ? "处理中..." : "确认全部通过" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="approveAllLoading" @click="approveAllModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="header-section">
        <h2>资料审核管理</h2>
        <p class="description">管理用户触发风控的资料修改请求</p>
      </div>

      <!-- 筛选栏 -->
      <div class="filter-section">
        <Chips v-model="statusFilter" :items="statusOptions" :format-label="formatStatusLabel" />
        <ButtonStyled
          v-if="['all', 'pending'].includes(statusFilter) && pendingCount > 0"
          color="primary"
        >
          <button :disabled="approveAllLoading" @click="approveAll">
            <CheckIcon aria-hidden="true" />
            {{ approveAllLoading ? "处理中..." : `一键全部通过 (${pendingCount})` }}
          </button>
        </ButtonStyled>
      </div>

      <!-- 加载中 -->
      <div v-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>

      <!-- 审核列表 -->
      <div v-else-if="reviews.length > 0" class="reviews-container">
        <div class="reviews-list">
          <div v-for="review in reviews" :key="review.id" class="review-item">
            <div class="review-main">
              <div class="review-header">
                <div class="user-info">
                  <nuxt-link :to="`/user/${review.username}`" class="user-link">
                    <Avatar :src="review.avatar_url" :alt="review.username" size="sm" circle />
                    <span class="username">{{ review.username }}</span>
                  </nuxt-link>
                </div>
                <div class="review-badges">
                  <span class="type-badge" :class="`type-${review.review_type}`">
                    {{ getTypeName(review.review_type) }}
                  </span>
                  <span class="status-badge" :class="`status-${review.status}`">
                    {{ formatStatusLabel(review.status) }}
                  </span>
                </div>
              </div>

              <!-- 内容预览 -->
              <div class="review-preview">
                <template v-if="review.review_type === 'avatar'">
                  <div class="avatar-preview">
                    <Avatar
                      :src="getAvatarUrl(review.old_value, 'avatar_url')"
                      size="xs"
                      circle
                      alt="旧"
                    />
                    <span class="preview-arrow">&rarr;</span>
                    <Avatar
                      :src="getAvatarUrl(review.new_value, 'avatar_url')"
                      size="xs"
                      circle
                      alt="新"
                    />
                  </div>
                </template>
                <template v-else>
                  <div class="text-preview">
                    <span class="old-text">{{ truncate(review.old_value || "(空)", 30) }}</span>
                    <span class="preview-arrow">&rarr;</span>
                    <span class="new-text">{{ truncate(review.new_value, 30) }}</span>
                  </div>
                </template>
              </div>

              <div class="review-meta">
                <span class="risk-labels">{{ review.risk_labels }}</span>
                <span class="meta-time">{{ formatDateTime(review.created_at) }}</span>
              </div>

              <div v-if="review.review_notes" class="review-notes-display">
                <span class="label">审核备注：</span>
                <span>{{ review.review_notes }}</span>
              </div>
            </div>

            <div v-if="review.status === 'pending'" class="review-actions">
              <button class="btn btn-primary" @click="reviewModal?.open(review)">
                <EditIcon aria-hidden="true" />
                审核
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            statusFilter === "all" ? "暂无审核记录" : `暂无${formatStatusLabel(statusFilter)}的审核`
          }}
        </p>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from "vue";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import Avatar from "~/components/ui/Avatar.vue";
import Chips from "~/components/ui/Chips.vue";
import ProfileReviewModal from "~/components/ui/ProfileReviewModal.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import EditIcon from "~/assets/images/utils/edit.svg?component";

const auth = await useAuth();
const app = useNuxtApp();

// 权限守卫：仅管理员/版主可访问
if (!["admin", "moderator"].includes(auth.value?.user?.role)) {
  await navigateTo("/");
}

useHead({
  title: "资料审核管理 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const loading = ref(true);
const reviews = ref([]);
const statusFilter = ref("pending");
const statusOptions = ["all", "pending", "approved", "rejected", "cancelled"];

const reviewModal = ref(null);
const approveAllModal = ref(null);
const approveAllLoading = ref(false);

const pendingCount = computed(() => {
  return reviews.value.filter((r) => r.status === "pending").length;
});

const getTypeName = (type) => {
  const types = { avatar: "头像", username: "用户名", bio: "简介" };
  return types[type] || type;
};

const formatStatusLabel = (status) => {
  const labels = {
    all: "全部",
    pending: "待审核",
    approved: "已批准",
    rejected: "已拒绝",
    cancelled: "已撤销",
  };
  return labels[status] || status;
};

const formatDateTime = (date) => {
  return app.$dayjs(date).format("YYYY-MM-DD HH:mm");
};

const truncate = (text, maxLen) => {
  if (!text) return "";
  return text.length > maxLen ? text.substring(0, maxLen) + "..." : text;
};

const getAvatarUrl = (jsonStr, field) => {
  try {
    const obj = JSON.parse(jsonStr);
    return obj[field] || null;
  } catch {
    return null;
  }
};

const fetchReviews = async () => {
  loading.value = true;
  try {
    const params = { count: 100 };
    if (statusFilter.value !== "all") {
      params.status = statusFilter.value;
    }
    const response = await useBaseFetch("moderation/profile-reviews", {
      method: "GET",
      params,
      internal: true,
    });
    if (response && Array.isArray(response)) {
      reviews.value = response;
    }
  } catch (error) {
    console.error("加载审核列表失败:", error);
    addNotification({
      group: "main",
      title: "加载失败",
      text: "无法加载审核列表",
      type: "error",
    });
  }
  loading.value = false;
};

const approveAll = () => {
  if (approveAllLoading.value) return;
  approveAllModal.value?.show();
};

const doApproveAll = async () => {
  approveAllLoading.value = true;
  try {
    const result = await useBaseFetch("moderation/profile-reviews/approve-all", {
      method: "POST",
      body: { notes: null },
      internal: true,
    });
    approveAllModal.value?.hide();
    addNotification({
      group: "main",
      title: "批量审批完成",
      text: `已通过 ${result?.approved || 0} 条${result?.failed ? `，失败 ${result.failed} 条` : ""}`,
      type: result?.failed ? "warning" : "success",
    });
    await fetchReviews();
  } catch (error) {
    console.error("批量审批失败:", error);
    addNotification({
      group: "main",
      title: "批量审批失败",
      text: error?.data?.description || "操作失败，请重试",
      type: "error",
    });
  } finally {
    approveAllLoading.value = false;
  }
};

watch(statusFilter, () => {
  fetchReviews();
});

onMounted(() => {
  fetchReviews();
});
</script>

<style lang="scss" scoped>
.header-section {
  margin-bottom: 1rem;

  h2 {
    margin: 0 0 0.25rem;
    font-size: 1.5rem;
  }

  .description {
    color: var(--color-text-secondary);
    margin: 0;
  }
}

.filter-section {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.loading-section {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 3rem;
  color: var(--color-text-secondary);
}

.reviews-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.review-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 1rem;
  border: 1px solid var(--color-button-bg);
  border-radius: var(--radius-lg);
  background: var(--color-raised-bg);
  gap: 1rem;
}

.review-main {
  flex: 1;
  min-width: 0;
}

.review-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.user-info {
  display: flex;
  align-items: center;
}

.user-link {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  text-decoration: none;
  color: var(--color-text);

  &:hover {
    color: var(--color-brand);
  }
}

.username {
  font-weight: 600;
}

.review-badges {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.type-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-md);
  font-size: 0.75rem;
  font-weight: 600;

  &.type-avatar {
    background: var(--color-brand-highlight);
    color: var(--color-brand);
  }

  &.type-username {
    background: rgba(59, 130, 246, 0.1);
    color: rgb(59, 130, 246);
  }

  &.type-bio {
    background: rgba(16, 185, 129, 0.1);
    color: rgb(16, 185, 129);
  }
}

.status-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-md);
  font-size: 0.75rem;
  font-weight: 600;

  &.status-pending {
    background: rgba(245, 158, 11, 0.15);
    color: rgb(245, 158, 11);
  }

  &.status-approved {
    background: rgba(34, 197, 94, 0.15);
    color: rgb(34, 197, 94);
  }

  &.status-rejected {
    background: rgba(239, 68, 68, 0.15);
    color: rgb(239, 68, 68);
  }

  &.status-cancelled {
    background: rgba(107, 114, 128, 0.15);
    color: rgb(107, 114, 128);
  }
}

.review-preview {
  margin-bottom: 0.5rem;
}

.avatar-preview {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.text-preview {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.old-text {
  color: var(--color-text-secondary);
  text-decoration: line-through;
}

.new-text {
  color: var(--color-text);
  font-weight: 500;
}

.preview-arrow {
  color: var(--color-text-secondary);
}

.review-meta {
  display: flex;
  align-items: center;
  gap: 1rem;
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.risk-labels {
  color: rgb(239, 68, 68);
  font-size: 0.75rem;
}

.review-notes-display {
  margin-top: 0.5rem;
  font-size: 0.875rem;
  color: var(--color-text-secondary);

  .label {
    font-weight: 600;
  }
}

.review-actions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.approve-all-content {
  padding: 1rem;

  p {
    margin: 0;
    line-height: 1.6;
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0 1rem 1rem;
}

.empty-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 3rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
  }
}
</style>
