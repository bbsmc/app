<template>
  <div>
    <NewModal ref="confirmWithdrawModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">确认提现</div>
      </template>

      <div class="confirm-body">
        <div class="confirm-grid">
          <span class="label">提现金额</span>
          <strong>{{ $formatMoney(parsedAmount) }}</strong>
          <span class="label">到账金额</span>
          <strong>{{ $formatMoney(quoteAfterTaxAmount) }}</strong>
          <span class="label">服务费</span>
          <strong>{{ $formatMoney(userServiceFee) }}</strong>
          <span v-if="quoteTaxTotal > 0" class="label">税费</span>
          <strong v-if="quoteTaxTotal > 0">{{ $formatMoney(quoteTaxTotal) }}</strong>
          <span class="label">余额扣除</span>
          <strong>{{ $formatMoney(totalDebit) }}</strong>
          <span class="label">支付宝账号</span>
          <span class="monospace">{{ profile.alipay_account_masked }}</span>
          <span class="label">户名</span>
          <strong>{{ profile.real_name }}</strong>
        </div>

        <p class="confirm-note">提交后申请会进入处理中，由管理员核对并确认转账。</p>
        <p class="confirm-note">
          提现服务费是支付给财税代理公司的服务费，BBSMC 承担约 4%，用户承担
          3%，用户部分会从本次提现金额中扣除。
        </p>
        <p class="confirm-note">
          使用财税平台是为了完成合规结算、实名签约、税费试算和代扣代缴等流程，让创作者激励可以合规发放。税费以订单税费试算返回为准。
        </p>

        <div v-if="visibleTaxDetails.length" class="quote-details">
          <strong>税费明细</strong>
          <div v-for="item in visibleTaxDetails" :key="item.label" class="quote-row">
            <span>{{ item.label }}</span>
            <strong>{{ $formatMoney(item.amount) }}</strong>
          </div>
        </div>

        <div class="alipay-transfer-tip">
          <strong>支付宝收款提醒</strong>
          <p>
            请确认支付宝已允许陌生人通过手机号、邮箱向您转账。可在支付宝首页搜索
            <strong>“隐私”</strong
            >，进入第一个结果，选择<strong>常用隐私设置</strong>，开启对应转账权限。
          </p>
        </div>
      </div>

      <div class="modal-actions">
        <ButtonStyled color="green">
          <button :disabled="submitting || !canSubmit" type="button" @click="submitWithdraw">
            <TransferIcon aria-hidden="true" />
            {{ submitting ? "提交中..." : "确认提交" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" type="button" @click="confirmWithdrawModal?.hide()">
            取消
          </button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <Breadcrumbs
        current-title="提现"
        :link-stack="[{ href: '/dashboard/revenue', label: '收益' }]"
      />

      <!-- 前置条件未达成 -->
      <template v-if="!profile.kyc_completed || profile.sign_status !== 'signed'">
        <h2>提现暂未开放</h2>
        <p>提现前需要先完成以下步骤：</p>
        <ul class="checklist">
          <li :class="{ done: profile.kyc_completed }">
            <CheckIcon v-if="profile.kyc_completed" />
            <RadioButtonIcon v-else />
            完善实名信息与支付宝账号
          </li>
          <li :class="{ done: profile.sign_status === 'signed' }">
            <CheckIcon v-if="profile.sign_status === 'signed'" />
            <RadioButtonIcon v-else />
            完成 H5 实名签约
          </li>
        </ul>
        <div class="button-group">
          <nuxt-link to="/dashboard/revenue" class="iconified-button brand-button">
            <UserIcon /> 前往「实名 &amp; 签约」
          </nuxt-link>
          <nuxt-link to="/dashboard/revenue" class="iconified-button"> <XIcon /> 返回 </nuxt-link>
        </div>
      </template>

      <!-- 提现表单 -->
      <template v-else>
        <h2>提现到支付宝</h2>
        <p>
          提现到支付宝账号
          <strong>{{ profile.alipay_account_masked }}</strong>
          （户名：{{ profile.real_name }}）。
        </p>

        <div class="balance-block">
          <div>
            <span class="label">可提现余额</span>
            <strong class="balance-value">{{ $formatMoney(userBalance.available) }}</strong>
          </div>
          <div>
            <span class="label">待结算</span>
            <strong>{{ $formatMoney(userBalance.pending) }}</strong>
          </div>
        </div>

        <h3>提现金额</h3>
        <div class="amount-row">
          <span class="prefix">¥</span>
          <input
            v-model="amountInput"
            type="text"
            inputmode="decimal"
            :placeholder="`最低 ¥${minWithdraw}`"
            @input="onAmountInput"
          />
        </div>
        <p class="hint">
          手续费按提现金额的 3% 从本次提现金额中扣除。点击确认提现后会向云账户试算本次税费。最低提现
          ¥{{ minWithdraw }}，单笔最高 ¥{{ maxWithdraw }}。
        </p>
        <div class="quick-amounts">
          <button
            v-for="v in quickValues"
            :key="v"
            type="button"
            class="iconified-button"
            @click="amountInput = String(v)"
          >
            ¥{{ v }}
          </button>
          <button
            type="button"
            class="iconified-button"
            @click="amountInput = maxAllowed.toFixed(2)"
          >
            全部提现
          </button>
        </div>

        <div class="agreements">
          <Checkbox v-model="agreedTransfer" description="确认转账">
            我确认将
            <strong>{{ $formatMoney(parsedAmount) }}</strong>
            申请提现至支付宝账号 {{ profile.alipay_account_masked }}（户名
            {{ profile.real_name }}），服务费将从本次提现金额中扣除
          </Checkbox>
          <Checkbox v-model="agreedTerms" description="同意条款">
            我已阅读并同意
            <nuxt-link to="/legal/incentive" class="text-link" target="_blank"
              >《创作者激励计划协议》</nuxt-link
            >
          </Checkbox>
        </div>

        <p v-for="(error, i) in knownErrors" :key="i" class="invalid">{{ error }}</p>

        <div class="button-group">
          <nuxt-link to="/dashboard/revenue" class="iconified-button"> <XIcon /> 取消 </nuxt-link>
          <button
            class="iconified-button brand-button"
            :disabled="!canOpenConfirm || submitting || quoteLoading"
            @click="openConfirmWithdraw"
          >
            <TransferIcon />
            {{ quoteLoading ? "测算中..." : submitting ? "提交中..." : "确认提现" }}
          </button>
        </div>
      </template>
    </section>
  </div>
</template>

<script setup>
import { TransferIcon, XIcon, CheckIcon, RadioButtonIcon, UserIcon } from "@modrinth/assets";
import { Breadcrumbs, ButtonStyled, Checkbox, NewModal } from "@modrinth/ui";

const data = useNuxtApp();

const minWithdraw = 5;
const maxWithdraw = 50000;
const quickValues = [5, 50, 100, 500, 1000];

const [{ data: userBalance, refresh: refreshBalance }, { data: profile }] = await Promise.all([
  useAsyncData("payout/balance", () => useBaseFetch("payout/balance", { apiVersion: 3 })),
  useAsyncData("yunzhanghu/profile", () => useBaseFetch("yunzhanghu/profile", { apiVersion: 3 })),
]);

const amountInput = ref("");
const agreedTransfer = ref(false);
const agreedTerms = ref(false);
const submitting = ref(false);
const confirmWithdrawModal = ref(null);
const payoutQuote = ref(null);
const quoteLoading = ref(false);

function onAmountInput() {
  // 只保留数字和一个小数点
  amountInput.value = amountInput.value.replace(/[^\d.]/g, "");
}

const parsedAmount = computed(() => {
  const v = parseFloat(amountInput.value);
  return isFinite(v) && v > 0 ? Math.floor(v * 100) / 100 : 0;
});

const maxAllowed = computed(() => {
  return Math.min(userBalance.value?.available || 0, maxWithdraw);
});

const userServiceFee = computed(() => Number(payoutQuote.value?.user_fee || 0));
const totalDebit = computed(() => parsedAmount.value);
const quoteAfterTaxAmount = computed(() =>
  payoutQuote.value?.after_tax_amount == null
    ? Number(payoutQuote.value?.arrival_amount || parsedAmount.value - userServiceFee.value)
    : Number(payoutQuote.value.after_tax_amount),
);
const quoteTaxTotal = computed(() => Number(payoutQuote.value?.user_tax || 0));
const visibleTaxDetails = computed(() => {
  const detail = payoutQuote.value?.tax_detail || {};
  return [
    ["个税", detail.user_personal_tax],
    ["附加税费", detail.user_additional_tax],
  ]
    .map(([label, amount]) => ({ label, amount: Number(amount || 0) }))
    .filter((item) => item.amount > 0);
});

const knownErrors = computed(() => {
  const errs = [];
  if (!amountInput.value) return errs;
  if (!parsedAmount.value) {
    errs.push(`请输入合法的提现金额`);
    return errs;
  }
  if (parsedAmount.value < minWithdraw) {
    errs.push(`提现金额不能低于 ¥${minWithdraw}`);
  }
  if (parsedAmount.value > maxAllowed.value) {
    errs.push(`提现金额不能超过 ¥${maxAllowed.value.toFixed(2)}`);
  }
  return errs;
});

const canOpenConfirm = computed(
  () =>
    parsedAmount.value >= minWithdraw &&
    parsedAmount.value <= maxAllowed.value &&
    agreedTransfer.value &&
    agreedTerms.value,
);

const canSubmit = computed(
  () =>
    canOpenConfirm.value &&
    !!payoutQuote.value &&
    totalDebit.value <= (userBalance.value?.available || 0),
);

watch(parsedAmount, () => {
  payoutQuote.value = null;
});

async function openConfirmWithdraw() {
  if (!canOpenConfirm.value || submitting.value || quoteLoading.value) return;
  quoteLoading.value = true;
  payoutQuote.value = null;
  try {
    payoutQuote.value = await useBaseFetch("payout/quote", {
      method: "POST",
      body: {
        amount: parsedAmount.value,
        method: "yunzhanghu_alipay",
      },
      apiVersion: 3,
    });
    confirmWithdrawModal.value?.show();
  } catch (err) {
    data.$notify({
      group: "main",
      title: "试算失败",
      text: err?.data?.description || err?.message || "云账户试算失败",
      type: "error",
    });
  } finally {
    quoteLoading.value = false;
  }
}

async function submitWithdraw() {
  if (!canSubmit.value || submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch("payout", {
      method: "POST",
      body: {
        amount: parsedAmount.value,
        method: "yunzhanghu_alipay",
      },
      apiVersion: 3,
    });
    confirmWithdrawModal.value?.hide();
    await refreshBalance();
    data.$notify({
      group: "main",
      title: "提现申请已提交",
      text: "申请已进入处理中，等待管理员核对并确认转账。可在转账记录页查看进度。",
      type: "success",
    });
    await navigateTo("/dashboard/revenue/transfers");
  } catch (err) {
    data.$notify({
      group: "main",
      title: "提现失败",
      text: err?.data?.description || err?.message || "未知错误",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
}

useHead({ title: "提现 - BBSMC" });
</script>

<style lang="scss" scoped>
.confirm-body {
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);
}

.confirm-grid {
  display: grid;
  grid-template-columns: 6rem minmax(0, 1fr);
  gap: var(--gap-xs) var(--gap-md);
  align-items: center;
}

.label {
  color: var(--color-text);
}

.monospace {
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}

.confirm-note {
  margin: 0;
  color: var(--color-text);
}

.quote-details {
  display: grid;
  gap: var(--gap-xs);
  padding: var(--gap-sm) var(--gap-md);
  background-color: var(--color-bg);
  border-radius: var(--radius-sm);
}

.quote-row {
  display: flex;
  justify-content: space-between;
  gap: var(--gap-md);
  color: var(--color-text);
}

.alipay-transfer-tip {
  padding: var(--gap-sm) var(--gap-md);
  border-left: 3px solid var(--color-brand);
  border-radius: var(--radius-sm);
  background-color: var(--color-bg);
  font-size: var(--font-size-sm);

  p {
    margin: var(--gap-xs) 0 0;
    color: var(--color-text);
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--gap-sm);
  margin-top: var(--gap-lg);
}

.checklist {
  list-style: none;
  padding: 0;
  margin: var(--gap-md) 0;

  li {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: var(--gap-xs) 0;

    &.done {
      color: var(--color-green);
      text-decoration: line-through;
    }
  }
}

.balance-block {
  display: flex;
  gap: var(--gap-xl);
  margin: var(--gap-md) 0;
  padding: var(--gap-md);
  background-color: var(--color-bg);
  border-radius: var(--radius-md);

  .label {
    display: block;
    color: var(--color-text);
    font-size: var(--font-size-sm);
  }

  .balance-value {
    color: var(--color-brand);
    font-size: 1.5rem;
  }
}

.amount-row {
  display: flex;
  align-items: center;
  gap: var(--gap-sm);
  max-width: 24rem;

  .prefix {
    font-size: 1.25rem;
    color: var(--color-text-dark);
  }

  input {
    flex: 1;
    font-size: 1.25rem;
  }
}

.hint {
  color: var(--color-text);
  font-size: var(--font-size-sm);
  margin-top: var(--gap-xs);
}

.quick-amounts {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gap-sm);
  margin-top: var(--gap-sm);
}

.agreements {
  margin-top: var(--gap-lg);
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm);
}

.invalid {
  color: var(--color-red);
}

.button-group {
  margin-top: var(--spacing-card-md);
  display: flex;
  gap: var(--gap-sm);
}

@media screen and (max-width: 600px) {
  .confirm-grid {
    grid-template-columns: 1fr;
  }

  .modal-actions {
    flex-direction: column-reverse;
    align-items: stretch;
  }
}
</style>
