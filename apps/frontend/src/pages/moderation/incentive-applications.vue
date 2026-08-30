<template>
  <div>
    <!-- 审核备注弹窗 -->
    <NewModal ref="reviewModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">
          {{ reviewing?.action === "approved" ? "通过申请" : "拒绝申请" }}
        </div>
      </template>
      <div class="modal-content">
        <p>项目：<b>{{ reviewing?.item?.project_title }}</b></p>
        <p>申请人：<b>{{ reviewing?.item?.applicant_username }}</b></p>
        <p v-if="reviewing?.item?.reason"><b>申请理由：</b>{{ reviewing.item.reason }}</p>
        <div class="form-group">
          <label>审核备注（可选）</label>
          <textarea
            v-model="reviewNotes"
            rows="3"
            maxlength="500"
            :placeholder="reviewing?.action === 'rejected' ? '请说明拒绝原因' : '可填写说明'"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled :color="reviewing?.action === 'approved' ? 'primary' : 'red'">
          <button :disabled="submitting" @click="doReview">
            {{ submitting ? "处理中..." : "确认" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="reviewModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="header-section">
        <h2>创作者激励申请审核</h2>
        <p class="description">审核作者提交的项目创作者激励开通申请</p>
      </div>

      <div class="filter-section">
        <Chips v-model="statusFilter" :items="statusOptions" :format-label="formatStatusLabel" />
      </div>

      <div v-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>

      <div v-else-if="items.length > 0" class="reviews-list">
        <div v-for="item in items" :key="item.id" class="review-item">
          <div class="review-main">
            <div class="review-header">
              <div class="user-info">
                <nuxt-link
                  :to="`/${'project'}/${item.project_slug || item.project_id}`"
                  class="project-link"
                >
                  <b>{{ item.project_title || `项目 ${item.project_id}` }}</b>
                </nuxt-link>
                <span class="meta">
                  申请人：
                  <nuxt-link :to="`/user/${item.applicant_username}`">
                    @{{ item.applicant_username || item.applicant_user_id }}
                  </nuxt-link>
                </span>
              </div>
              <span class="status-badge" :class="`status-${item.status}`">
                {{ formatStatusLabel(item.status) }}
              </span>
            </div>

            <div v-if="item.reason" class="review-content">
              <span class="label">申请理由：</span>
              <span>{{ item.reason }}</span>
            </div>

            <div class="review-meta">
              <span>提交时间：{{ formatDateTime(item.created_at) }}</span>
              <span v-if="item.reviewed_at">
                · 审核时间：{{ formatDateTime(item.reviewed_at) }}
              </span>
            </div>

            <div v-if="item.review_notes" class="review-notes-display">
              <span class="label">审核备注：</span>
              <span>{{ item.review_notes }}</span>
            </div>
          </div>

          <div class="review-actions">
            <button class="btn btn-secondary" @click="openThread(item)">
              查看对话
            </button>
            <template v-if="item.status === 'pending'">
              <button class="btn btn-primary" @click="openReview(item, 'approved')">
                <CheckIcon aria-hidden="true" />
                通过
              </button>
              <button class="btn btn-danger" @click="openReview(item, 'rejected')">
                拒绝
              </button>
            </template>
          </div>
        </div>
      </div>

      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            statusFilter === "all"
              ? "暂无申请记录"
              : `暂无${formatStatusLabel(statusFilter)}的申请`
          }}
        </p>
      </div>
    </section>

    <!-- thread 对话弹窗 -->
    <NewModal ref="threadModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">
          对话：{{ activeThreadItem?.project_title }}
        </div>
      </template>
      <div class="thread-modal">
        <div v-if="threadLoading" class="loading-section">
          <UpdatedIcon class="animate-spin" /> 加载中...
        </div>
        <template v-else>
          <div class="messages-list">
            <div
              v-for="m in activeThread?.messages"
              :key="m.id"
              class="message"
              :class="{ 'is-system': !m.author_id, 'is-mine': m.author_id === currentUserId }"
            >
              <div class="message-meta">
                <span class="author">
                  <template v-if="!m.author_id">系统</template>
                  <template v-else-if="m.author_id === activeThreadItem?.applicant_user_id">
                    @{{ activeThreadItem?.applicant_username || "申请人" }}
                  </template>
                  <template v-else>管理员</template>
                </span>
                <span class="time">{{ formatDateTime(m.created) }}</span>
              </div>
              <div class="message-body">{{ extractText(m.body) }}</div>
            </div>
            <div v-if="!activeThread?.messages?.length" class="empty">暂无消息</div>
          </div>

          <div v-if="activeThreadItem?.status === 'pending'" class="reply-area">
            <textarea
              v-model="threadReply"
              rows="2"
              maxlength="1000"
              placeholder="回复申请人..."
              :disabled="threadSending"
            />
            <button
              class="btn btn-primary"
              :disabled="!threadReply.trim() || threadSending"
              @click="sendThreadReply"
            >
              发送
            </button>
          </div>
        </template>
      </div>
    </NewModal>
  </div>
</template>

<script setup>
import { ref, watch, onMounted } from "vue";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import Chips from "~/components/ui/Chips.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";

const auth = await useAuth();
const app = useNuxtApp();

if (auth.value?.user?.role !== "admin") {
  await navigateTo("/");
}

useHead({
  title: "激励申请审核 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const loading = ref(true);
const submitting = ref(false);
const items = ref([]);
const statusFilter = ref("pending");
const statusOptions = ["pending", "approved", "rejected", "withdrawn", "all"];

const reviewModal = ref(null);
const reviewing = ref(null);
const reviewNotes = ref("");

const threadModal = ref(null);
const threadLoading = ref(false);
const threadSending = ref(false);
const activeThreadItem = ref(null);
const activeThread = ref(null);
const threadReply = ref("");
const currentUserId = auth.value?.user?.id;

const extractText = (body) => {
  if (!body) return "";
  if (typeof body === "string") return body;
  if (body.body) return body.body;
  return JSON.stringify(body);
};

const openThread = async (item) => {
  activeThreadItem.value = item;
  activeThread.value = null;
  threadReply.value = "";
  threadModal.value?.show();
  threadLoading.value = true;
  try {
    if (item.thread_id) {
      activeThread.value = await useBaseFetch(`thread/${item.thread_id}`, {
        method: "GET",
      });
    }
  } catch (e) {
    addNotification({
      group: "main",
      title: "加载失败",
      text: e?.data?.description || "无法加载对话",
      type: "error",
    });
  } finally {
    threadLoading.value = false;
  }
};

const sendThreadReply = async () => {
  if (!threadReply.value.trim() || !activeThreadItem.value?.thread_id) return;
  threadSending.value = true;
  try {
    await useBaseFetch(`thread/${activeThreadItem.value.thread_id}`, {
      method: "POST",
      body: {
        body: {
          type: "text",
          body: threadReply.value.trim(),
          private: false,
          replying_to: null,
        },
      },
    });
    threadReply.value = "";
    activeThread.value = await useBaseFetch(
      `thread/${activeThreadItem.value.thread_id}`,
      { method: "GET" },
    );
  } catch (e) {
    addNotification({
      group: "main",
      title: "发送失败",
      text: e?.data?.description || "无法发送消息",
      type: "error",
    });
  } finally {
    threadSending.value = false;
  }
};

const formatStatusLabel = (status) => {
  const m = {
    all: "全部",
    pending: "待审核",
    approved: "已通过",
    rejected: "已拒绝",
    withdrawn: "已撤回",
  };
  return m[status] || status;
};

const formatDateTime = (s) =>
  app.$dayjs(s).tz("Asia/Shanghai").format("YYYY-MM-DD HH:mm");

const fetchItems = async () => {
  loading.value = true;
  try {
    const data = await useBaseFetch("admin/incentive/applications", {
      method: "GET",
      internal: true,
      params: { status: statusFilter.value },
    });
    items.value = Array.isArray(data) ? data : [];
  } catch (e) {
    console.error("加载申请列表失败:", e);
    addNotification({
      group: "main",
      title: "加载失败",
      text: e?.data?.description || "无法加载申请列表",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

const openReview = (item, action) => {
  reviewing.value = { item, action };
  reviewNotes.value = "";
  reviewModal.value?.show();
};

const doReview = async () => {
  if (!reviewing.value || submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(
      `admin/incentive/applications/${reviewing.value.item.id}`,
      {
        method: "PATCH",
        internal: true,
        body: {
          status: reviewing.value.action,
          review_notes: reviewNotes.value || null,
        },
      },
    );
    addNotification({
      group: "main",
      title: "成功",
      text: reviewing.value.action === "approved" ? "已通过申请" : "已拒绝申请",
      type: "success",
    });
    reviewModal.value?.hide();
    await fetchItems();
  } catch (e) {
    addNotification({
      group: "main",
      title: "操作失败",
      text: e?.data?.description || "审核失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

watch(statusFilter, fetchItems);
onMounted(fetchItems);
</script>

<style lang="scss" scoped>
.header-section {
  margin-bottom: 1rem;
  h2 {
    margin: 0;
  }
  .description {
    color: var(--color-text-secondary);
    margin: 0.25rem 0 0;
  }
}
.filter-section {
  margin-bottom: 1rem;
}
.loading-section,
.empty-section {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
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
  gap: 1rem;
  padding: 1rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);

  .review-main {
    flex: 1;
  }
  .review-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    .user-info {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      .meta {
        color: var(--color-text-secondary);
        font-size: 0.85rem;
      }
    }
  }
  .review-content {
    margin-bottom: 0.5rem;
    .label {
      color: var(--color-text-secondary);
      margin-right: 0.25rem;
    }
  }
  .review-meta {
    color: var(--color-text-secondary);
    font-size: 0.85rem;
  }
  .review-notes-display {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: var(--color-bg-secondary, var(--color-bg));
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
  }
  .review-actions {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }
}
.status-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
  &.status-pending {
    background: var(--color-orange-bg);
    color: var(--color-orange);
  }
  &.status-approved {
    background: var(--color-green-bg);
    color: var(--color-green);
  }
  &.status-rejected {
    background: var(--color-red-bg);
    color: var(--color-red);
  }
  &.status-withdrawn {
    background: var(--color-divider);
    color: var(--color-text-secondary);
  }
}
.modal-content {
  padding: 0 1rem;
  p {
    margin: 0.5rem 0;
  }
}
.form-group {
  margin-top: 1rem;
  label {
    display: block;
    margin-bottom: 0.5rem;
  }
  textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-sm);
  }
}
.modal-actions {
  display: flex;
  gap: 0.5rem;
  padding: 1rem;
}

.thread-modal {
  padding: 0 1rem 1rem;

  .messages-list {
    max-height: 420px;
    overflow-y: auto;
    padding: 0.5rem;
    background: var(--color-bg);
    border-radius: var(--radius-md);
    margin-bottom: 1rem;

    .empty {
      padding: 1rem;
      color: var(--color-text-secondary);
      text-align: center;
    }

    .message {
      padding: 0.75rem;
      margin-bottom: 0.5rem;
      border-radius: var(--radius-sm);
      background: var(--color-bg-secondary, var(--color-raised-bg));

      &.is-mine {
        background: var(--color-blue-bg, #dbeafe);
        margin-left: 2rem;
      }
      &.is-system {
        background: transparent;
        color: var(--color-text-secondary);
        font-style: italic;
        text-align: center;
      }

      .message-meta {
        display: flex;
        justify-content: space-between;
        font-size: 0.85rem;
        color: var(--color-text-secondary);
        margin-bottom: 0.25rem;
        .author { font-weight: 600; }
      }
      .message-body {
        white-space: pre-wrap;
        word-break: break-word;
      }
    }
  }

  .reply-area {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    textarea {
      flex: 1;
      padding: 0.5rem;
      border: 1px solid var(--color-divider);
      border-radius: var(--radius-sm);
      resize: vertical;
    }
  }
}
</style>
