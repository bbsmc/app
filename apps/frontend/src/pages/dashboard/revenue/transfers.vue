<template>
  <div>
    <section class="universal-card payout-history">
      <Breadcrumbs
        current-title="转账记录"
        :link-stack="[{ href: '/dashboard/revenue', label: '收益' }]"
      />
      <h2>转账记录</h2>
      <p>您在 BBSMC 余额中的所有提现记录将在此处列出：</p>
      <div class="input-group">
        <DropdownSelect
          v-model="selectedYear"
          :options="years"
          :display-name="(x) => (x === 'all' ? '所有年份' : x)"
          name="按年份筛选"
        />
        <DropdownSelect
          v-model="selectedMethod"
          :options="methods"
          :display-name="(x) => (x === 'all' ? '所有方式' : formatPayoutMethod(x))"
          name="按方式筛选"
        />
      </div>
      <p>
        {{
          selectedYear !== "all"
            ? selectedMethod !== "all"
              ? formatMessage(messages.transfersTotalYearMethod, {
                  amount: $formatMoney(totalAmount),
                  year: selectedYear,
                  method: formatPayoutMethod(selectedMethod),
                })
              : formatMessage(messages.transfersTotalYear, {
                  amount: $formatMoney(totalAmount),
                  year: selectedYear,
                })
            : selectedMethod !== "all"
              ? formatMessage(messages.transfersTotalMethod, {
                  amount: $formatMoney(totalAmount),
                  method: formatPayoutMethod(selectedMethod),
                })
              : formatMessage(messages.transfersTotal, { amount: $formatMoney(totalAmount) })
        }}
      </p>
      <p v-if="inTransitCount > 0" class="auto-sync-hint">
        当前有
        <strong>{{ inTransitCount }}</strong>
        笔订单处理中，管理员确认转账后系统会继续同步支付宝转账最新状态。
      </p>
      <div
        v-for="payout in paginatedPayouts"
        :key="payout.id"
        class="universal-card recessed payout"
      >
        <div class="platform">
          <AlipayIcon v-if="isAlipayMethod(payout.method)" />
          <UnknownIcon v-else />
        </div>
        <div class="payout-info">
          <div>
            <strong>
              {{ $dayjs(payout.created).format("YYYY-MM-DD HH:mm:ss") }}
            </strong>
          </div>
          <div>
            <span class="amount">{{ $formatMoney(payout.amount) }}</span>
            <template v-if="payout.yunzhanghu_details || hasPositiveAmount(payout.fee)">
              ⋅ 到账 {{ $formatMoney(receivedAmount(payout)) }}
            </template>
            <template v-if="hasPositiveAmount(serviceFee(payout))">
              ⋅ 服务费 {{ $formatMoney(serviceFee(payout)) }}
            </template>
            <template v-if="hasPositiveAmount(payout.yunzhanghu_details?.tax)">
              ⋅ 税费 {{ $formatMoney(payout.yunzhanghu_details.tax) }}
            </template>
          </div>
          <div class="payout-status">
            <span>
              <Badge v-if="payout.status === 'success'" color="green" type="成功" />
              <Badge v-else-if="payout.status === 'cancelling'" color="yellow" type="取消中" />
              <Badge
                v-else-if="payout.status === 'cancelled'"
                color="red"
                :type="payout.reject_reason ? '已退回' : '已取消'"
              />
              <Badge v-else-if="payout.status === 'failed'" color="red" type="失败" />
              <Badge v-else-if="payout.status === 'in-transit'" color="yellow" type="处理中" />
              <Badge v-else :type="payout.status" />
            </span>
            <template v-if="payout.method">
              <span>⋅</span>
              <span>
                {{ formatPayoutMethod(payout.method) }}
                <template v-if="payout.method_address">({{ payout.method_address }})</template>
              </span>
            </template>
          </div>
          <p v-if="shouldShowRejectReason(payout)" class="reject-reason">
            <strong>退回原因：</strong>{{ payout.reject_reason }}
          </p>
        </div>
      </div>
      <Pagination
        :page="currentPage"
        :count="pages"
        :link-function="pageLink"
        @switch-page="changePage"
      />
    </section>
  </div>
</template>
<script setup>
import { UnknownIcon } from "@modrinth/assets";
import { Badge, Breadcrumbs, DropdownSelect } from "@modrinth/ui";
import dayjs from "dayjs";
import Pagination from "~/components/ui/Pagination.vue";
import AlipayIcon from "~/assets/images/external/alipay.svg?component";

const vintl = useVIntl();
const { formatMessage } = vintl;

useHead({
  title: "转账记录 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const route = useNativeRoute();
const router = useNativeRouter();

const { data: payouts, refresh } = await useAsyncData(`payout`, () =>
  useBaseFetch(`payout`, {
    apiVersion: 3,
  }),
);

const pageSize = 10;

const sortedPayouts = computed(() =>
  [...(payouts.value || [])].sort((a, b) => dayjs(b.created) - dayjs(a.created)),
);

const years = computed(() => {
  const values = sortedPayouts.value.map((x) => dayjs(x.created).year());
  return ["all", ...new Set(values)];
});

const selectedYear = ref("all");

const methods = computed(() => {
  const values = sortedPayouts.value.filter((x) => x.method).map((x) => x.method);
  return ["all", ...new Set(values)];
});

const selectedMethod = ref("all");

const filteredPayouts = computed(() =>
  sortedPayouts.value
    .filter((x) => selectedYear.value === "all" || dayjs(x.created).year() === selectedYear.value)
    .filter((x) => selectedMethod.value === "all" || x.method === selectedMethod.value),
);

const page = computed(() => {
  const raw = Array.isArray(route.query.page) ? route.query.page[0] : route.query.page;
  const parsed = Number.parseInt(raw || "1", 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 1;
});

const pages = computed(() => Math.max(1, Math.ceil(filteredPayouts.value.length / pageSize)));
const currentPage = computed(() => Math.min(page.value, pages.value));

const paginatedPayouts = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return filteredPayouts.value.slice(start, start + pageSize);
});

const totalAmount = computed(() =>
  filteredPayouts.value.reduce((sum, payout) => sum + payout.amount, 0),
);

const inTransitCount = computed(
  () => sortedPayouts.value.filter((p) => p.status === "in-transit").length,
);

function isAlipayMethod(method) {
  return method === "yunzhanghu_alipay" || method === "alipay";
}

function formatPayoutMethod(method) {
  if (isAlipayMethod(method)) return "支付宝";
  return method || "未知方式";
}

function shouldShowRejectReason(payout) {
  return ["cancelled", "failed"].includes(payout.status) && !!payout.reject_reason;
}

function hasPositiveAmount(value) {
  return Number(value || 0) > 0;
}

function serviceFee(payout) {
  return Number(payout.yunzhanghu_details?.service_fee ?? payout.fee ?? 0);
}

function receivedAmount(payout) {
  if (payout.yunzhanghu_details?.received_amount != null) {
    return Number(payout.yunzhanghu_details.received_amount);
  }
  return Math.max(0, Number(payout.amount || 0) - Number(payout.fee || 0));
}

function changePage(newPage) {
  const query = { ...route.query };
  if (newPage > 1) {
    query.page = String(newPage);
  } else {
    delete query.page;
  }
  router.push({ query });
}

function pageLink(newPage) {
  return newPage > 1 ? `?page=${newPage}` : "?";
}

watch([selectedYear, selectedMethod], () => {
  if (page.value !== 1) {
    changePage(1);
  }
});

// 页面打开时每 15 秒自动 refresh 列表（后端每分钟更新一次状态，前端拉的快一点能更及时看到结果）
let pollTimer = null;
onMounted(() => {
  pollTimer = setInterval(() => {
    if (inTransitCount.value > 0) {
      refresh();
    }
  }, 15000);
});
onBeforeUnmount(() => {
  if (pollTimer) clearInterval(pollTimer);
});

const messages = defineMessages({
  transfersTotal: {
    id: "revenue.transfers.total",
    defaultMessage: "您已累计提现 {amount}。",
  },
  transfersTotalYear: {
    id: "revenue.transfers.total.year",
    defaultMessage: "您在 {year} 年已提现 {amount}。",
  },
  transfersTotalMethod: {
    id: "revenue.transfers.total.method",
    defaultMessage: "您已通过 {method} 提现 {amount}。",
  },
  transfersTotalYearMethod: {
    id: "revenue.transfers.total.year_method",
    defaultMessage: "您在 {year} 年已通过 {method} 提现 {amount}。",
  },
});
</script>
<style lang="scss" scoped>
.auto-sync-hint {
  padding: var(--gap-sm) var(--gap-md);
  background-color: var(--color-bg);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: var(--font-size-sm);
}

.payout {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;

  .platform {
    display: flex;
    padding: 0.75rem;
    background-color: var(--color-raised-bg);
    width: fit-content;
    height: fit-content;
    border-radius: 20rem;

    svg {
      width: 2rem;
      height: 2rem;
    }
  }

  .payout-status {
    display: flex;
    gap: 0.5ch;
  }

  .amount {
    color: var(--color-heading);
    font-weight: 500;
  }

  .reject-reason {
    margin: 0;
    color: var(--color-red);
    font-size: var(--font-size-sm);
    overflow-wrap: anywhere;
  }

  @media screen and (min-width: 800px) {
    flex-direction: row;
    align-items: center;

    .input-group {
      margin-left: auto;
    }
  }
}
</style>
