import { FetchError } from "ofetch";
import { addNotification } from "~/composables/notifs.js";

export default defineNuxtPlugin((nuxtApp) => {
  // 全局错误处理器
  nuxtApp.hook("vue:error", (error, _instance, _info) => {
    // 检查是否为 FetchError
    if (error instanceof FetchError) {
      const statusCode = error.response?.status;
      const errorData = error.data;

      // 处理 429 限流错误
      if (statusCode === 429 || errorData?.error === "ratelimit_error") {
        const description = errorData?.description || "您的请求过于频繁";
        addNotification({
          group: "main",
          title: "请求过于频繁",
          text: `${description}，请稍后再试。`,
          type: "warn",
        });
        // 阻止错误继续传播
        return;
      }

      // 处理封禁错误
      if (errorData?.error === "user_banned") {
        const description = errorData?.description || "您的账户已被封禁";
        addNotification({
          group: "main",
          title: "操作受限",
          text: `${description}。如有疑问，请前往账户设置查看详情或发起申诉。`,
          type: "error",
        });
      }
    }
  });
});
