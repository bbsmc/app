export const useUser = async (force = false) => {
  const nuxtApp = useNuxtApp();
  const user = useState("user", () => {});

  if (!user.value || force || (user.value && Date.now() - user.value.lastUpdated > 300000)) {
    user.value = await nuxtApp.runWithContext(() => initUser());
  }

  return user;
};

export const initUser = async () => {
  const config = useRuntimeConfig();
  const auth = useState("auth");

  const user = {
    collections: [],
    follows: [],
    subscriptions: [],
    lastUpdated: 0,
  };

  if (auth.value?.user && auth.value?.user.id) {
    try {
      const headers = {
        Authorization: auth.value.token,
      };

      const userId = auth.value.user.id;
      const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;

      const [follows, collections] = await Promise.all([
        $fetch(`${base}user/${userId}/follows`, { headers }),
        $fetch(`${base.replace(/\/v\d\/?$/, "/v3/")}user/${userId}/collections`, { headers }),
      ]);

      user.collections = collections;
      user.follows = follows;
      user.lastUpdated = Date.now();
    } catch (err) {
      console.error(err);
    }
  }

  return user;
};

export const initUserCollections = async () => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const userState = useState("user");

  if (auth.value?.user && auth.value?.user.id) {
    try {
      let base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
      base = base.replace(/\/v\d\/?$/, "/v3/");
      userState.value.collections = await $fetch(`${base}user/${auth.value.user.id}/collections`, {
        headers: { Authorization: auth.value.token },
      });
    } catch (err) {
      console.error(err);
    }
  }
};

export const initUserFollows = async () => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const userState = useState("user");

  if (auth.value?.user && auth.value?.user.id) {
    try {
      const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
      userState.value.follows = await $fetch(`${base}user/${auth.value.user.id}/follows`, {
        headers: { Authorization: auth.value.token },
      });
    } catch (err) {
      console.error(err);
    }
  }
};

export const initUserProjects = async () => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const userState = useState("user");

  if (auth.value?.user && auth.value?.user.id) {
    try {
      const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
      userState.value.projects = await $fetch(`${base}user/${auth.value.user.id}/projects`, {
        headers: { Authorization: auth.value.token },
      });
    } catch (err) {
      console.error(err);
    }
  }
};

export const userCollectProject = async (collection, projectId) => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const userState = useState("user");

  await initUserCollections();

  const collectionId = collection.id;
  const user = userState.value;

  const latestCollection = user.collections.find((x) => x.id === collectionId);
  if (!latestCollection) {
    throw new Error("未找到此收藏。它是否已被删除？");
  }

  const add = !latestCollection.projects.includes(projectId);
  const projects = add
    ? [...latestCollection.projects, projectId]
    : [...latestCollection.projects].filter((x) => x !== projectId);

  const idx = user.collections.findIndex((x) => x.id === latestCollection.id);
  if (idx >= 0) {
    user.collections[idx].projects = projects;
  }

  let base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
  base = base.replace(/\/v\d\/?$/, "/v3/");
  await $fetch(`${base}collection/${collection.id}`, {
    method: "PATCH",
    body: {
      new_projects: projects,
    },
    headers: { Authorization: auth.value.token },
  });
};

export const userFollowProject = (project) => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const userState = useState("user");
  const user = userState.value;

  const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;

  if (user.follows.find((x) => x.id === project.id)) {
    user.follows = user.follows.filter((x) => x.id !== project.id);
    project.followers--;

    setTimeout(() => {
      $fetch(`${base}project/${project.id}/follow`, {
        method: "DELETE",
        headers: { Authorization: auth.value.token },
      });
    });
  } else {
    user.follows = user.follows.concat(project);
    project.followers++;

    setTimeout(() => {
      $fetch(`${base}project/${project.id}/follow`, {
        method: "POST",
        headers: { Authorization: auth.value.token },
      });
    });
  }
};

export const resendVerifyEmail = async () => {
  const app = useNuxtApp();
  const config = useRuntimeConfig();
  const auth = useState("auth");

  startLoading();
  try {
    const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
    await $fetch(`${base}auth/email/resend_verify`, {
      method: "POST",
      headers: { Authorization: auth.value.token },
    });

    app.$notify({
      group: "main",
      title: "邮件已发送",
      text: `已向 ${auth.value.user.email} 发送包含验证链接的邮件。`,
      type: "success",
    });
  } catch (err) {
    app.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
  stopLoading();
};

export const logout = async () => {
  const config = useRuntimeConfig();
  const auth = useState("auth");
  const authCookie = useCookie("auth-token");

  startLoading();
  try {
    const base = import.meta.server ? config.apiBaseUrl : config.public.apiBaseUrl;
    await $fetch(`${base}session/${auth.value.token}`, {
      method: "DELETE",
      headers: { Authorization: auth.value.token },
    });
  } catch {
    /* empty */
  }

  auth.value = { user: null, token: "", headers: {} };
  authCookie.value = null;
  stopLoading();
  await navigateTo("/");
};
