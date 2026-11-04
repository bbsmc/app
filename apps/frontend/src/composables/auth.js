export const useAuth = async (oldToken = null) => {
  const auth = useState("auth", () => ({
    user: null,
    token: "",
    headers: {},
  }));

  if (!auth.value.user || oldToken) {
    auth.value = await initAuth(oldToken);
  }

  return auth;
};

// 上游修复: aeb9f5a07 (#6186) - cookie / query 的值可能被 destr 解析成数字、布尔或
// JSON（对象/数组），直接 startsWith 会抛 TypeError 并让整页 SSR 500。统一归一成字符串。
function normalizeAuthToken(value) {
  return typeof value === "string" ? value : "";
}

function getErrorStatus(error) {
  const status = error?.statusCode ?? error?.status;
  return typeof status === "number" ? status : null;
}

// 上游修复: 295db3fb7 - 只有在真正的鉴权失败（401/403）时才刷新 token / 清除 cookie；
// 超时、5xx、网络错误、429 都不应导致登出。
function isAuthFailure(error) {
  const status = getErrorStatus(error);
  return status === 401 || status === 403;
}

export const initAuth = async (oldToken = null) => {
  const auth = {
    user: null,
    token: "",
  };

  if (oldToken === "none") {
    return auth;
  }

  const route = useRoute();
  const authCookie = useCookie("auth-token", {
    maxAge: 60 * 60 * 24 * 365 * 10,
    sameSite: "lax",
    secure: true,
    httpOnly: false,
    path: "/",
  });

  const normalizedOldToken = normalizeAuthToken(oldToken);
  if (normalizedOldToken) {
    authCookie.value = normalizedOldToken;
  }

  const oauthCode = normalizeAuthToken(route.query.code);
  if (oauthCode && !route.fullPath.includes("new_account=true")) {
    authCookie.value = oauthCode;
  }

  if (route.fullPath.includes("new_account=true") && route.path !== "/auth/welcome") {
    const redirect = route.path.startsWith("/auth/") ? null : route.fullPath;

    await navigateTo(
      `/auth/welcome?authToken=${oauthCode}${
        redirect ? `&redirect=${encodeURIComponent(redirect)}` : ""
      }`,
    );
  }

  const tokenStr = normalizeAuthToken(authCookie.value);
  // cookie 被写成了非字符串，清除以免后续报错
  if (authCookie.value != null && tokenStr === "") {
    authCookie.value = null;
    return auth;
  }

  let shouldRefresh = false;
  if (tokenStr) {
    auth.token = tokenStr;

    if (!auth.token.startsWith("mra_")) {
      return auth;
    }

    try {
      auth.user = await useBaseFetch(
        "user",
        {
          headers: {
            Authorization: auth.token,
          },
        },
        true,
      );
    } catch (error) {
      // 仅鉴权失败才尝试刷新；超时/5xx/网络错误保留现状，不登出
      shouldRefresh = isAuthFailure(error);
    }
  }

  if (!auth.user && auth.token && shouldRefresh) {
    try {
      const session = await useBaseFetch(
        "session/refresh",
        {
          method: "POST",
          headers: {
            Authorization: auth.token,
          },
        },
        true,
      );

      const refreshed = normalizeAuthToken(session?.session);
      if (refreshed) {
        auth.token = refreshed;
        authCookie.value = auth.token;

        try {
          auth.user = await useBaseFetch(
            "user",
            {
              headers: {
                Authorization: auth.token,
              },
            },
            true,
          );
        } catch (error) {
          if (isAuthFailure(error)) {
            authCookie.value = null;
            auth.token = "";
            auth.user = null;
          }
        }
      } else {
        authCookie.value = null;
        auth.token = "";
      }
    } catch (error) {
      // 刷新失败：仅鉴权失败才清除 cookie
      if (isAuthFailure(error)) {
        authCookie.value = null;
        auth.token = "";
      }
    }
  }

  return auth;
};

// Upstream fix e368e35e7: Properly finish auth when redirect URI is supplied
export const getAuthUrl = (provider, redirect = "/dashboard") => {
  const config = useRuntimeConfig();
  const route = useNativeRoute();

  const fullURL = route.query.launcher
    ? "https://launcher-files.modrinth.com"
    : `${config.public.siteUrl}/auth/sign-in?redirect=${redirect}`;

  return `${config.public.apiBaseUrl}auth/init?provider=${provider}&url=${encodeURIComponent(fullURL)}`;
};

export const getAuthInitUrl = async (provider, callbackUrl, token = null) => {
  const config = useRuntimeConfig();
  const params = new URLSearchParams({
    provider,
    url: callbackUrl,
    return_url: "true",
  });

  if (token) {
    params.set("token", token);
  }

  const response = await $fetch(`${config.public.apiBaseUrl}auth/init?${params.toString()}`);
  return response.url;
};

export const getOAuthPopupCallbackUrl = (redirect = "/dashboard") => {
  const config = useRuntimeConfig();
  const callbackUrl = new URL("/auth/oauth-popup", config.public.siteUrl);
  callbackUrl.searchParams.set("redirect", redirect);
  return callbackUrl.toString();
};

export const removeAuthProvider = async (provider) => {
  startLoading();
  try {
    const auth = await useAuth();

    await useBaseFetch("auth/provider", {
      method: "DELETE",
      body: {
        provider,
      },
    });
    await useAuth(auth.value.token);
  } catch (err) {
    const data = useNuxtApp();
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
};
