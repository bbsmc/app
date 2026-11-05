import {
  SITEMAP_CHUNK_SIZE,
  getSiteUrl,
  getSitemapUrls,
  renderUrlset,
} from "../../utils/sitemap.ts";

// /sitemap/{n}.xml：URL 总数超过 50000 时，由 /sitemap.xml 的索引指向这里的分片。
// Nitro 路由（radix3）不支持 `sitemap-:page.xml` 这种混合段，所以分片放在 /sitemap/ 目录下，页码带 .xml 后缀。
export default defineEventHandler(async (event) => {
  const match = /^(\d+)(?:\.xml)?$/.exec(getRouterParam(event, "page") ?? "");
  const page = Number.parseInt(match?.[1] ?? "", 10);
  if (!Number.isInteger(page) || page < 1) {
    throw createError({ statusCode: 404, statusMessage: "Not Found" });
  }

  const urls = await getSitemapUrls(event);
  const start = (page - 1) * SITEMAP_CHUNK_SIZE;
  // 总数没超过单个 sitemap 上限时不存在分片（避免和 /sitemap.xml 重复），页码越界同样 404
  if (urls.length <= SITEMAP_CHUNK_SIZE || start >= urls.length) {
    throw createError({ statusCode: 404, statusMessage: "Not Found" });
  }

  setHeader(event, "content-type", "application/xml; charset=utf-8");
  setHeader(event, "cache-control", "public, max-age=3600");
  return renderUrlset(getSiteUrl(event), urls.slice(start, start + SITEMAP_CHUNK_SIZE));
});
