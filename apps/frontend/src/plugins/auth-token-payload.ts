// 上游修复: 46f7315d1 (#7872) - 让会话 token 不进入 SSR payload。
// 服务端渲染完成后把 payload 里的 token 清空，客户端再从 auth-token cookie 还原，
// 这样即便某层缓存存下了 HTML，泄露的也只是空 token。
//
// BBSMC 适配说明：上游用的是 useAuthState() + @/composables/auth-cookie.ts，
// 本仓库没有这两者，这里直接复用 useState("auth") 和 auth.js 里一致的 cookie 配置。
export default defineNuxtPlugin({
  name: "auth-token-payload",
  enforce: "pre",
  setup() {
    const auth = useState("auth", () => ({
      user: null,
      token: "",
      headers: {},
    }));

    if (import.meta.server) {
      const nuxtApp = useNuxtApp();
      nuxtApp.hooks.hook("app:rendered", () => {
        if (auth.value) {
          auth.value = { ...auth.value, token: "" };
        }
      });
      return;
    }

    // 客户端：从 cookie 还原 token（与 initAuth 使用相同的 cookie 名与选项）
    const authCookie = useCookie("auth-token", {
      maxAge: 60 * 60 * 24 * 365 * 10,
      sameSite: "lax",
      secure: true,
      httpOnly: false,
      path: "/",
    });
    if (auth.value) {
      auth.value.token = authCookie.value ?? "";
    }
  },
});
