// 上游修复: 46f7315d1 (#7872) - 渲染响应统一标记为 private, no-store，
// 避免 SSR HTML（内含会话 token 和用户隐私数据）被 CDN / 反代缓存后发给其他访客。
// 仅影响动态渲染响应（HTML、/_payload.js、island），不影响 /_nuxt/** 静态资源。
export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook("render:response", (response) => {
    response.headers ??= {};
    response.headers["cache-control"] = "private, no-store";
  });
});
