<template>
  <div>
    <!-- 拒绝原因弹窗 -->
    <NewModal ref="rejectModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">拒绝翻译链接</div>
      </template>
      <div class="reject-content">
        <p class="text-secondary">请说明拒绝的原因，以便申请者了解并改进：</p>
        <textarea
          v-model="rejectReason"
          class="reject-textarea"
          placeholder="例如：翻译质量不符合要求、版本不匹配、资源重复等..."
          rows="5"
          required
        ></textarea>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="danger">
          <button :disabled="!rejectReason || rejectingLink" @click="confirmReject">
            <CrossIcon aria-hidden="true" />
            确认拒绝
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="$refs.rejectModal.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="header-section">
        <h2>翻译链接审核管理</h2>
        <p class="description">管理所有项目的翻译版本链接审核请求</p>
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

      <!-- 链接列表 -->
      <div v-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>

      <div v-else-if="translationLinks.length > 0" class="links-container">
        <div class="links-list">
          <div v-for="link in translationLinks" :key="link.id" class="link-item">
            <div class="link-main">
              <div class="link-info">
                <!-- 翻译项目信息 -->
                <div class="project-section translation">
                  <span class="section-label">翻译版本：</span>
                  <nuxt-link
                    :to="`/project/${link.translation_project_slug}/version/${link.translation_version_number}`"
                    class="project-link"
                  >
                    <Avatar
                      :src="link.translation_project_icon"
                      :alt="link.translation_project_title"
                      size="sm"
                    />
                    <div class="project-details">
                      <span class="project-title">{{ link.translation_project_title }}</span>
                      <span class="version-number">v{{ link.translation_version_number }}</span>
                    </div>
                  </nuxt-link>
                </div>

                <!-- 箭头 -->
                <div class="link-arrow">
                  <RightArrowIcon aria-hidden="true" />
                </div>

                <!-- 目标项目信息 -->
                <div class="project-section target">
                  <span class="section-label">目标版本：</span>
                  <nuxt-link
                    :to="`/project/${link.target_project_slug}/version/${link.target_version_number}`"
                    class="project-link"
                  >
                    <Avatar
                      :src="link.target_project_icon"
                      :alt="link.target_project_title"
                      size="sm"
                    />
                    <div class="project-details">
                      <span class="project-title">{{ link.target_project_title }}</span>
                      <span class="version-number">v{{ link.target_version_number }}</span>
                    </div>
                  </nuxt-link>
                </div>
              </div>

              <div class="link-meta">
                <div class="meta-item">
                  <span class="meta-label">语言：</span>
                  <span class="language-badge">{{ getLanguageName(link.language_code) }}</span>
                </div>
                <div class="meta-item">
                  <span class="meta-label">提交者：</span>
                  <nuxt-link :to="`/user/${link.submitter_username}`" class="user-link">
                    <Avatar
                      :src="link.submitter_avatar"
                      :alt="link.submitter_username"
                      size="xs"
                      circle
                    />
                    {{ link.submitter_username }}
                  </nuxt-link>
                </div>
                <div class="meta-item">
                  <span class="meta-label">提交时间：</span>
                  <span>{{ formatDateTime(link.created) }}</span>
                </div>
                <div class="meta-item">
                  <span class="status-badge" :class="`status-${link.approval_status}`">
                    {{ getStatusLabel(link.approval_status) }}
                  </span>
                </div>
              </div>

              <div v-if="link.description" class="link-description">
                <span class="desc-label">说明：</span>
                {{ link.description }}
              </div>
            </div>

            <!-- 操作按钮 -->
            <div class="link-actions">
              <template v-if="link.approval_status === 'pending'">
                <button
                  class="btn btn-primary"
                  :disabled="processingLinks.includes(link.id)"
                  @click="approveLink(link)"
                >
                  <CheckIcon aria-hidden="true" />
                  批准
                </button>
                <button
                  class="btn btn-danger"
                  :disabled="processingLinks.includes(link.id)"
                  @click="openRejectModal(link)"
                >
                  <CrossIcon aria-hidden="true" />
                  拒绝
                </button>
              </template>
              <template v-else-if="link.approval_status === 'approved'">
                <button
                  class="btn btn-secondary"
                  :disabled="processingLinks.includes(link.id)"
                  @click="revokeLink(link)"
                >
                  <TrashIcon aria-hidden="true" />
                  撤销
                </button>
              </template>
              <button class="btn btn-secondary" @click="viewDetails(link)">
                <EyeIcon aria-hidden="true" />
                查看详情
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

      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            statusFilter === "all" ? "暂无翻译链接" : `暂无${formatStatusLabel(statusFilter)}的链接`
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
import { useAuth } from "~/composables/auth.js";
import { useBaseFetch } from "~/composables/fetch.js";
import { addNotification } from "~/composables/notifs.js";
import Avatar from "~/components/ui/Avatar.vue";
import Chips from "~/components/ui/Chips.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";
import TrashIcon from "~/assets/images/utils/trash.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import EyeIcon from "~/assets/images/utils/eye.svg?component";
import ChevronLeftIcon from "~/assets/images/utils/chevron-left.svg?component";
import ChevronRightIcon from "~/assets/images/utils/chevron-right.svg?component";
import RightArrowIcon from "~/assets/images/utils/right-arrow.svg?component";

const router = useRouter();
const auth = await useAuth();
const app = useNuxtApp();

// 页面标题
useHead({
  title: "翻译链接审核 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

// 常量
const ITEMS_PER_PAGE = 20;

// 响应式状态
const loading = ref(true);
const translationLinks = ref([]);
const processingLinks = ref([]);
const totalCount = ref(0);
const currentPage = ref(1);
const statusFilter = ref("all");
const statusOptions = ["all", "pending", "approved", "rejected"];

// 拒绝相关
const rejectModal = ref(null);
const rejectReason = ref("");
const rejectingLink = ref(false);
const pendingRejectLink = ref(null);

// 计算属性
const totalPages = computed(() => Math.ceil(totalCount.value / ITEMS_PER_PAGE));

// 显示的页码
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

// 格式化状态标签
const formatStatusLabel = (status) => {
  const labels = {
    all: "全部",
    pending: "审核中",
    approved: "已通过",
    rejected: "已拒绝",
  };
  return labels[status] || status;
};

// 获取状态标签
const getStatusLabel = (status) => {
  const labels = {
    pending: "审核中",
    approved: "已通过",
    rejected: "已拒绝",
  };
  return labels[status] || status;
};

// 获取语言名称
const getLanguageName = (code) => {
  const languages = {
    zh_CN: "简体中文",
    zh_TW: "繁体中文",
    en_US: "英语",
    ja_JP: "日语",
    ko_KR: "韩语",
  };
  return languages[code] || code;
};

// 格式化日期时间
const formatDateTime = (date) => {
  return app.$dayjs(date).format("YYYY-MM-DD HH:mm");
};

// 获取翻译链接列表
const fetchTranslationLinks = async () => {
  loading.value = true;
  try {
    const params = {
      page: currentPage.value,
      limit: ITEMS_PER_PAGE,
    };

    if (statusFilter.value !== "all") {
      params.status = statusFilter.value;
    }

    // 调用管理员API获取所有项目的翻译链接
    const response = await useBaseFetch("moderation/translation-links", {
      method: "GET",
      params,
    });

    if (response) {
      translationLinks.value = response.links || [];
      totalCount.value = response.total || 0;
    }
  } catch (error) {
    console.error("获取翻译链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "获取翻译链接列表失败",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

// 切换页码
const changePage = (page) => {
  if (page < 1 || page > totalPages.value) return;
  currentPage.value = page;
  fetchTranslationLinks();
  // 滚动到顶部
  if (import.meta.client) window.scrollTo({ top: 0, behavior: "smooth" });
};

// 批准链接
const approveLink = async (link) => {
  if (processingLinks.value.includes(link.id)) return;

  processingLinks.value.push(link.id);
  try {
    await useBaseFetch(
      `version/${link.translation_version_id}/link/${link.target_version_id}/approve`,
      {
        method: "POST",
      },
    );

    addNotification({
      group: "main",
      title: "成功",
      text: "已批准翻译链接",
      type: "success",
    });

    // 更新本地状态
    const index = translationLinks.value.findIndex((l) => l.id === link.id);
    if (index !== -1) {
      translationLinks.value[index].approval_status = "approved";
    }
  } catch (error) {
    console.error("批准链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "批准链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
  }
};

// 打开拒绝弹窗
const openRejectModal = (link) => {
  pendingRejectLink.value = link;
  rejectReason.value = "";
  rejectModal.value?.show();
};

// 确认拒绝
const confirmReject = async () => {
  const link = pendingRejectLink.value;
  if (!link || !rejectReason.value || rejectingLink.value) return;

  rejectingLink.value = true;
  processingLinks.value.push(link.id);

  try {
    // 先发送拒绝原因到thread
    await useBaseFetch(
      `version/${link.translation_version_id}/link/${link.target_version_id}/thread`,
      {
        method: "POST",
        body: {
          body: `您的翻译链接申请已被拒绝。\n\n拒绝原因：\n${rejectReason.value}\n\n请在修改后重新提交申请。`,
        },
      },
    );

    // 拒绝链接
    await useBaseFetch(
      `version/${link.translation_version_id}/link/${link.target_version_id}/reject`,
      {
        method: "POST",
      },
    );

    rejectModal.value?.hide();

    addNotification({
      group: "main",
      title: "成功",
      text: "已拒绝翻译链接并发送拒绝原因",
      type: "success",
    });

    // 更新本地状态
    const index = translationLinks.value.findIndex((l) => l.id === link.id);
    if (index !== -1) {
      translationLinks.value[index].approval_status = "rejected";
    }
  } catch (error) {
    console.error("拒绝链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "拒绝链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
    rejectingLink.value = false;
    pendingRejectLink.value = null;
    rejectReason.value = "";
  }
};

// 撤销链接
const revokeLink = async (link) => {
  if (processingLinks.value.includes(link.id)) return;

  if (!confirm("确定要撤销这个已批准的翻译链接吗？")) return;

  processingLinks.value.push(link.id);
  try {
    await useBaseFetch(
      `version/${link.translation_version_id}/link/${link.target_version_id}/revoke`,
      {
        method: "POST",
      },
    );

    addNotification({
      group: "main",
      title: "成功",
      text: "已撤销翻译链接",
      type: "success",
    });

    // 从列表中移除
    translationLinks.value = translationLinks.value.filter((l) => l.id !== link.id);
    totalCount.value--;
  } catch (error) {
    console.error("撤销链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "撤销链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
  }
};

// 查看详情
const viewDetails = (link) => {
  // 跳转到目标项目的版本页面
  router.push(`/project/${link.target_project_slug}/version/${link.target_version_number}`);
};

// 监听筛选条件变化
watch(statusFilter, () => {
  currentPage.value = 1;
  fetchTranslationLinks();
});

// 检查权限
onMounted(() => {
  if (
    !auth.value?.user ||
    (auth.value.user.role !== "admin" && auth.value.user.role !== "moderator")
  ) {
    router.push("/");
    addNotification({
      group: "main",
      title: "权限不足",
      text: "您没有权限访问此页面",
      type: "error",
    });
  } else {
    fetchTranslationLinks();
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

.links-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.links-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.link-item {
  padding: 1.25rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-divider);
  transition: all 0.2s;

  &:hover {
    border-color: var(--color-primary);
  }
}

.link-main {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.link-info {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.project-section {
  flex: 1;
  min-width: 250px;

  .section-label {
    display: block;
    font-size: 0.85rem;
    color: var(--color-text-secondary);
    margin-bottom: 0.5rem;
  }
}

.project-link {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  text-decoration: none;
  color: var(--color-text);
  padding: 0.5rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  transition: all 0.2s;

  &:hover {
    background: var(--color-button-bg);
  }
}

.project-details {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;

  .project-title {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .version-number {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
}

.link-arrow {
  display: flex;
  align-items: center;
  color: var(--color-text-disabled);

  svg {
    width: 1.5rem;
    height: 1.5rem;
  }
}

.link-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  align-items: center;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  .meta-label {
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
}

.language-badge {
  padding: 0.25rem 0.5rem;
  background: var(--color-primary-bg);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
  font-weight: 500;
}

.user-link {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  color: var(--color-primary);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

.status-badge {
  padding: 0.25rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.025em;
  display: inline-block;

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

.link-description {
  padding: 0.75rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  font-size: 0.9rem;

  .desc-label {
    font-weight: 500;
    margin-right: 0.5rem;
  }
}

.link-actions {
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

  &.btn-danger {
    background: var(--color-danger);
    color: white;

    &:hover:not(:disabled) {
      background: var(--color-danger-dark);
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

// 弹窗样式
.reject-content {
  padding: 1rem;

  p {
    margin-bottom: 1rem;
    color: var(--color-text-secondary);
    font-size: 0.95rem;
  }
}

.reject-textarea {
  width: 100%;
  min-height: 120px;
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
