<template>
  <NuxtLayout>
    <ModrinthLoadingIndicator />
    <Notifications />
    <NuxtPage />
  </NuxtLayout>
</template>
<script setup lang="ts">
import { provideNotificationManager } from "@modrinth/ui";
import ModrinthLoadingIndicator from "~/components/ui/modrinth-loading-indicator.ts";
import Notifications from "~/components/ui/Notifications.vue";
import { addNotification } from "~/composables/notifs.js";
import { createModrinthClient, provideModrinthClient } from "~/providers/api-client.ts";

// Provide notification manager for components that use injectNotificationManager
provideNotificationManager({
  addNotification,
});

// Provide modrinth client for components that use injectModrinthClient
// (新版本管理 modal 等使用此 client 调用 v3 API)
const config = useRuntimeConfig();

// 直接引用 auth state（不 await，避免 SSR 阻塞）。token 通过 getter 在每次请求时读取最新值。
const authState = useState<{ token?: string }>("auth", () => ({ token: "" }));

// BBSMC apiBaseUrl 形如 "http://api.bbsmc.net/v2/"，需要去掉 /v\d+/ 后缀
// 因为 NuxtModrinthClient 内部会按 version 自己拼接 /v2 或 /v3
const rawApiBaseUrl = (config.public.apiBaseUrl as string) || "";
const apiBaseUrl = rawApiBaseUrl.replace(/\/v\d+\/?$/, "/");

const authProxy = {
  get token() {
    return authState.value?.token;
  },
};

const modrinthClient = createModrinthClient(authProxy, {
  apiBaseUrl,
  archonBaseUrl: apiBaseUrl, // BBSMC 不使用 archon (Modrinth Servers)，传同一个占位
  rateLimitKey: (config as any).rateLimitKey,
});

provideModrinthClient(modrinthClient);
</script>
