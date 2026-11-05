import type { H3Event } from "h3";

/**
 * sitemap 数据源全部走 labrinth 公开 API，不碰数据库：
 * - 资源页：/v2/search 分页枚举。搜索索引里只有 Approved / Archived 的公开资源，未审核的不会出现
 * - 帖子页：/v3/forum/{category}/lists 分页枚举，接口本身已过滤 deleted
 * 结果用 defineCachedFunction 缓存 1 小时，过期后先返回旧数据、后台刷新（swr）。
 */

/** sitemaps.org 规定单个 sitemap 最多 50000 条 URL，超过时 /sitemap.xml 自动改为 sitemap 索引 */
export const SITEMAP_CHUNK_SIZE = 50_000;

/** labrinth /v2/search 服务端会把 limit clamp 到 100 */
const SEARCH_PAGE_SIZE = 100;
/** 论坛列表接口没有上限，但每条都要统计回复数，别一次拉太多 */
const FORUM_PAGE_SIZE = 50;
/** 单个枚举任务同时向 labrinth 发出的请求数（资源和论坛两个任务并行，峰值为 2 倍），避免触发限流 */
const CONCURRENCY = 6;

export interface SitemapUrl {
  /** 站内路径，渲染时再拼 siteUrl，缓存内容与域名无关 */
  loc: string;
  /** ISO 8601 时间 */
  lastmod?: string;
}

/** 与 src/public/robots.txt 的 Disallow 保持一致，兜底过滤 */
const DISALLOWED_PREFIXES = [
  "/dashboard",
  "/settings",
  "/moderation",
  "/auth",
  "/report",
  "/flags",
];

/** 固定的公开页面：首页、各资源类型列表页、论坛板块、教程和协议页（分页 ?o= 不收录） */
const STATIC_PATHS = [
  "/",
  "/mods",
  "/modpacks",
  "/plugins",
  "/resourcepacks",
  "/shaders",
  "/datapacks",
  "/softwares",
  "/languages",
  "/maps",
  "/forums/chat",
  "/forums/notice",
  "/forums/project",
  "/forums/article",
  "/install-tutorial",
  "/legal/terms",
  "/legal/privacy",
  "/legal/rules",
  "/legal/copyright",
  "/legal/incentive",
  "/legal/incentive-info",
];

/** 独立帖子 /d/<id> 只在这三个板块；project 板块的帖子都挂在资源下，由 /{type}/{slug}/forum 覆盖 */
const STANDALONE_FORUM_CATEGORIES = ["chat", "notice", "article"];

interface SearchHit {
  project_id: string;
  project_type: string;
  slug: string | null;
  date_modified?: string;
  gallery?: string[];
  versions?: string[];
}

interface SearchResponse {
  hits: SearchHit[];
  total_hits: number;
}

interface ForumListItem {
  id: string;
  project_id?: string | null;
  deleted?: boolean;
  created_at?: string;
  updated_at?: string | null;
  last_post_time?: string;
}

interface ForumListResponse {
  forums: ForumListItem[];
  pagination: { total: number };
}

interface ApiContext {
  v2: string;
  v3: string;
  headers: Record<string, string>;
}

function getApiContext(event?: H3Event): ApiContext {
  const config = useRuntimeConfig(event);
  // 服务端配置形如 https://api.bbsmc.net/v2/，去掉版本段后分别拼 v2 / v3
  const base = `${String(config.apiBaseUrl || config.public.apiBaseUrl).replace(/\/+$/, "")}/`;
  const root = base.replace(/\/v\d+\/$/, "/");
  const headers: Record<string, string> = {
    "user-agent": "BBSMC sitemap generator (support@bbsmc.net)",
  };
  if (config.rateLimitKey) {
    headers["x-ratelimit-key"] = String(config.rateLimitKey);
  }
  return { v2: `${root}v2/`, v3: `${root}v3/`, headers };
}

export function getSiteUrl(event: H3Event): string {
  return String(useRuntimeConfig(event).public.siteUrl).replace(/\/+$/, "");
}

function apiFetch<T>(ctx: ApiContext, url: string): Promise<T> {
  return $fetch<T>(url, {
    headers: ctx.headers,
    timeout: 15_000,
    retry: 2,
    retryDelay: 1000,
  });
}

/** 限制并发地跑完 items，结果顺序与输入一致 */
async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  fn: (item: T) => Promise<R>,
): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await fn(items[index] as T);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

function toLastmod(value?: string | null): string | undefined {
  if (!value) {
    return undefined;
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? undefined : date.toISOString();
}

/** 多个时间里取最晚的一个 */
function newest(...values: (string | null | undefined)[]): string | undefined {
  let best: Date | undefined;
  for (const value of values) {
    if (!value) {
      continue;
    }
    const date = new Date(value);
    if (!Number.isNaN(date.getTime()) && (!best || date > best)) {
      best = date;
    }
  }
  return best?.toISOString();
}

async function fetchProjectHits(ctx: ApiContext): Promise<SearchHit[]> {
  const fetchPage = (offset: number) =>
    apiFetch<SearchResponse>(
      ctx,
      `${ctx.v2}search?limit=${SEARCH_PAGE_SIZE}&offset=${offset}&index=newest`,
    );
  const first = await fetchPage(0);
  const offsets: number[] = [];
  for (
    let offset = SEARCH_PAGE_SIZE;
    offset < (first.total_hits ?? 0);
    offset += SEARCH_PAGE_SIZE
  ) {
    offsets.push(offset);
  }
  const rest = await mapWithConcurrency(offsets, CONCURRENCY, fetchPage);
  return [first, ...rest].flatMap((page) => page.hits ?? []);
}

async function fetchForumList(ctx: ApiContext, category: string): Promise<ForumListItem[]> {
  const fetchPage = (page: number) =>
    apiFetch<ForumListResponse>(
      ctx,
      `${ctx.v3}forum/${category}/lists?page=${page}&page_size=${FORUM_PAGE_SIZE}`,
    );
  const first = await fetchPage(1);
  const totalPages = Math.ceil((first.pagination?.total ?? 0) / FORUM_PAGE_SIZE);
  const pages: number[] = [];
  for (let page = 2; page <= totalPages; page++) {
    pages.push(page);
  }
  const rest = await mapWithConcurrency(pages, CONCURRENCY, fetchPage);
  return [first, ...rest]
    .flatMap((response) => response.forums ?? [])
    .filter((discussion) => discussion && !discussion.deleted);
}

/** 四个板块按顺序拉完（每个板块内部分页并发），和资源枚举并行跑 */
async function fetchForumLists(ctx: ApiContext): Promise<Record<string, ForumListItem[]>> {
  const lists: Record<string, ForumListItem[]> = {};
  for (const category of ["project", ...STANDALONE_FORUM_CATEGORIES]) {
    lists[category] = await fetchForumList(ctx, category);
  }
  return lists;
}

async function buildSitemapUrls(event?: H3Event): Promise<SitemapUrl[]> {
  const ctx = getApiContext(event);
  const urls: SitemapUrl[] = STATIC_PATHS.map((loc) => ({ loc }));

  const [hits, forumLists] = await Promise.all([fetchProjectHits(ctx), fetchForumLists(ctx)]);

  // project 板块的帖子都挂在某个资源下：只给真的有讨论的资源收录 /forum 子页，并用最近回复时间作 lastmod
  const discussedProjects = new Map<string, string | undefined>();
  for (const discussion of forumLists.project ?? []) {
    if (!discussion.project_id) {
      continue;
    }
    discussedProjects.set(
      discussion.project_id,
      newest(
        discussedProjects.get(discussion.project_id),
        discussion.last_post_time,
        discussion.updated_at,
        discussion.created_at,
      ),
    );
  }

  const seenProjects = new Set<string>();
  for (const hit of hits) {
    if (!hit?.project_id || !hit.project_type || seenProjects.has(hit.project_id)) {
      continue;
    }
    seenProjects.add(hit.project_id);
    const base = `/${hit.project_type}/${hit.slug || hit.project_id}`;
    const lastmod = toLastmod(hit.date_modified);
    urls.push({ loc: base, lastmod });
    // 子页只收录有内容的：没有版本就不收 versions / changelog，没有图集就不收 gallery
    if (hit.versions?.length) {
      urls.push({ loc: `${base}/versions`, lastmod }, { loc: `${base}/changelog`, lastmod });
    }
    if (hit.gallery?.length) {
      urls.push({ loc: `${base}/gallery`, lastmod });
    }
    if (discussedProjects.has(hit.project_id)) {
      urls.push({ loc: `${base}/forum`, lastmod: discussedProjects.get(hit.project_id) });
    }
  }

  for (const category of STANDALONE_FORUM_CATEGORIES) {
    for (const discussion of forumLists[category] ?? []) {
      if (!discussion.id || discussion.project_id) {
        continue;
      }
      urls.push({
        loc: `/d/${discussion.id}`,
        lastmod: newest(discussion.updated_at, discussion.last_post_time, discussion.created_at),
      });
    }
  }

  return urls.filter((url) => !DISALLOWED_PREFIXES.some((prefix) => url.loc.startsWith(prefix)));
}

export const getSitemapUrls = defineCachedFunction(buildSitemapUrls, {
  name: "sitemap-urls",
  getKey: () => "all",
  maxAge: 60 * 60,
  swr: true,
});

function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

export function renderUrlset(siteUrl: string, urls: SitemapUrl[]): string {
  const entries = urls.map(
    ({ loc, lastmod }) =>
      `  <url><loc>${escapeXml(encodeURI(`${siteUrl}${loc}`))}</loc>${
        lastmod ? `<lastmod>${lastmod}</lastmod>` : ""
      }</url>`,
  );
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...entries,
    "</urlset>",
    "",
  ].join("\n");
}

export function renderSitemapIndex(locs: string[]): string {
  const entries = locs.map((loc) => `  <sitemap><loc>${escapeXml(loc)}</loc></sitemap>`);
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...entries,
    "</sitemapindex>",
    "",
  ].join("\n");
}
