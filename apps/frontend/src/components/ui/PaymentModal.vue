<template>
  <Teleport to="body">
    <NewModal ref="modal" :header="modalTitle">
      <template #title>
        <Avatar :src="project?.icon_url" :alt="project?.title" class="icon" size="32px" />
        <span class="text-lg font-extrabold text-contrast">{{ modalTitle }}</span>
      </template>

      <div class="payment-modal-content">
        <!-- 加载中 -->
        <div v-if="loading" class="loading-state">
          <UpdatedIcon class="animate-spin" />
          <span>正在创建订单...</span>
        </div>

        <!-- 错误状态 -->
        <div v-else-if="error" class="error-state">
          <XIcon class="error-icon" />
          <p class="error-message">{{ error }}</p>
          <ButtonStyled color="brand">
            <button @click="retryCreateOrder">
              <UndoIcon />
              重试
            </button>
          </ButtonStyled>
        </div>

        <!-- 订单创建成功，显示二维码 -->
        <div v-else-if="order" class="order-state">
          <!-- 支付成功 -->
          <div v-if="paymentSuccess" class="success-state">
            <CheckCircleIcon class="success-icon" />
            <h3>支付成功！</h3>
            <p>您已成功购买此资源，现在可以下载所有文件了。</p>
            <ButtonStyled color="brand">
              <button @click="closeModal">
                <DownloadIcon />
                开始下载
              </button>
            </ButtonStyled>
          </div>

          <!-- 等待支付 -->
          <div v-else class="qr-state">
            <div class="order-info">
              <div class="info-row">
                <span class="label">订单号：</span>
                <span class="value mono">{{ order.order_no }}</span>
              </div>
              <div class="info-row">
                <span class="label">金额：</span>
                <span class="value price">¥ {{ order.amount }}</span>
              </div>
              <div class="info-row">
                <span class="label">资源：</span>
                <span class="value">{{ order.project?.title }}</span>
              </div>
            </div>

            <div class="qr-section">
              <p class="qr-hint">请使用{{ paymentMethodName }}扫描二维码完成支付</p>
              <div class="qr-code-wrapper">
                <img
                  v-if="order.qr_code_url"
                  :src="order.qr_code_url"
                  alt="支付二维码"
                  class="qr-code"
                />
                <div v-else class="qr-placeholder">二维码加载失败</div>
              </div>
              <div class="payment-status">
                <UpdatedIcon class="status-icon animate-spin" />
                <span>等待支付中...</span>
              </div>
            </div>

            <div class="order-tips">
              <p>• 请在 15 分钟内完成支付，超时订单将自动关闭</p>
              <p>• 支付完成后页面将自动刷新</p>
              <p>• 如遇支付问题，请联系客服</p>
            </div>

            <div class="modal-actions">
              <ButtonStyled>
                <button @click="cancelPayment">取消支付</button>
              </ButtonStyled>
            </div>
          </div>
        </div>
      </div>
    </NewModal>
  </Teleport>
</template>

<script setup>
import { ButtonStyled, NewModal } from "@modrinth/ui";
import Avatar from "~/components/ui/Avatar.vue";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import XIcon from "~/assets/images/utils/x.svg?component";
import UndoIcon from "~/assets/images/utils/undo.svg?component";
import CheckCircleIcon from "~/assets/images/utils/check-circle.svg?component";
import DownloadIcon from "~/assets/images/utils/download.svg?component";

const app = useNuxtApp();
const modal = ref(null);

const props = defineProps({
  project: {
    type: Object,
    required: true,
  },
  pricing: {
    type: Object,
    default: null,
  },
});

const emit = defineEmits(["payment-success", "close"]);

// 状态
const loading = ref(false);
const error = ref(null);
const order = ref(null);
const paymentMethod = ref("alipay");
const paymentSuccess = ref(false);
const pollingInterval = ref(null);

// 计算属性
const modalTitle = computed(() => {
  if (paymentSuccess.value) return "支付成功";
  if (order.value) return "扫码支付";
  return "购买资源";
});

const paymentMethodName = computed(() => {
  return "支付宝";
});

// 方法
const show = async (method = "alipay") => {
  paymentMethod.value = method;
  loading.value = false;
  error.value = null;
  order.value = null;
  paymentSuccess.value = false;
  // 支付弹窗始终居中显示，不传递 event
  modal.value?.show();
  await createOrder();
};

const hide = () => {
  stopPolling();
  modal.value?.hide();
};

const createOrder = async () => {
  loading.value = true;
  error.value = null;

  try {
    const response = await useBaseFetch("order", {
      method: "POST",
      apiVersion: 3,
      body: {
        project_id: props.project.id,
        payment_method: paymentMethod.value,
      },
    });

    order.value = response;
    startPolling();
  } catch (err) {
    console.error("创建订单失败:", err);
    error.value = err.data?.description || err.message || "创建订单失败，请稍后重试";
  } finally {
    loading.value = false;
  }
};

const retryCreateOrder = () => {
  createOrder();
};

const startPolling = () => {
  if (pollingInterval.value) return;

  // 每 3 秒轮询一次订单状态
  pollingInterval.value = setInterval(async () => {
    if (!order.value) {
      stopPolling();
      return;
    }

    try {
      const status = await useBaseFetch(`order/${order.value.order_no}/status`, {
        method: "GET",
        apiVersion: 3,
      });

      if (status.status === "paid") {
        paymentSuccess.value = true;
        stopPolling();
        emit("payment-success");
        app.$notify({
          group: "main",
          title: "支付成功",
          text: "您已成功购买此资源",
          type: "success",
        });
      } else if (status.status === "expired" || status.status === "failed") {
        stopPolling();
        error.value = "订单已过期或支付失败，请重新创建订单";
        order.value = null;
      }
    } catch (err) {
      console.error("查询订单状态失败:", err);
    }
  }, 3000);
};

const stopPolling = () => {
  if (pollingInterval.value) {
    clearInterval(pollingInterval.value);
    pollingInterval.value = null;
  }
};

const cancelPayment = () => {
  stopPolling();
  hide();
};

const closeModal = () => {
  hide();
  emit("close");
};

// 清理
onUnmounted(() => {
  stopPolling();
});

defineExpose({
  show,
  hide,
});
</script>

<style scoped lang="scss">
.payment-modal-content {
  width: 400px;
  min-height: 300px;
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  gap: 1rem;
  min-height: 200px;

  svg {
    width: 2rem;
    height: 2rem;
  }
}

.error-state {
  .error-icon {
    color: var(--color-danger);
  }

  .error-message {
    color: var(--color-danger);
    text-align: center;
  }
}

.success-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 2rem;
  gap: 1rem;
  text-align: center;

  .success-icon {
    width: 3rem;
    height: 3rem;
    color: var(--color-brand);
  }

  h3 {
    margin: 0;
    color: var(--color-brand);
  }

  p {
    margin: 0;
    color: var(--color-text-secondary);
  }
}

.order-info {
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  padding: 1rem;
  margin-bottom: 1rem;

  .info-row {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0;

    .label {
      color: var(--color-text-secondary);
    }

    .value {
      color: var(--color-text);

      &.mono {
        font-family: monospace;
        font-size: 0.85rem;
      }

      &.price {
        font-weight: 600;
        color: var(--color-brand);
        font-size: 1.1rem;
      }
    }
  }
}

.qr-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;

  .qr-hint {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }

  .qr-code-wrapper {
    background: white;
    padding: 1rem;
    border-radius: var(--radius-md);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .qr-code {
    width: 200px;
    height: 200px;
    display: block;
  }

  .qr-placeholder {
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg);
    color: var(--color-text-secondary);
  }

  .payment-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-text-secondary);
    font-size: 0.9rem;

    .status-icon {
      width: 1rem;
      height: 1rem;
    }
  }
}

.order-tips {
  margin-top: 1rem;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-sm);
  font-size: 0.8rem;
  color: var(--color-text-secondary);

  p {
    margin: 0.25rem 0;
  }
}

.modal-actions {
  display: flex;
  justify-content: center;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
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
