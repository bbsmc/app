<template>
  <div>
    <ModalConfirm
      ref="modal_confirm"
      title="你确定要注销该账户吗?"
      description="这将会 **立即删除您的所有用户数据和关注**. 这不会删除您的已发布的资源. 注销后将无法恢复账户."
      proceed-label="删除此账户"
      :confirmation-text="auth.user.username"
      :has-to-type="true"
      @proceed="deleteAccount"
    />
    <Modal ref="changeEmailModal" :header="`${auth.user.email ? '修改' : '新增'} 电子邮箱`">
      <div class="universal-modal">
        <p>您的帐户信息不会公开显示</p>
        <label for="email-input"><span class="label__title">电子邮箱地址</span> </label>
        <input
          id="email-input"
          v-model="email"
          maxlength="2048"
          type="email"
          :placeholder="`输入邮箱地址...`"
          @keyup.enter="saveEmail()"
        />
        <div class="input-group push-right">
          <button class="iconified-button" @click="$refs.changeEmailModal.hide()">
            <XIcon />
            取消
          </button>
          <button
            type="button"
            class="iconified-button brand-button"
            :disabled="!email"
            @click="saveEmail()"
          >
            <SaveIcon />
            保存
          </button>
        </div>
      </div>
    </Modal>
    <Modal
      ref="managePhoneNumberModal"
      :header="`${auth.user.has_phonenumber ? '修改' : '设置'}手机号`"
    >
      <div class="universal-modal">
        <label for="new-phone-number"><span class="label__title">手机号</span></label>
        <input
          id="new-phone-number"
          v-model="newPhoneNumber"
          maxlength="2048"
          type="text"
          autocomplete="current-password"
          :placeholder="`${auth.user.has_phonenumber ? '修改' : '设置'}手机号`"
        />

        <!-- // 验证码 -->
        <!-- 验证码输入框右边加一个按钮验证码  -->
        <label for="verification-code"><span class="label__title">验证码</span></label>
        <div class="input-group">
          <input
            id="verification-code"
            v-model="verificationCode"
            maxlength="20"
            type="text"
            autocomplete="current-password"
            :placeholder="`验证码`"
          />
          <button
            type="button"
            class="iconified-button"
            :disabled="isCooldown"
            @click="sendVerificationCode"
          >
            {{ isCooldown ? `重新发送(${cooldownTime})` : "发送验证码" }}
          </button>
        </div>

        <TACaptcha v-if="!token" ref="captcha" v-model="token" />

        <p></p>
        <div class="input-group push-right">
          <button class="iconified-button" @click="$refs.managePhoneNumberModal.hide()">
            <XIcon />
            取消
          </button>
          <button
            type="button"
            class="iconified-button brand-button"
            :disabled="!verificationCode || verificationCode.length !== 6"
            @click="savePhoneNumber"
          >
            提交
          </button>
        </div>
      </div>
    </Modal>
    <Modal
      ref="managePasswordModal"
      :header="`${removePasswordMode ? '删除' : auth.user.has_password ? '修改' : '设置'}密码`"
    >
      <div class="universal-modal">
        <ul
          v-if="newPassword !== confirmNewPassword && confirmNewPassword.length > 0"
          class="known-errors"
        >
          <li>输入的密码不匹配！</li>
        </ul>
        <label v-if="removePasswordMode" for="old-password">
          <span class="label__title">确认密码</span>
          <span class="label__description">请输入您的密码以继续。</span>
        </label>
        <label v-else-if="auth.user.has_password" for="old-password">
          <span class="label__title">当前密码</span>
        </label>
        <input
          v-if="auth.user.has_password"
          id="old-password"
          v-model="oldPassword"
          maxlength="2048"
          type="password"
          autocomplete="current-password"
          :placeholder="`${removePasswordMode ? '确认' : '当前'} 密码`"
        />
        <template v-if="!removePasswordMode">
          <label for="new-password"><span class="label__title">新密码</span></label>
          <input
            id="new-password"
            v-model="newPassword"
            maxlength="2048"
            type="password"
            autocomplete="new-password"
            placeholder="新密码"
          />
          <label for="confirm-new-password"><span class="label__title">再次输入密码</span></label>
          <input
            id="confirm-new-password"
            v-model="confirmNewPassword"
            maxlength="2048"
            type="password"
            autocomplete="new-password"
            placeholder="再次输入一次新密码"
          />
        </template>
        <p></p>
        <div class="input-group push-right">
          <button class="iconified-button" @click="$refs.managePasswordModal.hide()">
            <XIcon />
            取消
          </button>
          <template v-if="removePasswordMode">
            <button
              type="button"
              class="iconified-button danger-button"
              :disabled="!oldPassword"
              @click="savePassword"
            >
              <TrashIcon />
              删除密码
            </button>
          </template>
          <template v-else>
            <button
              v-if="auth.user.has_password && auth.user.auth_providers.length > 0"
              type="button"
              class="iconified-button danger-button"
              @click="removePasswordMode = true"
            >
              <TrashIcon />
              删除密码
            </button>
            <button
              type="button"
              class="iconified-button brand-button"
              :disabled="
                newPassword.length == 0 ||
                (auth.user.has_password && oldPassword.length == 0) ||
                newPassword !== confirmNewPassword
              "
              @click="savePassword"
            >
              <SaveIcon />
              保存密码
            </button>
          </template>
        </div>
      </div>
    </Modal>
    <Modal
      ref="manageTwoFactorModal"
      :header="`${auth.user.has_totp && twoFactorStep === 0 ? '移除' : '设置'} 双重验证`"
    >
      <div class="universal-modal">
        <template v-if="auth.user.has_totp && twoFactorStep === 0">
          <label for="two-factor-code">
            <span class="label__title">输入双重验证码</span>
            <span class="label__description">请输入双重验证码以继续。</span>
          </label>
          <input
            id="two-factor-code"
            v-model="twoFactorCode"
            maxlength="11"
            type="text"
            placeholder="输入验证码..."
            @keyup.enter="removeTwoFactor()"
          />
          <p v-if="twoFactorIncorrect" class="known-errors">输入的验证码不正确！</p>
          <div class="input-group push-right">
            <button class="iconified-button" @click="$refs.manageTwoFactorModal.hide()">
              <XIcon />
              取消
            </button>
            <button class="iconified-button danger-button" @click="removeTwoFactor">
              <TrashIcon />
              移除双重验证
            </button>
          </div>
        </template>
        <template v-else>
          <template v-if="twoFactorStep === 0">
            <p>
              双重验证通过要求访问第二设备来保护您的账号安全。
              <br /><br />
              使用 <a href="https://authy.com/">Authy</a>、<a
                href="https://www.microsoft.com/en-us/security/mobile-authenticator-app"
                >Microsoft Authenticator</a
              >
              或其他双重验证应用扫描二维码。
            </p>
            <qrcode-vue
              v-if="twoFactorSecret"
              :value="`otpauth://totp/${encodeURIComponent(
                auth.user.email,
              )}?secret=${twoFactorSecret}&issuer=BBSMC`"
              :size="250"
              :margin="2"
              level="H"
            />
            <p>
              如果无法扫描二维码，您可以手动输入密钥：
              <strong>{{ twoFactorSecret }}</strong>
            </p>
          </template>
          <template v-if="twoFactorStep === 1">
            <label for="verify-code">
              <span class="label__title">验证码</span>
              <span class="label__description">输入验证器中的一次性验证码以确认。 </span>
            </label>
            <input
              id="verify-code"
              v-model="twoFactorCode"
              maxlength="6"
              type="text"
              autocomplete="one-time-code"
              placeholder="输入验证码..."
              @keyup.enter="verifyTwoFactorCode()"
            />
            <p v-if="twoFactorIncorrect" class="known-errors">输入的验证码不正确！</p>
          </template>
          <template v-if="twoFactorStep === 2">
            <p>
              请下载并妥善保存这些备用码。如果您丢失了设备，可以使用这些备用码替代双重验证码！请像保护密码一样保护这些备用码。
            </p>
            <p>备用码只能使用一次。</p>
            <ul>
              <li v-for="code in backupCodes" :key="code">{{ code }}</li>
            </ul>
          </template>
          <div class="input-group push-right">
            <button v-if="twoFactorStep === 1" class="iconified-button" @click="twoFactorStep = 0">
              <LeftArrowIcon />
              返回
            </button>
            <button
              v-if="twoFactorStep !== 2"
              class="iconified-button"
              @click="$refs.manageTwoFactorModal.hide()"
            >
              <XIcon />
              取消
            </button>
            <button
              v-if="twoFactorStep <= 1"
              class="iconified-button brand-button"
              @click="twoFactorStep === 1 ? verifyTwoFactorCode() : (twoFactorStep = 1)"
            >
              <RightArrowIcon />
              继续
            </button>
            <button
              v-if="twoFactorStep === 2"
              class="iconified-button brand-button"
              @click="$refs.manageTwoFactorModal.hide()"
            >
              <CheckIcon />
              完成设置
            </button>
          </div>
        </template>
      </div>
    </Modal>
    <Modal ref="manageProvidersModal" header="第三方登录管理">
      <div class="universal-modal">
        <div class="table">
          <div class="table-head table-row">
            <div class="table-text table-cell">登录方式</div>
            <div class="table-text table-cell">操作</div>
          </div>
          <div v-for="provider in authProviders" :key="provider.id" class="table-row">
            <div class="table-text table-cell">
              <span> <component :is="provider.icon" /> {{ provider.display }} </span>
            </div>
            <div class="table-text manage table-cell">
              <button
                v-if="auth.user.auth_providers.includes(provider.id)"
                class="btn"
                @click="removeAuthProvider(provider.id)"
              >
                <TrashIcon /> 解绑
              </button>
              <a
                v-else
                class="btn"
                :href="`${getAuthUrl(provider.id, '/settings/account')}&token=${auth.token}`"
              >
                <ExternalIcon /> 绑定
              </a>
            </div>
          </div>
        </div>
        <p></p>
        <div class="input-group push-right">
          <button class="iconified-button" @click="$refs.manageProvidersModal.hide()">
            <XIcon />
            关闭
          </button>
        </div>
      </div>
    </Modal>
    <section class="universal-card">
      <h2 class="text-2xl">账号安全</h2>

      <div class="adjacent-input">
        <label for="theme-selector">
          <span class="label__title">手机号</span>
          <span v-if="auth.user.has_phonenumber" class="label__description">
            更改<template v-if="auth.user.auth_providers.length > 0">或删除</template>您账户的手机号
          </span>

          <span v-else class="label__description">
            根据《互联网论坛社区服务管理规定》第八条，您需要绑定手机号后才可以发布信息
          </span>
        </label>
        <div>
          <button
            class="iconified-button"
            @click="
              () => {
                oldPassword = '';
                newPassword = '';
                confirmNewPassword = '';
                removePasswordMode = false;
                $refs.managePhoneNumberModal.show();
              }
            "
          >
            <KeyIcon />
            <template v-if="auth.user.has_phonenumber"> 修改手机号 </template>
            <template v-else> 设置手机号 </template>
          </button>
        </div>
      </div>

      <div class="adjacent-input">
        <label for="theme-selector">
          <span class="label__title">邮箱</span>
          <span class="label__description">更改与您的帐户关联的电子邮件</span>
        </label>
        <div>
          <button class="iconified-button" @click="$refs.changeEmailModal.show()">
            <template v-if="auth.user.email">
              <EditIcon />
              更改电子邮箱
            </template>
            <template v-else>
              <PlusIcon />
              设置邮箱
            </template>
          </button>
        </div>
      </div>
      <div class="adjacent-input">
        <label for="theme-selector">
          <span class="label__title">密码</span>
          <span v-if="auth.user.has_password" class="label__description">
            更改<template v-if="auth.user.auth_providers.length > 0">或删除</template
            >您账户的登录密码
          </span>
          <span v-else class="label__description"> 设置密码来登录您的帐户。 </span>
        </label>
        <div>
          <button
            class="iconified-button"
            @click="
              () => {
                oldPassword = '';
                newPassword = '';
                confirmNewPassword = '';
                removePasswordMode = false;
                $refs.managePasswordModal.show();
              }
            "
          >
            <KeyIcon />
            <template v-if="auth.user.has_password"> 修改密码 </template>
            <template v-else> 设置密码 </template>
          </button>
        </div>
      </div>
      <!--      <div class="adjacent-input">-->
      <!--        <label for="theme-selector">-->
      <!--          <span class="label__title">Two-factor authentication</span>-->
      <!--          <span class="label__description">-->
      <!--            Add an additional layer of security to your account during login.-->
      <!--          </span>-->
      <!--        </label>-->
      <!--        <div>-->
      <!--          <button class="iconified-button" @click="showTwoFactorModal">-->
      <!--            <template v-if="auth.user.has_totp"> <TrashIcon /> Remove 2FA </template>-->
      <!--            <template v-else> <PlusIcon /> Setup 2FA </template>-->
      <!--          </button>-->
      <!--        </div>-->
      <!--      </div>-->
      <div class="adjacent-input">
        <label>
          <span class="label__title">管理第三方登录</span>
          <span class="label__description">
            添加或移除您账户的第三方登录方式，包括 GitHub、Microsoft、 哔哩哔哩 和 Google。
          </span>
        </label>
        <div>
          <button class="iconified-button" @click="$refs.manageProvidersModal.show()">
            <ExternalIcon /> 管理登录方式
          </button>
        </div>
      </div>
    </section>

    <!--    <section id="data-export" class="universal-card">-->
    <!--      <h2>Data export</h2>-->
    <!--      <p>-->
    <!--        Request a copy of all your personal data you have uploaded to Modrinth. This may take-->
    <!--        several minutes to complete.-->
    <!--      </p>-->
    <!--      <a v-if="generated" class="iconified-button" :href="generated" download="export.json">-->
    <!--        <DownloadIcon />-->
    <!--        Download export-->
    <!--      </a>-->
    <!--      <button v-else class="iconified-button" :disabled="generatingExport" @click="exportData">-->
    <!--        <template v-if="generatingExport"> <UpdatedIcon /> Generating export... </template>-->
    <!--        <template v-else> <UpdatedIcon /> Generate export </template>-->
    <!--      </button>-->
    <!--    </section>-->

    <!-- 封禁状态与申诉 -->
    <section
      v-if="auth.user.active_bans && auth.user.active_bans.length > 0"
      id="ban-status"
      class="universal-card ban-section"
    >
      <h2 class="ban-title text-2xl">
        <BanIcon class="ban-title-icon" />
        账户封禁状态
      </h2>
      <p class="ban-description">
        您的账户当前存在以下封禁，部分功能可能受到限制。如果您认为存在误判，可以发起申诉。
      </p>
      <div class="ban-list">
        <div
          v-for="ban in auth.user.active_bans"
          :key="ban.id"
          :data-ban-id="ban.id"
          class="ban-item"
        >
          <div class="ban-header">
            <span class="ban-type" :class="`ban-type-${ban.ban_type}`">
              {{ getBanTypeName(ban.ban_type) }}
            </span>
            <span class="ban-date"> 封禁时间：{{ formatBanDate(ban.banned_at) }} </span>
          </div>
          <div class="ban-reason"><strong>封禁原因：</strong>{{ ban.reason || "未提供原因" }}</div>
          <div v-if="ban.expires_at" class="ban-expires">
            <strong>过期时间：</strong>{{ formatBanDate(ban.expires_at) }}
          </div>
          <div v-else class="ban-expires permanent">
            <strong>永久封禁</strong>
          </div>

          <!-- 申诉状态 -->
          <div v-if="ban.appeal" class="appeal-status">
            <div class="appeal-header">
              <div v-if="ban.appeal.status === 'pending'" class="appeal-pending">
                <InfoIcon class="status-icon" />
                <span>申诉审核中...</span>
              </div>
              <div v-else-if="ban.appeal.status === 'approved'" class="appeal-approved">
                <CheckIcon class="status-icon" />
                <span>申诉已通过</span>
              </div>
              <div v-else-if="ban.appeal.status === 'rejected'" class="appeal-rejected">
                <XIcon class="status-icon" />
                <span>申诉已拒绝</span>
              </div>
              <button
                v-if="ban.appeal.thread_id"
                class="btn btn-secondary"
                @click="toggleAppealThread(ban)"
              >
                <MessageIcon />
                {{ expandedThreads[ban.id] ? "收起对话" : "查看对话" }}
              </button>
            </div>

            <!-- 申诉对话线程 -->
            <div v-if="expandedThreads[ban.id] && appealThreads[ban.id]" class="appeal-thread">
              <ConversationThread
                :thread="appealThreads[ban.id]"
                :auth="auth"
                :current-member="null"
                :project="null"
                :report="null"
                @update-thread="refreshAppealThread(ban)"
              />
            </div>
          </div>

          <!-- 发起申诉按钮 -->
          <div v-else class="appeal-actions">
            <button class="iconified-button" @click="openAppealModal(ban)">
              <EditIcon />
              发起申诉
            </button>
          </div>
        </div>
      </div>
    </section>

    <section id="delete-account" class="universal-card">
      <h2>注销账户</h2>
      <p>
        一旦注销帐户，将无法恢复。注销帐户将从我们的服务器中删除所有附加数据（已发布的资源除外）。
      </p>
      <button
        type="button"
        class="iconified-button danger-button"
        @click="$refs.modal_confirm.show()"
      >
        <TrashIcon />
        注销账户
      </button>
    </section>

    <!-- 申诉模态框 -->
    <NewModal ref="appealModal" header="申诉封禁">
      <div class="flex flex-col gap-3">
        <div class="flex flex-col gap-2">
          <span class="text-secondary">
            请说明您认为此封禁存在问题的原因。超级管理员将会审核您的申诉并通过消息线程与您沟通。
          </span>
        </div>
        <div class="flex flex-col gap-2">
          <label for="appeal-reason" class="flex flex-col gap-1">
            <span class="text-lg font-semibold text-contrast">
              申诉理由
              <span class="text-brand-red">*</span>
            </span>
            <span>请详细说明您认为封禁存在问题的理由（10-2000字符）</span>
          </label>
          <div class="textarea-wrapper">
            <textarea
              id="appeal-reason"
              v-model="appealReason"
              rows="6"
              maxlength="2000"
              placeholder="请输入您的申诉理由..."
            />
          </div>
        </div>
        <div class="flex gap-2">
          <ButtonStyled color="brand">
            <button :disabled="!appealReason || appealReason.length < 10" @click="submitAppeal">
              <CheckIcon aria-hidden="true" />
              提交申诉
            </button>
          </ButtonStyled>
          <ButtonStyled>
            <button @click="appealModal.hide()">
              <XIcon aria-hidden="true" />
              取消
            </button>
          </ButtonStyled>
        </div>
      </div>
    </NewModal>
  </div>
</template>

<script setup>
import {
  CheckIcon,
  EditIcon,
  ExternalIcon,
  LeftArrowIcon,
  PlusIcon,
  RightArrowIcon,
  SaveIcon,
  TrashIcon,
  XIcon,
  BanIcon,
  MessageIcon,
  InfoIcon,
} from "@modrinth/assets";
import dayjs from "dayjs";
import QrcodeVue from "qrcode.vue";
import { NewModal, ButtonStyled } from "@modrinth/ui";
import GitHubIcon from "assets/icons/auth/sso-github.svg";
import MicrosoftIcon from "assets/icons/auth/sso-microsoft.svg";
import BilibiliIcon from "assets/icons/auth/sso-bilibili.svg";
// import GoogleIcon from "assets/icons/auth/sso-google.svg";
import QQIcon from "assets/icons/auth/sso-qq.svg";
import KeyIcon from "assets/icons/auth/key.svg";
import ModalConfirm from "~/components/ui/ModalConfirm.vue";
import Modal from "~/components/ui/Modal.vue";
import TACaptcha from "@/components/ui/TACaptcha.vue";
import ConversationThread from "~/components/ui/thread/ConversationThread.vue";

useHead({
  title: "账户与安全 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

definePageMeta({
  middleware: "auth",
});

const data = useNuxtApp();
const auth = await useAuth();
const token = ref("");
const tokenCode = ref("");

const changeEmailModal = ref();
const email = ref(auth.value.user.email);
async function saveEmail() {
  if (!email.value) {
    return;
  }

  startLoading();
  try {
    await useBaseFetch(`auth/email`, {
      method: "PATCH",
      body: {
        email: email.value,
      },
    });
    changeEmailModal.value.hide();
    await useAuth(auth.value.token);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
}

const managePasswordModal = ref();
const removePasswordMode = ref(false);
const managePhoneNumberModal = ref();
const oldPassword = ref("");
const newPassword = ref("");

const newPhoneNumber = ref("");

const confirmNewPassword = ref("");
async function savePassword() {
  if (newPassword.value !== confirmNewPassword.value) {
    return;
  }

  startLoading();
  try {
    await useBaseFetch(`auth/password`, {
      method: "PATCH",
      body: {
        old_password: auth.value.user.has_password ? oldPassword.value : null,
        new_password: removePasswordMode.value ? null : newPassword.value,
      },
    });
    managePasswordModal.value.hide();
    await useAuth(auth.value.token);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
}

const manageTwoFactorModal = ref();
const twoFactorSecret = ref(null);
const twoFactorFlow = ref(null);
const twoFactorStep = ref(0);
const twoFactorIncorrect = ref(false);
const twoFactorCode = ref(null);
const backupCodes = ref([]);
async function verifyTwoFactorCode() {
  startLoading();
  try {
    const res = await useBaseFetch("auth/2fa", {
      method: "POST",
      body: {
        code: twoFactorCode.value ? twoFactorCode.value : "",
        flow: twoFactorFlow.value,
      },
    });

    backupCodes.value = res.backup_codes;
    twoFactorStep.value = 2;
    await useAuth(auth.value.token);
  } catch {
    twoFactorIncorrect.value = true;
  }
  stopLoading();
}

async function removeTwoFactor() {
  startLoading();
  try {
    await useBaseFetch("auth/2fa", {
      method: "DELETE",
      body: {
        code: twoFactorCode.value ? twoFactorCode.value.toString() : "",
      },
    });
    manageTwoFactorModal.value.hide();
    await useAuth(auth.value.token);
  } catch {
    twoFactorIncorrect.value = true;
  }
  stopLoading();
}

// 申诉相关
const appealModal = ref();
const appealReason = ref("");
const currentAppealBan = ref(null);
const expandedThreads = ref({}); // 记录哪些 ban 的对话被展开
const appealThreads = ref({}); // 存储已加载的 thread 数据

// 页面加载时自动展开有申诉的封禁对话
onMounted(async () => {
  if (auth.value?.user?.active_bans) {
    for (const ban of auth.value.user.active_bans) {
      if (ban.appeal?.thread_id) {
        try {
          const thread = await useBaseFetch(`thread/${ban.appeal.thread_id}`);
          appealThreads.value[ban.id] = thread;
          expandedThreads.value[ban.id] = true;
        } catch (err) {
          console.error("Failed to load appeal thread:", err);
        }
      }
    }
  }
});

function openAppealModal(ban) {
  console.log("Opening appeal modal for ban:", ban);
  currentAppealBan.value = ban;
  appealReason.value = "";
  appealModal.value.show();
}

// 切换申诉对话显示/隐藏
async function toggleAppealThread(ban) {
  const banId = ban.id;

  if (expandedThreads.value[banId]) {
    // 如果已展开，则收起
    expandedThreads.value[banId] = false;
  } else {
    // 如果未展开，则加载并展开
    expandedThreads.value[banId] = true;

    if (!appealThreads.value[banId]) {
      // 如果还没加载过，则加载 thread 数据
      try {
        const thread = await useBaseFetch(`thread/${ban.appeal.thread_id}`);
        appealThreads.value[banId] = thread;
      } catch (err) {
        data.$notify({
          group: "main",
          title: "加载失败",
          text: "无法加载对话数据",
          type: "error",
        });
        expandedThreads.value[banId] = false;
      }
    }
  }
}

// 刷新申诉对话
async function refreshAppealThread(ban) {
  if (ban.appeal?.thread_id) {
    try {
      const thread = await useBaseFetch(`thread/${ban.appeal.thread_id}`);
      appealThreads.value[ban.id] = thread;
    } catch (err) {
      console.error("Failed to refresh thread:", err);
    }
  }
}

async function submitAppeal() {
  if (!appealReason.value || appealReason.value.length < 10) {
    data.$notify({
      group: "main",
      title: "错误",
      text: "申诉理由至少需要10个字符",
      type: "error",
    });
    return;
  }

  console.log("Submitting appeal for ban:", currentAppealBan.value);
  console.log("Ban ID:", currentAppealBan.value?.id);

  if (!currentAppealBan.value?.id) {
    data.$notify({
      group: "main",
      title: "错误",
      text: "封禁ID不存在，请刷新页面后重试",
      type: "error",
    });
    return;
  }

  startLoading();
  try {
    const result = await useBaseFetch(`user/bans/${currentAppealBan.value.id}/appeal`, {
      method: "POST",
      body: {
        reason: appealReason.value,
      },
    });

    data.$notify({
      group: "main",
      title: "申诉已提交",
      text: "您的申诉已提交，超级管理员将会审核并通过消息线程与您沟通",
      type: "success",
    });

    appealModal.value.hide();

    // 刷新用户数据以获取最新的申诉状态
    await useAuth(auth.value.token);

    // 如果有 thread_id，自动展开对话区域并加载 thread
    if (result.thread_id) {
      const banId = currentAppealBan.value.id;
      try {
        const thread = await useBaseFetch(`thread/${result.thread_id}`);
        appealThreads.value[banId] = thread;
        expandedThreads.value[banId] = true;

        // 滚动到申诉对话区域
        setTimeout(() => {
          const appealElement = document.querySelector(`[data-ban-id="${banId}"]`);
          if (appealElement) {
            appealElement.scrollIntoView({ behavior: "smooth", block: "nearest" });
          }
        }, 100);
      } catch (err) {
        console.error("Failed to load thread:", err);
      }
    }
  } catch (err) {
    data.$notify({
      group: "main",
      title: "申诉失败",
      text: err.data ? err.data.description : "提交申诉时出错",
      type: "error",
    });
  }
  stopLoading();
}

function getBanTypeName(type) {
  const types = {
    global: "全局封禁",
    resource: "资源封禁",
    forum: "论坛封禁",
  };
  return types[type] || type;
}

function formatBanDate(dateStr) {
  return dayjs(dateStr).format("YYYY-MM-DD HH:mm");
}

const authProviders = [
  {
    id: "github",
    display: "GitHub",
    icon: GitHubIcon,
  },
  {
    id: "microsoft",
    display: "Microsoft",
    icon: MicrosoftIcon,
  },
  // {
  //   id: "google",
  //   display: "Google",
  //   icon: GoogleIcon,
  // },
  {
    id: "bilibili",
    display: "哔哩哔哩",
    icon: BilibiliIcon,
  },
  {
    id: "qq",
    display: "QQ",
    icon: QQIcon,
  },
];

async function deleteAccount() {
  startLoading();
  try {
    await useBaseFetch(`user/${auth.value.user.id}`, {
      method: "DELETE",
    });
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }

  useCookie("auth-token").value = null;
  await navigateTo("/", { external: true });

  stopLoading();
}

const verificationCode = ref("");
const isCooldown = ref(false);
const cooldownTime = ref(90); // 冷却时间为90秒

async function sendVerificationCode() {
  if (isCooldown.value) {
    return; // 如果在冷却时间内，不执行发送验证码
  }

  if (!newPhoneNumber.value || newPhoneNumber.value.length !== 11) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "请先输入正确的11位手机号",
      type: "error",
    });
    return;
  }

  if (!token.value) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "请先点击下面的人机验证",
      type: "error",
    });
    return;
  }
  // 发送验证码的逻辑
  try {
    const res = await useBaseFetch("auth/phone_number_code", {
      method: "POST",
      body: {
        phone_number: newPhoneNumber.value, // 假设你要发送到的手机号
        challenge: token.value,
      },
    });
    tokenCode.value = res.token;
    // console.log(tokenCode.value);
    // 启动冷却时间
    isCooldown.value = true;
    cooldownTime.value = 90; // 重置冷却时间为90秒

    const countdown = setInterval(() => {
      cooldownTime.value--;
      if (cooldownTime.value <= 0) {
        clearInterval(countdown);
        isCooldown.value = false; // 90秒后解除冷却
      }
    }, 1000);
  } catch (err) {
    // 处理错误
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}

async function savePhoneNumber() {
  if (!verificationCode.value || verificationCode.value.length !== 6) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "请输入正确的6位验证码",
      type: "error",
    });
    return;
  }
  try {
    await useBaseFetch("auth/phone_number_bind", {
      method: "POST",
      body: {
        code: verificationCode.value,
        token: tokenCode.value,
        phone_number: newPhoneNumber.value,
      },
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "手机号绑定成功",
      type: "success",
    });
    managePhoneNumberModal.value.hide();
    await useAuth(auth.value.token);
    verificationCode.value = "";
    newPhoneNumber.value = "";
    tokenCode.value = "";
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}
</script>
<style lang="scss" scoped>
canvas {
  margin: 0 auto;
  border-radius: var(--size-rounded-card);
}

.table-row {
  grid-template-columns: 1fr 10rem;

  span {
    display: flex;
    align-items: center;
    margin: auto 0;

    svg {
      width: 1.25rem;
      height: 1.25rem;
      margin-right: 0.35rem;
    }
  }
}

// 封禁状态样式
.ban-section {
  border: 1px solid var(--color-red, #ef4444);
  background: var(--color-red-bg, rgba(239, 68, 68, 0.05));
}

.ban-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--color-red, #ef4444);
}

.ban-title-icon {
  width: 1.5rem;
  height: 1.5rem;
}

.ban-description {
  color: var(--color-secondary);
  margin-bottom: 1rem;
}

.ban-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.ban-item {
  padding: 1rem;
  background: var(--color-raised-bg);
  border-radius: var(--size-rounded-card);
  border: 1px solid var(--color-divider);
}

.ban-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.ban-type {
  padding: 0.25rem 0.5rem;
  border-radius: var(--size-rounded-sm);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}

.ban-type-global {
  background: var(--color-red-bg, rgba(239, 68, 68, 0.1));
  color: var(--color-red, #ef4444);
}

.ban-type-resource {
  background: var(--color-orange-bg, rgba(249, 115, 22, 0.1));
  color: var(--color-orange, #f97316);
}

.ban-type-forum {
  background: var(--color-yellow-bg, rgba(234, 179, 8, 0.1));
  color: var(--color-yellow, #eab308);
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
}

.ban-expires.permanent {
  color: var(--color-red, #ef4444);
}

.appeal-section {
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);

  p {
    margin-bottom: 0.75rem;
    color: var(--color-secondary);
  }
}

.appeal-status {
  margin-top: 0.75rem;
  padding: 0.75rem;
  background: var(--color-bg);
  border-radius: var(--size-rounded-sm);
  border: 1px solid var(--color-divider);
}

.appeal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.appeal-pending,
.appeal-approved,
.appeal-rejected {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.appeal-pending {
  color: var(--color-orange, #f97316);
}

.appeal-approved {
  color: var(--color-green, #10b981);
}

.appeal-rejected {
  color: var(--color-red, #ef4444);
}

.status-icon {
  width: 1.25rem;
  height: 1.25rem;
}

.appeal-thread {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
}

.appeal-actions {
  margin-top: 0.75rem;
}
</style>
