<template>
  <div>
    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">定价设置</span>
        </h3>
      </div>

      <!-- 非付费项目提示 -->
      <div v-if="!project.is_paid" class="notice-card warning">
        <InfoIcon class="notice-icon" />
        <div class="notice-content">
          <strong>此项目不是付费资源</strong>
          <p>只有在创建项目时设置为付费资源的项目才能配置定价。</p>
        </div>
      </div>

      <!-- 非高级创作者提示 -->
      <div v-else-if="!isPremiumCreator" class="notice-card warning">
        <InfoIcon class="notice-icon" />
        <div class="notice-content">
          <strong>无权限修改定价</strong>
          <p>只有高级创作者才能设置或修改定价。请前往设置页面申请成为高级创作者。</p>
        </div>
      </div>

      <!-- 付费项目定价设置 -->
      <template v-else>
        <div v-if="loading" class="loading-state">
          <UpdatedIcon class="animate-spin" />
          <span>加载中...</span>
        </div>

        <template v-else>
          <!-- 当前定价信息 -->
          <div v-if="pricing" class="pricing-info">
            <div class="pricing-row">
              <span class="label">当前价格：</span>
              <span class="value price">{{ pricing.price }} 元</span>
            </div>
            <div class="pricing-row">
              <span class="label">授权有效期：</span>
              <span class="value">
                {{ pricing.is_permanent ? "永久授权" : `${pricing.validity_days} 天` }}
              </span>
            </div>
          </div>
          <div v-else class="no-pricing">
            <p>尚未设置定价，请设置价格后才能发布。</p>
          </div>

          <!-- 定价表单 -->
          <div class="pricing-form">
            <h4>{{ pricing ? "修改定价" : "设置定价" }}</h4>

            <div class="form-group">
              <label for="price">
                <span class="label-title">价格</span>
                <span class="required">*</span>
              </label>
              <div class="input-with-unit">
                <input
                  id="price"
                  v-model.number="form.price"
                  type="number"
                  min="1"
                  max="1000"
                  placeholder="请输入价格"
                  :disabled="!hasPermission || saving"
                />
                <span class="unit">元</span>
              </div>
              <span class="hint">价格范围：1 - 1000 元</span>
            </div>

            <div class="form-group">
              <label for="validity-days">
                <span class="label-title">授权有效期</span>
              </label>
              <div class="input-with-unit">
                <input
                  id="validity-days"
                  v-model.number="form.validityDays"
                  type="number"
                  min="1"
                  max="3650"
                  placeholder="留空表示永久授权"
                  :disabled="!hasPermission || saving"
                />
                <span class="unit">天</span>
              </div>
              <span class="hint">留空表示永久授权，最多 3650 天（10年）</span>
            </div>

            <div class="form-actions">
              <ButtonStyled color="primary">
                <button :disabled="!canSave || saving" @click="savePricing">
                  <UpdatedIcon v-if="saving" class="animate-spin" />
                  <SaveIcon v-else />
                  {{ saving ? "保存中..." : "保存定价" }}
                </button>
              </ButtonStyled>
            </div>
          </div>

          <!-- 提示信息 -->
          <div class="notice-card info">
            <InfoIcon class="notice-icon" />
            <div class="notice-content">
              <strong>注意事项</strong>
              <ul>
                <li>定价设置后，已购买用户不受价格变更影响</li>
                <li>平台收取约 2.5% 的支付通道手续费</li>
                <li>收入将结算到您配置的支付商户账户</li>
              </ul>
            </div>
          </div>
        </template>
      </template>
    </section>
  </div>
</template>

<script setup>
import { ButtonStyled } from "@modrinth/ui";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import SaveIcon from "~/assets/images/utils/save.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";

const nuxtApp = useNuxtApp();
const auth = await useAuth();

useHead({
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const props = defineProps({
  project: {
    type: Object,
    required: true,
    default: () => ({}),
  },
  currentMember: {
    type: Object,
    required: true,
    default: () => ({}),
  },
});

// 检查是否是高级创作者
const isPremiumCreator = computed(() => auth.value?.user?.is_premium_creator ?? false);

// 权限检查：需要是高级创作者且有 EDIT_DETAILS 权限
const hasPermission = computed(() => {
  if (!isPremiumCreator.value) return false;
  if (!props.currentMember) return false;
  const EDIT_DETAILS = 1 << 2;
  return (props.currentMember.permissions & EDIT_DETAILS) === EDIT_DETAILS;
});

// 状态
const loading = ref(true);
const saving = ref(false);
const pricing = ref(null);
const form = ref({
  price: null,
  validityDays: null,
});

// 验证
const canSave = computed(() => {
  const price = form.value.price;
  const days = form.value.validityDays;

  if (!price || price < 1 || price > 1000) return false;
  // 验证 validityDays 必须是整数
  if (days !== null && days !== "" && days !== undefined) {
    if (!Number.isInteger(days) || days < 1 || days > 3650) return false;
  }

  return hasPermission.value;
});

// 加载定价信息
const loadPricing = async () => {
  if (!props.project.is_paid) {
    loading.value = false;
    return;
  }

  try {
    const data = await useBaseFetch(`project/${props.project.id}/pricing`, {
      method: "GET",
      apiVersion: 3,
    });
    // 验证 API 返回数据的完整性
    if (data && typeof data.price === "number") {
      pricing.value = data;
      form.value.price = data.price;
      form.value.validityDays = data.validity_days ?? null;
    } else {
      pricing.value = null;
    }
  } catch (error) {
    if (error.statusCode !== 400 && error.statusCode !== 404) {
      console.error("加载定价失败:", error);
    }
    pricing.value = null;
  } finally {
    loading.value = false;
  }
};

// 保存定价
const savePricing = async () => {
  if (!canSave.value || saving.value) return;

  saving.value = true;
  try {
    const body = {
      price: form.value.price,
      validity_days: form.value.validityDays || null,
    };

    const data = await useBaseFetch(`project/${props.project.id}/pricing`, {
      method: pricing.value ? "PATCH" : "POST",
      apiVersion: 3,
      body,
    });

    pricing.value = data;
    // 保存成功后更新表单值
    form.value.price = data.price;
    form.value.validityDays = data.validity_days ?? null;
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "定价已保存",
      type: "success",
    });
  } catch (error) {
    console.error("保存定价失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "保存定价失败",
      type: "error",
    });
  } finally {
    saving.value = false;
  }
};

// 初始化
onMounted(() => {
  loadPricing();
});
</script>

<style scoped lang="scss">
.notice-card {
  display: flex;
  gap: 0.75rem;
  padding: 1rem;
  margin-bottom: 1.5rem;
  border-radius: var(--radius-md);

  .notice-icon {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
    margin-top: 0.125rem;
  }

  .notice-content {
    font-size: 0.9rem;

    p {
      margin: 0.5rem 0 0 0;
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

  &.warning {
    background: rgba(251, 191, 36, 0.1);
    border: 1px solid rgba(251, 191, 36, 0.3);

    .notice-icon {
      color: rgb(217, 119, 6);
    }
  }

  &.info {
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);

    .notice-icon {
      color: rgb(37, 99, 235);
    }

    strong {
      color: rgb(37, 99, 235);
    }
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  color: var(--color-text-secondary);

  svg {
    width: 1.25rem;
    height: 1.25rem;
  }
}

.pricing-info {
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  margin-bottom: 1.5rem;
}

.pricing-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.5rem;

  &:last-child {
    margin-bottom: 0;
  }

  .label {
    color: var(--color-text-secondary);
    min-width: 6rem;
  }

  .value {
    color: var(--color-text);

    &.price {
      font-weight: 600;
      color: var(--color-brand);
    }
  }
}

.no-pricing {
  padding: 1.5rem;
  background: var(--color-raised-bg);
  border: 1px dashed var(--color-divider);
  border-radius: var(--radius-md);
  text-align: center;
  margin-bottom: 1.5rem;

  p {
    margin: 0;
    color: var(--color-text-secondary);
  }
}

.pricing-form {
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  margin-bottom: 1.5rem;

  h4 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
  }
}

.form-group {
  margin-bottom: 1rem;

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
  }

  .hint {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.8rem;
    color: var(--color-text-secondary);
  }
}

.input-with-unit {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    background: var(--color-bg);
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-sm);
    font-size: 0.95rem;

    &:focus {
      outline: none;
      border-color: var(--color-brand);
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  .unit {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
}

.form-actions {
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
