<template>
  <div>
    <!-- 关闭确认弹窗 -->
    <NewModal ref="disableModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">关闭激励</div>
      </template>
      <div class="modal-content">
        <p>
          项目：<b>{{ active?.title || active?.project_id }}</b>
        </p>
        <p>
          累计有效下载：<b>{{ active?.lifetime_eff_downloads }}</b>
        </p>
        <p>
          待结算金额：<b class="text-amber">¥{{ formatMoney(active?.pending_amount) }}</b>
        </p>
        <p>已结算金额：¥{{ formatMoney(active?.settled_amount) }}</p>

        <div class="form-group mt-4">
          <label class="checkbox-row">
            <input v-model="voidPending" type="checkbox" />
            <span>
              <b>同时作废所有待结算金额</b>
              <small class="block text-secondary">
                未达 7 天结算窗口的金额将永远不再结算给作者。 适用于发现刷量嫌疑时强制中止。
              </small>
            </span>
          </label>
        </div>

        <div class="form-group">
          <label>关闭原因 / 备注</label>
          <textarea
            v-model="disableNotes"
            rows="3"
            maxlength="500"
            placeholder="留下关闭原因，会写入审计日志"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled :color="voidPending ? 'red' : 'orange'">
          <button :disabled="submitting" @click="doDisable">
            {{ submitting ? "处理中..." : voidPending ? "关闭并作废待结算" : "关闭激励" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="disableModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <!-- 手动开通弹窗 -->
    <NewModal ref="enableModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">手动开通激励</div>
      </template>
      <div class="modal-content">
        <p>
          项目：<b>{{ active?.title || active?.project_id }}</b>
        </p>
        <p class="hint">
          手动开通会跳过作者申请流程，直接将该项目标记为「已开通激励」状态。
          已累计的有效下载和待结算金额会保留。
        </p>
        <div class="form-group">
          <label>备注（可选）</label>
          <textarea
            v-model="enableNotes"
            rows="3"
            maxlength="500"
            placeholder="留下开通原因，会写入审计日志"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="green">
          <button :disabled="submitting" @click="doEnable">
            {{ submitting ? "处理中..." : "确认开通" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="enableModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <!-- 批量申请提现弹窗 -->
    <NewModal
      ref="batchPayoutModal"
      :closable="!batchSubmitting"
      :close-on-esc="!batchSubmitting"
      :on-show="handleBatchModalShow"
      :on-hide="handleBatchModalHide"
    >
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">批量申请提现</div>
      </template>

      <div class="batch-payout-content" :aria-busy="batchPreviewLoading || batchSubmitting">
        <div v-if="batchPreviewLoading" class="batch-state" role="status" aria-live="polite">
          <UpdatedIcon class="animate-spin" aria-hidden="true" />
          <span>正在计算已认证用户的待提现金额与收益来源...</span>
        </div>

        <div v-else-if="batchPreviewError" class="batch-state batch-state-error" role="alert">
          <InfoIcon aria-hidden="true" />
          <div>
            <b>计算失败</b>
            <p>{{ batchPreviewError }}</p>
          </div>
          <ButtonStyled>
            <button @click="loadBatchPayoutPreview">重试</button>
          </ButtonStyled>
        </div>

        <template v-else-if="batchPreview">
          <div class="batch-summary">
            <div>
              <span>本次可申请用户</span>
              <b>{{ batchPreview.user_count }} 人</b>
            </div>
            <div>
              <span>本次申请总额</span>
              <b class="text-amber">{{ $formatMoney(batchPreview.total_amount) }}</b>
            </div>
          </div>

          <p class="batch-notice">
            仅包含已完成云账户实名认证、签约且资料完整的用户。收益来源按历史提现先进先出重建，下面各资源金额之和等于该用户本次申请金额。
          </p>

          <div
            class="batch-expiry"
            :class="{ expired: batchApplyExpired }"
            role="status"
            aria-live="polite"
          >
            <span>{{ batchExpiryText }}</span>
            <ButtonStyled v-if="batchCanRecalculate">
              <button :disabled="batchPreviewLoading" @click="loadBatchPayoutPreview">
                重新计算
              </button>
            </ButtonStyled>
          </div>

          <section
            v-if="batchApplyResult"
            class="batch-apply-result"
            :class="{ partial: batchApplyResult.status !== 'completed' }"
            aria-live="polite"
          >
            <div class="batch-result-heading">
              <b>{{ batchApplyResultTitle }}</b>
              <span>
                已创建 {{ batchApplyResult.created_count }} 人，跳过
                {{ batchApplyResult.skipped_count }} 人，失败 {{ batchApplyResult.failed_count }} 人
              </span>
            </div>
            <div class="batch-result-list">
              <div
                v-for="item in batchApplyResult.items || []"
                :key="item.user_id"
                class="batch-result-item"
              >
                <div>
                  <b>{{ item.username }}</b>
                  <span>{{ batchApplyItemDescription(item) }}</span>
                </div>
                <div class="batch-result-amount">
                  <b>{{ $formatMoney(item.amount) }}</b>
                  <span :class="`result-${item.status}`">
                    {{ batchApplyStatusLabel(item.status) }}
                  </span>
                </div>
              </div>
            </div>
          </section>

          <div v-if="batchPreview.users?.length" class="batch-user-list">
            <details v-for="user in batchPreview.users" :key="user.user_id" class="batch-user">
              <summary>
                <div class="batch-user-copy">
                  <nuxt-link
                    :to="`/user/${user.user_id}`"
                    target="_blank"
                    rel="noopener"
                    :aria-label="`${user.username}（新窗口打开）`"
                    @click.stop
                  >
                    {{ user.username }}
                  </nuxt-link>
                  <span>
                    {{ user.sources.length }} 个收益来源
                    <template v-if="user.order_count > 1">
                      · 将拆分为 {{ user.order_count }} 笔提现申请
                    </template>
                  </span>
                </div>
                <b>{{ $formatMoney(user.amount) }}</b>
                <DropdownIcon class="batch-user-chevron" aria-hidden="true" />
              </summary>

              <div v-if="user.orders?.length > 1" class="batch-order-list">
                <span class="batch-order-heading">拆单明细</span>
                <div v-for="order in user.orders" :key="order.chunk_index">
                  <span>第 {{ order.chunk_index + 1 }} 笔</span>
                  <b>{{ $formatMoney(order.amount) }}</b>
                </div>
              </div>

              <div class="batch-source-list">
                <div
                  v-for="(source, index) in user.sources"
                  :key="`${source.project_id || 'unassigned'}-${index}`"
                  class="batch-source"
                >
                  <nuxt-link
                    v-if="source.slug || source.project_id"
                    :to="`/project/${source.slug || source.project_id}`"
                    target="_blank"
                    rel="noopener"
                    :title="source.title"
                    :aria-label="`${source.title}（新窗口打开）`"
                  >
                    {{ source.title }}
                  </nuxt-link>
                  <span v-else :title="source.title">{{ source.title }}</span>
                  <b>{{ $formatMoney(source.amount) }}</b>
                </div>
              </div>
            </details>
          </div>

          <div v-else class="batch-state">
            <InfoIcon aria-hidden="true" />
            <span>当前没有同时满足认证、签约及最低提现金额要求的用户。</span>
          </div>

          <details v-if="batchPreview.excluded_count" class="batch-excluded">
            <summary>已排除 {{ batchPreview.excluded_count }} 位当前无法申请提现的用户</summary>
            <div v-if="batchPreview.excluded_users?.length" class="batch-excluded-list">
              <div
                v-for="user in batchPreview.excluded_users"
                :key="user.user_id"
                class="batch-excluded-item"
              >
                <div>
                  <nuxt-link
                    :to="`/user/${user.user_id}`"
                    target="_blank"
                    rel="noopener"
                    :aria-label="`${user.username}（新窗口打开）`"
                  >
                    {{ user.username }}
                  </nuxt-link>
                  <span>{{ batchExcludedReason(user.reason_code) }}</span>
                </div>
                <b v-if="user.amount != null">{{ $formatMoney(user.amount) }}</b>
              </div>
            </div>
            <p v-else>当前无法创建提现申请。</p>
          </details>
        </template>
      </div>

      <template #actions>
        <div class="batch-confirmation">
          <label>
            <span id="batch-confirmation-help">
              输入 <code>{{ BATCH_CONFIRMATION_TEXT }}</code> 确认创建上述提现申请
            </span>
            <input
              v-model="batchConfirmText"
              :disabled="batchSubmitting || batchApplyExpired || !batchPreview?.users?.length"
              :placeholder="BATCH_CONFIRMATION_TEXT"
              aria-describedby="batch-confirmation-help"
              autocomplete="off"
            />
          </label>
          <div class="batch-confirmation-actions">
            <ButtonStyled>
              <button :disabled="batchSubmitting" @click="batchPayoutModal?.hide()">取消</button>
            </ButtonStyled>
            <ButtonStyled color="red">
              <button :disabled="batchApplyDisabled" @click="applyBatchPayout">
                {{ batchApplyButtonLabel }}
              </button>
            </ButtonStyled>
          </div>
        </div>
      </template>
    </NewModal>

    <!-- 全局分析图表 -->
    <section class="universal-card">
      <div class="header-section stats-header">
        <div>
          <h2>平台激励统计</h2>
          <p class="description">实时查看平台激励的累计与今日数据，用于评估预览阶段真实预算。</p>
        </div>
        <ButtonStyled v-if="canBatchPayout" circular type="transparent">
          <OverflowMenu :options="batchPayoutMenuOptions" aria-label="更多激励操作">
            <MoreVerticalIcon aria-hidden="true" />
            <template #batch-payout>
              <TransferIcon aria-hidden="true" />
              执行全部提现
            </template>
          </OverflowMenu>
        </ButtonStyled>
      </div>

      <div v-if="loadingStats" class="batch-state" role="status" aria-live="polite">
        <UpdatedIcon class="animate-spin" aria-hidden="true" />
        <span>正在加载平台激励统计...</span>
      </div>

      <div v-else-if="statsError" class="batch-state batch-state-error" role="alert">
        <InfoIcon aria-hidden="true" />
        <div>
          <b>统计加载失败</b>
          <p>{{ statsError }}</p>
        </div>
        <ButtonStyled>
          <button @click="fetchStats">重试</button>
        </ButtonStyled>
      </div>

      <template v-else-if="stats">
        <!-- 汇总卡 -->
        <div class="summary-grid">
          <div class="summary-card">
            <span class="label">激活项目</span>
            <span class="value">{{ formatNumber(stats.total_projects) }}</span>
            <span class="hint">已开通 {{ stats.total_enabled }}</span>
          </div>
          <div class="summary-card">
            <span class="label">累计有效下载</span>
            <span class="value">{{ formatNumber(stats.total_eff_downloads) }}</span>
          </div>
          <div class="summary-card highlight">
            <span class="label">待结算总额</span>
            <span class="value money pending">¥{{ formatMoney(stats.total_pending) }}</span>
          </div>
          <div class="summary-card">
            <span class="label">全部待提现金额</span>
            <span class="value money pending">{{ $formatMoney(stats.total_withdrawable) }}</span>
          </div>
          <div class="summary-card">
            <span class="label">已结算总额</span>
            <span class="value money settled">¥{{ formatMoney(stats.total_settled) }}</span>
          </div>
          <div v-if="parseFloat(stats.total_voided) > 0" class="summary-card">
            <span class="label">已作废</span>
            <span class="value money voided">¥{{ formatMoney(stats.total_voided) }}</span>
          </div>
        </div>

        <!-- 今日卡片 -->
        <div class="today-grid">
          <div class="today-card">
            <span class="label">今日有效下载</span>
            <span class="value">{{ formatNumber(stats.today_eff_downloads) }}</span>
          </div>
          <div class="today-card">
            <span class="label">今日产生金额</span>
            <span class="value text-amber">¥{{ formatMoney(stats.today_amount) }}</span>
          </div>
          <div class="today-card">
            <span class="label">今日活跃项目</span>
            <span class="value">{{ formatNumber(stats.today_active_projects) }}</span>
          </div>
        </div>

        <!-- 30 天趋势图 -->
        <div v-if="stats.daily_trend?.length" class="charts-block">
          <h4>近 30 天趋势</h4>
          <div class="chart-grid">
            <client-only>
              <Chart
                name="每日有效下载"
                type="bar"
                :labels="trendLabels"
                :data="trendDownloadsData"
                :colors="['var(--color-brand)']"
                :hide-toolbar="true"
                :hide-legend="true"
              />
            </client-only>
            <client-only>
              <Chart
                name="每日产生金额"
                type="bar"
                :labels="trendLabels"
                :data="trendAmountData"
                :colors="['var(--color-green)']"
                prefix="¥"
                :hide-toolbar="true"
                :hide-legend="true"
              />
            </client-only>
            <client-only>
              <Chart
                name="每日活跃项目数"
                type="area"
                :labels="trendLabels"
                :data="trendProjectsData"
                :colors="['var(--color-blue, #2563eb)']"
                :hide-toolbar="true"
                :hide-legend="true"
              />
            </client-only>
          </div>
        </div>

        <!-- 档位分布 -->
        <div v-if="stats.tier_distribution?.length" class="charts-block">
          <h4>项目档位分布（按累计有效下载）</h4>
          <div class="tier-grid">
            <div v-for="t in stats.tier_distribution" :key="t.tier" class="tier-card">
              <div class="tier-label">{{ formatTierLabel(t.tier) }}</div>
              <div class="tier-count">{{ t.project_count }} 个项目</div>
              <div class="tier-downloads">累计 {{ formatNumber(t.total_downloads) }} 次下载</div>
              <div class="tier-bar">
                <div class="tier-bar-fill" :style="{ width: tierBarWidth(t) }" />
              </div>
            </div>
          </div>
        </div>

        <!-- Top 20 项目 -->
        <div v-if="stats.top_projects?.length" class="charts-block">
          <h4>Top 20 项目（按待结算金额）</h4>
          <div class="top-list">
            <div v-for="(p, idx) in stats.top_projects" :key="p.project_id" class="top-row">
              <span class="rank">#{{ idx + 1 }}</span>
              <nuxt-link
                :to="`/project/${p.slug || p.project_id}`"
                class="title link"
                target="_blank"
              >
                {{ p.title || p.project_id }}
              </nuxt-link>
              <span class="downloads">{{ formatNumber(p.lifetime_eff_downloads) }} 次</span>
              <span class="amount text-amber">¥{{ formatMoney(p.pending_amount) }}</span>
              <div class="bar">
                <div class="bar-fill" :style="{ width: topBarWidth(p) }" />
              </div>
            </div>
          </div>
        </div>
      </template>
    </section>

    <section class="universal-card">
      <div class="header-section">
        <h2>项目列表</h2>
        <p class="description">
          展示所有有激励数据或已开通激励的项目。只有审核通过并开通激励的资源会继续累计有效下载和待结算金额。
        </p>
      </div>

      <!-- 筛选 -->
      <div class="filter-section">
        <Chips v-model="filterMode" :items="filterOptions" :format-label="formatFilterLabel" />
        <span class="filter-count">{{ filteredItems.length }} 个项目</span>
      </div>

      <!-- 列表 -->
      <div v-if="loading" class="loading-section">
        <UpdatedIcon class="animate-spin" />
        <span>加载中...</span>
      </div>

      <div v-else-if="filteredItems.length > 0" class="projects-list">
        <div
          v-for="item in filteredItems"
          :key="item.project_id"
          class="project-row"
          :class="{ 'is-disabled': !item.enabled }"
        >
          <div class="project-main">
            <div class="title-row">
              <nuxt-link
                :to="`/project/${item.slug || item.project_id}`"
                class="link"
                target="_blank"
              >
                <b>{{ item.title || item.project_id }}</b>
              </nuxt-link>
              <span class="status-badge" :class="item.enabled ? 'enabled' : 'auto'">
                {{ item.enabled ? "已开通" : "未开通" }}
              </span>
              <span v-if="parseFloat(item.pending_amount) > 0" class="pulse-dot"></span>
            </div>

            <div class="meta-row">
              <span v-if="item.slug" class="muted">slug: {{ item.slug }}</span>
              <span v-if="item.enabled_at">开通：{{ formatDateTime(item.enabled_at) }}</span>
              <span v-if="item.last_event_at">
                最后活动：{{ formatRelative(item.last_event_at) }}
              </span>
            </div>

            <div v-if="item.notes" class="notes-row">
              <span class="muted">备注：</span>{{ item.notes }}
            </div>

            <div class="stats-row">
              <div class="stat">
                <span class="stat-label">有效下载</span>
                <span class="stat-value">{{ formatNumber(item.lifetime_eff_downloads) }}</span>
              </div>
              <div class="stat">
                <span class="stat-label">待结算</span>
                <span class="stat-value text-amber">¥{{ formatMoney(item.pending_amount) }}</span>
              </div>
              <div class="stat">
                <span class="stat-label">已结算</span>
                <span class="stat-value text-green">¥{{ formatMoney(item.settled_amount) }}</span>
              </div>
              <div v-if="parseFloat(item.voided_amount) > 0" class="stat">
                <span class="stat-label">已作废</span>
                <span class="stat-value text-red">¥{{ formatMoney(item.voided_amount) }}</span>
              </div>
            </div>
          </div>

          <div class="project-actions">
            <button v-if="item.enabled" class="btn btn-danger" @click="openDisable(item)">
              关闭激励
            </button>
            <button v-else class="btn btn-secondary" @click="openEnable(item)">手动开通</button>
          </div>
        </div>
      </div>

      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            filterMode === "all"
              ? "暂无项目数据"
              : filterMode === "enabled"
                ? "暂无已开通激励的项目"
                : "暂无未开通激励的项目"
          }}
        </p>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, onBeforeUnmount, onMounted } from "vue";
import { DropdownIcon, MoreVerticalIcon, TransferIcon } from "@modrinth/assets";
import { NewModal, ButtonStyled, OverflowMenu } from "@modrinth/ui";
import Chips from "~/components/ui/Chips.vue";
import Chart from "~/components/ui/charts/Chart.client.vue";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";

const auth = await useAuth();
const app = useNuxtApp();

if (auth.value?.user?.role !== "admin") {
  await navigateTo("/");
}

useHead({
  title: "激励项目监控 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const loading = ref(true);
const loadingStats = ref(true);
const submitting = ref(false);
const items = ref([]);
const stats = ref(null);
const statsError = ref("");

const BATCH_CONFIRMATION_TEXT = "我确认批量申请提现";
const batchPayoutModal = ref(null);
const batchPreview = ref(null);
const batchPreviewLoading = ref(false);
const batchPreviewError = ref("");
const batchConfirmText = ref("");
const batchSubmitting = ref(false);
const batchApplyResult = ref(null);
const batchNow = ref(Date.now());
const batchModalOpen = ref(false);
let batchExpiryTimer;
const canBatchPayout = computed(() => auth.value?.user?.username === "BBSMC");
const batchConfirmed = computed(() => batchConfirmText.value === BATCH_CONFIRMATION_TEXT);
const batchExpiresAt = computed(() => {
  const timestamp = Date.parse(batchPreview.value?.expires_at || "");
  return Number.isFinite(timestamp) ? timestamp : null;
});
const batchExpired = computed(
  () => batchExpiresAt.value !== null && batchNow.value >= batchExpiresAt.value,
);
const batchApplyExpired = computed(() => !batchApplyResult.value && batchExpired.value);
const batchHasRetryableItems = computed(() =>
  (batchApplyResult.value?.items || []).some((item) => item.retryable),
);
const batchCanRecalculate = computed(
  () =>
    batchApplyExpired.value ||
    (batchApplyResult.value?.status === "partial" && !batchHasRetryableItems.value),
);
const batchExpiryText = computed(() => {
  if (batchApplyResult.value) {
    return batchHasRetryableItems.value
      ? "本批次已经开始处理，可继续重试标记为可重试的失败项。"
      : "本批次没有可重试的失败项，请重新计算后再发起新批次。";
  }
  if (batchExpiresAt.value === null) return "预览有效期未知，提交时将由服务端重新校验。";
  if (batchExpired.value) return "本次预览已过期，请重新计算后再申请。";

  const seconds = Math.max(0, Math.ceil((batchExpiresAt.value - batchNow.value) / 1000));
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `预览将在 ${minutes}:${String(remainingSeconds).padStart(2, "0")} 后过期`;
});
const batchApplyDisabled = computed(
  () =>
    batchSubmitting.value ||
    batchPreviewLoading.value ||
    batchApplyExpired.value ||
    !batchPreview.value?.users?.length ||
    !batchConfirmed.value ||
    (batchApplyResult.value?.status === "partial" && !batchHasRetryableItems.value),
);
const batchApplyButtonLabel = computed(() => {
  if (batchSubmitting.value) return batchApplyResult.value ? "正在重试..." : "正在批量申请...";
  if (batchApplyResult.value?.status === "partial") {
    return batchHasRetryableItems.value ? "重试失败项" : "没有可重试项";
  }
  return "确认批量申请提现";
});
const batchApplyResultTitle = computed(() => {
  if (batchApplyResult.value?.status === "completed") return "批量申请已全部创建";
  if ((batchApplyResult.value?.created_count || 0) > 0) return "批量申请部分完成";
  return "本次未创建提现申请";
});
const batchPayoutMenuOptions = [
  {
    id: "batch-payout",
    action: () => openBatchPayoutModal(),
    color: "red",
    hoverFilled: true,
  },
];

const BATCH_EXCLUDED_REASONS = {
  below_minimum: "待提现金额低于最低提现额度",
  banned: "该用户当前不可参与批量提现",
  missing_profile: "尚未绑定云账户资料",
  profile_unreadable: "云账户资料当前无法读取",
  not_signed: "尚未完成云账户签约",
  sign_operation_pending: "云账户签约状态正在处理中",
  incomplete_profile: "云账户认证、签约或收款资料不完整",
  source_attribution_failed: "收益来源当前无法完整归集",
};

const BATCH_APPLY_REASONS = {
  banned: "该用户当前不可参与批量提现",
  missing_profile: "云账户资料不存在",
  profile_unreadable: "云账户资料当前无法读取",
  not_signed: "用户当前未完成云账户签约",
  sign_operation_pending: "用户签约或解约操作正在处理中",
  incomplete_profile: "云账户认证、签约或收款资料不完整",
  profile_changed: "云账户资料在预览后发生变化",
  user_missing: "用户不存在",
  balance_changed: "待提现余额在预览后发生变化",
  sources_changed: "收益来源在预览后发生变化",
  quote_failed: "云账户试算失败，可稍后重试",
  database_error: "保存提现申请失败，可稍后重试",
};

const batchExcludedReason = (reasonCode) =>
  BATCH_EXCLUDED_REASONS[reasonCode] || "当前无法创建提现申请";
const batchApplyStatusLabel = (status) =>
  ({ created: "已创建", skipped: "已跳过", failed: "失败" })[status] || "未处理";
const batchApplyItemDescription = (item) => {
  if (item.status === "created") {
    const orderCount = item.payout_ids?.length || (item.payout_id ? 1 : 0);
    return orderCount > 1 ? `已创建 ${orderCount} 笔提现申请` : "提现申请已创建";
  }
  return BATCH_APPLY_REASONS[item.reason_code] || "当前无法创建提现申请";
};

const stopBatchExpiryTimer = () => {
  if (batchExpiryTimer) {
    window.clearInterval(batchExpiryTimer);
    batchExpiryTimer = undefined;
  }
};

const startBatchExpiryTimer = () => {
  stopBatchExpiryTimer();
  batchNow.value = Date.now();
  batchExpiryTimer = window.setInterval(() => {
    batchNow.value = Date.now();
  }, 1000);
};

const handleBatchModalShow = () => {
  batchModalOpen.value = true;
};

const handleBatchModalHide = () => {
  batchModalOpen.value = false;
  stopBatchExpiryTimer();
};

const filterMode = ref("all");
const filterOptions = ["all", "enabled", "auto"];
const formatFilterLabel = (v) => ({ all: "全部", enabled: "已开通", auto: "未开通" })[v] || v;

const disableModal = ref(null);
const enableModal = ref(null);
const active = ref(null);
const voidPending = ref(false);
const disableNotes = ref("");
const enableNotes = ref("");

const formatDateTime = (s) =>
  s ? app.$dayjs(s).tz("Asia/Shanghai").format("YYYY-MM-DD HH:mm") : "";
const formatRelative = (s) => (s ? app.$dayjs(s).fromNow() : "");

const formatNumber = (n) => {
  const x = Number(n) || 0;
  return x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
};

const formatMoney = (n) => {
  const x = Number(n) || 0;
  return x.toFixed(x < 1 ? 4 : 2).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
};

const filteredItems = computed(() => {
  if (filterMode.value === "enabled") return items.value.filter((i) => i.enabled);
  if (filterMode.value === "auto") return items.value.filter((i) => !i.enabled);
  return items.value;
});

const fetchItems = async () => {
  loading.value = true;
  try {
    const data = await useBaseFetch("admin/incentive/projects", {
      method: "GET",
      internal: true,
    });
    items.value = Array.isArray(data) ? data : [];
  } catch (e) {
    addNotification({
      group: "main",
      title: "加载失败",
      text: e?.data?.description || "无法加载项目列表",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

const fetchStats = async () => {
  loadingStats.value = true;
  statsError.value = "";
  try {
    stats.value = await useBaseFetch("admin/incentive/stats", {
      method: "GET",
      internal: true,
    });
    return true;
  } catch (e) {
    console.error("加载 stats 失败", e);
    statsError.value = e?.data?.description || e?.message || "无法加载平台激励统计，请稍后重试。";
    return false;
  } finally {
    loadingStats.value = false;
  }
};

const loadBatchPayoutPreview = async () => {
  if (!canBatchPayout.value || batchPreviewLoading.value) return;

  batchPreviewLoading.value = true;
  batchPreviewError.value = "";
  batchPreview.value = null;
  batchApplyResult.value = null;
  batchConfirmText.value = "";
  stopBatchExpiryTimer();
  try {
    batchPreview.value = await useBaseFetch("payout/admin/batch/preview", {
      method: "POST",
      apiVersion: 3,
    });
    if (batchModalOpen.value) startBatchExpiryTimer();
  } catch (e) {
    batchPreviewError.value = e?.data?.description || e?.message || "无法计算本次批量提现预览";
  } finally {
    batchPreviewLoading.value = false;
  }
};

const openBatchPayoutModal = () => {
  if (!canBatchPayout.value || batchSubmitting.value) return;
  batchPayoutModal.value?.show();
  loadBatchPayoutPreview();
};

const applyBatchPayout = async () => {
  if (
    batchSubmitting.value ||
    batchApplyDisabled.value ||
    !batchConfirmed.value ||
    !batchPreview.value?.batch_id ||
    !batchPreview.value?.users?.length
  ) {
    return;
  }

  batchSubmitting.value = true;
  try {
    const batchId = batchPreview.value.batch_id;
    const result = await useBaseFetch(`payout/admin/batch/${batchId}/apply`, {
      method: "POST",
      apiVersion: 3,
      body: { confirmation: batchConfirmText.value },
    });
    batchApplyResult.value = result;
    stopBatchExpiryTimer();
    const omitted = (result.skipped_count || 0) + (result.failed_count || 0);
    addNotification({
      group: "main",
      title:
        result.status === "completed"
          ? "批量申请已创建"
          : result.created_count > 0
            ? "批量申请部分完成"
            : "批量申请未创建",
      text:
        omitted > 0
          ? `已为 ${result.created_count} 位用户创建 ${app.$formatMoney(result.total_created)} 提现申请，另有 ${omitted} 位未创建，请在弹窗中查看逐项结果。`
          : `已为 ${result.created_count} 位用户创建 ${app.$formatMoney(result.total_created)} 提现申请。`,
      type: result.status === "completed" ? "success" : "warn",
    });
    if (result.created_count > 0) {
      const refreshed = await fetchStats();
      if (!refreshed) {
        addNotification({
          group: "main",
          title: "提现已创建，统计刷新失败",
          text: "提现申请已经保存，但页面统计仍是上次数据，请稍后刷新页面。",
          type: "warn",
        });
      }
    }
    if (result.status === "completed") {
      batchPayoutModal.value?.hide();
    }
  } catch (e) {
    addNotification({
      group: "main",
      title: "批量申请失败",
      text: e?.data?.description || e?.message || "无法创建批量提现申请",
      type: "error",
    });
  } finally {
    batchSubmitting.value = false;
  }
};

// 趋势图数据
const trendLabels = computed(() => stats.value?.daily_trend?.map((d) => d.date) || []);
const trendDownloadsData = computed(() => [
  {
    name: "有效下载",
    data:
      stats.value?.daily_trend?.map((d) => ({
        x: new Date(d.date).getTime(),
        y: d.effective_downloads,
      })) || [],
  },
]);
const trendAmountData = computed(() => [
  {
    name: "金额（元）",
    data:
      stats.value?.daily_trend?.map((d) => ({
        x: new Date(d.date).getTime(),
        y: parseFloat(parseFloat(d.daily_amount || "0").toFixed(2)),
      })) || [],
  },
]);
const trendProjectsData = computed(() => [
  {
    name: "活跃项目",
    data:
      stats.value?.daily_trend?.map((d) => ({
        x: new Date(d.date).getTime(),
        y: d.active_projects,
      })) || [],
  },
]);

// 档位分布
const formatTierLabel = (tier) => {
  const m = {
    "01_<100": "< 100 次",
    "02_100-1K": "100 - 1,000 次",
    "03_1K-1W": "1K - 10K 次",
    "04_1W-10W": "10K - 100K 次",
    "05_>10W": "> 100K 次",
  };
  return m[tier] || tier;
};

const tierBarWidth = (t) => {
  const max = Math.max(...(stats.value?.tier_distribution?.map((x) => x.project_count) || [1]));
  return `${(t.project_count / max) * 100}%`;
};

// Top 项目条形图宽度
const topBarWidth = (p) => {
  const max = parseFloat(stats.value?.top_projects?.[0]?.pending_amount || "1");
  if (max <= 0) return "0%";
  const v = parseFloat(p.pending_amount || "0");
  return `${(v / max) * 100}%`;
};

const openDisable = (item) => {
  active.value = item;
  voidPending.value = false;
  disableNotes.value = "";
  disableModal.value?.show();
};

const openEnable = (item) => {
  active.value = item;
  enableNotes.value = "";
  enableModal.value?.show();
};

const doDisable = async () => {
  if (!active.value || submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(`admin/projects/${active.value.project_id}/incentive`, {
      method: "PATCH",
      internal: true,
      body: {
        enable: false,
        void_pending: voidPending.value,
        notes: disableNotes.value || null,
      },
    });
    addNotification({
      group: "main",
      title: "成功",
      text: voidPending.value ? "激励已关闭，待结算已作废" : "激励已关闭",
      type: "success",
    });
    disableModal.value?.hide();
    await fetchItems();
  } catch (e) {
    addNotification({
      group: "main",
      title: "操作失败",
      text: e?.data?.description || "关闭失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

const doEnable = async () => {
  if (!active.value || submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(`admin/projects/${active.value.project_id}/incentive`, {
      method: "PATCH",
      internal: true,
      body: {
        enable: true,
        notes: enableNotes.value || null,
      },
    });
    addNotification({
      group: "main",
      title: "成功",
      text: "激励已开通",
      type: "success",
    });
    enableModal.value?.hide();
    await fetchItems();
  } catch (e) {
    addNotification({
      group: "main",
      title: "操作失败",
      text: e?.data?.description || "开通失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

onMounted(() => {
  fetchItems();
  fetchStats();
});

onBeforeUnmount(() => {
  stopBatchExpiryTimer();
});
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

.stats-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.batch-payout-content {
  width: min(52rem, calc(100vw - 5rem));
  max-width: 100%;
}

.batch-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  min-height: 9rem;
  color: var(--color-text-secondary);

  > svg {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
  }

  p {
    margin: 0.25rem 0 0;
  }

  &.batch-state-error {
    justify-content: flex-start;
    color: var(--color-red, #dc2626);

    > div {
      flex: 1;
    }
  }
}

.batch-summary {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;

  > div {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.875rem 1rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  span {
    color: var(--color-text-secondary);
    font-size: 0.85rem;
  }

  b {
    font-size: 1.25rem;
  }
}

.batch-notice {
  margin: 0.875rem 0;
  color: var(--color-text-secondary);
  font-size: 0.875rem;
  line-height: 1.5;
}

.batch-expiry {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.875rem;
  padding: 0.625rem 0.75rem;
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  background: var(--color-bg);
  color: var(--color-text-secondary);
  font-size: 0.875rem;

  &.expired {
    border-color: var(--color-orange, #d97706);
    color: var(--color-orange, #d97706);
  }
}

.batch-apply-result {
  margin-bottom: 0.875rem;
  padding: 0.75rem;
  border: 1px solid var(--color-green);
  border-radius: var(--radius-md);
  background: var(--color-bg);

  &.partial {
    border-color: var(--color-orange, #d97706);
  }
}

.batch-result-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.625rem;

  span {
    color: var(--color-text-secondary);
    font-size: 0.8rem;
  }
}

.batch-result-list {
  display: flex;
  flex-direction: column;
}

.batch-result-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.75rem;
  padding: 0.5rem 0;

  & + & {
    border-top: 1px solid var(--color-divider);
  }

  > div {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;

    > span {
      color: var(--color-text-secondary);
      font-size: 0.8rem;
    }
  }

  .batch-result-amount {
    align-items: flex-end;
    white-space: nowrap;
  }

  .result-created {
    color: var(--color-green);
  }

  .result-failed,
  .result-skipped {
    color: var(--color-orange, #d97706);
  }
}

.batch-user-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.batch-user {
  flex: 0 0 auto;
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  background: var(--color-bg);
  overflow: hidden;

  summary {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 1rem;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    cursor: pointer;
    list-style: none;
    user-select: none;

    &::-webkit-details-marker {
      display: none;
    }

    &:hover {
      background: var(--color-raised-bg, var(--color-bg));
    }

    &:focus-visible {
      outline: 2px solid var(--color-brand);
      outline-offset: -2px;
    }

    > b {
      color: var(--color-orange, #d97706);
      white-space: nowrap;
    }
  }

  .batch-user-chevron {
    width: 1rem;
    height: 1rem;
    color: var(--color-text-secondary);
    transition: transform 0.15s ease;
  }

  &[open] {
    border-color: var(--color-orange, #d97706);

    > summary {
      background: var(--color-raised-bg, var(--color-bg));
    }

    .batch-user-chevron {
      transform: rotate(180deg);
    }
  }
}

.batch-user-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 0.125rem;

  a {
    width: fit-content;
    max-width: 100%;
    overflow: hidden;
    color: var(--color-contrast);
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  span {
    color: var(--color-text-secondary);
    font-size: 0.8rem;
  }
}

.batch-source-list {
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--color-divider);
  background: var(--color-raised-bg, var(--color-bg));
}

.batch-order-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-top: 1px solid var(--color-divider);
  background: var(--color-bg);

  .batch-order-heading {
    grid-column: 1 / -1;
    color: var(--color-text-secondary);
    font-size: 0.8rem;
    font-weight: 600;
  }

  > div {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
    border-radius: var(--radius-sm);
    background: var(--color-raised-bg, var(--color-bg));
    font-size: 0.85rem;
  }
}

.batch-source {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 1rem;
  padding: 0.625rem 1rem;
  font-size: 0.875rem;

  & + & {
    border-top: 1px solid var(--color-divider);
  }

  a,
  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  b {
    white-space: nowrap;
  }
}

.batch-excluded {
  margin-top: 0.875rem;
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  background: var(--color-bg);
  color: var(--color-text-secondary);

  > summary {
    padding: 0.75rem 1rem;
    cursor: pointer;
    font-weight: 600;
  }

  > p {
    margin: 0;
    padding: 0 1rem 0.75rem;
  }
}

.batch-excluded-list {
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--color-divider);
}

.batch-excluded-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.75rem;
  padding: 0.625rem 1rem;

  & + & {
    border-top: 1px solid var(--color-divider);
  }

  > div {
    display: flex;
    flex-direction: column;
    min-width: 0;

    a {
      width: fit-content;
      max-width: 100%;
      overflow: hidden;
      color: var(--color-contrast);
      font-weight: 600;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    span {
      font-size: 0.8rem;
    }
  }

  > b {
    color: var(--color-orange, #d97706);
    white-space: nowrap;
  }
}

.batch-confirmation {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;

  label {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 15rem;
    font-size: 0.85rem;
  }

  code {
    padding: 0.125rem 0.35rem;
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-contrast);
  }

  input {
    width: 100%;
    padding: 0.625rem 0.75rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-contrast);
  }
}

.batch-confirmation-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 1.5rem;

  .summary-card {
    background: var(--color-bg);
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    &.highlight {
      border-color: var(--color-orange, #f59e0b);
      background: var(--color-orange-bg, #fef3c7);
    }

    .label {
      color: var(--color-text-secondary);
      font-size: 0.85rem;
    }
    .value {
      font-size: 1.5rem;
      font-weight: 700;
      &.money {
        font-size: 1.35rem;
      }
      &.pending {
        color: var(--color-orange, #d97706);
      }
      &.settled {
        color: var(--color-green, #059669);
      }
      &.voided {
        color: var(--color-red, #dc2626);
      }
    }
    .hint {
      color: var(--color-text-secondary);
      font-size: 0.8rem;
    }
  }
}

.today-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  margin-bottom: 1.5rem;

  .today-card {
    background: var(--color-raised-bg, var(--color-bg));
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    .label {
      color: var(--color-text-secondary);
      font-size: 0.85rem;
    }
    .value {
      font-size: 1.25rem;
      font-weight: 600;
    }
  }
}

.charts-block {
  margin-top: 2rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--color-divider);

  h4 {
    margin-bottom: 1rem;
  }
}

.chart-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 1rem;
}

.tier-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.75rem;

  .tier-card {
    background: var(--color-bg);
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;

    .tier-label {
      font-weight: 600;
      margin-bottom: 0.25rem;
    }
    .tier-count {
      font-size: 1.25rem;
      font-weight: 700;
      color: var(--color-brand);
    }
    .tier-downloads {
      color: var(--color-text-secondary);
      font-size: 0.85rem;
      margin-bottom: 0.5rem;
    }
    .tier-bar {
      height: 4px;
      background: var(--color-divider);
      border-radius: 2px;
      overflow: hidden;

      .tier-bar-fill {
        height: 100%;
        background: var(--color-brand);
        border-radius: 2px;
        transition: width 0.3s;
      }
    }
  }
}

.top-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;

  .top-row {
    display: grid;
    grid-template-columns: 2.5rem 1fr 8rem 7rem;
    grid-template-rows: auto auto;
    align-items: center;
    gap: 0.5rem 0.75rem;
    padding: 0.5rem 0.75rem;
    background: var(--color-bg);
    border-radius: var(--radius-sm);

    .rank {
      color: var(--color-text-secondary);
      font-weight: 600;
    }
    .title {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .downloads {
      color: var(--color-text-secondary);
      font-size: 0.85rem;
      text-align: right;
    }
    .amount {
      font-weight: 700;
      text-align: right;
    }
    .bar {
      grid-column: 1 / -1;
      height: 3px;
      background: var(--color-divider);
      border-radius: 2px;
      overflow: hidden;

      .bar-fill {
        height: 100%;
        background: linear-gradient(90deg, var(--color-orange, #f59e0b), var(--color-red, #dc2626));
        border-radius: 2px;
        transition: width 0.4s;
      }
    }
  }
}

.filter-section {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
  .filter-count {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
}

.loading-section,
.empty-section {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 3rem;
  color: var(--color-text-secondary);
}

.projects-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.project-row {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.25rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);
  transition: border-color 0.15s;

  &:hover {
    border-color: var(--color-button-bg);
  }

  &.is-disabled {
    background: transparent;
  }

  .project-main {
    flex: 1;
    min-width: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    flex-wrap: wrap;
  }

  .meta-row,
  .notes-row {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    color: var(--color-text-secondary);
    font-size: 0.85rem;
    margin-top: 0.25rem;

    .muted {
      color: var(--color-text-secondary);
    }
  }

  .stats-row {
    display: flex;
    flex-wrap: wrap;
    gap: 1.5rem;
    margin-top: 0.75rem;

    .stat {
      display: flex;
      flex-direction: column;
      gap: 0.125rem;

      .stat-label {
        color: var(--color-text-secondary);
        font-size: 0.8rem;
      }
      .stat-value {
        font-size: 1.05rem;
        font-weight: 600;
      }
    }
  }
}

.status-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.85rem;

  &.enabled {
    background: var(--color-green-bg, #d1fae5);
    color: var(--color-green, #059669);
  }
  &.auto {
    background: var(--color-blue-bg, #dbeafe);
    color: var(--color-blue, #2563eb);
  }
}

.pulse-dot {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  background: var(--color-orange, #f59e0b);
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.text-amber {
  color: var(--color-orange, #d97706);
}
.text-green {
  color: var(--color-green, #059669);
}
.text-red {
  color: var(--color-red, #dc2626);
}

.project-actions {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  flex-shrink: 0;
}

.modal-content {
  padding: 0 1rem;
  p {
    margin: 0.5rem 0;
  }
  .hint {
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
  code {
    background: var(--color-bg);
    padding: 0.1rem 0.35rem;
    border-radius: var(--radius-sm);
    font-size: 0.85em;
  }
}

.form-group {
  margin: 1rem 0;
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
  .checkbox-row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    cursor: pointer;
    small {
      font-size: 0.85rem;
    }
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  padding: 1rem;
  justify-content: flex-end;
}

@media screen and (max-width: 650px) {
  .batch-payout-content {
    width: 100%;
  }

  .batch-summary {
    grid-template-columns: 1fr;
  }

  .batch-confirmation {
    align-items: stretch;
    flex-direction: column;

    label {
      min-width: 0;
    }
  }

  .batch-confirmation-actions {
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .batch-expiry,
  .batch-result-heading {
    align-items: stretch;
    flex-direction: column;
  }

  .batch-user summary {
    grid-template-columns: minmax(0, 1fr) 1rem;
    gap: 0.5rem;
    padding: 0.75rem;

    > b {
      grid-column: 1 / -1;
      grid-row: 2;
    }

    .batch-user-chevron {
      grid-column: 2;
      grid-row: 1;
    }
  }

  .batch-result-item,
  .batch-excluded-item {
    grid-template-columns: 1fr;
  }

  .batch-result-item .batch-result-amount {
    align-items: flex-start;
  }
}

@media (prefers-reduced-motion: reduce) {
  .batch-user-chevron {
    transition: none !important;
  }
}
</style>
