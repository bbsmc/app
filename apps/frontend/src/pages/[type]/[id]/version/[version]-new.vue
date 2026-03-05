<template>
  <div class="normal-page__content flex flex-col gap-4">
    <nuxt-link
      :to="versionsListLink"
      class="flex w-fit items-center gap-1 text-brand-blue hover:underline"
    >
      <ChevronLeftIcon />
      {{
        hasBackLink ? formatMessage(messages.backToVersions) : formatMessage(messages.allVersions)
      }}
    </nuxt-link>
    <div class="flex gap-3">
      <VersionChannelIndicator :channel="version.version_type" large />
      <div class="flex flex-col gap-1">
        <h1 class="m-0 text-xl font-extrabold leading-none text-contrast">
          {{ version.version_number }}
        </h1>
        <span class="text-sm font-semibold text-secondary"> {{ version.name }} </span>
      </div>
    </div>
    <div class="flex gap-2">
      <ButtonStyled color="brand">
        <button><DownloadIcon /> 下载</button>
      </ButtonStyled>
      <ButtonStyled>
        <button><ShareIcon /> 分享</button>
      </ButtonStyled>
      <ButtonStyled circular type="transparent">
        <button>
          <MoreVerticalIcon />
        </button>
      </ButtonStyled>
    </div>
    <div>
      <h2 class="text-lg font-extrabold text-contrast">文件</h2>
      <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div
          v-for="(file, index) in version.files"
          :key="index"
          class="flex gap-2 rounded-2xl bg-bg-raised p-4"
        >
          <div
            :class="`flex h-9 w-9 items-center justify-center rounded-full ${file.primary ? 'bg-brand-highlight text-brand' : 'bg-button-bg text-secondary'}`"
          >
            <FileIcon />
          </div>
          <div class="flex flex-grow flex-col">
            <span class="font-extrabold text-contrast">{{
              file.primary ? "主文件" : "附属资源"
            }}</span>
            <span class="text-sm font-semibold text-secondary"
              >{{ file.filename }} • {{ formatBytes(file.size) }}</span
            >
          </div>
          <div>
            <ButtonStyled circular type="transparent">
              <button>
                <DownloadIcon />
              </button>
            </ButtonStyled>
          </div>
        </div>
      </div>
      <h2 class="text-lg font-extrabold text-contrast">依赖</h2>
      <h2 class="text-lg font-extrabold text-contrast">更新日志</h2>
      <div class="rounded-2xl bg-bg-raised px-6 py-4">
        <div
          class="markdown-body"
          v-html="renderHighlightedString(version.changelog ?? '暂无更新日志')"
        />
      </div>
    </div>
  </div>
  <div class="normal-page__sidebar">
    <div class="padding-lg h-[250px] rounded-2xl bg-bg-raised"></div>
  </div>
</template>
<script setup lang="ts">
import {
  ChevronLeftIcon,
  DownloadIcon,
  FileIcon,
  MoreVerticalIcon,
  ShareIcon,
} from "@modrinth/assets";
import { ButtonStyled, VersionChannelIndicator } from "@modrinth/ui";
import { formatBytes, renderHighlightedString } from "@modrinth/utils";

const router = useRouter();

const props = defineProps<{
  project: Project;
  versions: Version[];
  featuredVersions: Version[];
  members: User[];
  currentMember: User;
  dependencies: Dependency[];
  resetProject: (opts?: { dedupe?: "cancel" | "defer" }) => Promise<void>;
}>();

const version = computed(() => {
  let version: Version | undefined;

  if (route.params.version === "latest") {
    let versionList = props.versions;
    if (route.query.loader) {
      versionList = versionList.filter((x) => x.loaders.includes(route.query.loader));
    }
    if (route.query.version) {
      versionList = versionList.filter((x) => x.game_versions.includes(route.query.version));
    }
    version = versionList.reduce((a, b) => (a.date_published > b.date_published ? a : b));
  } else {
    version = props.versions.find(
      (x) => x.id === route.params.version || x.displayUrlEnding === route.params.version,
    );
  }

  if (!version) {
    throw createError({
      fatal: true,
      statusCode: 404,
      message: "未找到该版本",
    });
  }

  return version;
});

// const data = useNuxtApp();
const route = useNativeRoute();

// const auth = await useAuth();
// const tags = useTags();

const versionsListLink = computed(() => {
  if (router.options.history.state.back) {
    if (router.options.history.state.back.includes("/versions")) {
      return router.options.history.state.back;
    }
  }
  return `/${props.project.project_type}/${
    props.project.slug ? props.project.slug : props.project.id
  }/versions`;
});

const hasBackLink = computed(
  () =>
    router.options.history.state.back && router.options.history.state.back.endsWith("/versions"),
);

const { formatMessage } = useVIntl();
const messages = defineMessages({
  backToVersions: {
    id: "project.version.back-to-versions",
    defaultMessage: "返回版本列表",
  },
  allVersions: {
    id: "project.version.all-versions",
    defaultMessage: "所有版本",
  },
});
</script>
<style lang="scss"></style>
