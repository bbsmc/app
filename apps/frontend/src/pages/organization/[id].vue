<template>
  <div
    v-if="organization"
    class="experimental-styles-within new-page sidebar"
    :class="{ 'alt-layout': cosmetics.leftContentLayout || routeHasSettings }"
  >
    <ModalCreation ref="" :organization-id="organization.id" />
    <template v-if="routeHasSettings">
      <div class="normal-page__sidebar">
        <div class="universal-card">
          <Breadcrumbs
            current-title="设置"
            :link-stack="[
              { href: `/dashboard/organizations`, label: '团队' },
              {
                href: `/organization/${organization.slug}`,
                label: organization.name,
                allowTrimming: true,
              },
            ]"
          />
          <div class="page-header__settings">
            <Avatar size="sm" :src="organization.icon_url" />
            <div class="title-section">
              <h2 class="settings-title">
                <nuxt-link :to="`/organization/${organization.slug}/settings`">
                  {{ organization.name }}
                </nuxt-link>
              </h2>
              <span>
                {{ $formatNumber(acceptedMembers?.length || 0) }}
                成员<template v-if="acceptedMembers?.length !== 1"></template>
              </span>
            </div>
          </div>

          <h2>团队设置</h2>

          <NavStack>
            <NavStackItem :link="`/organization/${organization.slug}/settings`" label="主要">
              <SettingsIcon />
            </NavStackItem>
            <NavStackItem
              :link="`/organization/${organization.slug}/settings/members`"
              label="成员"
            >
              <UsersIcon />
            </NavStackItem>
            <NavStackItem
              :link="`/organization/${organization.slug}/settings/projects`"
              label="资源"
            >
              <BoxIcon />
            </NavStackItem>
            <NavStackItem
              :link="`/organization/${organization.slug}/settings/analytics`"
              label="分析"
            >
              <ChartIcon />
            </NavStackItem>
          </NavStack>
        </div>
      </div>
      <div class="normal-page__content">
        <NuxtPage />
      </div>
    </template>
    <template v-else>
      <div class="normal-page__header py-4">
        <ContentPageHeader>
          <template #icon>
            <Avatar :src="organization.icon_url" :alt="organization.name" size="96px" />
          </template>
          <template #title>
            {{ organization.name }}
          </template>
          <template #title-suffix>
            <div class="ml-1 flex items-center gap-2 font-semibold"><OrganizationIcon /> 团队</div>
          </template>
          <template #summary>
            {{ organization.description }}
          </template>
          <template #stats>
            <div
              class="flex items-center gap-2 border-0 border-r border-solid border-button-bg pr-4 font-semibold"
            >
              <UsersIcon class="h-6 w-6 text-secondary" />
              {{ formatCompactNumber(acceptedMembers?.length || 0) }}
              成员
            </div>
            <div
              class="flex items-center gap-2 border-0 border-r border-solid border-button-bg pr-4 font-semibold"
            >
              <BoxIcon class="h-6 w-6 text-secondary" />
              {{ formatCompactNumber(projectTotalHits) }}
              资源
            </div>
            <div class="flex items-center gap-2 font-semibold">
              <DownloadIcon class="h-6 w-6 text-secondary" />
              {{ formatCompactNumber(sumDownloads) }}
              次下载
            </div>
          </template>
          <template #actions>
            <ButtonStyled v-if="auth.user && currentMember" size="large">
              <NuxtLink :to="`/organization/${organization.slug}/settings`">
                <SettingsIcon aria-hidden="true" />
                管理
              </NuxtLink>
            </ButtonStyled>
            <ButtonStyled size="large" circular type="transparent">
              <OverflowMenu
                :options="[
                  {
                    id: 'manage-projects',
                    action: () =>
                      navigateTo('/organization/' + organization.slug + '/settings/projects'),
                    hoverOnly: true,
                    shown: auth.user && currentMember,
                  },
                  { divider: true, shown: auth.user && currentMember },
                  { id: 'copy-id', action: () => copyId() },
                ]"
                aria-label="More options"
              >
                <MoreVerticalIcon aria-hidden="true" />
                <template #manage-projects>
                  <BoxIcon aria-hidden="true" />
                  管理资源
                </template>
                <template #copy-id>
                  <ClipboardCopyIcon aria-hidden="true" />
                  {{ formatMessage(commonMessages.copyIdButton) }}
                </template>
              </OverflowMenu>
            </ButtonStyled>
          </template>
        </ContentPageHeader>
      </div>
      <div class="normal-page__sidebar">
        <div class="card flex-card">
          <h2>成员</h2>
          <div class="details-list">
            <template v-for="member in acceptedMembers" :key="member.user.id">
              <nuxt-link
                class="details-list__item details-list__item--type-large"
                :to="`/user/${member.user.username}`"
              >
                <Avatar :src="member.user.avatar_url" circle />
                <div class="rows">
                  <span class="flex items-center gap-1">
                    {{ member.user.username }}
                    <CrownIcon
                      v-if="member.is_owner"
                      v-tooltip="'Organization owner'"
                      class="text-brand-orange"
                    />
                  </span>
                  <span class="details-list__item__text--style-secondary">
                    {{ member.role ? member.role : "成员" }}
                  </span>
                </div>
              </nuxt-link>
            </template>
          </div>
        </div>
      </div>
      <div class="normal-page__content">
        <div v-if="isInvited" class="universal-card information invited">
          <h2>邀请你加入 {{ organization.name }}</h2>
          <p>您被邀请加入 {{ organization.name }}.</p>
          <div class="input-group">
            <button class="iconified-button brand-button" @click="onAcceptInvite">
              <CheckIcon />接受
            </button>
            <button class="iconified-button danger-button" @click="onDeclineInvite">
              <XIcon />拒绝
            </button>
          </div>
        </div>
        <div v-if="navLinks.length > 1" class="mb-4 max-w-full overflow-x-auto">
          <NavTabs :links="navLinks" />
        </div>
        <template v-if="projectTotalHits > 0">
          <div class="project-list display-mode--list">
            <ProjectCard
              v-for="project in (selectedProjectType
                ? projects.filter((x) => x.project_types.includes(selectedProjectType))
                : projects
              )
                .slice()
                .sort((a, b) => b.downloads - a.downloads)"
              :id="project.slug || project.id"
              :key="project.id"
              :name="project.name"
              :display="cosmetics.searchDisplayMode.user"
              :featured-image="project.gallery.find((element) => element.featured)?.url"
              project-type-url="project"
              :description="project.summary"
              :created-at="project.published"
              :updated-at="project.updated"
              :downloads="project.downloads.toString()"
              :follows="project.followers.toString()"
              :icon-url="project.icon_url"
              :categories="project.categories"
              :client-side="project.client_side"
              :server-side="project.server_side"
              :status="
                auth.user && (auth.user.id === user.id || tags.staffRoles.includes(auth.user.role))
                  ? project.status
                  : null
              "
              :type="project.project_types[0] ?? 'project'"
              :color="project.color"
            />
          </div>
          <Pagination
            :page="projectCurrentPage"
            :count="projectPageCount"
            :link-function="projectPageLink"
            class="mt-4 justify-end"
            @switch-page="changeProjectPage"
          />
        </template>

        <div v-else-if="true" class="error">
          <UpToDate class="icon" /><br />
          <span class="preserve-lines text">
            该团队尚未发布项目。
            <template v-if="isPermission(currentMember?.organization_permissions, 1 << 4)">
              你想要
              <a class="link" @click="$refs.modal_creation.show()">发布一个资源吗</a>?
            </template>
          </span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import {
  BoxIcon,
  MoreVerticalIcon,
  UsersIcon,
  SettingsIcon,
  ChartIcon,
  CheckIcon,
  XIcon,
  ClipboardCopyIcon,
} from "@modrinth/assets";
import { Avatar, ButtonStyled, Breadcrumbs, ContentPageHeader, OverflowMenu } from "@modrinth/ui";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";
import ModalCreation from "~/components/ui/ModalCreation.vue";
import UpToDate from "~/assets/images/illustrations/up_to_date.svg?component";
import ProjectCard from "~/components/ui/ProjectCard.vue";

import OrganizationIcon from "~/assets/images/utils/organization.svg?component";
import DownloadIcon from "~/assets/images/utils/download.svg?component";
import CrownIcon from "~/assets/images/utils/crown.svg?component";
import { acceptTeamInvite, removeTeamMember } from "~/helpers/teams.js";
import NavTabs from "~/components/ui/NavTabs.vue";
import Pagination from "~/components/ui/Pagination.vue";

const vintl = useVIntl();
const { formatMessage } = vintl;

const formatCompactNumber = useCompactNumber();

const nuxtApp = useNuxtApp();
const auth = await useAuth();
const user = await useUser();
const cosmetics = useCosmetics();
const route = useNativeRoute();
const router = useNativeRouter();
const tags = useTags();
const PROJECT_PAGE_SIZE = 10;

const projectPage = computed(() => {
  const raw = Array.isArray(route.query.page) ? route.query.page[0] : route.query.page;
  const parsed = Number.parseInt(raw || "1", 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 1;
});

const selectedProjectType = computed(() => {
  const raw = Array.isArray(route.params.projectType)
    ? route.params.projectType[0]
    : route.params.projectType;
  if (!raw) return undefined;
  return raw.endsWith("s") ? raw.slice(0, -1) : raw;
});

let orgId = useRouteId();
const projectPagination = useState(`organization/${orgId}/projects-pagination`, () => ({
  offset: 0,
  limit: PROJECT_PAGE_SIZE,
  total_hits: 0,
  total_downloads: 0,
}));
const projectTypeOptions = useState(`organization/${orgId}/project-types`, () => []);
const projectDataKey = computed(
  () => `organization/${orgId}/projects:${selectedProjectType.value ?? "all"}:${projectPage.value}`,
);

const fetchWithAuth = (url, options = {}) => {
  const headers = { ...(options.headers ?? {}) };
  if (auth.value?.token) {
    headers.Authorization = auth.value.token;
  }

  return nuxtApp.runWithContext(() => useBaseFetch(url, { ...options, headers }, true));
};

// hacky way to show the edit button on the corner of the card.
const routeHasSettings = computed(() => route.path.includes("settings"));

const [
  { data: organization, refresh: refreshOrganization },
  { data: projects, refresh: refreshProjects },
] = await Promise.all([
  useAsyncData(`organization/${orgId}`, () =>
    fetchWithAuth(`organization/${orgId}`, { apiVersion: 3 }),
  ),
  useAsyncData(
    projectDataKey.value,
    async () => {
      const query = {
        page: projectPage.value,
        limit: PROJECT_PAGE_SIZE,
      };
      if (selectedProjectType.value) {
        query.project_type = selectedProjectType.value;
      }

      const response = await fetchWithAuth(`organization/${orgId}/projects`, {
        apiVersion: 3,
        query,
      });

      if (
        !Array.isArray(response) &&
        projectPage.value > 1 &&
        (response?.hits?.length ?? 0) === 0 &&
        (response?.total_hits ?? 0) > 0
      ) {
        return fetchWithAuth(`organization/${orgId}/projects`, {
          apiVersion: 3,
          query: {
            ...query,
            page: 1,
          },
        });
      }

      return response;
    },
    {
      watch: [projectPage, selectedProjectType],
      transform: (response) => {
        const isLegacyResponse = Array.isArray(response);
        const projects = isLegacyResponse ? response : (response?.hits ?? []);

        for (const project of projects) {
          project.categories = project.categories.concat(project.loaders);

          if (project.mrpack_loaders) {
            project.categories = project.categories.concat(project.mrpack_loaders);
          }

          const singleplayer = project.singleplayer && project.singleplayer[0];
          const clientAndServer = project.client_and_server && project.client_and_server[0];
          const clientOnly = project.client_only && project.client_only[0];
          const serverOnly = project.server_only && project.server_only[0];

          // quick and dirty hack to show envs as legacy
          if (singleplayer && clientAndServer && !clientOnly && !serverOnly) {
            project.client_side = "required";
            project.server_side = "required";
          } else if (singleplayer && clientAndServer && clientOnly && !serverOnly) {
            project.client_side = "required";
            project.server_side = "unsupported";
          } else if (singleplayer && clientAndServer && !clientOnly && serverOnly) {
            project.client_side = "unsupported";
            project.server_side = "required";
          } else if (singleplayer && clientAndServer && clientOnly && serverOnly) {
            project.client_side = "optional";
            project.server_side = "optional";
          }
        }

        if (isLegacyResponse) {
          const filteredProjects = selectedProjectType.value
            ? projects.filter((project) =>
                project.project_types.includes(selectedProjectType.value),
              )
            : projects;
          const offset = (projectPage.value - 1) * PROJECT_PAGE_SIZE;

          projectPagination.value = {
            offset,
            limit: PROJECT_PAGE_SIZE,
            total_hits: filteredProjects.length,
            total_downloads: filteredProjects.reduce(
              (sum, project) => sum + (project.downloads ?? 0),
              0,
            ),
          };
          projectTypeOptions.value = Array.from(
            new Set(projects.flatMap((project) => project.project_types ?? [])),
          );

          return filteredProjects
            .slice()
            .sort((a, b) => b.downloads - a.downloads)
            .slice(offset, offset + PROJECT_PAGE_SIZE);
        }

        projectPagination.value = {
          offset: response?.offset ?? 0,
          limit: response?.limit ?? PROJECT_PAGE_SIZE,
          total_hits: response?.total_hits ?? projects.length,
          total_downloads:
            response?.total_downloads ??
            projects.reduce((sum, project) => sum + (project.downloads ?? 0), 0),
        };
        projectTypeOptions.value = response?.project_types ?? [];

        return projects;
      },
    },
  ),
]);

const refresh = async () => {
  await Promise.all([refreshOrganization(), refreshProjects()]);
};

if (!organization.value) {
  throw createError({
    fatal: true,
    statusCode: 404,
    message: "Organization not found",
  });
}

// Filter accepted, sort by role, then by name and Owner role always goes first
const acceptedMembers = computed(() => {
  const acceptedMembers = organization.value.members?.filter((x) => x.accepted);
  const owner = acceptedMembers.find((x) => x.is_owner);
  const rest = acceptedMembers.filter((x) => !x.is_owner) || [];

  rest.sort((a, b) => {
    if (a.role === b.role) {
      return a.user.username.localeCompare(b.user.username);
    } else {
      return a.role.localeCompare(b.role);
    }
  });

  return [owner, ...rest];
});

const currentMember = computed(() => {
  if (auth.value.user && organization.value) {
    const member = organization.value.members.find((x) => x.user.id === auth.value.user.id);

    if (member) {
      return member;
    }

    if (tags.value.staffRoles.includes(auth.value.user.role)) {
      return {
        user: auth.value.user,
        role: auth.value.user.role,
        permissions: auth.value.user.role === "admin" ? 2047 : 12,
        accepted: true,
        payouts_split: 0,
        avatar_url: auth.value.user.avatar_url,
        name: auth.value.user.username,
      };
    }
  }

  return null;
});

const hasPermission = computed(() => {
  const EDIT_DETAILS = 1 << 2;
  return currentMember.value && (currentMember.value.permissions & EDIT_DETAILS) === EDIT_DETAILS;
});

const isInvited = computed(() => {
  return currentMember.value?.accepted === false;
});

const projectTypes = computed(() => {
  const obj = {};

  for (const projectType of projectTypeOptions.value) {
    obj[projectType] = true;
  }
  for (const project of projects.value ?? []) {
    for (const projectType of project.project_types ?? []) {
      obj[projectType] = true;
    }
  }

  delete obj.project;

  return Object.keys(obj);
});
const projectTotalHits = computed(() => projectPagination.value.total_hits);
const projectPageCount = computed(() =>
  Math.max(1, Math.ceil(projectTotalHits.value / PROJECT_PAGE_SIZE)),
);
const projectCurrentPage = computed(() =>
  Math.max(1, Math.floor((projectPagination.value.offset || 0) / PROJECT_PAGE_SIZE) + 1),
);
const sumDownloads = computed(() => {
  if (typeof projectPagination.value.total_downloads === "number") {
    return projectPagination.value.total_downloads;
  }

  let sum = 0;

  for (const project of projects.value ?? []) {
    sum += project.downloads;
  }

  return sum;
});

const patchIcon = async (icon) => {
  const ext = icon.name.split(".").pop();
  await useBaseFetch(`organization/${organization.value.id}/icon`, {
    method: "PATCH",
    body: icon,
    query: { ext },
    apiVersion: 3,
  });
};

const deleteIcon = async () => {
  await useBaseFetch(`organization/${organization.value.id}/icon`, {
    method: "DELETE",
    apiVersion: 3,
  });
};

const patchOrganization = async (id, newData) => {
  await useBaseFetch(`organization/${id}`, {
    method: "PATCH",
    body: newData,
    apiVersion: 3,
  });

  if (newData.slug) {
    orgId = newData.slug;
  }
};

const onAcceptInvite = useClientTry(async () => {
  await acceptTeamInvite(organization.value.team_id);
  await refreshOrganization();
});

const onDeclineInvite = useClientTry(async () => {
  await removeTeamMember(organization.value.team_id, auth.value?.user.id);
  await refreshOrganization();
});

function projectPageQuery(page) {
  const query = { ...route.query };
  if (page > 1) {
    query.page = String(page);
  } else {
    delete query.page;
  }
  return query;
}

function projectPageLink(page) {
  const query = projectPageQuery(page);
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item !== undefined && item !== null) params.append(key, String(item));
      }
    } else {
      params.set(key, String(value));
    }
  }
  const queryString = params.toString();
  return queryString ? `?${queryString}` : "?";
}

function changeProjectPage(page) {
  router.push({ query: projectPageQuery(page) });
}

provide("organizationContext", {
  organization,
  projects,
  projectPage,
  projectPageCount,
  projectPageLink,
  changeProjectPage,
  refresh,
  currentMember,
  hasPermission,
  patchIcon,
  deleteIcon,
  patchOrganization,
});

const title = `${organization.value.name} - BBSMC 组织 | 我的世界 Minecraft 创作者团队`;
const description = organization.value.description
  ? `${organization.value.description} - 在 BBSMC 浏览 ${organization.value.name} 组织发布的 Minecraft 模组、整合包和其他资源。`
  : `在 BBSMC 浏览 ${organization.value.name} 组织发布的 Minecraft 模组、整合包、光影和资源包，发现更多高质量创作者和资源。`;

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: organization.value.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
});

useHead({
  script: [
    {
      type: "application/ld+json",
      children: JSON.stringify({
        "@context": "https://schema.org",
        "@type": "Organization",
        name: organization.value.name,
        description: organization.value.description || undefined,
        image: organization.value.icon_url || undefined,
        url: `https://bbsmc.net/organization/${organization.value.slug || organization.value.id}`,
      }),
    },
  ],
});

const navLinks = computed(() => [
  {
    label: formatMessage(commonMessages.allProjectType),
    href: `/organization/${organization.value.slug}`,
  },
  ...projectTypes.value
    .map((x) => {
      return {
        label: formatMessage(getProjectTypeMessage(x, true)),
        href: `/organization/${organization.value.slug}/${x}s`,
      };
    })
    .slice()
    .sort((a, b) => a.label.localeCompare(b.label)),
]);

async function copyId() {
  await navigator.clipboard.writeText(organization.value.id);
}
</script>

<style scoped lang="scss">
.page-header__settings {
  display: flex;
  flex-direction: row;
  gap: var(--gap-md);
  margin-bottom: var(--gap-md);

  .title-section {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--gap-xs);
  }

  .settings-title {
    margin: 0 !important;
    font-size: var(--font-size-md);
  }
}

.page-header__icon {
  margin-block: 0 !important;
}

.universal-card {
  h1 {
    margin-bottom: var(--gap-md);
  }
}

.creator-list {
  display: flex;
  flex-direction: column;
  padding: var(--gap-xl);

  h3 {
    margin: 0 0 var(--gap-sm);
  }

  .creator {
    display: grid;
    gap: var(--gap-xs);
    background-color: var(--color-raised-bg);
    padding: var(--gap-sm);
    margin-left: -0.5rem;
    border-radius: var(--radius-lg);
    grid-template:
      "avatar name" auto
      "avatar role" auto
      / auto 1fr;
    p {
      margin: 0;
    }

    .name {
      grid-area: name;
      align-self: flex-end;
      margin-left: var(--gap-xs);
      font-weight: bold;

      display: flex;
      align-items: center;
      gap: 0.25rem;

      svg {
        color: var(--color-orange);
      }
    }

    .role {
      grid-area: role;
      align-self: flex-start;
      margin-left: var(--gap-xs);
    }

    .avatar {
      grid-area: avatar;
    }
  }
}

.secondary-stat {
  align-items: center;
  display: flex;
  margin-bottom: 0.8rem;
}

.secondary-stat__icon {
  height: 1rem;
  width: 1rem;
}

.secondary-stat__text {
  margin-left: 0.4rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.title {
  margin: var(--gap-md) 0 var(--spacing-card-xs) 0;
  font-size: var(--font-size-xl);
  color: var(--color-text-dark);
}

.organization-label {
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.organization-description {
  margin-top: var(--spacing-card-sm);
  margin-bottom: 0;
}

.title-and-link {
  display: flex;
  justify-content: space-between;
  align-items: center;

  h3 {
    margin: 0;
  }

  a {
    display: flex;
    align-items: center;
    gap: var(--gap-xs);
    color: var(--color-blue);
  }
}

.project-overview {
  gap: var(--gap-md);
  padding: var(--gap-xl);

  .project-card {
    padding: 0;
    border-radius: 0;
    background-color: transparent;
    box-shadow: none;

    :deep(.title) {
      font-size: var(--font-size-nm) !important;
    }
  }
}

.popout-heading {
  padding: var(--gap-sm) var(--gap-md);
  margin: 0;
  font-size: var(--font-size-md);
  color: var(--color-text);
}

.popout-checkbox {
  padding: var(--gap-sm) var(--gap-md);
}
</style>
