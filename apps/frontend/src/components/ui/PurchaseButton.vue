<template>
  <div v-if="project.is_paid" class="card flex-card experimental-styles-within">
    <!-- 标题 -->
    <h2>
      <CurrencyIcon class="header-icon" />
      付费资源
    </h2>

    <!-- 管理员/版主权限 -->
    <div v-if="isStaff" class="access-state staff">
      <div class="access-badge staff">
        <ShieldIcon class="badge-icon" />
        <span>{{ auth.user.role === "admin" ? "管理员" : "版主" }}权限</span>
      </div>
      <p class="access-hint">您拥有管理权限，可以访问所有文件</p>
    </div>

    <!-- 团队成员权限 -->
    <div v-else-if="isTeamMember" class="access-state member">
      <div class="access-badge member">
        <UsersIcon class="badge-icon" />
        <span>团队成员</span>
      </div>
      <p class="access-hint">您是资源团队成员，可以访问所有文件</p>
    </div>

    <!-- 已购买状态 -->
    <div v-else-if="hasPurchased" class="access-state purchased">
      <div class="access-badge purchased">
        <CheckCircleIcon class="badge-icon" />
        <span>已购买</span>
      </div>
      <p class="access-hint">您已拥有此资源，可以下载所有文件</p>
    </div>

    <!-- 未购买 -->
    <template v-else>
      <!-- 价格信息 -->
      <div class="price-display">
        <span class="currency">¥</span>
        <span class="amount">{{ formattedPrice }}</span>
      </div>
      <div class="price-meta">
        <span v-if="pricing?.validity_days" class="validity">
          <ClockIcon class="meta-icon" />
          有效期 {{ pricing.validity_days }} 天
        </span>
        <span v-else-if="pricing" class="validity permanent">
          <InfinityIcon class="meta-icon" />
          永久授权
        </span>
      </div>

      <!-- 未登录提示 -->
      <div v-if="!isLoggedIn" class="action-section">
        <ButtonStyled color="brand" class="full-width-btn">
          <NuxtLink to="/auth/sign-in">
            <RightArrowIcon />
            登录后购买
          </NuxtLink>
        </ButtonStyled>
      </div>

      <!-- 已登录，显示支付宝支付按钮 -->
      <div v-else class="action-section">
        <ButtonStyled color="blue" class="full-width-btn">
          <button :disabled="!pricing" @click="showPayment('alipay')">
            <AlipayIcon />
            支付宝支付
          </button>
        </ButtonStyled>
      </div>

      <!-- 提示信息 -->
      <div class="purchase-tips">
        <p>• 购买后可下载所有版本文件</p>
      </div>
    </template>

    <!-- 支付弹窗 -->
    <PaymentModal
      ref="paymentModal"
      :project="project"
      :pricing="pricing"
      @payment-success="onPaymentSuccess"
      @close="onPaymentClose"
    />
  </div>
</template>

<script setup>
import { ButtonStyled } from "@modrinth/ui";
import PaymentModal from "./PaymentModal.vue";
import CheckCircleIcon from "~/assets/images/utils/check-circle.svg?component";
import RightArrowIcon from "~/assets/images/utils/right-arrow.svg?component";
import CurrencyIcon from "~/assets/images/utils/currency.svg?component";

// 简单的图标组件
const ClockIcon = {
  template: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="1em" height="1em"><circle cx="12" cy="12" r="10"/><polyline points="12,6 12,12 16,14"/></svg>`,
};

const InfinityIcon = {
  template: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="1em" height="1em"><path d="M18.178 8c5.096 0 5.096 8 0 8-5.095 0-7.133-8-12.739-8-4.585 0-4.585 8 0 8 5.606 0 7.644-8 12.74-8z"/></svg>`,
};

const AlipayIcon = {
  template: `<svg viewBox="0 0 24 24" fill="currentColor" width="1em" height="1em"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm4.64 13.19c-.18.08-1.77.86-2.12.98-.35.13-.7.19-1.05.19-.87 0-1.56-.39-2.06-1.08-.5-.69-.75-1.63-.75-2.83 0-1.15.27-2.09.81-2.82.54-.73 1.25-1.09 2.14-1.09.82 0 1.46.31 1.93.92.47.61.7 1.43.7 2.47v.59h-4.23c.05.58.22 1.02.52 1.32.3.3.71.45 1.23.45.4 0 .75-.07 1.07-.21.32-.14.62-.33.89-.57l.69 1.13c-.35.3-.74.52-1.17.68-.43.16-.89.24-1.38.24-.94 0-1.69-.29-2.26-.88-.57-.59-.86-1.38-.86-2.39 0-1.06.28-1.92.85-2.58.57-.66 1.31-.99 2.21-.99.81 0 1.46.27 1.94.81.48.54.72 1.29.72 2.24v.62h-4.29c.05.58.22 1.02.51 1.32.29.3.7.45 1.21.45.4 0 .76-.07 1.08-.21.32-.14.62-.33.89-.57l.69 1.13z"/></svg>`,
};

const ShieldIcon = {
  template: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="1em" height="1em"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>`,
};

const UsersIcon = {
  template: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="1em" height="1em"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>`,
};

const auth = await useAuth();
const router = useRouter();
const tags = useTags();

const props = defineProps({
  project: {
    type: Object,
    required: true,
  },
  currentMember: {
    type: Object,
    default: null,
  },
});

const emit = defineEmits(["purchase-success"]);

const paymentModal = ref(null);

// 计算属性
const isLoggedIn = computed(() => !!auth.value?.user);

// 是否为管理员/版主
const isStaff = computed(() => {
  if (!auth.value?.user) return false;
  return tags.value.staffRoles?.includes(auth.value.user.role) || false;
});

// 是否为团队成员（但不是管理员/版主）
const isTeamMember = computed(() => {
  if (isStaff.value) return false; // 管理员/版主优先显示
  return !!props.currentMember;
});

// 是否已购买（真正购买的用户）
const hasPurchased = computed(() => {
  if (isStaff.value || isTeamMember.value) return false; // 排除管理员和团队成员
  return props.project.user_has_purchased === true;
});

// 是否有访问权限（任意一种方式）
const hasAccess = computed(() => {
  return isStaff.value || isTeamMember.value || hasPurchased.value;
});

// 定价信息直接从项目数据获取
const pricing = computed(() => {
  if (props.project.price != null) {
    return {
      price: props.project.price,
      validity_days: props.project.validity_days,
    };
  }
  return null;
});

// 格式化价格（不显示小数）
const formattedPrice = computed(() => {
  if (pricing.value?.price != null) {
    return Math.floor(Number(pricing.value.price));
  }
  return "?";
});

// 方法
const showPayment = (method) => {
  paymentModal.value?.show(method);
};

const onPaymentSuccess = () => {
  emit("purchase-success");
  // 刷新页面以更新购买状态
  router.go(0);
};

const onPaymentClose = () => {
  // 关闭时也刷新以确保状态同步
  if (props.project.user_has_purchased !== true) {
    router.go(0);
  }
};
</script>

<style scoped lang="scss">
// 高亮脉冲动画（从父组件触发）
:global(.highlight-pulse) .card {
  animation: pulse-highlight 0.5s ease-in-out 3;
}

@keyframes pulse-highlight {
  0%,
  100% {
    box-shadow: 0 0 0 0 rgba(var(--color-brand-rgb), 0);
  }
  50% {
    box-shadow: 0 0 0 8px rgba(var(--color-brand-rgb), 0.3);
  }
}

h2 {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  .header-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: var(--color-brand);
  }
}

.access-state {
  text-align: center;
  padding: 0.5rem 0;
}

.access-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-radius: var(--radius-md);
  font-weight: 600;

  .badge-icon {
    width: 1.25rem;
    height: 1.25rem;
  }

  // 已购买 - 绿色
  &.purchased {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.3);
    color: rgb(22, 163, 74);
  }

  // 管理员/版主 - 紫色
  &.staff {
    background: rgba(139, 92, 246, 0.1);
    border: 1px solid rgba(139, 92, 246, 0.3);
    color: rgb(124, 58, 237);
  }

  // 团队成员 - 蓝色
  &.member {
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);
    color: rgb(37, 99, 235);
  }
}

.access-hint {
  margin: 0.75rem 0 0 0;
  font-size: 0.85rem;
  color: var(--color-text-secondary);
}

.price-display {
  display: flex;
  align-items: baseline;
  justify-content: center;
  gap: 0.25rem;
  padding: 0.5rem 0;

  .currency {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-brand);
  }

  .amount {
    font-size: 2.5rem;
    font-weight: 700;
    color: var(--color-brand);
    line-height: 1;
  }
}

.price-meta {
  text-align: center;
  margin-bottom: 1rem;

  .validity {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.85rem;
    color: var(--color-text-secondary);

    &.permanent {
      color: rgb(22, 163, 74);
    }

    .meta-icon {
      width: 0.9rem;
      height: 0.9rem;
    }
  }
}

.action-section {
  margin-bottom: 0.75rem;
}

.full-width-btn {
  width: 100%;

  a,
  button {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;

    svg {
      width: 1.25rem;
      height: 1.25rem;
    }
  }
}

.purchase-tips {
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-divider);
  font-size: 0.75rem;
  color: var(--color-text-secondary);

  p {
    margin: 0.25rem 0;
  }
}
</style>
