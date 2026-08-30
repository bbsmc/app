<template>
  <NewModal ref="modal" header="微信登录" :on-hide="reset">
    <div class="wechat-login-modal">
      <div :id="containerId" class="wechat-login-container" />
      <div v-if="isLoading" class="wechat-login-loading">正在打开微信登录...</div>
    </div>
  </NewModal>
</template>

<script setup>
import { NewModal } from "@modrinth/ui";
import { getAuthInitUrl, getOAuthPopupCallbackUrl } from "@/composables/auth.js";

const props = defineProps({
  redirectTarget: {
    type: String,
    default: "/dashboard",
  },
});

const emit = defineEmits(["authenticated"]);

const WECHAT_LOGIN_SCRIPT_URL = "https://res.wx.qq.com/connect/zh_CN/htmledition/js/wxLogin.js";
const containerId = "wechat-login-container";

const modal = ref();
const isLoading = ref(false);
let scriptLoadingPromise = null;

async function show(event) {
  isLoading.value = true;
  modal.value?.show(event);

  try {
    await nextTick();
    clearLoginContainer();

    const callbackUrl = getOAuthPopupCallbackUrl(props.redirectTarget);
    const authUrl = await getAuthInitUrl("wechat", callbackUrl);

    await loadWeChatLoginScript();
    renderWeChatLogin(authUrl);
  } catch (err) {
    addNotification({
      group: "main",
      title: "发生错误",
      text: err.data?.description || err.message || "无法打开微信登录。",
      type: "error",
    });
    modal.value?.hide();
  } finally {
    isLoading.value = false;
  }
}

function hide() {
  modal.value?.hide();
}

function reset() {
  isLoading.value = false;
  clearLoginContainer();
}

function clearLoginContainer() {
  document.getElementById(containerId)?.replaceChildren();
}

function loadWeChatLoginScript() {
  if (window.WxLogin) {
    return Promise.resolve();
  }

  if (scriptLoadingPromise) {
    return scriptLoadingPromise;
  }

  scriptLoadingPromise = new Promise((resolve, reject) => {
    const existingScript = document.querySelector(`script[src="${WECHAT_LOGIN_SCRIPT_URL}"]`);

    if (existingScript) {
      existingScript.addEventListener("load", resolve, { once: true });
      existingScript.addEventListener("error", reject, { once: true });
      return;
    }

    const script = document.createElement("script");
    script.src = WECHAT_LOGIN_SCRIPT_URL;
    script.async = true;
    script.onload = resolve;
    script.onerror = reject;
    document.head.appendChild(script);
  });

  return scriptLoadingPromise;
}

function renderWeChatLogin(authUrl) {
  const url = new URL(authUrl);
  const appid = url.searchParams.get("appid");
  const redirectUri = url.searchParams.get("redirect_uri");
  const state = url.searchParams.get("state");

  if (!appid || !redirectUri || !state) {
    throw new Error("微信登录初始化参数无效。");
  }

  const surfaceColor = getLoginSurfaceColor();
  const isDark = isDarkSurface(surfaceColor);

  return new window.WxLogin({
    self_redirect: true,
    id: containerId,
    appid,
    scope: url.searchParams.get("scope") || "snsapi_login",
    redirect_uri: encodeURIComponent(redirectUri),
    state,
    style: isDark ? "white" : "black",
    color_scheme: isDark ? "dark" : "light",
    fast_login: "1",
    href: buildWeChatLoginStyle(isDark, surfaceColor),
  });
}

function getLoginSurfaceColor() {
  const loginModal = document.querySelector(".wechat-login-modal");
  const modalBackground = loginModal ? getComputedStyle(loginModal).backgroundColor : "";

  return (
    modalBackground ||
    getComputedStyle(document.documentElement).getPropertyValue("--color-bg-raised").trim() ||
    getComputedStyle(document.body).backgroundColor ||
    "#ffffff"
  );
}

function isDarkSurface(surfaceColor) {
  const match = surfaceColor.match(/\d+(\.\d+)?/g);

  if (!match || match.length < 3) {
    return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
  }

  const [red, green, blue] = match.map(Number);
  const luminance = (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255;
  return luminance < 0.5;
}

function buildWeChatLoginStyle(isDark, backgroundColor) {
  const textColor = isDark ? "#f5f7fa" : "#333333";
  const secondaryColor = isDark ? "#9ca3af" : "#8492a6";
  const css = `
    html,
    body,
    .impowerBox,
    .web_qrcode_area,
    .web_qrcode_panel_area,
    .qlogin_mod {
      background: ${backgroundColor} !important;
    }

    .title {
      display: none;
    }

    .impowerBox .qrcode {
      width: 180px;
      border: none;
      margin: 0 auto;
    }

    .impowerBox .info {
      width: 260px;
    }

    .impowerBox .status {
      padding: 0;
    }

    .impowerBox .status p,
    .web_qrcode_switch_wrp,
    .qlogin_msg {
      font-size: 14px;
      color: ${secondaryColor} !important;
    }

    .qlogin_mod,
    .qlogin_user_nickname {
      color: ${textColor} !important;
    }

    .qlogin_btn {
      border-radius: 4px;
    }
  `;

  return `data:text/css;base64,${btoa(css)}`;
}

function handleMessage(event) {
  if (event.origin !== window.location.origin) {
    return;
  }

  if (event.data?.type !== "bbsmc:oauth-popup-result") {
    return;
  }

  emit("authenticated", event.data);
}

onMounted(() => {
  window.addEventListener("message", handleMessage);
});

onBeforeUnmount(() => {
  window.removeEventListener("message", handleMessage);
});

defineExpose({
  show,
  hide,
});
</script>

<style scoped>
.wechat-login-modal {
  position: relative;
  width: min(300px, calc(100vw - 2rem));
  height: 330px;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  overflow: hidden;
  background: var(--color-bg-raised);
}

.wechat-login-container {
  width: 300px;
  height: 330px;
  overflow: hidden;
}

.wechat-login-container :deep(iframe) {
  display: block;
  width: 300px;
  height: 330px;
  border: 0;
  background: transparent;
}

.wechat-login-loading {
  position: absolute;
  inset: 0;
  flex: 1;
  display: grid;
  place-items: center;
  background: var(--color-bg-raised);
  color: var(--color-secondary);
  text-align: center;
}

:global(.modal-body:has(.wechat-login-modal)) {
  --_max-width: 380px;

  overflow: hidden;
  border-radius: 6px !important;
}

:global(.modal-body:has(.wechat-login-modal) > div:nth-child(2)) {
  padding: 0;
  overflow: hidden;
}
</style>
