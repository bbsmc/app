import { getSitemapUrls } from "../utils/sitemap.ts";

// 服务启动后预热 sitemap 缓存：生成一次要向 labrinth 发几十个请求（约 10 秒），
// 不预热的话重启后第一个来抓 sitemap 的爬虫就要等这一轮。dev 模式下 Nitro 会频繁重建，不预热。
export default defineNitroPlugin(() => {
  if (import.meta.dev) {
    return;
  }
  getSitemapUrls().catch((error) => {
    console.warn("[sitemap] 预热失败，将在首次请求时重新生成:", error);
  });
});
