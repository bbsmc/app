<template>
  <div>
    <ModalConfirm
      ref="deleteMerchantModal"
      title="确定要删除商户配置吗？"
      description="删除后您将无法接收付费插件的收入。此操作不可撤销。"
      proceed-label="确认删除"
      @proceed="deleteMerchant"
    />
    <section class="universal-card">
      <h2 class="text-2xl">高级创作者</h2>
      <p class="description">
        成为高级创作者可以发布付费插件，获得销售收入。平台收取支付通道约 2.5% 手续费。
      </p>
      <div class="notice-card">
        <InfoIcon class="notice-icon" />
        <div class="notice-content">
          <strong>重要提示：</strong>
          <ul>
            <li>只能发布<strong>服务端插件</strong>进行付费销售</li>
            <li>只能出售<strong>原创</strong>服务器插件，禁止销售他人作品或二次分发</li>
          </ul>
        </div>
      </div>

      <!-- 已是高级创作者 -->
      <div v-if="auth.user.is_premium_creator" class="status-card success">
        <CheckIcon class="status-icon" />
        <div class="status-content">
          <h3>您已是高级创作者</h3>
          <p>您可以发布付费插件了！</p>
        </div>
      </div>

      <!-- 商户配置（仅高级创作者可见） -->
      <div v-if="auth.user.is_premium_creator" class="merchant-section">
        <h3>支付商户配置</h3>
        <p class="merchant-desc">配置您的支付商户信息，用于接收付费插件的销售收入。</p>

        <!-- 已配置商户（非编辑状态） -->
        <div v-if="merchant && !showMerchantForm" class="merchant-card">
          <div class="merchant-info">
            <div class="merchant-row">
              <span class="label">店铺 ID：</span>
              <span class="value">{{ merchant.sid }}</span>
            </div>
            <div class="merchant-row">
              <span class="label">验证状态：</span>
              <span :class="['value', 'status', merchant.verified ? 'verified' : 'unverified']">
                {{ merchant.verified ? "已验证" : "未验证" }}
              </span>
            </div>
            <div class="merchant-row">
              <span class="label">配置时间：</span>
              <span class="value">{{ formatDateTime(merchant.updated_at) }}</span>
            </div>
          </div>
          <div class="merchant-actions">
            <button class="btn btn-secondary" :disabled="verifying" @click="verifyMerchant">
              <UpdatedIcon v-if="verifying" class="animate-spin" />
              <CheckIcon v-else />
              {{ verifying ? "验证中..." : "重新验证" }}
            </button>
            <button class="btn btn-secondary" @click="editMerchant">
              <EditIcon />
              修改配置
            </button>
            <button class="btn btn-danger" :disabled="deleting" @click="showDeleteMerchantModal">
              <XIcon />
              删除配置
            </button>
          </div>
        </div>

        <!-- 商户配置表单（新建或编辑） -->
        <div v-else-if="showMerchantForm" class="merchant-form">
          <div class="form-group">
            <label for="merchant-sid">
              <span class="label-title">店铺 ID (SID)</span>
              <span class="required">*</span>
            </label>
            <input
              id="merchant-sid"
              v-model="merchantForm.sid"
              type="number"
              min="1"
              placeholder="请输入支付平台的店铺 ID"
            />
          </div>
          <div class="form-group">
            <label for="merchant-key">
              <span class="label-title">密钥</span>
              <span class="required">*</span>
            </label>
            <input
              id="merchant-key"
              v-model="merchantForm.secret_key"
              type="password"
              placeholder="请输入支付平台的密钥"
              maxlength="128"
            />
          </div>
          <div class="form-actions">
            <ButtonStyled color="primary">
              <button :disabled="!canSubmitMerchant || savingMerchant" @click="saveMerchant">
                <UpdatedIcon v-if="savingMerchant" class="animate-spin" />
                <CheckIcon v-else />
                {{ savingMerchant ? "保存中..." : "保存配置" }}
              </button>
            </ButtonStyled>
            <ButtonStyled>
              <button @click="cancelMerchantForm">取消</button>
            </ButtonStyled>
          </div>
        </div>

        <!-- 未配置商户引导 -->
        <template v-else>
          <div class="merchant-guide">
            <div class="guide-content">
              <h4>如何开通支付商户？</h4>
              <p>
                为保障交易安全，平台使用接受监管的合法一清支付宝支付商户进行资金结算。
                如需开通商户账号，请联系客服进行申请。
              </p>
              <ul class="guide-steps">
                <li>扫描右侧二维码添加客服企业微信</li>
                <li>提供您的 BBSMC 用户名和联系方式</li>
                <li>客服将协助您完成商户开通</li>
                <li>获取店铺 ID 和密钥后在下方配置</li>
              </ul>
            </div>
            <div class="guide-qr">
              <img :src="paymentServiceQr" alt="客服微信二维码" />
              <p class="qr-hint">微信扫一扫，添加客服</p>
              <p class="qr-time">在线时间: 9:00 - 23:00</p>
            </div>
          </div>
          <div class="merchant-empty">
            <p>您还没有配置支付商户，请先完成配置才能发布付费插件。</p>
            <button class="btn btn-primary" @click="showMerchantForm = true">
              <PlusIcon />
              配置商户
            </button>
          </div>
        </template>
      </div>

      <!-- 有待处理或已审核的申请 -->
      <template v-else-if="application">
        <!-- 待审核 -->
        <div v-if="application.status === 'pending'" class="status-card pending">
          <InfoIcon class="status-icon" />
          <div class="status-content">
            <h3>申请审核中</h3>
            <p>您的申请正在等待超级管理员审核，请耐心等待。</p>
            <div class="meta">提交时间：{{ formatDateTime(application.created_at) }}</div>
          </div>
        </div>

        <!-- 已批准 -->
        <div v-else-if="application.status === 'approved'" class="status-card success">
          <CheckIcon class="status-icon" />
          <div class="status-content">
            <h3>申请已批准</h3>
            <p>恭喜！您的高级创作者申请已通过审核。</p>
            <div class="meta">审核时间：{{ formatDateTime(application.reviewed_at) }}</div>
          </div>
        </div>

        <!-- 已拒绝 -->
        <div v-else-if="application.status === 'rejected'" class="status-card rejected">
          <XIcon class="status-icon" />
          <div class="status-content">
            <h3>申请已拒绝</h3>
            <p v-if="application.review_note">审核备注：{{ application.review_note }}</p>
            <p v-else>您的申请未通过审核，您可以查看对话了解详情或重新申请。</p>
            <div class="meta">审核时间：{{ formatDateTime(application.reviewed_at) }}</div>
          </div>
        </div>

        <!-- 对话线程 -->
        <div v-if="application.thread_id" class="thread-section">
          <button class="btn btn-secondary" @click="toggleThread">
            <MessageIcon aria-hidden="true" />
            {{ threadExpanded ? "收起对话" : "查看对话" }}
          </button>
          <div v-if="threadExpanded && thread" class="thread-container">
            <ConversationThread
              :thread="thread"
              :auth="auth"
              :current-member="null"
              :project="null"
              :report="null"
              @update-thread="refreshThread"
            />
          </div>
        </div>

        <!-- 重新申请按钮（仅拒绝状态） -->
        <div v-if="application.status === 'rejected'" class="reapply-section">
          <button class="btn btn-primary" @click="showApplyForm = true">
            <EditIcon aria-hidden="true" />
            重新申请
          </button>
        </div>
      </template>

      <!-- 申请表单 -->
      <template v-else>
        <div v-if="!showApplyForm && !application" class="apply-intro">
          <h3>申请条件</h3>
          <ul class="requirements">
            <li>需要提供真实姓名和联系方式（用于收款和沟通）</li>
            <li>需要提供身份证号（用于实名认证）</li>
            <li>建议提供作品链接展示您的开发能力</li>
          </ul>
          <button class="btn btn-primary" @click="showApplyForm = true">
            <PlusIcon aria-hidden="true" />
            开始申请
          </button>
        </div>

        <div v-if="showApplyForm" class="apply-form">
          <h3>填写申请信息</h3>
          <div class="form-group">
            <label for="real-name">
              <span class="label-title">真实姓名</span>
              <span class="required">*</span>
            </label>
            <input
              id="real-name"
              v-model="form.real_name"
              type="text"
              placeholder="请输入您的真实姓名"
              maxlength="100"
            />
          </div>

          <div class="form-group">
            <label for="contact-info">
              <span class="label-title">联系方式</span>
              <span class="required">*</span>
              <span class="label-hint">（QQ/微信/手机号）</span>
            </label>
            <input
              id="contact-info"
              v-model="form.contact_info"
              type="text"
              placeholder="请输入您的联系方式"
              maxlength="255"
            />
          </div>

          <div class="form-group">
            <label for="id-card">
              <span class="label-title">身份证号</span>
              <span class="required">*</span>
            </label>
            <input
              id="id-card"
              v-model="form.id_card_number"
              type="text"
              placeholder="请输入您的身份证号"
              maxlength="18"
            />
          </div>

          <div class="form-group">
            <label for="portfolio">
              <span class="label-title">作品链接</span>
              <span class="label-hint">（可选，GitHub/Gitee/其他）</span>
            </label>
            <input
              id="portfolio"
              v-model="form.portfolio_links"
              type="text"
              placeholder="请输入您的作品链接"
            />
          </div>

          <div class="form-group">
            <label for="reason">
              <span class="label-title">申请理由</span>
              <span class="label-hint">（可选）</span>
            </label>
            <textarea
              id="reason"
              v-model="form.application_reason"
              rows="4"
              placeholder="请简单介绍您的开发经验和申请理由"
            ></textarea>
          </div>

          <div class="form-actions">
            <ButtonStyled color="primary">
              <button :disabled="!canSubmit || submitting" @click="submitApplication">
                <SendIcon v-if="!submitting" aria-hidden="true" />
                <UpdatedIcon v-else aria-hidden="true" class="animate-spin" />
                {{ submitting ? "提交中..." : "提交申请" }}
              </button>
            </ButtonStyled>
            <ButtonStyled>
              <button @click="cancelForm">取消</button>
            </ButtonStyled>
          </div>
        </div>
      </template>
    </section>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { ButtonStyled } from "@modrinth/ui";
import ConversationThread from "~/components/ui/thread/ConversationThread.vue";
import ModalConfirm from "~/components/ui/ModalConfirm.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import XIcon from "~/assets/images/utils/x.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import MessageIcon from "~/assets/images/utils/message.svg?component";
import EditIcon from "~/assets/images/utils/edit.svg?component";
import PlusIcon from "~/assets/images/utils/plus.svg?component";
import SendIcon from "~/assets/images/utils/send.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import paymentServiceQr from "~/assets/images/payment-service-qr.png";

const auth = await useAuth();
const nuxtApp = useNuxtApp();

useHead({
  title: "高级创作者 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

// SSR 数据获取 - 使用 useAsyncData 避免数据滞后
const { data: application, refresh: refreshApplication } = await useAsyncData(
  "creator-application",
  async () => {
    if (!auth.value?.user) return null;
    try {
      return await useBaseFetch("creator/application", { method: "GET" });
    } catch (error) {
      if (error.statusCode !== 404) {
        console.error("获取申请失败:", error);
      }
      return null;
    }
  },
);

// 获取商户配置
const { data: merchant, refresh: refreshMerchant } = await useAsyncData(
  "merchant-config",
  async () => {
    if (!auth.value?.user?.is_premium_creator) return null;
    try {
      return await useBaseFetch("payment/merchant", { method: "GET", apiVersion: 3 });
    } catch (error) {
      if (error.statusCode !== 404) {
        console.error("获取商户配置失败:", error);
      }
      return null;
    }
  },
);

// 响应式状态
const showApplyForm = ref(false);
const submitting = ref(false);
const threadExpanded = ref(false);
const thread = ref(null);

// 商户配置状态
const showMerchantForm = ref(false);
const savingMerchant = ref(false);
const verifying = ref(false);
const deleting = ref(false);
const deleteMerchantModal = ref(null);
const merchantForm = ref({
  sid: "",
  secret_key: "",
});

const form = ref({
  real_name: "",
  contact_info: "",
  id_card_number: "",
  portfolio_links: "",
  application_reason: "",
});

// 计算属性 - 添加身份证号格式验证
const canSubmit = computed(() => {
  const idCard = form.value.id_card_number.trim();
  // 简单的身份证格式验证：18位，最后一位可以是数字或X
  const idCardValid = /^\d{17}[\dXx]$/.test(idCard);
  return form.value.real_name.trim() && form.value.contact_info.trim() && idCardValid;
});

// 商户表单验证
const canSubmitMerchant = computed(() => {
  const sid = merchantForm.value.sid;
  const key = merchantForm.value.secret_key?.trim();
  return sid && Number(sid) > 0 && key && key.length >= 8;
});

// 辅助函数
const formatDateTime = (date) => nuxtApp.$dayjs(date).format("YYYY-MM-DD HH:mm");

// 提交申请
const submitApplication = async () => {
  if (!canSubmit.value || submitting.value) return;
  submitting.value = true;
  try {
    const body = {
      real_name: form.value.real_name.trim(),
      contact_info: form.value.contact_info.trim(),
      id_card_number: form.value.id_card_number.trim(),
      portfolio_links: form.value.portfolio_links.trim() || undefined,
      application_reason: form.value.application_reason.trim() || undefined,
    };
    await useBaseFetch("creator/apply", { method: "POST", body });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "申请已提交，请等待审核",
      type: "success",
    });
    showApplyForm.value = false;
    await refreshApplication();
  } catch (error) {
    console.error("提交申请失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "提交申请失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

// 取消表单
const cancelForm = () => {
  showApplyForm.value = false;
  form.value = {
    real_name: "",
    contact_info: "",
    id_card_number: "",
    portfolio_links: "",
    application_reason: "",
  };
};

// 切换对话显示
const toggleThread = async () => {
  if (threadExpanded.value) {
    threadExpanded.value = false;
  } else {
    threadExpanded.value = true;
    if (!thread.value && application.value?.thread_id) {
      try {
        thread.value = await useBaseFetch(`thread/${application.value.thread_id}`);
      } catch (err) {
        console.error("加载对话失败:", err);
        nuxtApp.$notify({ group: "main", title: "错误", text: "加载对话失败", type: "error" });
        threadExpanded.value = false;
      }
    }
  }
};

// 刷新对话
const refreshThread = async () => {
  if (application.value?.thread_id) {
    try {
      thread.value = await useBaseFetch(`thread/${application.value.thread_id}`);
    } catch (err) {
      console.error("刷新对话失败:", err);
    }
  }
};

// ============ 商户配置相关方法 ============

// 保存商户配置
const saveMerchant = async () => {
  if (!canSubmitMerchant.value || savingMerchant.value) return;
  savingMerchant.value = true;
  try {
    await useBaseFetch("payment/merchant", {
      method: "POST",
      apiVersion: 3,
      body: {
        sid: Number(merchantForm.value.sid),
        secret_key: merchantForm.value.secret_key.trim(),
      },
    });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "商户配置已保存并验证通过",
      type: "success",
    });
    showMerchantForm.value = false;
    merchantForm.value = { sid: "", secret_key: "" };
    await refreshMerchant();
  } catch (error) {
    console.error("保存商户配置失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "保存商户配置失败",
      type: "error",
    });
  } finally {
    savingMerchant.value = false;
  }
};

// 验证商户配置
const verifyMerchant = async () => {
  if (verifying.value) return;
  verifying.value = true;
  try {
    const result = await useBaseFetch("payment/merchant/verify", { method: "GET", apiVersion: 3 });
    if (result.success) {
      nuxtApp.$notify({
        group: "main",
        title: "验证成功",
        text: result.message || "商户配置验证通过",
        type: "success",
      });
      await refreshMerchant();
    } else {
      nuxtApp.$notify({
        group: "main",
        title: "验证失败",
        text: result.message || "商户配置验证未通过",
        type: "error",
      });
    }
  } catch (error) {
    console.error("验证商户配置失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "验证商户配置失败",
      type: "error",
    });
  } finally {
    verifying.value = false;
  }
};

// 显示删除确认框
const showDeleteMerchantModal = () => {
  deleteMerchantModal.value?.show();
};

// 删除商户配置
const deleteMerchant = async () => {
  if (deleting.value) return;
  deleting.value = true;
  try {
    await useBaseFetch("payment/merchant", { method: "DELETE", apiVersion: 3 });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "商户配置已删除",
      type: "success",
    });
    await refreshMerchant();
  } catch (error) {
    console.error("删除商户配置失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "删除商户配置失败",
      type: "error",
    });
  } finally {
    deleting.value = false;
  }
};

// 编辑商户配置
const editMerchant = () => {
  if (merchant.value) {
    merchantForm.value.sid = merchant.value.sid.toString();
    merchantForm.value.secret_key = ""; // 密钥需要重新输入
  }
  showMerchantForm.value = true;
};

// 取消商户表单
const cancelMerchantForm = () => {
  showMerchantForm.value = false;
  merchantForm.value = { sid: "", secret_key: "" };
};
</script>

<style scoped lang="scss">
.description {
  margin: 0 0 1rem 0;
  color: var(--color-text-secondary);
  font-size: 0.95rem;
}

.notice-card {
  display: flex;
  gap: 0.75rem;
  padding: 1rem;
  margin-bottom: 1.5rem;
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: var(--radius-md);

  .notice-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: rgb(37, 99, 235);
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .notice-content {
    font-size: 0.9rem;
    color: var(--color-text);

    strong {
      color: rgb(37, 99, 235);
    }

    ul {
      margin: 0.5rem 0 0 0;
      padding-left: 1.25rem;

      li {
        margin-bottom: 0.25rem;
        &:last-child {
          margin-bottom: 0;
        }
      }
    }
  }
}

.status-card {
  display: flex;
  gap: 1rem;
  padding: 1.25rem;
  border-radius: var(--radius-lg);
  margin-bottom: 1rem;

  .status-icon {
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
  }

  .status-content {
    h3 {
      margin: 0 0 0.5rem 0;
      font-size: 1.1rem;
    }
    p {
      margin: 0 0 0.5rem 0;
    }
    .meta {
      font-size: 0.85rem;
      color: var(--color-text-secondary);
    }
  }

  &.success {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.3);
    .status-icon {
      color: rgb(21, 128, 61);
    }
  }

  &.pending {
    background: rgba(251, 191, 36, 0.1);
    border: 1px solid rgba(251, 191, 36, 0.3);
    .status-icon {
      color: rgb(217, 119, 6);
    }
  }

  &.rejected {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    .status-icon {
      color: rgb(185, 28, 28);
    }
  }
}

.thread-section {
  margin-top: 1rem;

  .thread-container {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--color-bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-divider);
  }
}

.reapply-section {
  margin-top: 1rem;
}

.apply-intro {
  h3 {
    margin: 0 0 1rem 0;
  }

  .requirements {
    margin: 0 0 1.5rem 0;
    padding-left: 1.5rem;

    li {
      margin-bottom: 0.5rem;
      color: var(--color-text-secondary);
    }
  }
}

.apply-form {
  h3 {
    margin: 0 0 1.5rem 0;
  }
}

.form-group {
  margin-bottom: 1.25rem;

  label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
    font-weight: 500;

    .label-title {
      color: var(--color-text);
    }

    .required {
      color: var(--color-danger);
    }

    .label-hint {
      font-weight: normal;
      color: var(--color-text-secondary);
      font-size: 0.85rem;
    }
  }

  input,
  textarea {
    width: 100%;
    padding: 0.75rem;
    background: var(--color-raised-bg);
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    font-size: 0.95rem;
    font-family: inherit;

    &:focus {
      outline: none;
      border-color: var(--color-primary);
    }
  }

  textarea {
    resize: vertical;
    min-height: 100px;
  }
}

.form-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 1.5rem;

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

.btn {
  padding: 0.5rem 1rem;
  border-radius: var(--radius-md);
  border: none;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.5rem;
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

// ============ 商户配置样式 ============

.merchant-section {
  margin-top: 1.5rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--color-divider);

  h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.1rem;
  }

  .merchant-desc {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
    margin: 0 0 1rem 0;
  }
}

.merchant-guide {
  display: flex;
  gap: 1.5rem;
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  margin-bottom: 1rem;

  .guide-content {
    flex: 1;

    h4 {
      margin: 0 0 0.75rem 0;
      font-size: 1rem;
      color: var(--color-text);
    }

    p {
      margin: 0 0 0.75rem 0;
      font-size: 0.9rem;
      color: var(--color-text-secondary);
      line-height: 1.5;
    }

    .guide-steps {
      margin: 0;
      padding-left: 1.25rem;
      font-size: 0.85rem;
      color: var(--color-text-secondary);

      li {
        margin-bottom: 0.35rem;
        &:last-child {
          margin-bottom: 0;
        }
      }
    }
  }

  .guide-qr {
    flex-shrink: 0;
    text-align: center;

    img {
      width: 120px;
      height: 120px;
      border-radius: var(--radius-sm);
    }

    .qr-hint {
      margin: 0.5rem 0 0.25rem 0;
      font-size: 0.8rem;
      color: var(--color-text);
    }

    .qr-time {
      margin: 0;
      font-size: 0.75rem;
      color: var(--color-text-secondary);
    }
  }

  @media (max-width: 600px) {
    flex-direction: column;
    align-items: center;
    text-align: center;

    .guide-content {
      .guide-steps {
        text-align: left;
      }
    }
  }
}

.merchant-card {
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
}

.merchant-info {
  margin-bottom: 1rem;
}

.merchant-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.5rem;

  &:last-child {
    margin-bottom: 0;
  }

  .label {
    color: var(--color-text-secondary);
    min-width: 5rem;
  }

  .value {
    color: var(--color-text);

    &.status.verified {
      color: rgb(21, 128, 61);
    }

    &.status.unverified {
      color: rgb(217, 119, 6);
    }
  }
}

.merchant-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.merchant-empty {
  padding: 1.5rem;
  background: var(--color-raised-bg);
  border: 1px dashed var(--color-divider);
  border-radius: var(--radius-md);
  text-align: center;

  p {
    margin: 0 0 1rem 0;
    color: var(--color-text-secondary);
  }
}

.merchant-form {
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  margin-top: 1rem;
}

.btn-danger {
  background: rgba(239, 68, 68, 0.1);
  color: rgb(185, 28, 28);
  border: 1px solid rgba(239, 68, 68, 0.3);

  &:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.2);
  }
}
</style>
