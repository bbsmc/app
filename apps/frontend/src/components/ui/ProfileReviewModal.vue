<template>
  <NewModal ref="modal">
    <template #title>
      <div class="truncate text-lg font-extrabold text-contrast">审核资料修改</div>
    </template>
    <div class="review-content">
      <div v-if="review" class="review-summary">
        <div class="summary-row">
          <span class="label">用户：</span>
          <nuxt-link :to="`/user/${review.username}`" class="user-link">
            <Avatar :src="review.avatar_url" :alt="review.username" size="xs" circle />
            <span>{{ review.username }}</span>
          </nuxt-link>
        </div>
        <div class="summary-row">
          <span class="label">修改类型：</span>
          <span class="type-badge" :class="`type-${review.review_type}`">
            {{ getTypeName(review.review_type) }}
          </span>
        </div>
        <div class="summary-row">
          <span class="label">风控标签：</span>
          <span class="risk-labels">{{ review.risk_labels }}</span>
        </div>

        <!-- 内容对比 -->
        <div class="diff-section">
          <template v-if="review.review_type === 'avatar'">
            <div class="avatar-diff">
              <div class="diff-item">
                <span class="diff-label">当前头像</span>
                <Avatar
                  :src="getAvatarUrl(review.old_value, 'avatar_url')"
                  size="md"
                  circle
                  alt="当前头像"
                />
              </div>
              <span class="diff-arrow">&rarr;</span>
              <div class="diff-item">
                <span class="diff-label">新头像</span>
                <Avatar
                  :src="getAvatarUrl(review.new_value, 'avatar_url')"
                  size="md"
                  circle
                  alt="新头像"
                />
              </div>
            </div>
          </template>
          <template v-else>
            <div class="text-diff">
              <div class="diff-item">
                <span class="diff-label">旧值</span>
                <div class="diff-value old">{{ review.old_value || "(空)" }}</div>
              </div>
              <div class="diff-item">
                <span class="diff-label">新值</span>
                <div class="diff-value new">{{ review.new_value }}</div>
              </div>
            </div>
          </template>
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
            :class="{ active: decision === 'approved' }"
            @click="decision = 'approved'"
          >
            <CheckIcon aria-hidden="true" />
            批准修改
          </button>
          <button
            class="decision-btn reject"
            :class="{ active: decision === 'rejected' }"
            @click="decision = 'rejected'"
          >
            <CrossIcon aria-hidden="true" />
            拒绝修改
          </button>
        </div>

        <label class="form-label">
          <span>审核备注</span>
          <span class="optional">（可选，将通知用户）</span>
        </label>
        <textarea
          v-model="notes"
          class="review-textarea"
          placeholder="请输入审核备注..."
          rows="3"
        ></textarea>
      </div>
    </div>
    <div class="modal-actions">
      <ButtonStyled
        :color="
          decision === 'approved' ? 'primary' : decision === 'rejected' ? 'danger' : 'default'
        "
      >
        <button :disabled="!decision || submitting" @click="submit">
          <CheckIcon v-if="decision === 'approved'" aria-hidden="true" />
          <CrossIcon v-else-if="decision === 'rejected'" aria-hidden="true" />
          {{
            decision === "approved"
              ? "确认批准"
              : decision === "rejected"
                ? "确认拒绝"
                : "请选择审核决定"
          }}
        </button>
      </ButtonStyled>
      <ButtonStyled>
        <button @click="modal?.hide()">取消</button>
      </ButtonStyled>
    </div>
  </NewModal>
</template>

<script setup>
import { ref } from "vue";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import Avatar from "~/components/ui/Avatar.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";

const emit = defineEmits(["reviewed"]);

const modal = ref(null);
const review = ref(null);
const decision = ref("");
const notes = ref("");
const submitting = ref(false);

const getTypeName = (type) => {
  const types = { avatar: "头像", username: "用户名", bio: "简介" };
  return types[type] || type;
};

const getAvatarUrl = (jsonStr, field) => {
  try {
    const obj = JSON.parse(jsonStr);
    return obj[field] || null;
  } catch {
    return null;
  }
};

const open = (item) => {
  review.value = item;
  decision.value = "";
  notes.value = "";
  modal.value?.show();
};

const submit = async () => {
  if (!review.value || !decision.value) return;
  submitting.value = true;
  try {
    const action = decision.value === "approved" ? "approve" : "reject";
    await useBaseFetch(`moderation/profile-reviews/${review.value.id}/${action}`, {
      method: "POST",
      body: { notes: notes.value || null },
      internal: true,
    });
    addNotification({
      group: "main",
      title: "审核成功",
      text: `已${decision.value === "approved" ? "批准" : "拒绝"}该资料修改`,
      type: "success",
    });
    modal.value?.hide();
    emit("reviewed");
  } catch (error) {
    console.error("提交审核失败:", error);
    addNotification({
      group: "main",
      title: "审核失败",
      text: error?.data?.description || "操作失败，请重试",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

defineExpose({ open });
</script>

<style lang="scss" scoped>
.review-content {
  padding: 1rem;
  min-width: 500px;
}

.review-summary {
  margin-bottom: 1.5rem;
}

.summary-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;

  .label {
    font-weight: 600;
    color: var(--color-text-secondary);
    min-width: 5rem;
  }
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

.risk-labels {
  color: rgb(239, 68, 68);
  font-size: 0.75rem;
}

.diff-section {
  margin-top: 1rem;
  padding: 1rem;
  border: 1px solid var(--color-button-bg);
  border-radius: var(--radius-lg);
  background: var(--color-bg);
}

.avatar-diff {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2rem;
}

.diff-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.diff-label {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  font-weight: 600;
}

.diff-arrow {
  font-size: 1.5rem;
  color: var(--color-text-secondary);
}

.text-diff {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.diff-value {
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  word-break: break-all;

  &.old {
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: var(--color-text-secondary);
  }

  &.new {
    background: rgba(34, 197, 94, 0.05);
    border: 1px solid rgba(34, 197, 94, 0.2);
    color: var(--color-text);
  }
}

.review-form {
  .form-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
    font-weight: 600;

    .required {
      color: rgb(239, 68, 68);
    }

    .optional {
      font-weight: 400;
      color: var(--color-text-secondary);
      font-size: 0.875rem;
    }
  }
}

.decision-buttons {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.decision-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border: 2px solid var(--color-button-bg);
  border-radius: var(--radius-lg);
  background: var(--color-raised-bg);
  color: var(--color-text);
  cursor: pointer;
  transition: all 0.2s;
  font-weight: 500;

  svg {
    width: 1rem;
    height: 1rem;
  }

  &:hover {
    border-color: var(--color-text-secondary);
  }

  &.approve.active {
    border-color: rgb(34, 197, 94);
    background: rgba(34, 197, 94, 0.1);
    color: rgb(34, 197, 94);
  }

  &.reject.active {
    border-color: rgb(239, 68, 68);
    background: rgba(239, 68, 68, 0.1);
    color: rgb(239, 68, 68);
  }
}

.review-textarea {
  width: 100%;
  padding: 0.75rem;
  border: 1px solid var(--color-button-bg);
  border-radius: var(--radius-md);
  background: var(--color-bg);
  color: var(--color-text);
  resize: vertical;
  font-family: inherit;
  margin-bottom: 1rem;

  &:focus {
    outline: none;
    border-color: var(--color-brand);
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0 1rem 1rem;
}
</style>
