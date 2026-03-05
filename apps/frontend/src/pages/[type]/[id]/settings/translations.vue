<template>
  <div>
    <!-- 撤销确认弹窗 -->
    <ConfirmModal
      ref="modalConfirmRevoke"
      title="确认撤销翻译链接"
      :description="`您确定要撤销该翻译链接吗？撤销后该链接将重新进入待审核状态。`"
      proceed-label="确认撤销"
      :has-to-type="false"
      @proceed="confirmRevoke"
    />

    <!-- 拒绝原因弹窗 -->
    <NewModal ref="rejectModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">拒绝翻译链接</div>
      </template>
      <div class="reject-content">
        <p class="text-secondary">请说明拒绝的原因，以便申请者了解并改进：</p>
        <textarea
          v-model="rejectReason"
          class="reject-textarea"
          placeholder="例如：翻译质量不符合要求、版本不匹配、资源重复等..."
          rows="5"
          required
        ></textarea>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="danger">
          <button :disabled="!rejectReason || rejectingLink" @click="confirmReject">
            <CrossIcon aria-hidden="true" />
            确认拒绝
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="$refs.rejectModal.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <section class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">翻译版本审核</span>
        </h3>
        <span class="label__description">
          管理其他项目提交的翻译版本链接，批准或拒绝它们的关联请求
        </span>
      </div>

      <!-- 筛选组件 -->
      <div class="filter-section">
        <Chips v-model="statusFilter" :items="statusOptions" :format-label="formatStatusLabel" />
        <p class="filter-count">
          {{ formatStatusLabel(statusFilter) }}: {{ filteredLinks.length }} 个
        </p>
      </div>

      <!-- 过滤后的链接列表 -->
      <div v-if="filteredLinks.length > 0" class="filtered-links">
        <div class="links-list">
          <div
            v-for="link in filteredLinks"
            :key="link.id"
            class="link-item"
            :class="{ rejected: link.approval_status === 'rejected' }"
          >
            <div class="link-info">
              <div class="link-header">
                <nuxt-link
                  :to="`/project/${link.project_slug}/version/${link.version_number}`"
                  class="link-title"
                >
                  <Avatar
                    :src="link.project_icon"
                    :alt="link.project_title"
                    size="sm"
                    class="project-icon"
                  />
                  <div class="link-details">
                    <span class="project-name">{{ link.project_title }}</span>
                    <span class="version-info">v{{ link.version_number }}</span>
                  </div>
                </nuxt-link>
                <div class="link-meta">
                  <span class="language-badge">
                    {{ getLanguageName(link.language_code) }}
                  </span>
                  <span class="link-type">{{ link.link_type }}</span>
                  <span v-if="link.approval_status === 'approved'" class="status-badge approved"
                    >已通过</span
                  >
                  <span
                    v-else-if="link.approval_status === 'rejected'"
                    class="status-badge rejected"
                    >已拒绝</span
                  >
                  <span v-else class="status-badge pending">待审核</span>
                </div>
              </div>

              <div v-if="link.description" class="link-description">
                {{ link.description }}
              </div>

              <div class="link-target">
                <span class="target-label">目标版本：</span>
                <nuxt-link
                  v-if="project && project.slug"
                  :to="`/${$getProjectTypeForUrl(project.project_type, project.loaders)}/${project.slug}/version/${link.target_version}`"
                  class="target-link"
                >
                  {{ link.target_version }}
                </nuxt-link>
                <span v-else>{{ link.target_version }}</span>
              </div>

              <div class="link-submitter">
                <span class="submitter-label">提交者：</span>
                <nuxt-link :to="`/user/${link.submitter_username}`" class="submitter-link">
                  <Avatar
                    :src="link.submitter_avatar"
                    :alt="link.submitter_username"
                    size="xs"
                    circle
                  />
                  {{ link.submitter_username }}
                </nuxt-link>
                <span class="submit-time">{{ fromNow(link.created) }}</span>
              </div>
            </div>

            <!-- 根据状态显示不同的操作按钮 -->
            <div class="link-actions">
              <template v-if="link.approval_status === 'pending'">
                <button
                  class="btn btn-primary"
                  :disabled="processingLinks.includes(link.id)"
                  @click="approveLink(link)"
                >
                  <CheckIcon aria-hidden="true" />
                  批准
                </button>
                <button
                  class="btn btn-danger"
                  :disabled="processingLinks.includes(link.id)"
                  @click="openRejectModal(link)"
                >
                  <CrossIcon aria-hidden="true" />
                  拒绝
                </button>
              </template>
              <template v-else-if="link.approval_status === 'approved'">
                <button
                  class="btn btn-danger btn-small"
                  :disabled="processingLinks.includes(link.id)"
                  @click="revokeLink(link)"
                >
                  <TrashIcon aria-hidden="true" />
                  撤销
                </button>
              </template>
              <button class="btn btn-secondary" @click="toggleThread(link)">
                <MessageIcon aria-hidden="true" />
                {{ expandedThreads.includes(link.id) ? "隐藏" : "查看" }}消息
              </button>
            </div>

            <!-- Thread 消息区域 -->
            <div v-if="expandedThreads.includes(link.id)" class="thread-section">
              <div class="thread-header">
                <h5>审核消息</h5>
                <span class="thread-description">与提交者的对话记录</span>
              </div>
              <div v-if="threads[link.id]" class="thread-messages">
                <div
                  v-if="threads[link.id].messages && threads[link.id].messages.length > 0"
                  class="messages-list"
                >
                  <div
                    v-for="message in threads[link.id].messages"
                    :key="message.id"
                    class="message-item"
                    :class="{
                      'mod-message':
                        message.author_id && isStaff(getMessageAuthor(message, threads[link.id])),
                    }"
                  >
                    <div class="message-header">
                      <div class="message-author">
                        <Avatar
                          v-if="getMessageAuthor(message, threads[link.id])"
                          :src="getMessageAuthor(message, threads[link.id]).avatar_url"
                          :alt="getMessageAuthor(message, threads[link.id]).username"
                          size="xs"
                          circle
                        />
                        <span>{{
                          getMessageAuthor(message, threads[link.id])?.username || "系统"
                        }}</span>
                      </div>
                      <span class="message-time">{{ fromNow(message.created) }}</span>
                    </div>
                    <div class="message-body">
                      <template v-if="message.body.type === 'text'">
                        <div class="message-text" v-html="renderMarkdown(message.body.body)"></div>
                      </template>
                      <template v-else-if="message.body.type === 'status_change'">
                        <div class="status-change">
                          状态变更: {{ message.body.old_status }} → {{ message.body.new_status }}
                        </div>
                      </template>
                    </div>
                  </div>
                </div>
                <div v-else class="no-messages">
                  <InfoIcon aria-hidden="true" />
                  <p>暂无消息记录</p>
                </div>

                <!-- 发送消息区域 -->
                <div class="send-message">
                  <textarea
                    v-model="messageTexts[link.id]"
                    class="message-input"
                    placeholder="输入消息..."
                    rows="3"
                  ></textarea>
                  <div class="message-actions">
                    <button
                      v-if="!rejectingLinks.includes(link.id)"
                      class="btn btn-primary btn-small"
                      :disabled="!messageTexts[link.id] || sendingMessage[link.id]"
                      @click="sendMessage(link)"
                    >
                      <SendIcon aria-hidden="true" />
                      发送
                    </button>
                    <template v-else>
                      <button
                        class="btn btn-danger btn-small"
                        :disabled="!messageTexts[link.id] || sendingMessage[link.id]"
                        @click="confirmRejectWithMessage(link)"
                      >
                        <CrossIcon aria-hidden="true" />
                        确认拒绝并发送消息
                      </button>
                      <button class="btn btn-secondary btn-small" @click="cancelReject(link)">
                        取消
                      </button>
                    </template>
                  </div>
                </div>
              </div>
              <div v-else class="loading-thread">
                <UpdatedIcon aria-hidden="true" class="animate-spin" />
                <span>加载消息中...</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div v-else-if="loading" class="loading-section">
        <UpdatedIcon aria-hidden="true" class="animate-spin" />
        <span>加载中...</span>
      </div>
      <div v-else class="empty-section">
        <InfoIcon aria-hidden="true" />
        <p>
          {{
            statusFilter === "all" ? "暂无翻译链接" : `暂无${formatStatusLabel(statusFilter)}的链接`
          }}
        </p>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from "vue";
import { useRoute } from "vue-router";
import { renderString } from "@modrinth/utils";
import { ConfirmModal, NewModal, ButtonStyled } from "@modrinth/ui";
import { useAuth } from "~/composables/auth.js";
import { useBaseFetch } from "~/composables/fetch.js";
import { addNotification } from "~/composables/notifs.js";
import Avatar from "~/components/ui/Avatar.vue";
import CheckIcon from "~/assets/images/utils/check.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";
import TrashIcon from "~/assets/images/utils/trash.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UpdatedIcon from "~/assets/images/utils/updated.svg?component";
import MessageIcon from "~/assets/images/utils/message.svg?component";
import SendIcon from "~/assets/images/utils/send.svg?component";
import Chips from "~/components/ui/Chips.vue";

const props = defineProps({
  project: {
    type: Object,
    required: true,
  },
  currentMember: {
    type: Object,
    required: true,
  },
});

useHead({
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const route = useRoute();
const auth = useAuth();
const app = useNuxtApp();

// 权限常量（位标志）
const UPLOAD_VERSION = 1 << 0;
const EDIT_DETAILS = 1 << 2;

// 响应式状态
const pendingLinks = ref([]);
const approvedLinks = ref([]);
const rejectedLinks = ref([]);
const loading = ref(true);
const processingLinks = ref([]);
const expandedThreads = ref([]);
const threads = ref({});
const messageTexts = ref({});
const sendingMessage = ref({});
const rejectingLinks = ref([]); // 记录正在拒绝流程中的链接

// 弹窗引用
const modalConfirmRevoke = ref(null);
const pendingRevokeLink = ref(null); // 待撤销的链接
const rejectModal = ref(null); // 拒绝弹窗
const rejectReason = ref(""); // 拒绝原因
const rejectingLink = ref(false); // 正在拒绝中
const pendingRejectLink = ref(null); // 待拒绝的链接

// 筛选相关
const statusFilter = ref("pending"); // 默认显示待审核
const statusOptions = ["all", "pending", "approved", "rejected"];

// 权限检查
const hasPermission = computed(() => {
  return (
    (props.currentMember?.permissions & UPLOAD_VERSION) === UPLOAD_VERSION ||
    (props.currentMember?.permissions & EDIT_DETAILS) === EDIT_DETAILS ||
    auth.value?.user?.role === "admin" ||
    auth.value?.user?.role === "moderator"
  );
});

// 格式化状态标签
const formatStatusLabel = (status) => {
  switch (status) {
    case "all":
      return "全部";
    case "pending":
      return "待审核";
    case "approved":
      return "已通过";
    case "rejected":
      return "已拒绝";
    default:
      return status;
  }
};

// 根据筛选条件过滤链接
const filteredLinks = computed(() => {
  if (statusFilter.value === "all") {
    return [...pendingLinks.value, ...approvedLinks.value, ...rejectedLinks.value];
  } else if (statusFilter.value === "pending") {
    return pendingLinks.value;
  } else if (statusFilter.value === "approved") {
    return approvedLinks.value;
  } else if (statusFilter.value === "rejected") {
    return rejectedLinks.value;
  }
  return [];
});

// 获取语言名称
const getLanguageName = (code) => {
  const languages = {
    zh_CN: "简体中文",
    zh_TW: "繁体中文",
    en_US: "英语",
    ja_JP: "日语",
    ko_KR: "韩语",
  };
  return languages[code] || code;
};

// 相对时间
const fromNow = (date) => {
  return app.$dayjs(date).fromNow();
};

// 获取所有版本的翻译链接
const fetchTranslationLinks = async () => {
  loading.value = true;
  try {
    // 使用新的API端点获取所有翻译链接（包括待审核的）
    const allLinks = await useBaseFetch(`project/${route.params.id}/translation_links`);
    console.log("获取到的翻译链接:", allLinks);
    console.log("链接总数:", allLinks?.length || 0);

    if (allLinks && Array.isArray(allLinks)) {
      // 转换数据格式
      const formattedLinks = allLinks.map((link) => ({
        id: link.joining_version_id,
        project_id: link.project_id,
        project_slug: link.project_slug || link.project_id,
        project_title: link.project_title,
        project_icon: link.project_icon,
        version_number: link.version_number,
        version_id: link.joining_version_id,
        target_version: link.target_version_number,
        target_version_id: link.target_version_id,
        language_code: link.language_code,
        link_type: link.link_type,
        description: link.description,
        approval_status: link.approval_status,
        submitter_id: link.submitter_id,
        submitter_username: link.submitter_username,
        submitter_avatar: link.submitter_avatar,
        created: link.date_published,
        thread_id: link.thread_id, // 添加thread_id字段
      }));

      // 分类链接
      pendingLinks.value = formattedLinks.filter((link) => link.approval_status === "pending");
      approvedLinks.value = formattedLinks.filter((link) => link.approval_status === "approved");
      rejectedLinks.value = formattedLinks.filter((link) => link.approval_status === "rejected");

      console.log(
        "分类结果 - 待审核:",
        pendingLinks.value.length,
        "已通过:",
        approvedLinks.value.length,
        "已拒绝:",
        rejectedLinks.value.length,
      );

      // 为有thread_id的链接预加载thread
      const linksWithThread = formattedLinks.filter((link) => link.thread_id);
      for (const link of linksWithThread) {
        fetchThread(link);
      }
    } else {
      pendingLinks.value = [];
      approvedLinks.value = [];
      rejectedLinks.value = [];
    }
  } catch (error) {
    console.error("获取版本信息失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "获取翻译链接失败",
      type: "error",
    });
  } finally {
    loading.value = false;
  }
};

// 批准链接
const approveLink = async (link) => {
  if (!hasPermission.value || processingLinks.value.includes(link.id)) return;

  processingLinks.value.push(link.id);
  try {
    // 调用后端API批准链接
    await useBaseFetch(`version/${link.version_id}/link/${link.target_version_id}/approve`, {
      method: "POST",
    });

    addNotification({
      group: "main",
      title: "成功",
      text: "已批准翻译链接。数据正在更新...",
      type: "success",
    });

    // 重新获取数据以确保显示最新状态
    // 增加延迟时间以确保后端缓存已完全更新
    setTimeout(() => {
      fetchTranslationLinks();
    }, 1500);
  } catch (error) {
    console.error("批准链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "批准链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
  }
};

// 打开拒绝对话框（新版本，使用弹窗）
const openRejectModal = (link) => {
  pendingRejectLink.value = link;
  rejectReason.value = "";
  rejectModal.value?.show();
};

// 确认拒绝（从弹窗）
const confirmReject = async () => {
  const link = pendingRejectLink.value;
  if (!link || !rejectReason.value || rejectingLink.value) return;

  rejectingLink.value = true;
  processingLinks.value.push(link.id);

  try {
    // 先发送拒绝原因消息到thread
    const originalMessageText = messageTexts.value[link.id];
    messageTexts.value[link.id] =
      `您的翻译链接申请已被拒绝。\n\n拒绝原因：\n${rejectReason.value}\n\n请在修改后重新提交申请。`;
    await sendMessage(link);
    messageTexts.value[link.id] = originalMessageText || "";

    // 调用后端API拒绝链接
    await useBaseFetch(`version/${link.version_id}/link/${link.target_version_id}/reject`, {
      method: "POST",
    });

    // 关闭弹窗
    rejectModal.value?.hide();

    addNotification({
      group: "main",
      title: "成功",
      text: "已拒绝翻译链接并发送拒绝原因。数据正在更新...",
      type: "success",
    });

    // 重新获取数据
    setTimeout(() => {
      fetchTranslationLinks();
    }, 1500);
  } catch (error) {
    console.error("拒绝链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "拒绝链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
    rejectingLink.value = false;
    pendingRejectLink.value = null;
    rejectReason.value = "";
  }
};

// 保留旧的openRejectDialog和相关方法，以便thread内的拒绝按钮使用
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const openRejectDialog = (link) => {
  // 展开thread区域
  if (!expandedThreads.value.includes(link.id)) {
    toggleThread(link);
  }
  // 标记为拒绝流程
  if (!rejectingLinks.value.includes(link.id)) {
    rejectingLinks.value.push(link.id);
  }
  // 预填拒绝消息模板
  messageTexts.value[link.id] =
    "您的翻译链接申请已被拒绝。\n\n拒绝原因：\n- [请填写具体原因]\n\n请在修改后重新提交申请。";
};

// 确认拒绝并发送消息
const confirmRejectWithMessage = async (link) => {
  await rejectLink(link, true);
  // 移除拒绝流程标记
  rejectingLinks.value = rejectingLinks.value.filter((id) => id !== link.id);
};

// 取消拒绝
const cancelReject = (link) => {
  // 移除拒绝流程标记
  rejectingLinks.value = rejectingLinks.value.filter((id) => id !== link.id);
  // 清空消息
  messageTexts.value[link.id] = "";
  // 收起thread
  const index = expandedThreads.value.indexOf(link.id);
  if (index !== -1) {
    expandedThreads.value.splice(index, 1);
  }
};

// 拒绝链接（带消息）
const rejectLink = async (link, withMessage = false) => {
  if (!hasPermission.value || processingLinks.value.includes(link.id)) return;

  processingLinks.value.push(link.id);
  try {
    // 如果有消息，先发送消息（会自动创建thread如果不存在）
    if (withMessage && messageTexts.value[link.id]) {
      await sendMessage(link);
    }

    // 调用后端API拒绝链接
    await useBaseFetch(`version/${link.version_id}/link/${link.target_version_id}/reject`, {
      method: "POST",
    });

    addNotification({
      group: "main",
      title: "成功",
      text: withMessage
        ? "已拒绝翻译链接并发送消息。数据正在更新..."
        : "已拒绝翻译链接。数据正在更新...",
      type: "success",
    });

    // 重新获取数据
    setTimeout(() => {
      fetchTranslationLinks();
    }, 1500);
  } catch (error) {
    console.error("拒绝链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "拒绝链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
  }
};

// 撤销已批准的链接 - 显示确认弹窗
const revokeLink = (link) => {
  if (!hasPermission.value || processingLinks.value.includes(link.id)) return;

  pendingRevokeLink.value = link;
  modalConfirmRevoke.value?.show();
};

// 确认撤销操作
const confirmRevoke = async () => {
  const link = pendingRevokeLink.value;
  if (!link) return;

  processingLinks.value.push(link.id);
  try {
    // 调用后端API撤销链接
    await useBaseFetch(`version/${link.version_id}/link/${link.target_version_id}/revoke`, {
      method: "POST",
    });

    addNotification({
      group: "main",
      title: "成功",
      text: "已撤销翻译链接。数据正在更新...",
      type: "success",
    });

    // 重新获取数据
    setTimeout(() => {
      fetchTranslationLinks();
    }, 1500);
  } catch (error) {
    console.error("撤销链接失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "撤销链接失败",
      type: "error",
    });
  } finally {
    processingLinks.value = processingLinks.value.filter((id) => id !== link.id);
    pendingRevokeLink.value = null;
  }
};

// 切换thread显示
const toggleThread = (link) => {
  const index = expandedThreads.value.indexOf(link.id);
  if (index === -1) {
    expandedThreads.value.push(link.id);
    // 如果有thread_id，获取thread
    if (link.thread_id && !threads.value[link.id]) {
      fetchThread(link);
    } else if (!link.thread_id) {
      // 如果没有thread_id，创建一个空的thread对象
      // 第一次发送消息时会自动创建thread
      threads.value[link.id] = {
        id: null,
        messages: [],
        members: [],
        type: "version_link",
      };
    }
  } else {
    expandedThreads.value.splice(index, 1);
  }
};

// 获取thread
const fetchThread = async (link) => {
  if (!link.thread_id) {
    console.log("Link没有thread_id:", link);
    return;
  }

  console.log("正在获取thread:", link.thread_id);
  try {
    const thread = await useBaseFetch(`thread/${link.thread_id}`);
    console.log("获取到的thread:", thread);
    threads.value[link.id] = thread;
  } catch (error) {
    console.error("获取thread失败:", error);
    // 创建空thread作为后备
    threads.value[link.id] = {
      id: link.thread_id,
      messages: [],
      members: [],
    };
  }
};

// 发送消息
const sendMessage = async (link) => {
  const messageText = messageTexts.value[link.id];
  if (!messageText || sendingMessage.value[link.id]) return;

  sendingMessage.value[link.id] = true;
  try {
    // 使用版本链接专用的thread API
    // 如果thread不存在，后端会自动创建
    const response = await useBaseFetch(
      `version/${link.version_id}/link/${link.target_version_id}/thread`,
      {
        method: "POST",
        body: {
          body: messageText,
        },
      },
    );

    // 如果是第一次发送消息，保存thread_id
    if (response && response.thread_id && !link.thread_id) {
      link.thread_id = response.thread_id;
      // 初始化thread对象
      if (!threads.value[link.id]) {
        threads.value[link.id] = {
          id: response.thread_id,
          messages: [],
          members: [],
        };
      }
    }

    // 清空输入框
    messageTexts.value[link.id] = "";

    // 重新获取thread以显示新消息
    if (link.thread_id) {
      await fetchThread(link);
    }

    addNotification({
      group: "main",
      title: "成功",
      text: "消息已发送",
      type: "success",
    });
  } catch (error) {
    console.error("发送消息失败:", error);
    addNotification({
      group: "main",
      title: "错误",
      text: "发送消息失败",
      type: "error",
    });
  } finally {
    sendingMessage.value[link.id] = false;
  }
};

// 获取消息作者
const getMessageAuthor = (message, thread) => {
  if (!message.author_id) return null;
  return thread.members?.find((m) => m.id === message.author_id);
};

// 判断是否为管理员
const isStaff = (user) => {
  return user?.role === "admin" || user?.role === "moderator";
};

// 渲染Markdown
const renderMarkdown = (text) => {
  return renderString(text);
};

// 组件挂载时获取数据
onMounted(() => {
  if (hasPermission.value) {
    fetchTranslationLinks();
  }
});
</script>

<style scoped lang="scss">
.section-title {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 1.5rem 0 1rem;
  color: var(--color-heading);

  &:first-child {
    margin-top: 0;
  }
}

.links-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;

  &.compact {
    gap: 0.5rem;
  }
}

.link-item {
  padding: 1rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-divider);

  &.approved {
    background: var(--color-bg);
    padding: 0.75rem;
  }
}

.link-info {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;

  .approved & {
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }
}

.link-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 1rem;
}

.link-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  text-decoration: none;
  color: var(--color-text);

  &:hover {
    color: var(--color-primary);
  }

  &.compact {
    flex: 1;
  }
}

.project-icon {
  flex-shrink: 0;
}

.link-details {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.project-name {
  font-weight: 600;
  font-size: 1rem;

  .compact & {
    font-size: 0.95rem;
  }
}

.version-info {
  font-size: 0.875rem;
  color: var(--color-text-secondary);

  .compact & {
    margin-left: 0.5rem;
  }
}

.link-meta {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.language-badge {
  padding: 0.25rem 0.5rem;
  background: var(--color-primary-bg);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;

  &.small {
    padding: 0.125rem 0.375rem;
    font-size: 0.8rem;
  }
}

.link-type {
  padding: 0.25rem 0.5rem;
  background: var(--color-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  color: var(--color-text-secondary);
}

.link-description {
  padding: 0.75rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  font-size: 0.9rem;
  color: var(--color-text);
  line-height: 1.5;
}

.link-target,
.link-submitter {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.target-label,
.submitter-label {
  color: var(--color-text-secondary);
  font-weight: 500;
}

.target-link,
.submitter-link {
  color: var(--color-primary);
  text-decoration: none;
  display: flex;
  align-items: center;
  gap: 0.25rem;

  &:hover {
    text-decoration: underline;
  }
}

.submit-time {
  margin-left: auto;
  color: var(--color-text-disabled);
  font-size: 0.875rem;
}

.link-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 1rem;

  .approved & {
    margin-top: 0;
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
  gap: 0.25rem;
  transition: all 0.2s;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &.btn-primary {
    background: var(--color-primary);
    color: white;

    &:hover:not(:disabled) {
      background: var(--color-primary-dark);
    }
  }

  &.btn-danger {
    background: var(--color-danger);
    color: white;

    &:hover:not(:disabled) {
      background: var(--color-danger-dark);
    }
  }

  &.btn-small {
    padding: 0.25rem 0.5rem;
    font-size: 0.875rem;
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

.empty-state,
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem;
  gap: 1rem;
  color: var(--color-text-secondary);

  svg {
    width: 3rem;
    height: 3rem;
    opacity: 0.5;
    animation: spin 1s linear infinite;
  }

  p {
    margin: 0;
    font-size: 1rem;
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

// Thread 相关样式
.thread-section {
  margin-top: 1rem;
  padding: 1rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);
}

.thread-header {
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--color-divider);

  h5 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-heading);
  }

  .thread-description {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
  }
}

.thread-messages {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.messages-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-height: 400px;
  overflow-y: auto;
  padding: 0.5rem;
}

.message-item {
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-md);

  &.mod-message {
    background: var(--color-primary-bg);
    border: 1px solid var(--color-primary);
  }
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.message-author {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 600;
  font-size: 0.9rem;
}

.message-time {
  font-size: 0.875rem;
  color: var(--color-text-disabled);
}

.message-body {
  position: relative;
}

.message-text {
  font-size: 0.95rem;
  line-height: 1.5;
  color: var(--color-text);

  :deep(p) {
    margin: 0.5rem 0;

    &:first-child {
      margin-top: 0;
    }

    &:last-child {
      margin-bottom: 0;
    }
  }
}

.status-change {
  padding: 0.5rem;
  background: var(--color-bg);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  text-align: center;
}

.no-messages {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
    opacity: 0.5;
    margin-bottom: 0.5rem;
  }

  p {
    margin: 0;
    font-size: 0.9rem;
  }
}

.send-message {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
}

.message-input {
  width: 100%;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  font-size: 0.95rem;
  resize: vertical;
  min-height: 60px;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}

.message-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.loading-thread {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  gap: 0.75rem;
  color: var(--color-text-secondary);

  svg {
    width: 1.5rem;
    height: 1.5rem;
  }
}

.btn-secondary {
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-divider);

  &:hover:not(:disabled) {
    background: var(--color-raised-bg);
  }
}

.btn-small {
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
}

.status-badge {
  padding: 0.125rem 0.375rem;
  border-radius: var(--radius-sm);
  font-size: 0.8rem;
  font-weight: 500;
  margin-left: 0.5rem;

  &.pending {
    background: var(--color-warning-bg);
    color: var(--color-warning);
  }

  &.approved {
    background: var(--color-success-bg);
    color: var(--color-success);
  }

  &.rejected {
    background: var(--color-danger-bg);
    color: var(--color-danger);
  }
}

.filter-section {
  margin: 1rem 0;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--color-divider);

  .filter-count {
    margin-top: 0.75rem;
    margin-bottom: 0;
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
}

.empty-section,
.loading-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  gap: 1rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
  }

  p {
    margin: 0;
    font-size: 0.95rem;
  }
}

.link-item {
  &.expanded {
    .link-info {
      padding-bottom: 0;
    }
  }

  &.rejected {
    opacity: 0.9;

    .link-info {
      flex-direction: column;
      gap: 0.75rem;
    }
  }
}

/* 拒绝对话框样式 */
.reject-content {
  padding: 1rem;

  p {
    margin-bottom: 1rem;
    color: var(--color-text-secondary);
    font-size: 0.95rem;
  }
}

.reject-textarea {
  width: 100%;
  min-height: 120px;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  font-size: 0.95rem;
  font-family: inherit;
  resize: vertical;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  padding: 1rem;
  padding-top: 0;

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
</style>
