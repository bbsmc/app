<template>
  <div>
    <h1>注册到 BBSMC</h1>

    <section class="auth-form">
      <div class="iconified-input">
        <label for="email" hidden>{{ formatMessage(messages.emailLabel) }}</label>
        <MailIcon />
        <input
          id="email"
          v-model="email"
          type="email"
          autocomplete="username"
          class="auth-form__input"
          :placeholder="formatMessage(messages.emailLabel)"
        />
      </div>

      <div class="iconified-input">
        <label for="username" hidden>{{ formatMessage(messages.usernameLabel) }}</label>
        <UserIcon />
        <input
          id="username"
          v-model="username"
          type="text"
          autocomplete="username"
          class="auth-form__input"
          :placeholder="formatMessage(messages.usernameLabel)"
        />
      </div>

      <div class="iconified-input">
        <label for="password" hidden>{{ formatMessage(messages.passwordLabel) }}</label>
        <KeyIcon />
        <input
          id="password"
          v-model="password"
          class="auth-form__input"
          type="password"
          autocomplete="new-password"
          :placeholder="formatMessage(messages.passwordLabel)"
        />
      </div>

      <div class="iconified-input">
        <label for="confirm-password" hidden>{{
          formatMessage(messages.confirmPasswordLabel)
        }}</label>
        <KeyIcon />
        <input
          id="confirm-password"
          v-model="confirmPassword"
          type="password"
          autocomplete="new-password"
          class="auth-form__input"
          :placeholder="formatMessage(messages.confirmPasswordLabel)"
        />
      </div>

      <Checkbox
        v-model="subscribe"
        class="subscribe-btn"
        :label="formatMessage(messages.subscribeLabel)"
        :description="formatMessage(messages.subscribeLabel)"
      />

      <TACaptcha ref="captcha" v-model="token" />

      <button
        class="btn btn-primary continue-btn full-width-btn"
        :disabled="!token"
        @click="createAccount"
      >
        {{ formatMessage(messages.createAccountButton) }}
        <RightArrowIcon />
      </button>

      <div class="auth-form__additional-options">
        已有账号?
        <NuxtLink class="text-link" :to="signInLink">立即登录</NuxtLink>
      </div>
    </section>

    <div class="auth-divider">
      <span>或通过以下方式注册</span>
    </div>

    <section class="third-party">
      <a class="btn sso-btn" :href="getAuthUrl('github', redirectTarget)">
        <SSOGitHubIcon />
        <span>GitHub</span>
      </a>
      <a class="btn sso-btn" :href="getAuthUrl('microsoft', redirectTarget)">
        <SSOMicrosoftIcon />
        <span>Microsoft</span>
      </a>
      <a class="btn sso-btn" :href="getAuthUrl('bilibili', redirectTarget)">
        <SSOBilibiliIcon />
        <span>哔哩哔哩</span>
      </a>
      <!-- <a class="btn sso-btn" :href="getAuthUrl('google', redirectTarget)">
        <SSOGoogleIcon />
        <span>Google</span>
      </a> -->
      <a class="btn sso-btn" :href="getAuthUrl('qq', redirectTarget)">
        <SSOQQIcon />
        <span>QQ</span>
      </a>
    </section>

    <p class="legal-notice">
      注册即表示您同意<NuxtLink to="/legal/terms">用户协议</NuxtLink>和<NuxtLink to="/legal/privacy"
        >隐私政策</NuxtLink
      >
    </p>
  </div>
</template>

<script setup>
import { RightArrowIcon, UserIcon, KeyIcon, MailIcon } from "@modrinth/assets";
import { Checkbox } from "@modrinth/ui";
import SSOGitHubIcon from "assets/icons/auth/sso-github.svg";
import SSOMicrosoftIcon from "assets/icons/auth/sso-microsoft.svg";
import SSOBilibiliIcon from "assets/icons/auth/sso-bilibili.svg";
// import SSOGoogleIcon from "assets/icons/auth/sso-google.svg";
import SSOQQIcon from "assets/icons/auth/sso-qq.svg";
import TACaptcha from "@/components/ui/TACaptcha.vue";
import { getAuthUrl } from "@/composables/auth.js";

const { formatMessage } = useVIntl();

const messages = defineMessages({
  title: {
    id: "auth.sign-up.title",
    defaultMessage: "注册",
  },
  signUpWithTitle: {
    id: "auth.sign-up.title.sign-up-with",
    defaultMessage: "通过以下方式注册",
  },
  createAccountTitle: {
    id: "auth.sign-up.title.create-account",
    defaultMessage: "或自行创建账号",
  },
  emailLabel: {
    id: "auth.sign-up.email.label",
    defaultMessage: "邮箱",
  },
  usernameLabel: {
    id: "auth.sign-up.label.username",
    defaultMessage: "用户名",
  },
  passwordLabel: {
    id: "auth.sign-up.password.label",
    defaultMessage: "密码",
  },
  confirmPasswordLabel: {
    id: "auth.sign-up.confirm-password.label",
    defaultMessage: "确认密码",
  },
  subscribeLabel: {
    id: "auth.sign-up.subscribe.label",
    defaultMessage: "订阅 BBSMC 更新通知",
  },
  legalDisclaimer: {
    id: "auth.sign-up.legal-dislaimer",
    defaultMessage:
      "注册即表示您同意 BBSMC 的<terms-link>用户协议</terms-link>和<privacy-policy-link>隐私政策</privacy-policy-link>。",
  },
  createAccountButton: {
    id: "auth.sign-up.action.create-account",
    defaultMessage: "创建账号",
  },
  alreadyHaveAccountLabel: {
    id: "auth.sign-up.sign-in-option.title",
    defaultMessage: "已有账号？",
  },
});

useHead({
  title: () => `${formatMessage(messages.title)} - BBSMC`,
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const auth = await useAuth();
const route = useNativeRoute();

if (auth.value.user) {
  await navigateTo("/dashboard");
}

const captcha = ref();

const email = ref("");
const username = ref("");
const password = ref("");
const confirmPassword = ref("");
const token = ref("");
const subscribe = ref(true);

const redirectTarget = route.query.redirect || "/dashboard";

const signInLink = computed(
  () => `/auth/sign-in${route.query.redirect ? `?redirect=${route.query.redirect}` : ""}`,
);

async function createAccount() {
  startLoading();
  try {
    if (confirmPassword.value !== password.value) {
      addNotification({
        group: "main",
        title: formatMessage(commonMessages.errorNotificationTitle),
        text: formatMessage({
          id: "auth.sign-up.notification.password-mismatch.text",
          defaultMessage: "两次输入的密码不一致！",
        }),
        type: "error",
      });
      captcha.value?.resetCaptcha();
      token.value = "";
      stopLoading();
      return;
    }

    const res = await useBaseFetch("auth/create", {
      method: "POST",
      body: {
        username: username.value,
        password: password.value,
        email: email.value,
        challenge: token.value,
        sign_up_newsletter: subscribe.value,
      },
    });

    await useAuth(res.session);
    await useUser();

    if (route.query.redirect) {
      await navigateTo(route.query.redirect);
    } else {
      await navigateTo("/dashboard");
    }
  } catch (err) {
    addNotification({
      group: "main",
      title: formatMessage(commonMessages.errorNotificationTitle),
      text: err.data ? err.data.description : err,
      type: "error",
    });
    captcha.value?.resetCaptcha();
    token.value = "";
  }
  stopLoading();
}
</script>
