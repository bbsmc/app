<template>
  <section class="universal-card">
    <h2>审核资源</h2>
    <div class="input-group">
      <Chips
        v-model="projectType"
        :items="projectTypes"
        :format-label="
          (x) => {
            switch (x) {
              case 'all': {
                return '全部';
              }
              case 'mod': {
                return '模组';
              }
              case 'shaders': {
                return '光影';
              }
              case 'resourcepacks': {
                return '资源包';
              }
              case 'plugin': {
                return '插件';
              }
              case 'datapack': {
                return '数据包';
              }
              case 'modpack': {
                return '整合包';
              }
              case 'status_change': {
                return '状态变更';
              }
              case 'moderator_message': {
                return '社区管理员消息';
              }
              default: {
                return x;
              }
            }
          }
        "
      />
      <button v-if="oldestFirst" class="iconified-button push-right" @click="oldestFirst = false">
        <SortDescIcon />最早提交
      </button>
      <button v-else class="iconified-button push-right" @click="oldestFirst = true">
        <SortAscIcon />最新提交
      </button>
      <button
        class="btn btn-highlight"
        :disabled="projectsFiltered.length === 0"
        @click="goToProjects()"
      >
        <ModerationIcon /> 开始审核
      </button>
    </div>
    <p v-if="projectType !== 'all'" class="project-count">
      一共 {{ projects.length }} 待审核资源，{{ projectTypePlural }} 有{{ projectsFiltered.length }}
      个
    </p>
    <p v-else class="project-count">一共有 {{ projects.length }} 个资源未审核</p>
    <p v-if="projectsOver24Hours.length > 0" class="warning project-count">
      <WarningIcon /> {{ projectsOver24Hours.length }} 个 {{ projectTypePlural }} 已经超过 24
      小时未审核
    </p>
    <p v-if="projectsOver48Hours.length > 0" class="danger project-count">
      <WarningIcon /> {{ projectsOver48Hours.length }} {{ projectTypePlural }} 已经超过 48
      小时未审核
    </p>
    <div
      v-for="project in projectsFiltered.sort((a, b) => {
        if (oldestFirst) {
          return b.age - a.age;
        } else {
          return a.age - b.age;
        }
      })"
      :key="`project-${project.id}`"
      class="universal-card recessed project"
    >
      <div class="project-title">
        <div class="mobile-row">
          <nuxt-link
            :to="`/${project.inferred_project_type}/${project.slug}`"
            class="iconified-stacked-link"
          >
            <Avatar :src="project.icon_url" size="xs" no-shadow raised />
            <span class="stacked">
              <span class="title">{{ project.name }}</span>
              <!-- <span>{{ $formatProjectType(project.inferred_project_type) }}</span> -->
            </span>
          </nuxt-link>
        </div>
        <div class="mobile-row">
          by
          <nuxt-link
            v-if="project.owner"
            :to="`/user/${project.owner.user.username}`"
            class="iconified-link"
          >
            <Avatar :src="project.owner.user.avatar_url" circle size="xxs" raised />
            <span>{{ project.owner.user.username }}</span>
          </nuxt-link>
          <nuxt-link
            v-else-if="project.org"
            :to="`/organization/${project.org.slug}`"
            class="iconified-link"
          >
            <Avatar :src="project.org.icon_url" circle size="xxs" raised />
            <span>{{ project.org.name }}</span>
          </nuxt-link>
        </div>
        <div class="mobile-row">
          请求资源发布为
          <Badge :type="project.requested_status ? project.requested_status : 'approved'" />
        </div>
      </div>
      <div class="input-group">
        <nuxt-link
          :to="`/${project.inferred_project_type}/${project.slug}`"
          target="_blank"
          class="iconified-button raised-button"
        >
          <EyeIcon /> 查看该资源
        </nuxt-link>
      </div>
      <span v-if="project.queued" :class="`submitter-info ${project.age_warning}`">
        <WarningIcon v-if="project.age_warning" />
        提交于
        <span v-tooltip="formatDateTime(project.queued)">{{ fromNow(project.queued) }}</span>
      </span>
      <span v-else class="submitter-info"> <UnknownIcon /> Unknown queue date </span>
    </div>
  </section>
</template>
<script setup>
import { formatProjectType, formatDateTime } from "@modrinth/utils";
import Chips from "~/components/ui/Chips.vue";
import Avatar from "~/components/ui/Avatar.vue";
import UnknownIcon from "~/assets/images/utils/unknown.svg?component";
import EyeIcon from "~/assets/images/utils/eye.svg?component";
import SortAscIcon from "~/assets/images/utils/sort-asc.svg?component";
import SortDescIcon from "~/assets/images/utils/sort-desc.svg?component";
import WarningIcon from "~/assets/images/utils/issues.svg?component";
import ModerationIcon from "~/assets/images/sidebar/admin.svg?component";
import Badge from "~/components/ui/Badge.vue";

useHead({
  title: "审核资源 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const app = useNuxtApp();

const router = useRouter();

const now = app.$dayjs();
const TIME_24H = 86400000;
const TIME_48H = TIME_24H * 2;

const { data: projects } = await useAsyncData("moderation/projects?count=1000", () =>
  useBaseFetch("moderation/projects?count=1000", { internal: true }),
);
const members = ref([]);
const projectType = ref("all");
const oldestFirst = ref(true);

const projectsFiltered = computed(() =>
  (projects.value ?? []).filter(
    (x) =>
      projectType.value === "all" ||
      app.$getProjectTypeForUrl(x.project_types[0], x.loaders) === projectType.value,
  ),
);

const projectsOver24Hours = computed(() =>
  projectsFiltered.value.filter((project) => project.age >= TIME_24H && project.age < TIME_48H),
);
const projectsOver48Hours = computed(() =>
  projectsFiltered.value.filter((project) => project.age >= TIME_48H),
);
const projectTypePlural = computed(() =>
  projectType.value === "all" ? "资源" : formatProjectType(projectType.value).toLowerCase(),
);

const projectTypes = computed(() => {
  const set = new Set();
  set.add("all");

  if (projects.value) {
    for (const project of projects.value) {
      set.add(project.inferred_project_type);
    }
  }

  return [...set];
});

if (projects.value) {
  const teamIds = projects.value.map((x) => x.team_id);
  const organizationIds = projects.value.filter((x) => x.organization).map((x) => x.organization);

  const url = `teams?ids=${encodeURIComponent(JSON.stringify(teamIds))}`;
  const orgUrl = `organizations?ids=${encodeURIComponent(JSON.stringify(organizationIds))}`;
  const { data: result } = await useAsyncData(url, () => useBaseFetch(url));
  const { data: orgs } = await useAsyncData(orgUrl, () => useBaseFetch(orgUrl, { apiVersion: 3 }));

  if (result.value) {
    members.value = result.value;

    projects.value = projects.value.map((project) => {
      project.owner = members.value
        .flat()
        .find((x) => x.team_id === project.team_id && x.role === "Owner");
      project.org = orgs.value.find((x) => x.id === project.organization);
      project.age = project.queued ? now - app.$dayjs(project.queued) : Number.MAX_VALUE;
      project.age_warning = "";
      if (project.age > TIME_24H * 2) {
        project.age_warning = "danger";
      } else if (project.age > TIME_24H) {
        project.age_warning = "warning";
      }
      project.inferred_project_type = app.$getProjectTypeForUrl(
        project.project_types[0],
        project.loaders,
      );
      return project;
    });
  }
}
async function goToProjects() {
  const project = projectsFiltered.value[0];
  await router.push({
    name: "type-id",
    params: {
      type: project.project_types[0],
      id: project.slug ? project.slug : project.id,
    },
    state: {
      showChecklist: true,
      projects: projectsFiltered.value.slice(1).map((x) => (x.slug ? x.slug : x.id)),
    },
  });
}
</script>
<style lang="scss" scoped>
.project {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-card-sm);

  @media screen and (min-width: 650px) {
    display: grid;
    grid-template: "title action" "date action";
    grid-template-columns: 1fr auto;
  }
}

.submitter-info {
  margin: 0;
  grid-area: date;

  svg {
    vertical-align: top;
  }
}

.warning {
  color: var(--color-orange);
}

.danger {
  color: var(--color-red);
  font-weight: bold;
}

.project-count {
  margin-block: var(--spacing-card-md);

  svg {
    vertical-align: top;
  }
}

.input-group {
  grid-area: action;
}

.project-title {
  display: flex;
  gap: var(--spacing-card-xs);
  align-items: center;
  flex-wrap: wrap;

  .mobile-row {
    display: contents;
  }

  @media screen and (max-width: 800px) {
    flex-direction: column;
    align-items: flex-start;

    .mobile-row {
      display: flex;
      flex-direction: row;
      gap: var(--spacing-card-xs);
      align-items: center;
      flex-wrap: wrap;
    }
  }
}

:deep(.avatar) {
  flex-shrink: 0;

  &.size-xs {
    margin-right: var(--spacing-card-xs);
  }
}
</style>
