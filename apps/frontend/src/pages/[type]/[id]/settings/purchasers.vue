<template>
  <div>
    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">购买用户管理</span>
        </h3>
      </div>

      <!-- 非付费项目提示 -->
      <div v-if="!project.is_paid" class="notice-card warning">
        <InfoIcon class="notice-icon" />
        <div class="notice-content">
          <strong>此项目不是付费资源</strong>
          <p>只有付费资源才能查看购买用户列表。</p>
        </div>
      </div>

      <!-- 付费项目显示购买用户列表 -->
      <template v-else>
        <div v-if="loading" class="loading-state">
          <UpdatedIcon class="animate-spin" />
          <span>加载中...</span>
        </div>

        <template v-else>
          <!-- 统计信息 -->
          <div class="stats-row">
            <div class="stat-item">
              <UsersIcon class="stat-icon" />
              <span class="stat-label">购买用户总数</span>
              <span class="stat-value">{{ purchasersData?.total || 0 }}</span>
            </div>
          </div>

          <!-- 无购买用户 -->
          <div v-if="!purchasers.length" class="empty-state">
            <UsersIcon class="empty-icon" />
            <p class="empty-message">暂无用户购买此资源</p>
            <p class="empty-hint">当有用户购买后，会显示在这里</p>
          </div>

          <!-- 购买用户列表 -->
          <div v-else class="purchasers-list">
            <div v-for="purchaser in purchasers" :key="purchaser.user_id" class="purchaser-card">
              <div class="purchaser-info">
                <Avatar
                  :src="purchaser.avatar_url"
                  size="48px"
                  :alt="purchaser.username"
                  class="purchaser-avatar"
                />
                <div class="purchaser-details">
                  <span class="purchaser-name">{{ purchaser.username }}</span>
                  <span class="purchaser-id">ID: {{ purchaser.user_id }}</span>
                </div>
              </div>

              <div class="purchase-details">
                <div class="detail-row">
                  <CoinsIcon class="detail-icon" />
                  <span class="detail-label">支付金额</span>
                  <span class="detail-value price">¥{{ purchaser.amount }}</span>
                </div>
                <div class="detail-row">
                  <CalendarIcon class="detail-icon" />
                  <span class="detail-label">购买时间</span>
                  <span class="detail-value">{{ formatDate(purchaser.purchased_at) }}</span>
                </div>
                <div class="detail-row">
                  <HistoryIcon class="detail-icon" />
                  <span class="detail-label">到期时间</span>
                  <span
                    class="detail-value"
                    :class="{
                      expired: purchaser.expires_at && isExpired(purchaser.expires_at),
                      permanent: !purchaser.expires_at,
                    }"
                  >
                    {{
                      purchaser.expires_at
                        ? isExpired(purchaser.expires_at)
                          ? `已于 ${formatDate(purchaser.expires_at)} 过期`
                          : formatDate(purchaser.expires_at)
                        : "永久有效"
                    }}
                  </span>
                </div>
              </div>

              <div class="purchaser-actions">
                <Badge v-if="purchaser.is_active" type="approved"> 有效 </Badge>
                <Badge v-else type="rejected"> 已过期 </Badge>
                <ButtonStyled color="danger" type="transparent">
                  <button
                    :disabled="revoking === purchaser.user_id"
                    @click="confirmRevoke(purchaser)"
                  >
                    <TrashIcon />
                    {{ revoking === purchaser.user_id ? "撤销中..." : "撤销授权" }}
                  </button>
                </ButtonStyled>
              </div>
            </div>
          </div>
        </template>
      </template>
    </section>

    <!-- 确认弹窗 -->
    <ModalConfirm
      ref="confirmModal"
      :title="`撤销 ${revokeTarget?.username || ''} 的授权`"
      description="如果继续，该用户将失去对此资源的访问权限，需要重新购买才能再次访问。"
      proceed-label="撤销"
      :noblur="!(cosmetics?.advancedRendering ?? true)"
      @proceed="executeRevoke"
    />
  </div>
</template>

<script setup>
import { ButtonStyled } from "@modrinth/ui";
import Avatar from "~/components/ui/Avatar.vue";
import Badge from "~/components/ui/Badge.vue";
import ModalConfirm from "~/components/ui/ModalConfirm.vue";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import UsersIcon from "~/assets/images/utils/users.svg?component";
import CoinsIcon from "~/assets/images/utils/coins.svg?component";
import CalendarIcon from "~/assets/images/utils/calendar.svg?component";
import HistoryIcon from "~/assets/images/utils/history.svg?component";
import TrashIcon from "~/assets/images/utils/trash.svg?component";

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

useHead({
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const nuxtApp = useNuxtApp();
const cosmetics = useCosmetics();

const loading = ref(true);
const purchasersData = ref(null);
const revoking = ref(null);
const revokeTarget = ref(null);
const confirmModal = ref(null);

const purchasers = computed(() => purchasersData.value?.purchasers || []);

const formatDate = (dateString) => {
  if (!dateString) return "-";
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
};

const isExpired = (dateString) => {
  if (!dateString) return false;
  return new Date(dateString) < new Date();
};

const loadPurchasers = async () => {
  if (!props.project.is_paid) {
    loading.value = false;
    return;
  }

  try {
    const data = await useBaseFetch(`project/${props.project.id}/pricing/purchasers`, {
      method: "GET",
      apiVersion: 3,
    });
    purchasersData.value = data;
  } catch (error) {
    console.error("加载购买用户失败:", error);
    purchasersData.value = null;
  } finally {
    loading.value = false;
  }
};

const confirmRevoke = (purchaser) => {
  revokeTarget.value = purchaser;
  confirmModal.value?.show();
};

const executeRevoke = async () => {
  if (!revokeTarget.value) return;

  const userId = revokeTarget.value.user_id;
  revoking.value = userId;

  try {
    await useBaseFetch(`project/${props.project.id}/pricing/purchasers/${userId}`, {
      method: "DELETE",
      apiVersion: 3,
    });

    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "授权已撤销",
      type: "success",
    });

    // 刷新列表
    await loadPurchasers();
  } catch (error) {
    console.error("撤销授权失败:", error);
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: error.data?.description || "撤销授权失败",
      type: "error",
    });
  } finally {
    revoking.value = null;
    revokeTarget.value = null;
  }
};

onMounted(() => {
  loadPurchasers();
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
  }

  &.warning {
    background: rgba(251, 191, 36, 0.1);
    border: 1px solid rgba(251, 191, 36, 0.3);

    .notice-icon {
      color: rgb(217, 119, 6);
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

.stats-row {
  display: flex;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  .stat-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: var(--color-brand);
  }

  .stat-label {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }

  .stat-value {
    font-weight: 600;
    font-size: 1.1rem;
    color: var(--color-text);
  }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 3rem 2rem;
  text-align: center;

  .empty-icon {
    width: 48px;
    height: 48px;
    color: var(--color-text-secondary);
    opacity: 0.5;
    margin-bottom: 1rem;
  }

  .empty-message {
    color: var(--color-text);
    font-size: 1rem;
    font-weight: 500;
    margin: 0 0 0.5rem 0;
  }

  .empty-hint {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
    margin: 0;
  }
}

.purchasers-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-card-sm);
}

.purchaser-card {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 1rem;
  align-items: center;
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  transition: border-color 0.2s ease;

  &:hover {
    border-color: var(--color-brand);
  }

  @media screen and (max-width: 768px) {
    grid-template-columns: 1fr;
    gap: 0.75rem;
  }
}

.purchaser-info {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.purchaser-avatar {
  border-radius: var(--size-rounded-sm);
}

.purchaser-details {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.purchaser-name {
  font-weight: 600;
  color: var(--color-text);
}

.purchaser-id {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}

.purchase-details {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;

  @media screen and (max-width: 768px) {
    flex-direction: column;
    gap: 0.5rem;
  }
}

.detail-row {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.85rem;

  .detail-icon {
    width: 14px;
    height: 14px;
    color: var(--color-text-secondary);
  }

  .detail-label {
    color: var(--color-text-secondary);
    margin-right: 0.25rem;
  }

  .detail-value {
    color: var(--color-text);

    &.price {
      font-weight: 600;
      color: var(--color-brand);
    }

    &.expired {
      color: var(--color-red);
    }

    &.permanent {
      color: var(--color-green);
    }
  }
}

.purchaser-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-shrink: 0;

  button {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;

    svg {
      width: 14px;
      height: 14px;
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
