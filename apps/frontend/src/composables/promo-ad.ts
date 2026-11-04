// 侧边栏推广图（ResourcePromoAd，搜索页和资源详情页共用）的关闭状态。
// 存在 cookie 而不是 localStorage：SSR 时就能读到，已关闭的用户不会先看到广告再闪没。
// cookie 24 小时后由浏览器自动删除，广告恢复显示。两个广告位共用这一个状态。
const PROMO_AD_DISMISS_COOKIE = "promo-ad-dismissed";
const PROMO_AD_DISMISS_SECONDS = 60 * 60 * 24;

export function usePromoAdDismissal() {
  const cookie = useCookie<boolean | null>(PROMO_AD_DISMISS_COOKIE, {
    maxAge: PROMO_AD_DISMISS_SECONDS,
    sameSite: "lax",
    path: "/",
  });
  // 用 useState 让同一页面里的多个调用方（广告组件、搜索页轮播）立即看到关闭结果
  const dismissed = useState(PROMO_AD_DISMISS_COOKIE, () => cookie.value === true);

  function dismiss() {
    dismissed.value = true;
    cookie.value = true;
  }

  return { dismissed: readonly(dismissed), dismiss };
}
