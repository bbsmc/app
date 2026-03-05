<template>
  <!--  // 站内（包括公共文件和私有文件）-->
  <div
    v-if="isInternalDownload"
    class="grid grid-cols-[min-content_auto_min-content_min-content] items-center gap-2 rounded-2xl border-[1px] border-button-bg bg-bg p-2"
  >
    <VersionChannelIndicator :channel="version.version_type" />
    <div class="flex min-w-0 flex-col gap-1">
      <span class="my-0 truncate text-nowrap text-base font-extrabold leading-none text-contrast">
        [{{ isPaidDownload ? "付费下载" : "站内下载" }}] {{ version.name }}
      </span>
      <p class="m-0 truncate text-nowrap text-xs font-semibold text-secondary">
        {{ version.version_number }}
      </p>
    </div>
    <ButtonStyled color="brand">
      <a
        :href="downloadHref"
        :class="['min-w-0', { 'cursor-wait': isDownloading }]"
        :target="isPaidDownload ? undefined : '_blank'"
        @click="handleDownload"
      >
        <span v-if="isDownloading" class="animate-spin">...</span>
        <DownloadIcon v-else aria-hidden="true" />
      </a>
    </ButtonStyled>
    <ButtonStyled circular>
      <nuxt-link
        :to="`/project/${props.version.project_id}/version/${props.version.id}`"
        class="min-w-0"
        aria-label="Open project page"
        @click="emit('onNavigate')"
      >
        <ExternalIcon aria-hidden="true" />
      </nuxt-link>
    </ButtonStyled>
  </div>

  <!--  网盘 -->

  <div>
    <div
      v-for="(u, index) in props.version.disk_urls"
      :key="index"
      class="grid grid-cols-[min-content_auto_min-content_min-content] items-center gap-2 rounded-2xl border-[1px] border-button-bg bg-bg p-2"
    >
      <VersionChannelIndicator :channel="version.version_type" />
      <div class="flex min-w-0 flex-col gap-1">
        <span class="my-0 truncate text-nowrap text-base font-extrabold leading-none text-contrast">
          [{{
            u.platform === "quark"
              ? "夸克云盘"
              : u.platform === "baidu"
                ? "百度云盘"
                : u.platform === "curseforge"
                  ? "CurseForge"
                  : u.platform === "modrinth"
                    ? "Modrinth"
                    : u.platform === "xunlei"
                      ? "迅雷"
                      : "第三方云盘"
          }}] {{ version.name }}
        </span>
        <p class="m-0 truncate text-nowrap text-xs font-semibold text-secondary">
          {{ version.version_number }}
        </p>
      </div>
      <ButtonStyled color="brand">
        <a
          :href="u.url"
          @click="emit('onDownload', props.version.id)"
          target="_blank"
          class="min-w-0"
        >
          <DownloadIcon aria-hidden="true" />
        </a>
      </ButtonStyled>
      <ButtonStyled circular>
        <nuxt-link
          :to="`/project/${props.version.project_id}/version/${props.version.id}`"
          class="min-w-0"
          aria-label="Open project page"
          @click="emit('onNavigate')"
        >
          <ExternalIcon aria-hidden="true" />
        </nuxt-link>
      </ButtonStyled>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ButtonStyled, VersionChannelIndicator } from "@modrinth/ui";
import { DownloadIcon, ExternalIcon } from "@modrinth/assets";
import { usePrivateDownload, isPrivateUrl } from "~/composables/usePrivateDownload";

const props = defineProps<{
  version: Version;
}>();

const emit = defineEmits(["onDownload", "onNavigate"]);

const { isDownloading, download, getHref } = usePrivateDownload();

// 获取主文件
const primaryFile = computed(() => {
  return props.version.files.find((x) => x.primary) || props.version.files[0];
});

const downloadUrl = computed(() => primaryFile.value?.url || "");

// 是否是站内下载（CDN 或私有文件）
const isInternalDownload = computed(() => {
  return downloadUrl.value.includes("cdn.bbsmc.net") || isPrivateUrl(downloadUrl.value);
});

// 是否是付费下载（私有文件）
const isPaidDownload = computed(() => {
  return isPrivateUrl(downloadUrl.value);
});

// 获取下载链接
const downloadHref = computed(() => {
  if (!primaryFile.value) return "#";
  return getHref(primaryFile.value);
});

// 处理下载点击
const handleDownload = async (event: Event) => {
  emit("onDownload", props.version.id);

  if (isPaidDownload.value && primaryFile.value) {
    event.preventDefault();
    await download(primaryFile.value);
  }
};
</script>
