<template>
  <div>
    <template v-if="false">
      <label for="two-factor-code">
        <span class="label__title">输入双重验证码</span>
        <span class="label__description">
          {{ formatMessage(messages.twoFactorCodeLabelDescription) }}
        </span>
      </label>
      <input
        id="two-factor-code"
        v-model="twoFactorCode"
        maxlength="11"
        type="text"
        :placeholder="formatMessage(messages.twoFactorCodeInputPlaceholder)"
        autocomplete="one-time-code"
        autofocus
        @keyup.enter="begin2FASignIn"
      />

      <button class="btn btn-primary continue-btn" @click="begin2FASignIn">
        {{ formatMessage(commonMessages.signInButton) }}
        <RightArrowIcon />
      </button>
    </template>
    <template v-else>
      <h1>登录到 BBSMC</h1>

      <section class="auth-form">
        <div class="iconified-input">
          <label for="email" hidden>请输入邮箱或用户名</label>
          <MailIcon />
          <input
            id="email"
            v-model="email"
            type="text"
            autocomplete="username"
            class="auth-form__input"
            placeholder="邮箱或用户名"
          />
        </div>

        <div class="iconified-input">
          <label for="password" hidden>密码</label>
          <KeyIcon />
          <input
            id="password"
            v-model="password"
            type="password"
            autocomplete="current-password"
            class="auth-form__input"
            placeholder="密码"
          />
        </div>

        <TACaptcha ref="captcha" v-model="token" />

        <button
          class="btn btn-primary continue-btn full-width-btn"
          :disabled="!token"
          @click="beginPasswordSignIn()"
        >
          {{ formatMessage(commonMessages.signInButton) }}
          <RightArrowIcon />
        </button>

        <div class="auth-form__additional-options">
          <NuxtLink class="text-link" to="/auth/reset-password">忘记密码?</NuxtLink>
          <span class="separator-dot">·</span>
          <NuxtLink class="text-link" :to="signUpLink">创建账号</NuxtLink>
        </div>
      </section>

      <div class="auth-divider">
        <span>或通过以下方式登录</span>
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
        登录即表示您同意<NuxtLink to="/legal/terms">用户协议</NuxtLink>和<NuxtLink
          to="/legal/privacy"
          >隐私政策</NuxtLink
        >
      </p>
    </template>
  </div>
</template>

<script setup>
import { RightArrowIcon, KeyIcon, MailIcon } from "@modrinth/assets";
import SSOGitHubIcon from "assets/icons/auth/sso-github.svg";
import SSOMicrosoftIcon from "assets/icons/auth/sso-microsoft.svg";
import SSOBilibiliIcon from "assets/icons/auth/sso-bilibili.svg";
// import SSOGoogleIcon from "assets/icons/auth/sso-google.svg";
import SSOQQIcon from "assets/icons/auth/sso-qq.svg";
import TACaptcha from "@/components/ui/TACaptcha.vue";
import { getAuthUrl } from "@/composables/auth.js";

const captcha = ref();
const token = ref("");

const { formatMessage } = useVIntl();

const messages = defineMessages({
  additionalOptionsLabel: {
    id: "auth.sign-in.additional-options",
    defaultMessage:
      "<forgot-password-link>忘记密码？</forgot-password-link> • <create-account-link>创建账号</create-account-link>",
  },
  emailUsernameLabel: {
    id: "auth.sign-in.email-username.label",
    defaultMessage: "邮箱或用户名",
  },
  passwordLabel: {
    id: "auth.sign-in.password.label",
    defaultMessage: "密码",
  },
  signInWithLabel: {
    id: "auth.sign-in.sign-in-with",
    defaultMessage: "通过以下方式登录",
  },
  signInTitle: {
    id: "auth.sign-in.title",
    defaultMessage: "登录",
  },
  twoFactorCodeInputPlaceholder: {
    id: "auth.sign-in.2fa.placeholder",
    defaultMessage: "输入验证码...",
  },
  twoFactorCodeLabel: {
    id: "auth.sign-in.2fa.label",
    defaultMessage: "输入双重验证码",
  },
  twoFactorCodeLabelDescription: {
    id: "auth.sign-in.2fa.description",
    defaultMessage: "请输入双重验证码以继续。",
  },
  usePasswordLabel: {
    id: "auth.sign-in.use-password",
    defaultMessage: "或使用密码登录",
  },
});

useHead({
  title() {
    return `${formatMessage(messages.signInTitle)} - BBSMC`;
  },
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const auth = await useAuth();
const route = useNativeRoute();

if (route.query.code && !route.fullPath.includes("new_account=true")) {
  await finishSignIn();
}

if (auth.value.user) {
  await finishSignIn();
}

const email = ref("");
const password = ref("");

const flow = ref(route.query.flow);

const redirectTarget = route.query.redirect || "/dashboard";

const signUpLink = computed(
  () => `/auth/sign-up${route.query.redirect ? `?redirect=${route.query.redirect}` : ""}`,
);

async function beginPasswordSignIn() {
  startLoading();
  try {
    const res = await useBaseFetch("auth/login", {
      method: "POST",
      body: {
        username: email.value,
        password: password.value,
        challenge: token.value,
      },
    });

    if (res.flow) {
      flow.value = res.flow;
    } else {
      await finishSignIn(res.session);
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

const twoFactorCode = ref(null);
async function begin2FASignIn() {
  startLoading();
  try {
    const res = await useBaseFetch("auth/login/2fa", {
      method: "POST",
      body: {
        flow: flow.value,
        code: twoFactorCode.value ? twoFactorCode.value.toString() : twoFactorCode.value,
      },
    });

    await finishSignIn(res.session);
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

async function finishSignIn(token) {
  if (token) {
    await useAuth(token);
    await useUser();
  }

  if (route.query.redirect) {
    const redirect = decodeURIComponent(route.query.redirect);
    await navigateTo(redirect, {
      replace: true,
    });
  } else {
    await navigateTo("/dashboard");
  }
}
</script>
