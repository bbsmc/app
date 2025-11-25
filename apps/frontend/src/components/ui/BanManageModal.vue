<template>
  <Modal ref="modal" header="封禁管理">
    <div class="ban-manage-modal">
      <!-- 撤销确认弹窗 -->
      <div v-if="showRevokeConfirm" class="revoke-confirm-overlay" @click.self="cancelRevoke">
        <div class="revoke-confirm-modal">
          <h4>确认撤销封禁</h4>
          <div class="form-group">
            <label>撤销原因</label>
            <textarea
              v-model="revokeReason"
              class="form-input form-textarea"
              placeholder="请输入撤销原因..."
              rows="3"
            ></textarea>
          </div>
          <div class="form-actions">
            <button class="cancel-btn" @click="cancelRevoke">取消</button>
            <button
              class="submit-btn"
              :disabled="!revokeReason.trim() || revoking"
              @click="confirmRevoke"
            >
              {{ revoking ? '撤销中...' : '确认撤销' }}
            </button>
          </div>
        </div>
      </div>

      <!-- 当前封禁列表 -->
      <div class="ban-section">
        <h3 class="section-title">当前封禁</h3>
        <div v-if="loading" class="loading-state">
          加载中...
        </div>
        <div v-else-if="bans.length === 0" class="empty-state">
          该用户当前没有任何封禁
        </div>
        <div v-else class="ban-list">
          <div v-for="ban in bans" :key="ban.id" class="ban-item">
            <div class="ban-header">
              <span class="ban-type" :class="getBanTypeClass(ban.ban_type)">
                {{ getBanTypeName(ban.ban_type) }}
              </span>
              <span class="ban-date">
                {{ formatDate(ban.banned_at) }}
              </span>
            </div>
            <div class="ban-reason">
              <strong>原因：</strong>{{ ban.reason || '未提供原因' }}
            </div>
            <div v-if="ban.expires_at" class="ban-expires">
              <strong>过期时间：</strong>{{ formatDate(ban.expires_at) }}
            </div>
            <div v-else class="ban-expires permanent">
              <strong>永久封禁</strong>
            </div>
            <button
              class="revoke-btn"
              :disabled="revoking"
              @click="startRevoke(ban.id)"
            >
              撤销封禁
            </button>
          </div>
        </div>
      </div>

      <!-- 添加新封禁 -->
      <div class="ban-section">
        <h3 class="section-title">添加封禁</h3>
        <div class="form-group">
          <label>封禁类型</label>
          <select v-model="newBan.ban_type" class="form-input">
            <option value="global">全局封禁（禁止所有操作）</option>
            <option value="resource">资源封禁（禁止发布/编辑资源）</option>
            <option value="forum">论坛封禁（禁止发帖/评论）</option>
          </select>
        </div>
        <div class="form-group">
          <label>封禁原因</label>
          <textarea
            v-model="newBan.reason"
            class="form-input form-textarea"
            placeholder="请输入封禁原因..."
            rows="3"
          ></textarea>
        </div>
        <div class="form-group">
          <label>封禁时长</label>
          <select v-model="newBan.duration" class="form-input">
            <option value="1d">1 天</option>
            <option value="3d">3 天</option>
            <option value="7d">7 天</option>
            <option value="30d">30 天</option>
            <option value="90d">90 天</option>
            <option value="365d">365 天</option>
            <option value="permanent">永久</option>
          </select>
        </div>
        <div class="form-actions">
          <button class="cancel-btn" @click="hide">取消</button>
          <button
            class="submit-btn"
            :disabled="!canSubmit || submitting"
            @click="submitBan"
          >
            {{ submitting ? '提交中...' : '确认封禁' }}
          </button>
        </div>
      </div>
    </div>
  </Modal>
</template>

<script setup>
import { ref, computed } from 'vue';
import Modal from '~/components/ui/Modal.vue';
import dayjs from 'dayjs';

const props = defineProps({
  userId: {
    type: String,
    required: true,
  },
});

const emit = defineEmits(['updated']);

const data = useNuxtApp();
const modal = ref(null);

// 状态
const loading = ref(false);
const bans = ref([]);
const revoking = ref(null);
const submitting = ref(false);

// 撤销相关状态
const showRevokeConfirm = ref(false);
const revokeBanId = ref(null);
const revokeReason = ref('');

// 新封禁表单
const newBan = ref({
  ban_type: 'resource',
  reason: '',
  duration: '7d',
});

// 计算属性
const canSubmit = computed(() => {
  return newBan.value.reason.trim().length > 0;
});

// 方法
function show() {
  modal.value.show();
  fetchBans();
}

function hide() {
  modal.value.hide();
}

async function fetchBans() {
  loading.value = true;
  try {
    const result = await useBaseFetch(`bans/user/${props.userId}`, {
      apiVersion: 3,
    });
    // 后端返回分页结构 { bans: [...], total, page, limit }
    // 只显示活跃的封禁
    bans.value = (result?.bans || []).filter(ban => ban.is_active);
  } catch (err) {
    console.error('获取封禁列表失败:', err);
    data.$notify({
      group: 'main',
      title: '错误',
      text: '获取封禁列表失败',
      type: 'error',
    });
  } finally {
    loading.value = false;
  }
}

function startRevoke(banId) {
  revokeBanId.value = banId;
  revokeReason.value = '';
  showRevokeConfirm.value = true;
}

function cancelRevoke() {
  showRevokeConfirm.value = false;
  revokeBanId.value = null;
  revokeReason.value = '';
}

async function confirmRevoke() {
  if (!revokeBanId.value || !revokeReason.value.trim()) return;

  revoking.value = revokeBanId.value;
  try {
    await useBaseFetch(`bans/${revokeBanId.value}`, {
      apiVersion: 3,
      method: 'DELETE',
      body: {
        reason: revokeReason.value,
        notify_user: true,
      },
    });
    data.$notify({
      group: 'main',
      title: '成功',
      text: '封禁已撤销',
      type: 'success',
    });
    cancelRevoke();
    emit('updated');
    hide(); // 关闭主弹窗
  } catch (err) {
    console.error('撤销封禁失败:', err);
    data.$notify({
      group: 'main',
      title: '错误',
      text: err.data?.description || '撤销封禁失败',
      type: 'error',
    });
  } finally {
    revoking.value = null;
  }
}

async function submitBan() {
  submitting.value = true;
  try {
    // 计算过期时间
    let expires_at = null;
    if (newBan.value.duration !== 'permanent') {
      const durationMap = {
        '1d': 1,
        '3d': 3,
        '7d': 7,
        '30d': 30,
        '90d': 90,
        '365d': 365,
      };
      const days = durationMap[newBan.value.duration];
      expires_at = dayjs().add(days, 'day').toISOString();
    }

    await useBaseFetch(`bans/user/${props.userId}`, {
      apiVersion: 3,
      method: 'POST',
      body: {
        ban_type: newBan.value.ban_type,
        reason: newBan.value.reason,
        expires_at,
      },
    });

    data.$notify({
      group: 'main',
      title: '成功',
      text: '封禁已添加',
      type: 'success',
    });

    // 重置表单
    newBan.value = {
      ban_type: 'resource',
      reason: '',
      duration: '7d',
    };

    emit('updated');
    hide(); // 关闭主弹窗
  } catch (err) {
    console.error('添加封禁失败:', err);
    data.$notify({
      group: 'main',
      title: '错误',
      text: err.data?.description || '添加封禁失败',
      type: 'error',
    });
  } finally {
    submitting.value = false;
  }
}

function getBanTypeName(type) {
  const names = {
    global: '全局封禁',
    resource: '资源封禁',
    forum: '论坛封禁',
  };
  return names[type] || type;
}

function getBanTypeClass(type) {
  return `ban-type-${type}`;
}

function formatDate(dateStr) {
  return dayjs(dateStr).format('YYYY-MM-DD HH:mm');
}

// 暴露方法给父组件
defineExpose({
  show,
  hide,
});
</script>

<style scoped lang="scss">
.ban-manage-modal {
  padding: var(--spacing-card-bg);
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  max-height: 70vh;
  overflow-y: auto;
}

.ban-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.section-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-contrast);
  margin: 0;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--color-divider);
}

.loading-state,
.empty-state {
  padding: 1rem;
  text-align: center;
  color: var(--color-secondary);
  background: var(--color-raised-bg);
  border-radius: var(--radius-md);
}

.ban-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.ban-item {
  padding: 1rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);
}

.ban-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.ban-type {
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}

.ban-type-global {
  background: var(--color-red-bg);
  color: var(--color-red);
}

.ban-type-resource {
  background: var(--color-orange-bg);
  color: var(--color-orange);
}

.ban-type-forum {
  background: var(--color-yellow-bg);
  color: var(--color-yellow);
}

.ban-date {
  font-size: 0.75rem;
  color: var(--color-secondary);
}

.ban-reason {
  font-size: 0.875rem;
  color: var(--color-text);
  margin-bottom: 0.25rem;
}

.ban-expires {
  font-size: 0.75rem;
  color: var(--color-secondary);
  margin-bottom: 0.5rem;
}

.ban-expires.permanent {
  color: var(--color-red);
}

.revoke-btn {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  background: transparent;
  border: 1px solid var(--color-red);
  color: var(--color-red);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s ease;

  &:hover:not(:disabled) {
    background: var(--color-red);
    color: white;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;

  label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
  }
}

.form-input {
  padding: 0.5rem 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-size: 0.875rem;

  &:focus {
    outline: none;
    border-color: var(--color-brand);
  }
}

.form-textarea {
  resize: vertical;
  min-height: 4rem;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 0.5rem;
}

.cancel-btn {
  padding: 0.5rem 1rem;
  background: transparent;
  border: 1px solid var(--color-divider);
  color: var(--color-text);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s ease;

  &:hover {
    background: var(--color-raised-bg);
  }
}

.submit-btn {
  padding: 0.5rem 1rem;
  background: var(--color-red);
  border: none;
  color: white;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s ease;

  &:hover:not(:disabled) {
    opacity: 0.9;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

// 撤销确认弹窗
.revoke-confirm-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.revoke-confirm-modal {
  background: var(--color-bg);
  border-radius: var(--radius-md);
  padding: 1.5rem;
  width: 90%;
  max-width: 400px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);

  h4 {
    margin: 0 0 1rem 0;
    font-size: 1.125rem;
    color: var(--color-contrast);
  }
}
</style>
