import {
  SITEMAP_CHUNK_SIZE,
  getSiteUrl,
  getSitemapUrls,
  renderSitemapIndex,
  renderUrlset,
} from "../utils/sitemap.ts";

// /sitemap.xml：URL 总数不超过 50000 时直接输出 urlset；
// 超过时输出 sitemap 索引，指向 /sitemap/1.xml、/sitemap/2.xml …（见 routes/sitemap/[page].ts）
export default defineEventHandler(async (event) => {
  const urls = await getSitemapUrls(event);
  const siteUrl = getSiteUrl(event);

  setHeader(event, "content-type", "application/xml; charset=utf-8");
  setHeader(event, "cache-control", "public, max-age=3600");

  if (urls.length <= SITEMAP_CHUNK_SIZE) {
    return renderUrlset(siteUrl, urls);
  }
  const chunkCount = Math.ceil(urls.length / SITEMAP_CHUNK_SIZE);
  return renderSitemapIndex(
    Array.from({ length: chunkCount }, (_, index) => `${siteUrl}/sitemap/${index + 1}.xml`),
  );
});
