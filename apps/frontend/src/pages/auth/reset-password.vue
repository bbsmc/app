<template>
  <div>
    <h1>{{ formatMessage(messages.longTitle) }}</h1>
    <section class="auth-form">
      <template v-if="step === 'choose_method'">
        <p>
          {{ formatMessage(methodChoiceMessages.description) }}
        </p>

        <div class="iconified-input">
          <label for="email" hidden>
            {{ formatMessage(methodChoiceMessages.emailUsernameLabel) }}
          </label>
          <MailIcon />
          <input
            id="email"
            v-model="email"
            type="text"
            autocomplete="username"
            class="auth-form__input"
            :placeholder="formatMessage(methodChoiceMessages.emailUsernamePlaceholder)"
          />
        </div>

        <TACaptcha ref="captcha" v-model="token" />

        <button class="btn btn-primary centered-btn" :disabled="!token" @click="recovery">
          <SendIcon /> {{ formatMessage(methodChoiceMessages.action) }}
        </button>
      </template>
      <template v-else-if="step === 'passed_challenge'">
        <p>{{ formatMessage(postChallengeMessages.description) }}</p>

        <div class="iconified-input">
          <label for="password" hidden>{{ formatMessage(commonMessages.passwordLabel) }}</label>
          <KeyIcon />
          <input
            id="password"
            v-model="newPassword"
            type="password"
            autocomplete="new-password"
            class="auth-form__input"
            :placeholder="formatMessage(commonMessages.passwordLabel)"
          />
        </div>

        <div class="iconified-input">
          <label for="confirm-password" hidden>
            {{ formatMessage(commonMessages.passwordLabel) }}
          </label>
          <KeyIcon />
          <input
            id="confirm-password"
            v-model="confirmNewPassword"
            type="password"
            autocomplete="new-password"
            class="auth-form__input"
            :placeholder="formatMessage(postChallengeMessages.confirmPasswordLabel)"
          />
        </div>

        <button class="auth-form__input btn btn-primary continue-btn" @click="changePassword">
          {{ formatMessage(postChallengeMessages.action) }}
        </button>
      </template>
    </section>
  </div>
</template>
<script setup>
import { SendIcon, MailIcon, KeyIcon } from "@modrinth/assets";
import TACaptcha from "@/components/ui/TACaptcha.vue";

const { formatMessage } = useVIntl();

const methodChoiceMessages = defineMessages({
  description: {
    id: "auth.reset-password.method-choice.description",
    defaultMessage: "请在下方输入您的邮箱，我们将发送恢复链接帮助您找回账号。",
  },
  emailUsernameLabel: {
    id: "auth.reset-password.method-choice.email-username.label",
    defaultMessage: "邮箱或用户名",
  },
  emailUsernamePlaceholder: {
    id: "auth.reset-password.method-choice.email-username.placeholder",
    defaultMessage: "邮箱",
  },
  action: {
    id: "auth.reset-password.method-choice.action",
    defaultMessage: "发送恢复邮件",
  },
});

const postChallengeMessages = defineMessages({
  description: {
    id: "auth.reset-password.post-challenge.description",
    defaultMessage: "请在下方输入新密码以恢复您的账号。",
  },
  confirmPasswordLabel: {
    id: "auth.reset-password.post-challenge.confirm-password.label",
    defaultMessage: "确认密码",
  },
  action: {
    id: "auth.reset-password.post-challenge.action",
    defaultMessage: "重置密码",
  },
});

// NOTE(Brawaru): Vite uses esbuild for minification so can't combine these
// because it'll keep the original prop names compared to consts, which names
// will be mangled.
const emailSentNotificationMessages = defineMessages({
  title: {
    id: "auth.reset-password.notification.email-sent.title",
    defaultMessage: "邮件已发送",
  },
  text: {
    id: "auth.reset-password.notification.email-sent.text",
    defaultMessage: "包含操作说明的邮件已发送（如果该邮箱已绑定账号）。",
  },
});

const passwordResetNotificationMessages = defineMessages({
  title: {
    id: "auth.reset-password.notification.password-reset.title",
    defaultMessage: "密码重置成功",
  },
  text: {
    id: "auth.reset-password.notification.password-reset.text",
    defaultMessage: "您现在可以使用新密码登录了。",
  },
});

const messages = defineMessages({
  title: {
    id: "auth.reset-password.title",
    defaultMessage: "重置密码",
  },
  longTitle: {
    id: "auth.reset-password.title.long",
    defaultMessage: "重置您的密码",
  },
});

useHead({
  title: () => `${formatMessage(messages.title)} - BBSMC`,
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const auth = await useAuth();
if (auth.value.user) {
  await navigateTo("/dashboard");
}

const route = useNativeRoute();

const step = ref("choose_method");

if (route.query.flow) {
  step.value = "passed_challenge";
}

const captcha = ref();

const email = ref("");
const token = ref("");

async function recovery() {
  startLoading();
  try {
    await useBaseFetch("auth/password/reset", {
      method: "POST",
      body: {
        username: email.value,
        challenge: token.value,
      },
    });

    addNotification({
      group: "main",
      title: formatMessage(emailSentNotificationMessages.title),
      text: formatMessage(emailSentNotificationMessages.text),
      type: "success",
    });
  } catch (err) {
    addNotification({
      group: "main",
      title: formatMessage(commonMessages.errorNotificationTitle),
      text: err.data ? err.data.description : err,
      type: "error",
    });
    captcha.value?.reset();
  }
  stopLoading();
}

const newPassword = ref("");
const confirmNewPassword = ref("");

async function changePassword() {
  startLoading();
  try {
    await useBaseFetch("auth/password", {
      method: "PATCH",
      body: {
        new_password: newPassword.value,
        flow: route.query.flow,
      },
    });

    addNotification({
      group: "main",
      title: formatMessage(passwordResetNotificationMessages.title),
      text: formatMessage(passwordResetNotificationMessages.text),
      type: "success",
    });
    await navigateTo("/auth/sign-in");
  } catch (err) {
    addNotification({
      group: "main",
      title: formatMessage(commonMessages.errorNotificationTitle),
      text: err.data ? err.data.description : err,
      type: "error",
    });
    captcha.value?.reset();
  }
  stopLoading();
}
</script>
