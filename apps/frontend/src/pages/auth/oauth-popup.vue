<template>
  <div class="oauth-popup-status">
    {{ statusMessage }}
  </div>
</template>

<script setup>
const route = useRoute();

const statusMessage = computed(() => {
  if (route.query.error) {
    return "登录未完成，请返回登录窗口重试。";
  }

  if (route.query.code) {
    return "登录成功，正在返回...";
  }

  return "正在等待登录结果...";
});

useHead({
  title: "微信登录 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

onMounted(async () => {
  const payload = {
    type: "bbsmc:oauth-popup-result",
    code: typeof route.query.code === "string" ? route.query.code : null,
    error: typeof route.query.error === "string" ? route.query.error : null,
    flow: typeof route.query.flow === "string" ? route.query.flow : null,
    newAccount: route.query.new_account === "true",
  };

  if (window.parent && window.parent !== window) {
    window.parent.postMessage(payload, window.location.origin);
    return;
  }

  const redirect =
    typeof route.query.redirect === "string" && route.query.redirect
      ? route.query.redirect
      : "/dashboard";

  if (payload.code && payload.newAccount) {
    await navigateTo(
      `/auth/welcome?authToken=${encodeURIComponent(payload.code)}&redirect=${encodeURIComponent(
        redirect,
      )}`,
      { replace: true },
    );
  } else if (payload.code) {
    await navigateTo(
      `/auth/sign-in?code=${encodeURIComponent(payload.code)}&redirect=${encodeURIComponent(
        redirect,
      )}`,
      { replace: true },
    );
  } else if (payload.error) {
    const params = new URLSearchParams({
      error: payload.error,
      redirect,
    });

    if (payload.flow) {
      params.set("flow", payload.flow);
    }

    await navigateTo(`/auth/sign-in?${params.toString()}`, { replace: true });
  }
});
</script>

<style scoped>
.oauth-popup-status {
  min-height: 12rem;
  display: grid;
  place-items: center;
  color: var(--color-secondary);
  text-align: center;
}
</style>
