<template>
  <div class="issues-container" :style="themeVars">
    <!-- Issues列表主要内容 -->
    <div class="issues-wrapper">
      <div class="issues-header">
        <h2 class="issues-title">站内反馈</h2>
        <div v-if="!project.issues_url" class="issues-stats">
          <span class="stats-item">
            <span class="stats-count">{{ pagination?.total || 0 }}</span>
            个问题
          </span>
        </div>
        <div class="header-actions">
          <div v-if="hasEditPermission" class="settings-button">
            <button-styled @click="openSettingsModal">
              <SettingsIcon aria-hidden="true" />
              设置
            </button-styled>
          </div>
          <div
            v-if="
              isAuth &&
              (!project.issues_url || project.issues_type !== 0) &&
              project.issues_type !== 2
            "
            class="create-button"
          >
            <button-styled color="green" @click="openCreateIssue">
              <PlusIcon aria-hidden="true" />
              创建问题
            </button-styled>
          </div>
        </div>
      </div>

      <!-- 外部反馈地址提示 -->
      <div v-if="project.issues_url && project.issues_type === 0" class="external-issues-notice">
        <div class="notice-content">
          <span class="notice-icon">🔗</span>
          <div class="notice-text">
            <span>该项目已设置了站外反馈问题的地址，请前往</span>
            <a
              :href="project.issues_url"
              target="_blank"
              rel="noopener noreferrer"
              class="external-link"
            >
              {{ project.issues_url }}
            </a>
            <span>提交问题反馈</span>
          </div>
        </div>
      </div>

      <div v-if="project.issues_url && project.issues_type === 1" class="external-issues-notice">
        <div class="notice-content">
          <span class="notice-icon">🔗</span>
          <div class="notice-text">
            <span>该项目同时设置了站外反馈问题的地址，请优先前往</span>
            <a
              :href="project.issues_url"
              target="_blank"
              rel="noopener noreferrer"
              class="external-link"
            >
              {{ project.issues_url }}
            </a>
            <span>提交问题反馈</span>
          </div>
        </div>
      </div>
      <div v-if="project.issues_type === 2" class="external-issues-notice">
        <div class="notice-content">
          <span class="notice-icon">🔗</span>
          <div class="notice-text">
            <span>该项目关闭了站内反馈功能</span>
          </div>
        </div>
      </div>

      <!-- 过滤器 -->
      <!--      1.  project.issues_url 和 project.issues_type === 0 的时候 不显示-->
      <!--      2.  project.issues_type === 2 的时候 不显示-->
      <div
        v-if="(!project.issues_url || project.issues_type !== 0) && project.issues_type !== 2"
        class="filter-bar"
      >
        <div class="filter-tabs">
          <button
            class="filter-tab"
            :class="{ active: stateFilter === 'all' }"
            @click="setFilter('all')"
          >
            全部
          </button>
          <button
            class="filter-tab"
            :class="{ active: stateFilter === 'open' }"
            @click="setFilter('open')"
          >
            开放中
          </button>
          <button
            class="filter-tab"
            :class="{ active: stateFilter === 'closed' }"
            @click="setFilter('closed')"
          >
            已关闭
          </button>
        </div>
      </div>

      <!-- Issues列表 -->
      <div
        v-if="
          (!project.issues_url || project.issues_type !== 0) &&
          project.issues_type !== 2 &&
          issues &&
          issues.length > 0
        "
        class="issues-list"
      >
        <div v-for="issue in issues" :key="issue.id" class="card issue-card">
          <div class="issue-status-indicator" :class="{ closed: issue.state === 'closed' }">
            <span v-if="issue.state === 'open'" class="status-icon open">●</span>
            <span v-else class="status-icon closed">✓</span>
          </div>

          <div class="issue-content">
            <div class="issue-title-row">
              <NuxtLink
                :to="`/${route.params.type}/${route.params.id}/issues/${issue.id}`"
                class="issue-title-link"
              >
                {{ issue.title }}
              </NuxtLink>
              <div v-if="issue.labels && issue.labels.length > 0" class="issue-labels">
                <span
                  v-for="label in issue.labels"
                  :key="label.id"
                  class="issue-label"
                  :style="{ backgroundColor: label.color }"
                >
                  {{ label.name }}
                </span>
              </div>
            </div>

            <div class="issue-meta-row">
              <div class="issue-meta">
                <span class="issue-number">#{{ issue.id }}</span>
                <span class="meta-separator">·</span>
                <span class="meta-text">由</span>
                <NuxtLink :to="`/user/${issue.author.username}`" class="author-link">
                  {{ issue.author.username }}
                </NuxtLink>
                <span class="meta-text">于</span>
                <span class="issue-date">{{ formatRelativeTime(issue.created_at) }}</span>
                <span class="meta-text">创建</span>
                <span v-if="issue.comments_count > 0" class="meta-separator">·</span>
                <span v-if="issue.comments_count > 0" class="comments-count">
                  {{ issue.comments_count }} 条回复
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 分页控制 -->
      <div
        v-if="!project.issues_url && pagination && pagination.total > pageSize"
        class="pagination-controls"
      >
        <button-styled
          v-if="currentPage > 1"
          :disabled="pending"
          @click="changePage(currentPage - 1)"
        >
          上一页
        </button-styled>

        <div class="page-info">
          <span class="page-numbers">
            第 {{ currentPage }} 页，共 {{ Math.ceil(pagination.total / pageSize) }} 页
          </span>
          <span class="total-count"> 共 {{ pagination.total }} 个问题 </span>
        </div>

        <button-styled
          v-if="currentPage < Math.ceil(pagination.total / pageSize)"
          :disabled="pending"
          @click="changePage(currentPage + 1)"
        >
          下一页
        </button-styled>
      </div>
    </div>

    <!-- 空状态 -->
    <div
      v-if="!project.issues_url && !pending && (!issues || issues.length === 0)"
      class="empty-state"
    >
      <div class="empty-content">
        <h3>还没有问题</h3>
        <p>这个项目还没有创建任何问题。</p>
        <div v-if="isAuth && !project.issues_url" class="empty-actions">
          <button-styled color="green" @click="openCreateIssue">
            <PlusIcon aria-hidden="true" />
            创建第一个问题
          </button-styled>
        </div>
      </div>
    </div>

    <!-- 加载状态 -->
    <div v-if="!project.issues_url && pending" class="loading-state">
      <LoadingSpinner />
      <p>加载中...</p>
    </div>

    <!-- 创建Issue模态框 -->
    <div v-if="showCreateModal" class="create-modal-overlay" @click.self="closeCreateModal">
      <div class="create-modal">
        <div class="modal-header">
          <span>创建问题</span>
          <button class="close-button" @click="closeCreateModal">×</button>
        </div>

        <div class="modal-content">
          <div class="form-group">
            <label class="form-label">
              <span class="label-title">标题</span>
              <span class="label-description">简短描述问题</span>
            </label>
            <input
              v-model="newIssue.title"
              type="text"
              placeholder="请输入问题标题"
              maxlength="300"
              class="form-input"
            />
          </div>

          <div class="form-group">
            <label class="form-label">
              <span class="label-title">描述</span>
              <span class="label-description">详细描述问题</span>
            </label>
            <MarkdownEditor
              v-model="newIssue.body"
              :on-image-upload="onUploadHandler"
              placeholder="请详细描述问题..."
            />
          </div>
        </div>

        <div class="modal-actions">
          <button-styled color="red" @click="closeCreateModal"> 取消 </button-styled>
          <button-styled
            color="green"
            :disabled="!newIssue.title.trim() || !newIssue.body.trim() || isCreating"
            @click="createIssue"
          >
            <PlusIcon aria-hidden="true" />
            {{ isCreating ? "创建中..." : "创建问题" }}
          </button-styled>
        </div>
      </div>
    </div>

    <!-- 设置模态框 -->
    <div v-if="showSettingsModal" class="create-modal-overlay" @click.self="closeSettingsModal">
      <div class="create-modal settings-modal">
        <div class="modal-header">
          <span>反馈设置</span>
          <button class="close-button" @click="closeSettingsModal">×</button>
        </div>

        <div class="modal-content">
          <div class="form-group">
            <label class="form-label">
              <span class="label-title">反馈功能状态</span>
              <span class="label-description">选择此项目的反馈功能设置</span>
            </label>

            <div class="radio-group">
              <label class="radio-option">
                <input
                  v-model="tempIssuesType"
                  type="radio"
                  :value="0"
                  name="issuesType"
                  class="radio-input"
                />
                <div class="radio-content">
                  <span class="radio-title">仅使用外部反馈地址</span>
                  <span class="radio-description"
                    >若设置了项目链接的反馈地址则不显示站内反馈功能，引导至链接的反馈地址</span
                  >
                </div>
              </label>

              <label class="radio-option">
                <input
                  v-model="tempIssuesType"
                  type="radio"
                  :value="1"
                  name="issuesType"
                  class="radio-input"
                />
                <div class="radio-content">
                  <span class="radio-title">外部反馈地址和站内反馈并存</span>
                  <span class="radio-description"
                    >链接反馈地址和站内反馈同时存在，用户可以选择使用哪种方式</span
                  >
                </div>
              </label>

              <label class="radio-option">
                <input
                  v-model="tempIssuesType"
                  type="radio"
                  :value="2"
                  name="issuesType"
                  class="radio-input"
                />
                <div class="radio-content">
                  <span class="radio-title">完全关闭反馈功能</span>
                  <span class="radio-description">完全关闭站内反馈功能，不显示反馈相关内容</span>
                </div>
              </label>
            </div>
          </div>
        </div>

        <div class="modal-actions">
          <button-styled color="red" @click="closeSettingsModal"> 取消 </button-styled>
          <button-styled
            color="green"
            :disabled="!hasSettingsChanges || isSavingSettings"
            @click="saveIssuesSettings"
          >
            <SaveIcon aria-hidden="true" />
            {{ isSavingSettings ? "保存中..." : "保存设置" }}
          </button-styled>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ButtonStyled, MarkdownEditor } from "@modrinth/ui";

import { PlusIcon } from "@modrinth/assets";
import dayjs from "dayjs";
import SettingsIcon from "~/assets/images/utils/settings.svg?component";
import SaveIcon from "~/assets/images/utils/save.svg?component";
import { isDarkTheme } from "~/plugins/theme/themes.ts";
const auth = await useAuth();

const data = useNuxtApp();
const router = useNativeRouter();
const route = useNativeRoute();

const props = defineProps({
  project: {
    type: Object,
    default: () => ({}),
  },
  currentMember: {
    type: Object,
    default() {
      return null;
    },
  },
});

const isAuth = computed(() => {
  return !!auth.value.user;
});

// 权限检查
const hasEditPermission = computed(() => {
  const EDIT_BODY = 1 << 3;
  return props.currentMember && (props.currentMember.permissions & EDIT_BODY) === EDIT_BODY;
});

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--color-text-secondary": "#8f9ba8",
      "--color-text-primary": "#edeff1",
      "--color-bg-card": "var(--color-raised-bg)",
      "--color-bg-secondary": "#2d3139",
      "--color-bg-hover": "#363b44",
      "--color-border": "#363b44",
      "--color-highlight": "#007bff",
      "--color-success": "#28a745",
      "--color-closed": "#6f42c1",
      "--color-overlay": "rgba(0, 0, 0, 0.5)",
      "--color-modal-bg": "#26292f",
      "--color-notice-bg": "linear-gradient(135deg, #1e3a8a 0%, #581c87 100%)",
    };
  } else {
    return {
      "--color-text-secondary": "#666",
      "--color-text-primary": "var(--color-text-dark)",
      "--color-bg-card": "var(--color-raised-bg)",
      "--color-bg-secondary": "#f0f2f5",
      "--color-bg-hover": "#e6e8eb",
      "--color-border": "#dfe1e5",
      "--color-highlight": "#1a73e8",
      "--color-success": "#28a745",
      "--color-closed": "#6f42c1",
      "--color-overlay": "rgba(0, 0, 0, 0.3)",
      "--color-modal-bg": "#ffffff",
      "--color-notice-bg": "linear-gradient(135deg, #e3f2fd 0%, #f3e5f5 100%)",
    };
  }
});

// SEO 设置
const title = `${props.project.title} 问题追踪 - 我的世界资源反馈 | BBSMC`;
const description = `查看和提交 ${props.project.title} 的问题反馈与功能建议，追踪崩溃报错、兼容性冲突和玩法问题的处理进度，了解哪些问题已经修复、哪些正在处理。在 BBSMC 参与这款 Minecraft 我的世界资源的改进，帮助创作者把资源做得更好。`;
useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: props.project.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
});

// 响应式数据
const issues = ref([]);
const pagination = ref(null);
const pending = ref(true);
const currentPage = ref(1);
const pageSize = ref(20);
const stateFilter = ref("all");

const showCreateModal = ref(false);
const isCreating = ref(false);
const newIssue = ref({
  title: "",
  body: "",
});

// 设置模态框相关
const showSettingsModal = ref(false);
const isSavingSettings = ref(false);
const tempIssuesType = ref(props.project.issues_type || 0);

// 检查设置是否有变化
const hasSettingsChanges = computed(() => {
  return tempIssuesType.value !== props.project.issues_type;
});

// 加载Issues列表
async function loadIssues() {
  pending.value = true;
  try {
    const params = {
      page: currentPage.value,
      page_size: pageSize.value,
    };

    // 只有当不是'all'时才添加state参数
    if (stateFilter.value !== "all") {
      params.state = stateFilter.value;
    }

    const response = await useBaseFetch(`issues/project/${props.project.id}`, {
      apiVersion: 3,
      query: params,
    });

    issues.value = response.issues || [];
    pagination.value = response.pagination || null;
  } catch (err) {
    console.error("加载Issues失败:", err);
    data.$notify({
      group: "main",
      title: "加载失败",
      text: err.data?.description || "无法加载Issues列表",
      type: "error",
    });
  } finally {
    pending.value = false;
  }
}

// 设置过滤器
function setFilter(filter) {
  stateFilter.value = filter;
  currentPage.value = 1;
  loadIssues();
}

// 分页
function changePage(page) {
  currentPage.value = page;
  loadIssues();
}

// 打开创建Issue模态框
function openCreateIssue() {
  newIssue.value = { title: "", body: "" };
  showCreateModal.value = true;
}

// 关闭创建Issue模态框
function closeCreateModal() {
  showCreateModal.value = false;
}

// 创建Issue
async function createIssue() {
  if (!newIssue.value.title.trim() || !newIssue.value.body.trim()) {
    return;
  }

  isCreating.value = true;
  try {
    const response = await useBaseFetch(`issues/project/${props.project.id}`, {
      apiVersion: 3,
      method: "POST",
      body: {
        title: newIssue.value.title.trim(),
        body: newIssue.value.body.trim(),
      },
    });

    data.$notify({
      group: "main",
      title: "创建成功",
      text: "问题创建成功",
      type: "success",
    });

    closeCreateModal();
    router.push(`/${route.params.type}/${route.params.id}/issues/${response.issue}`);
  } catch (err) {
    console.error("创建问题失败:", err);
    data.$notify({
      group: "main",
      title: "创建失败",
      text: err.data?.description || "无法创建Issue",
      type: "error",
    });
  } finally {
    isCreating.value = false;
  }
}

// 图片上传处理函数
const onUploadHandler = async (file) => {
  const response = await useImageUpload(file, {
    context: "project",
    projectID: props.project.id,
  });
  return response.url;
};

// 设置模态框相关方法
function openSettingsModal() {
  tempIssuesType.value = props.project.issues_type || 0;
  showSettingsModal.value = true;
}

function closeSettingsModal() {
  showSettingsModal.value = false;
}

// 保存Issues设置
async function saveIssuesSettings() {
  if (!hasSettingsChanges.value) {
    return;
  }

  isSavingSettings.value = true;
  try {
    const patchData = {
      issues_type: tempIssuesType.value,
    };

    await useBaseFetch(`project/${props.project.id}`, {
      apiVersion: 3,
      method: "PATCH",
      body: patchData,
    });

    // 使用 emit 通知父组件更新项目数据
    // 注意：实际应该通过 emit 事件通知父组件，这里仅用于临时兼容
    Object.assign(props.project, { issues_type: tempIssuesType.value });

    data.$notify({
      group: "main",
      title: "保存成功",
      text: "反馈设置已更新",
      type: "success",
    });

    closeSettingsModal();

    // 重新加载Issues列表（如果需要）
    if (!props.project.issues_url) {
      loadIssues();
    }
  } catch (err) {
    console.error("保存设置失败:", err);
    data.$notify({
      group: "main",
      title: "保存失败",
      text: err.data?.description || "无法保存反馈设置",
      type: "error",
    });
  } finally {
    isSavingSettings.value = false;
  }
}

// 格式化相对时间
const formatRelativeTime = (dateString) => {
  return dayjs(dateString).fromNow();
};

// 初始化
onMounted(() => {
  // 只有在没有设置外部反馈地址时才加载issues
  if (!props.project.issues_url) {
    loadIssues();
  }
});
</script>

<style scoped>
.issues-container {
  min-height: 100vh;
}

.issues-wrapper {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
}

.issues-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  flex-wrap: wrap;
  gap: 16px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.issues-title {
  font-size: 2rem;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.issues-stats {
  display: flex;
  align-items: center;
  gap: 16px;
  color: var(--color-text-secondary);
}

.stats-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.stats-count {
  font-weight: 600;
  color: var(--color-text-primary);
}

.external-issues-notice {
  margin-bottom: 20px;
  padding: 16px;
  background: var(--color-notice-bg);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  border-left: 4px solid var(--color-highlight);
}

.notice-content {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.notice-icon {
  font-size: 20px;
  margin-top: 2px;
}

.notice-text {
  flex: 1;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  color: var(--color-text-primary);
  font-size: 14px;
  line-height: 1.5;
}

.external-link {
  color: var(--color-highlight);
  text-decoration: none;
  font-weight: 500;
  word-break: break-all;
}

.external-link:hover {
  text-decoration: underline;
}

.filter-bar {
  margin-bottom: 20px;
}

.filter-tabs {
  display: flex;
  gap: 8px;
}

.filter-tab {
  padding: 8px 16px;
  border: 1px solid var(--color-border);
  background: transparent;
  color: var(--color-text-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.filter-tab:hover {
  background: var(--color-bg-hover);
  color: var(--color-text-primary);
}

.filter-tab.active {
  background: var(--color-highlight);
  color: white;
  border-color: var(--color-highlight);
}

.issues-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 24px;
}

.issue-card {
  display: flex;
  padding: 16px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-bg-card);
  transition: border-color 0.2s ease;
}

.issue-card:hover {
  border-color: var(--color-highlight);
}

.issue-status-indicator {
  margin-right: 12px;
  margin-top: 2px;
}

.status-icon {
  font-size: 16px;
  font-weight: bold;
}

.status-icon.open {
  color: var(--color-success);
}

.status-icon.closed {
  color: var(--color-closed);
}

.issue-content {
  flex: 1;
  min-width: 0;
}

.issue-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.issue-title-link {
  color: var(--color-text-primary);
  text-decoration: none;
  font-weight: 600;
  font-size: 16px;
  word-break: break-word;
}

.issue-title-link:hover {
  color: var(--color-highlight);
}

.issue-labels {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.issue-label {
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.issue-meta-row {
  font-size: 14px;
  color: var(--color-text-secondary);
}

.issue-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.issue-number {
  font-weight: 500;
}

.meta-separator {
  color: var(--color-text-secondary);
  opacity: 0.6;
}

.author-link {
  color: var(--color-text-secondary);
  text-decoration: none;
  font-weight: 500;
}

.author-link:hover {
  color: var(--color-highlight);
}

.pagination-controls {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 20px;
  padding: 20px;
  margin-top: 20px;
}

.page-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  color: var(--color-text-secondary);
  font-size: 14px;
}

.empty-state {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 400px;
}

.empty-content {
  text-align: center;
  color: var(--color-text-secondary);
}

.empty-content h3 {
  color: var(--color-text-primary);
  margin-bottom: 8px;
}

.empty-actions {
  margin-top: 16px;
}

.loading-state {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  min-height: 400px;
  color: var(--color-text-secondary);
  gap: 16px;
}

.create-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--color-overlay);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.create-modal {
  background: var(--color-modal-bg);
  border-radius: 8px;
  width: 100%;
  max-width: 800px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-primary);
  font-size: 18px;
  font-weight: 600;
}

.close-button {
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  font-size: 24px;
  cursor: pointer;
  padding: 0 8px;
  transition: color 0.2s ease;
}

.close-button:hover {
  color: var(--color-text-primary);
}

.modal-content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}

.form-group {
  margin-bottom: 20px;
}

.form-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}

.label-title {
  font-weight: 600;
  color: var(--color-text-primary);
}

.label-description {
  font-size: 14px;
  color: var(--color-text-secondary);
}

.form-input {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-bg-card);
  color: var(--color-text-primary);
  font-size: 14px;
}

.form-input:focus {
  outline: none;
  border-color: var(--color-highlight);
  box-shadow: 0 0 0 3px rgba(26, 115, 232, 0.1);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 20px;
  border-top: 1px solid var(--color-border);
}

.settings-modal {
  max-width: 600px;
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.radio-option {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.radio-option:hover {
  border-color: var(--color-highlight);
  background: var(--color-bg-hover);
}

.radio-option:has(.radio-input:checked) {
  border-color: var(--color-highlight);
  background: var(--color-bg-hover);
}

.radio-input {
  margin: 0;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  margin-top: 2px;
}

.radio-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.radio-title {
  font-weight: 600;
  color: var(--color-text-primary);
  font-size: 14px;
}

.radio-description {
  font-size: 13px;
  color: var(--color-text-secondary);
  line-height: 1.4;
}

@media (max-width: 768px) {
  .issues-header {
    flex-direction: column;
    align-items: stretch;
  }

  .header-actions {
    justify-content: center;
    flex-wrap: wrap;
  }

  .filter-tabs {
    justify-content: center;
  }

  .pagination-controls {
    flex-direction: column;
    gap: 12px;
  }

  .create-modal {
    margin: 10px;
  }

  .issue-meta {
    flex-wrap: wrap;
  }

  .radio-option {
    padding: 12px;
  }

  .radio-description {
    font-size: 12px;
  }
}
</style>
