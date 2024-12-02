export const useBaseFetch = async (url, options = {}, skipAuth = false) => {
  const config = useRuntimeConfig();
  let base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;

  if (!options.headers) {
    options.headers = {};
  }

  if (import.meta.server) {
    options.headers["x-ratelimit-key"] = config.rateLimitKey;
  }

  if (!skipAuth) {
    const auth = await useAuth();

    options.headers.Authorization = auth.value.token;
  }

  if (options.apiVersion || options.internal) {
    // Base may end in /vD/ or /vD. We would need to replace the digit with the new version number
    // and keep the trailing slash if it exists
    const baseVersion = base.match(/\/v\d\//);

    const replaceStr = options.internal ? `/_internal/` : `/v${options.apiVersion}/`;

    if (baseVersion) {
      base = base.replace(baseVersion[0], replaceStr);
    } else {
      base = base.replace(/\/v\d$/, replaceStr);
    }

    delete options.apiVersion;
  }

  return await $fetch(`${base}${url}`, options);
};
export const useBaseFetchFile = async (url, options = {}, skipAuth = false) => {
  const config = useRuntimeConfig();
  let base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;

  if (!options.headers) {
    options.headers = {};
  }

  if (import.meta.server) {
    options.headers["x-ratelimit-key"] = config.rateLimitKey;
  }

  if (!skipAuth) {
    const auth = await useAuth();
    options.headers.Authorization = auth.value.token;
  }

  if (options.apiVersion || options.internal) {
    const baseVersion = base.match(/\/v\d\//);
    const replaceStr = options.internal ? `/_internal/` : `/v${options.apiVersion}/`;

    if (baseVersion) {
      base = base.replace(baseVersion[0], replaceStr);
    } else {
      base = base.replace(/\/v\d$/, replaceStr);
    }

    delete options.apiVersion;
  }

  // 判断是否为文件上传
  if (options.body instanceof FormData) {
    const { body, onUploadProgress,onError } = options;
    const xhr = new XMLHttpRequest();
    xhr.open(options.method || 'POST', `${base}${url}`, true);

    // 设置请求头
    for (const [key, value] of Object.entries(options.headers)) {
      xhr.setRequestHeader(key, value);
    }
    let lastLoaded = 0;
    let lastTime = Date.now();

    // 追踪上传进度
    if (onUploadProgress && xhr.upload) {
      xhr.upload.onprogress = function(event) {
        if (event.lengthComputable) {
          const percentComplete = (event.loaded / event.total) * 100;
          const currentTime = Date.now();
          const timeDiff = (currentTime - lastTime) / 1000; // 秒
          const bytesDiff = event.loaded - lastLoaded;
          const uploadSpeed = (bytesDiff / timeDiff) / (1024 * 1024); // Mbps

          lastLoaded = event.loaded;
          lastTime = currentTime;

          onUploadProgress(percentComplete, uploadSpeed);
        }
      };
    }

    // 返回一个 Promise，处理请求完成
    return new Promise((resolve, reject) => {
      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve(JSON.parse(xhr.responseText));
        } else {
          onError(JSON.parse(xhr.responseText));
        }
      };

      xhr.onerror = () => reject(xhr.statusText);

      xhr.send(body);
    });
  } else {
    // 非文件上传的请求使用 $fetch
    return await $fetch(`${base}${url}`, options);
  }
};
