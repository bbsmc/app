export const useUser = async (force = false) => {
  const user = useState("user", () => {});

  if (!user.value || force || (user.value && Date.now() - user.value.lastUpdated > 300000)) {
    user.value = await initUser();
  }

  return user;
};

export const initUser = async () => {
  const auth = (await useAuth()).value;

  const user = {
    collections: [],
    follows: [],
    subscriptions: [],
    lastUpdated: 0,
  };

  if (auth.user && auth.user.id) {
    try {
      const headers = {
        Authorization: auth.token,
      };

      const [follows, collections, subscriptions] = await Promise.all([
        useBaseFetch(`user/${auth.user.id}/follows`, { headers }, true),
        useBaseFetch(`user/${auth.user.id}/collections`, { apiVersion: 3, headers }, true),
        // useBaseFetch(`billing/subscriptions`, { internal: true, headers }, true),
      ]);

      user.collections = collections;
      user.follows = follows;
      user.subscriptions = subscriptions;
      user.lastUpdated = Date.now();
    } catch (err) {
      console.error(err);
    }
  }

  return user;
};

export const initUserCollections = async () => {
  const auth = (await useAuth()).value;
  const user = (await useUser()).value;

  if (auth.user && auth.user.id) {
    try {
      user.collections = await useBaseFetch(`user/${auth.user.id}/collections`, { apiVersion: 3 });
    } catch (err) {
      console.error(err);
    }
  }
};

export const initUserFollows = async () => {
  const auth = (await useAuth()).value;
  const user = (await useUser()).value;

  if (auth.user && auth.user.id) {
    try {
      user.follows = await useBaseFetch(`user/${auth.user.id}/follows`);
    } catch (err) {
      console.error(err);
    }
  }
};

export const initUserProjects = async () => {
  const auth = (await useAuth()).value;
  const user = (await useUser()).value;

  if (auth.user && auth.user.id) {
    try {
      user.projects = await useBaseFetch(`user/${auth.user.id}/projects`);
    } catch (err) {
      console.error(err);
    }
  }
};

export const userCollectProject = async (collection, projectId) => {
  const user = (await useUser()).value;
  await initUserCollections();

  const collectionId = collection.id;

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

  await useBaseFetch(`collection/${collection.id}`, {
    method: "PATCH",
    body: {
      new_projects: projects,
    },
    apiVersion: 3,
  });
};

export const userFollowProject = async (project) => {
  const user = (await useUser()).value;

  if (user.follows.find((x) => x.id === project.id)) {
    user.follows = user.follows.filter((x) => x.id !== project.id);
    project.followers--;

    setTimeout(() => {
      useBaseFetch(`project/${project.id}/follow`, {
        method: "DELETE",
      });
    });
  } else {
    user.follows = user.follows.concat(project);
    project.followers++;

    setTimeout(() => {
      useBaseFetch(`project/${project.id}/follow`, {
        method: "POST",
      });
    });
  }
};
export const resendVerifyEmail = async () => {
  const app = useNuxtApp();

  startLoading();
  try {
    await useBaseFetch("auth/email/resend_verify", {
      method: "POST",
    });

    const auth = await useAuth();
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
  startLoading();
  const auth = await useAuth();
  try {
    await useBaseFetch(`session/${auth.value.token}`, {
      method: "DELETE",
    });
  } catch {
    /* empty */
  }

  await useAuth("none");
  useCookie("auth-token").value = null;
  stopLoading();
};
