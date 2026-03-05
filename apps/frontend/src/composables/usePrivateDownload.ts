/**
 * 私有文件下载处理
 * 用于处理付费资源的私有文件下载，自动检测 private:// URL 并获取 presigned URL
 */

const PRIVATE_URL_PREFIX = "private://";

/**
 * 检查 URL 是否是私有文件
 */
export function isPrivateUrl(url: string): boolean {
  return url.startsWith(PRIVATE_URL_PREFIX);
}

/**
 * 从文件 URL 中提取用于 API 调用的 hash
 * 需要先获取文件的 hash，然后调用 /version_file/{hash}/download API
 */
export interface VersionFile {
  url: string;
  filename: string;
  hashes: {
    sha1?: string;
    sha512?: string;
  };
  primary: boolean;
  size: number;
  file_type?: string;
}

/**
 * 获取文件下载 URL
 * - 对于公共文件，直接返回原 URL
 * - 对于私有文件，调用 API 获取 presigned URL
 */
export async function getDownloadUrl(file: VersionFile): Promise<string> {
  // 公共文件直接返回 URL
  if (!isPrivateUrl(file.url)) {
    return file.url;
  }

  // 私有文件需要调用 API 获取 presigned URL
  const hash = file.hashes.sha1 || file.hashes.sha512;
  if (!hash) {
    throw new Error("文件缺少 hash 信息，无法下载");
  }

  const algorithm = file.hashes.sha1 ? "sha1" : "sha512";

  try {
    // 调用下载 API，后端会返回 307 重定向或 JSON 响应
    const response = await useBaseFetch(`version_file/${hash}/download`, {
      method: "GET",
      apiVersion: 3,
      query: { algorithm },
    });

    // API 返回 { url: "presigned_url" }
    if (response && typeof response === "object" && "url" in response) {
      return (response as { url: string }).url;
    }

    throw new Error("无法获取下载链接");
  } catch (error: any) {
    // 处理各种错误
    if (error.statusCode === 401) {
      throw new Error("请先登录后再下载");
    }
    if (error.statusCode === 403) {
      throw new Error("您需要购买此资源才能下载文件");
    }
    if (error.statusCode === 404) {
      throw new Error("文件不存在或已被删除");
    }
    if (error.statusCode === 500) {
      throw new Error("服务器错误，请稍后重试");
    }
    throw error;
  }
}

/**
 * 下载私有文件
 * 获取 presigned URL 并触发下载
 * 使用创建隐藏 a 标签的方式，避免被浏览器弹窗拦截器阻止
 */
export async function downloadPrivateFile(file: VersionFile): Promise<void> {
  const url = await getDownloadUrl(file);

  // 创建隐藏的 a 标签触发下载，避免 window.open 被拦截
  const link = document.createElement("a");
  link.href = url;
  link.download = file.filename || "";
  link.style.display = "none";
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/**
 * usePrivateDownload composable
 * 提供响应式的下载状态和方法
 */
export function usePrivateDownload() {
  const isDownloading = ref(false);
  const downloadError = ref<string | null>(null);
  const app = useNuxtApp();

  const download = async (file: VersionFile) => {
    isDownloading.value = true;
    downloadError.value = null;

    try {
      await downloadPrivateFile(file);
    } catch (error: any) {
      downloadError.value = error.message || "下载失败";
      app.$notify({
        group: "main",
        title: "下载失败",
        text: downloadError.value,
        type: "error",
      });
    } finally {
      isDownloading.value = false;
    }
  };

  /**
   * 获取文件的下载处理函数
   * 返回一个点击处理函数，用于绑定到下载按钮
   */
  const getDownloadHandler = (file: VersionFile) => {
    return async (event: Event) => {
      if (isPrivateUrl(file.url)) {
        event.preventDefault();
        await download(file);
      }
      // 公共文件让浏览器默认处理 <a> 标签
    };
  };

  /**
   * 获取文件的下载 URL
   * 对于公共文件返回原 URL，对于私有文件返回 # 并需要使用 getDownloadHandler
   */
  const getHref = (file: VersionFile): string => {
    if (isPrivateUrl(file.url)) {
      return "#"; // 私有文件使用 JS 处理
    }
    return file.url;
  };

  return {
    isDownloading,
    downloadError,
    download,
    getDownloadHandler,
    getHref,
    isPrivateUrl,
  };
}
