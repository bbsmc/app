<template>
  <div>
    <!-- 撤回确认弹窗 -->
    <NewModal ref="withdrawModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">撤回申请</div>
      </template>
      <div class="modal-content">
        <p>确认撤回当前的创作者激励申请？</p>
        <p class="hint">撤回后审核状态将变为「已撤回」，可以重新提交。</p>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="orange">
          <button :disabled="submitting" @click="doWithdraw">
            {{ submitting ? "处理中..." : "确认撤回" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="withdrawModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <!-- 管理员强制开通弹窗 -->
    <NewModal ref="forceEnableModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">强制开通激励</div>
      </template>
      <div class="modal-content">
        <p>
          资源：<b>{{ projectTitle }}</b>
        </p>
        <p class="hint">
          该操作会跳过作者申请流程，直接将此资源标记为「已开通激励」。已累计的有效下载和待结算金额会保留。
        </p>
        <p v-if="application?.status === 'pending'" class="hint">
          当前已有待审核申请，强制开通后资源会立即进入激励累计状态。
        </p>
        <div class="form-group">
          <label>备注（可选）</label>
          <textarea
            v-model="forceEnableNotes"
            rows="3"
            maxlength="500"
            placeholder="留下开通原因，会写入审计日志"
            :disabled="submitting"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="green">
          <button :disabled="submitting" @click="doForceEnable">
            {{ submitting ? "处理中..." : "确认开通" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="forceEnableModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <!-- 管理员强制关闭弹窗 -->
    <NewModal ref="forceDisableModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">强制关闭激励</div>
      </template>
      <div class="modal-content">
        <p>
          资源：<b>{{ projectTitle }}</b>
        </p>
        <p class="hint">
          关闭后该资源会停止累计新的创作者激励，已有待结算金额会保留。关闭理由会写入申请沟通记录，作者可以重新提交申请。
        </p>
        <div class="form-group">
          <label>关闭理由</label>
          <textarea
            v-model="forceDisableNotes"
            rows="3"
            maxlength="500"
            placeholder="说明关闭原因，作者会在沟通记录中看到"
            :disabled="submitting"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="red">
          <button :disabled="submitting || !forceDisableNotes.trim()" @click="doForceDisable">
            {{ submitting ? "处理中..." : "确认关闭" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="submitting" @click="forceDisableModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">创作者激励</span>
        </h3>
      </div>

      <div v-if="loading" class="loading-state">
        <UpdatedIcon class="animate-spin" />
        <span>加载中...</span>
      </div>

      <template v-else>
        <!-- 管理员只读视图（admin 但不是项目成员）-->
        <div
          v-if="overview?.viewer?.is_admin && !overview?.viewer?.is_team_member"
          class="notice-card info"
        >
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>管理员视图</strong>
            <p>你以站点管理员身份查看此项目的激励状态，无法代为提交/撤回申请。</p>
          </div>
        </div>

        <div v-if="canForceEnableIncentive" class="notice-card admin-force">
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>管理员强制开通</strong>
            <p>无需作者提交申请或等待审核，可直接为此资源开通创作者激励。</p>
            <div class="admin-action-row">
              <button class="btn btn-primary" :disabled="submitting" @click="openForceEnable">
                强制开通激励
              </button>
              <span class="admin-action-hint">操作会写入后台审计日志。</span>
            </div>
          </div>
        </div>

        <div v-if="canForceDisableIncentive" class="notice-card admin-disable">
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>管理员强制关闭</strong>
            <p>关闭后停止累计新的激励，作者可以在沟通记录中看到关闭理由并重新申请。</p>
            <div class="admin-action-row">
              <button class="btn btn-danger" :disabled="submitting" @click="openForceDisable">
                强制关闭激励
              </button>
              <span class="admin-action-hint">关闭理由会写入申请沟通记录。</span>
            </div>
          </div>
        </div>

        <!-- 已开通 -->
        <div v-if="overview?.incentive_enabled" class="notice-card success">
          <CheckIcon class="notice-icon" />
          <div class="notice-content">
            <strong>该资源已开通创作者激励</strong>
            <p>
              用户的有效下载会自动产生激励金额，满 7 天后结算为收益余额，提现每月初统一处理到账。
            </p>
          </div>
        </div>

        <!-- 申请审核中 -->
        <div v-else-if="application?.status === 'pending'" class="notice-card warning">
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>申请审核中</strong>
            <p>提交时间：{{ formatDate(application.created_at) }}</p>
            <button
              v-if="overview?.viewer?.can_manage"
              class="btn btn-secondary"
              :disabled="submitting"
              @click="withdraw"
            >
              撤回申请
            </button>
          </div>
        </div>

        <!-- 上次被拒 -->
        <div v-else-if="application?.status === 'rejected'" class="notice-card danger">
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>上次申请被拒</strong>
            <p v-if="application.review_notes"><b>原因：</b>{{ application.review_notes }}</p>
            <p>你可以根据建议修改后重新申请。</p>
          </div>
        </div>

        <!-- thread 沟通区（有申请就显示） -->
        <details
          v-if="application?.thread_id && thread"
          class="thread-section"
          :open="!overview?.incentive_enabled"
        >
          <summary class="thread-summary">
            <span>与管理员沟通</span>
            <span v-if="thread.messages?.length" class="thread-count">
              {{ thread.messages.length }} 条消息
            </span>
          </summary>

          <div class="messages-list">
            <div
              v-for="m in thread.messages"
              :key="m.id"
              class="message"
              :class="{ 'is-system': !m.author_id, 'is-mine': m.author_id === currentUserId }"
            >
              <div class="message-meta">
                <span class="author">
                  <template v-if="!m.author_id">系统</template>
                  <template v-else-if="m.author_id === currentUserId">我</template>
                  <template v-else>管理员</template>
                </span>
                <span class="time">{{ formatDate(m.created) }}</span>
              </div>
              <div class="message-body">{{ extractText(m.body) }}</div>
            </div>
            <div v-if="!thread.messages?.length" class="empty">暂无消息</div>
          </div>

          <div
            v-if="application.status === 'pending' && overview?.viewer?.can_manage"
            class="reply-area"
          >
            <textarea
              v-model="replyText"
              rows="2"
              maxlength="1000"
              placeholder="发送消息给管理员..."
              :disabled="sending"
            />
            <button
              class="btn btn-primary"
              :disabled="!replyText.trim() || sending"
              @click="sendReply"
            >
              发送
            </button>
          </div>
        </details>

        <div v-if="showIncentiveIntro" class="incentive-intro">
          <div class="intro-heading">
            <InfoIcon class="intro-icon" aria-hidden="true" />
            <div>
              <h4>创作者激励介绍</h4>
              <p>
                开通后，真实有效下载会按阶梯单价计入激励。金额待结算 7
                天并通过审计后转入收益余额，提现每月初处理到账。
              </p>
            </div>
            <NuxtLink to="/legal/incentive-info" target="_blank" class="intro-link">
              查看完整介绍
            </NuxtLink>
          </div>

          <div class="cooperation-highlight">
            <div class="cooperation-copy">
              <strong>服务商合作重点</strong>
              <p>
                开通激励不代表已经达成服务商合作。作者可扫码添加昱通云联企业微信客服，
                洽谈服务器面板销售服务商合作，并由客服创建技术支持群聊。
              </p>
              <ul>
                <li>服务商合作：资源广告入口下单可获得 25% 销售分成。</li>
                <li>
                  深度广告合作：整合包广告模组、客户端服务器列表或单人游戏页面广告植入可获得 30%
                  销售分成。
                </li>
                <li>技术支持：可咨询整合包模组 bug、简单兼容修复、服务端制作和联机卡顿调优。</li>
              </ul>
            </div>
            <div class="cooperation-qr">
              <img
                src="/yutong-yunlian-wechat.png"
                alt="昱通云联企业微信客服二维码"
                width="150"
                height="150"
              />
              <span>扫码添加企业微信客服</span>
            </div>
          </div>
        </div>

        <!-- 30 天图表 -->
        <div v-if="overview?.last_30_days?.length" class="chart-section">
          <h4>近 30 天数据</h4>
          <div class="chart-grid">
            <client-only>
              <Chart
                name="有效下载"
                type="bar"
                :labels="chartLabels"
                :data="downloadsChartData"
                :colors="['var(--color-brand)']"
                :hide-toolbar="true"
                :hide-legend="true"
              />
            </client-only>
            <client-only>
              <Chart
                name="每日激励金额"
                type="bar"
                :labels="chartLabels"
                :data="amountChartData"
                :colors="['var(--color-green)']"
                prefix="¥"
                :hide-toolbar="true"
                :hide-legend="true"
              />
            </client-only>
          </div>
        </div>

        <!-- 数据展示 -->
        <div v-if="overview" class="incentive-stats">
          <h4>当前激励状态</h4>
          <div class="stat-row">
            <span class="label">累计有效下载</span>
            <span class="value">{{ overview.lifetime_eff_downloads }}</span>
          </div>
          <div class="stat-row">
            <span class="label">当前单价</span>
            <span class="value">{{ overview.current_unit_payout }} 元/次</span>
          </div>
          <div v-if="overview.next_tier_remaining" class="stat-row">
            <span class="label">距离下一档位</span>
            <span class="value">还需 {{ overview.next_tier_remaining }} 次有效下载</span>
          </div>
          <div class="stat-row">
            <span class="label">结算窗口期</span>
            <span class="value">7 个自然日</span>
          </div>
          <div class="stat-row">
            <span class="label">待结算金额</span>
            <span class="value money">{{ overview.pending_amount }} 元</span>
          </div>
          <div class="stat-row">
            <span class="label">已结算金额</span>
            <span class="value money">{{ overview.settled_amount }} 元</span>
          </div>

          <div class="settlement-help">
            <InfoIcon class="help-icon" aria-hidden="true" />
            <div>
              <strong>结算与提现</strong>
              <p>
                有效下载产生的金额会先进入待结算，满 7
                个自然日并通过反作弊审计后，系统会自动转入收益余额。提现统一在每个月初批量处理到账，
                实名签约和收款账号处理请前往收益页面完成。
              </p>
              <NuxtLink to="/dashboard/revenue" class="iconified-button brand-button">
                <RightArrowIcon aria-hidden="true" />
                前往收益提现
              </NuxtLink>
            </div>
          </div>
        </div>

        <!-- 未开通且无 pending 申请，但查看者无管理权 → 提示 -->
        <div
          v-if="
            !overview?.incentive_enabled &&
            application?.status !== 'pending' &&
            !overview?.viewer?.can_manage
          "
          class="notice-card info"
        >
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>该资源尚未申请创作者激励</strong>
            <p v-if="overview?.viewer?.is_team_member">
              你的团队权限不足，无法提交申请（需要「编辑项目详情」权限）。
            </p>
            <p v-else-if="overview?.viewer?.is_admin">站点管理员只能查看，不能代为申请。</p>
          </div>
        </div>

        <div
          v-if="
            !overview?.incentive_enabled &&
            application?.status !== 'pending' &&
            overview?.viewer?.can_manage &&
            !canApplyIncentive
          "
          class="notice-card warning"
        >
          <InfoIcon class="notice-icon" />
          <div class="notice-content">
            <strong>资源通过审核后才能申请创作者激励</strong>
            <p>
              当前资源状态为「{{
                projectStatusLabel
              }}」。草稿、审核中、已拒绝或被暂扣的资源不能提交激励申请。
            </p>
          </div>
        </div>

        <!-- 申请表单（未开通、无待审核申请、且查看者有 manage 权限） -->
        <template v-if="showApplyForm">
          <div class="apply-form">
            <h4>申请开通创作者激励</h4>
            <div class="rules-card">
              <p><b>激励规则</b></p>
              <ul>
                <li>
                  每次有效下载产生激励金额：累计下载数 0-1000 次每次 0.02 元，1000-10000 次每次 0.01
                  元，10000 次以上每次 0.008 元
                </li>
                <li>同一用户/IP 段对单个项目每 7 天最多记一次有效下载</li>
                <li>项目团队成员自下载不计激励</li>
                <li>金额在事件发生 7 天后自动结算到收益余额，提现每月初统一处理到账</li>
                <li>若发现刷量行为，未结算金额将被作废</li>
              </ul>
              <p class="agreement-hint">
                提交申请即视为已阅读并同意
                <NuxtLink to="/legal/incentive" target="_blank">《创作者激励计划协议》</NuxtLink>。
              </p>
            </div>

            <div class="form-group">
              <label for="reason">
                <span class="label-title">申请理由（可选）</span>
              </label>
              <textarea
                id="reason"
                v-model="reason"
                rows="4"
                maxlength="1000"
                placeholder="可以简单介绍下项目情况、运营计划等"
                :disabled="submitting"
              />
            </div>

            <button class="btn btn-primary" :disabled="submitting" @click="submit">提交申请</button>
          </div>
        </template>
      </template>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from "vue";
import { InfoIcon, CheckIcon, RightArrowIcon, UpdatedIcon } from "@modrinth/assets";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import Chart from "~/components/ui/charts/Chart.client.vue";

const props = defineProps({
  project: {
    type: Object,
    required: true,
  },
});

const nuxtApp = useNuxtApp();
const auth = await useAuth();
const currentUserId = auth.value?.user?.id;

const loading = ref(true);
const submitting = ref(false);
const sending = ref(false);
const overview = ref(null);
const application = ref(null);
const thread = ref(null);
const reason = ref("");
const replyText = ref("");
const forceEnableNotes = ref("");
const forceDisableNotes = ref("");

const approvedProjectStatuses = new Set(["approved", "unlisted", "archived", "private"]);
const projectStatusLabels = {
  approved: "已发布",
  unlisted: "未列出",
  archived: "已归档",
  private: "私有",
  draft: "草稿",
  processing: "审核中",
  rejected: "已拒绝",
  withheld: "已暂扣",
  scheduled: "已计划",
};
const canApplyIncentive = computed(() => approvedProjectStatuses.has(props.project?.status));
const canForceEnableIncentive = computed(
  () => overview.value?.viewer?.is_admin && !overview.value?.incentive_enabled,
);
const canForceDisableIncentive = computed(
  () => overview.value?.viewer?.is_admin && overview.value?.incentive_enabled,
);
const showApplyForm = computed(
  () =>
    !overview.value?.incentive_enabled &&
    application.value?.status !== "pending" &&
    overview.value?.viewer?.can_manage &&
    canApplyIncentive.value,
);
const showIncentiveIntro = computed(
  () =>
    overview.value?.incentive_enabled ||
    application.value?.status === "pending" ||
    showApplyForm.value,
);
const projectTitle = computed(
  () => props.project?.title || props.project?.name || props.project?.id,
);
const projectStatusLabel = computed(
  () => projectStatusLabels[props.project?.status] || props.project?.status || "未知",
);

const formatDate = (s) =>
  s ? new Date(s).toLocaleString("zh-CN", { timeZone: "Asia/Shanghai" }) : "";

// 把 last_30_days 转成 Chart 组件需要的格式
const chartLabels = computed(() => overview.value?.last_30_days?.map((d) => d.date) || []);

const downloadsChartData = computed(() => [
  {
    name: "有效下载",
    data:
      overview.value?.last_30_days?.map((d) => ({
        x: new Date(d.date).getTime(),
        y: d.effective_downloads,
      })) || [],
  },
]);

const amountChartData = computed(() => [
  {
    name: "激励金额（元）",
    data:
      overview.value?.last_30_days?.map((d) => ({
        x: new Date(d.date).getTime(),
        // 保留 2 位小数避免显示如 0.02000001
        y: parseFloat(parseFloat(d.daily_amount || "0").toFixed(2)),
      })) || [],
  },
]);

const extractText = (body) => {
  if (!body) return "";
  if (typeof body === "string") return body;
  if (body.body) return body.body;
  return JSON.stringify(body);
};

const loadAll = async () => {
  loading.value = true;
  try {
    const [detail, appl] = await Promise.allSettled([
      useBaseFetch(`dashboard/incentive/${props.project.id}`, {
        method: "GET",
        apiVersion: 3,
      }),
      useBaseFetch(`project/${props.project.id}/incentive/application`, {
        method: "GET",
        apiVersion: 3,
      }),
    ]);

    overview.value = detail.status === "fulfilled" ? detail.value : null;
    application.value = appl.status === "fulfilled" && appl.value ? appl.value : null;

    if (application.value?.thread_id) {
      try {
        thread.value = await useBaseFetch(`thread/${application.value.thread_id}`, {
          method: "GET",
        });
      } catch (e) {
        thread.value = null;
      }
    } else {
      thread.value = null;
    }
  } catch (e) {
    console.error("加载激励信息失败", e);
  } finally {
    loading.value = false;
  }
};

const sendReply = async () => {
  if (!replyText.value.trim() || sending.value || !application.value?.thread_id) return;
  sending.value = true;
  try {
    await useBaseFetch(`thread/${application.value.thread_id}`, {
      method: "POST",
      body: {
        body: {
          type: "text",
          body: replyText.value.trim(),
          private: false,
          replying_to: null,
        },
      },
    });
    replyText.value = "";
    thread.value = await useBaseFetch(`thread/${application.value.thread_id}`, { method: "GET" });
  } catch (e) {
    nuxtApp.$notify({
      group: "main",
      title: "发送失败",
      text: e?.data?.description || "无法发送消息",
      type: "error",
    });
  } finally {
    sending.value = false;
  }
};

const submit = async () => {
  if (submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(`project/${props.project.id}/incentive/apply`, {
      method: "POST",
      apiVersion: 3,
      body: { reason: reason.value || null },
    });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "申请已提交，等待版主审核",
      type: "success",
    });
    reason.value = "";
    await loadAll();
  } catch (e) {
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: e?.data?.description || "提交失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

const withdrawModal = ref(null);
const forceEnableModal = ref(null);
const forceDisableModal = ref(null);

const withdraw = () => {
  if (submitting.value) return;
  withdrawModal.value?.show();
};

const openForceEnable = () => {
  if (submitting.value) return;
  forceEnableNotes.value = "";
  forceEnableModal.value?.show();
};

const openForceDisable = () => {
  if (submitting.value) return;
  forceDisableNotes.value = "";
  forceDisableModal.value?.show();
};

const doForceEnable = async () => {
  if (submitting.value || !canForceEnableIncentive.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(`admin/projects/${props.project.id}/incentive`, {
      method: "PATCH",
      internal: true,
      body: {
        enable: true,
        notes: forceEnableNotes.value || null,
      },
    });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "激励已开通",
      type: "success",
    });
    forceEnableModal.value?.hide();
    await loadAll();
  } catch (e) {
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: e?.data?.description || "开通失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

const doForceDisable = async () => {
  if (submitting.value || !canForceDisableIncentive.value || !forceDisableNotes.value.trim()) {
    return;
  }
  submitting.value = true;
  try {
    await useBaseFetch(`admin/projects/${props.project.id}/incentive`, {
      method: "PATCH",
      internal: true,
      body: {
        enable: false,
        void_pending: false,
        notes: forceDisableNotes.value.trim(),
      },
    });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "激励已关闭，作者可以重新申请",
      type: "success",
    });
    forceDisableModal.value?.hide();
    await loadAll();
  } catch (e) {
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: e?.data?.description || "关闭失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

const doWithdraw = async () => {
  if (submitting.value) return;
  submitting.value = true;
  try {
    await useBaseFetch(`project/${props.project.id}/incentive/application`, {
      method: "DELETE",
      apiVersion: 3,
    });
    nuxtApp.$notify({
      group: "main",
      title: "成功",
      text: "申请已撤回",
      type: "success",
    });
    withdrawModal.value?.hide();
    await loadAll();
  } catch (e) {
    nuxtApp.$notify({
      group: "main",
      title: "错误",
      text: e?.data?.description || "撤回失败",
      type: "error",
    });
  } finally {
    submitting.value = false;
  }
};

onMounted(loadAll);
</script>

<style lang="scss" scoped>
.notice-card {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1rem;
  margin-bottom: 1rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);

  &.success {
    background: var(--color-green-bg);
    border-color: var(--color-green);
  }
  &.warning {
    background: var(--color-orange-bg);
    border-color: var(--color-orange);
  }
  &.danger {
    background: var(--color-red-bg);
    border-color: var(--color-red);
  }
  &.info {
    background: var(--color-blue-bg, #dbeafe);
    border-color: var(--color-blue, #2563eb);
  }
  &.admin-force {
    background: var(--color-bg);
    border-color: var(--color-brand);
  }
  &.admin-disable {
    background: var(--color-bg);
    border-color: var(--color-red);
  }

  .notice-icon {
    flex-shrink: 0;
    width: 1.5rem;
    height: 1.5rem;
  }
  .notice-content {
    flex: 1;
    p {
      margin: 0.25rem 0;
    }

    .admin-action-row {
      display: flex;
      align-items: center;
      flex-wrap: wrap;
      gap: 0.75rem;
      margin-top: 0.75rem;
    }

    .admin-action-hint {
      color: var(--color-text-secondary);
      font-size: var(--font-size-sm);
    }
  }
}

.chart-section {
  margin-top: 1.5rem;
  h4 {
    margin-bottom: 0.75rem;
  }
  .chart-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;

    @media (max-width: 900px) {
      grid-template-columns: 1fr;
    }
  }
}

.incentive-stats {
  margin-top: 1.5rem;
  h4 {
    margin-bottom: 0.75rem;
  }
  .stat-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--color-divider);

    .label {
      color: var(--color-text-secondary);
    }
    .value {
      font-weight: 600;
      &.money {
        color: var(--color-green);
      }
    }
  }

  .settlement-help {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    margin-top: 1rem;
    padding: 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-green);
    background: var(--color-green-bg, rgba(34, 197, 94, 0.1));

    .help-icon {
      flex-shrink: 0;
      width: 1.25rem;
      height: 1.25rem;
      margin-top: 0.1rem;
      color: var(--color-green);
    }

    p {
      margin: 0.25rem 0 0.75rem;
      color: var(--color-text-secondary);
    }
  }
}

.incentive-intro {
  margin-top: 1rem;
  padding: 1rem;
  border: 1px solid var(--color-brand);
  border-radius: var(--radius-md);
  background: var(--color-bg);

  .intro-heading {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;

    h4 {
      margin: 0 0 0.35rem;
    }

    p {
      margin: 0;
      color: var(--color-text-secondary);
    }
  }

  .intro-icon {
    flex-shrink: 0;
    width: 1.25rem;
    height: 1.25rem;
    margin-top: 0.15rem;
    color: var(--color-brand);
  }

  .intro-link {
    flex-shrink: 0;
    margin-left: auto;
    color: var(--color-link);
    font-weight: 600;
    text-decoration: underline;

    &:hover {
      color: var(--color-link-hover);
    }
  }

  .cooperation-highlight {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-top: 1rem;
    padding: 1rem;
    border: 1px solid var(--color-orange);
    border-radius: var(--radius-md);
    background: var(--color-orange-bg, rgba(245, 158, 11, 0.1));
  }

  .cooperation-copy {
    flex: 1;

    p {
      margin: 0.35rem 0 0.5rem;
      color: var(--color-text-secondary);
    }

    ul {
      margin: 0.5rem 0 0;
      padding-left: 1.1rem;
    }

    li {
      margin-bottom: 0.35rem;
    }
  }

  .cooperation-qr {
    display: flex;
    align-items: center;
    flex-direction: column;
    flex-shrink: 0;
    gap: 0.4rem;
    width: 160px;
    color: var(--color-text-secondary);
    font-size: 0.85rem;
    text-align: center;

    img {
      width: 150px;
      height: 150px;
      border: 1px solid var(--color-divider);
      border-radius: var(--radius-md);
      background: var(--color-raised-bg);
      object-fit: contain;
    }
  }

  @media screen and (max-width: 700px) {
    .intro-heading,
    .cooperation-highlight {
      flex-direction: column;
    }

    .intro-link {
      margin-left: 0;
    }

    .cooperation-qr {
      align-items: flex-start;
      width: 100%;
      text-align: left;
    }
  }
}

.apply-form {
  margin-top: 1.5rem;
  h4 {
    margin-bottom: 0.75rem;
  }

  .agreement-hint {
    margin-top: 0.75rem;
    margin-bottom: 0;
    color: var(--color-text-secondary);
    font-size: 0.9rem;
    a {
      color: var(--color-link);
      text-decoration: underline;
      &:hover {
        color: var(--color-link-hover);
      }
    }
  }
  .rules-card {
    background: var(--color-bg);
    padding: 1rem;
    border-radius: var(--radius-md);
    margin-bottom: 1rem;
    ul {
      margin: 0.5rem 0 0 1rem;
      padding: 0;
      li {
        margin-bottom: 0.25rem;
        font-size: 0.9rem;
        color: var(--color-text-secondary);
      }
    }
  }

  .form-group {
    margin-bottom: 1rem;
    label {
      display: block;
      margin-bottom: 0.5rem;
      font-weight: 500;
    }
    textarea {
      width: 100%;
      padding: 0.5rem;
      border: 1px solid var(--color-divider);
      border-radius: var(--radius-sm);
      resize: vertical;
    }
  }
}

.loading-state {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 2rem;
  justify-content: center;
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

  .form-group {
    margin-top: 1rem;

    label {
      display: block;
      margin-bottom: 0.5rem;
      font-weight: 500;
    }

    textarea {
      width: 100%;
      padding: 0.5rem;
      border: 1px solid var(--color-divider);
      border-radius: var(--radius-sm);
      resize: vertical;
    }
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  padding: 1rem;
  justify-content: flex-end;
}

.thread-section {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);

  .thread-summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    color: var(--color-heading);
    font-weight: 700;
    cursor: pointer;
    user-select: none;

    .thread-count {
      color: var(--color-text-secondary);
      font-size: var(--font-size-sm);
      font-weight: 500;
    }
  }

  .messages-list {
    max-height: 360px;
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

        .author {
          font-weight: 600;
        }
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
    button {
      flex-shrink: 0;
    }
  }
}
</style>
