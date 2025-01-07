<template>
  <div v-if="!version.disk_only"
    class="grid grid-cols-[min-content_auto_min-content_min-content] items-center gap-2 rounded-2xl border-[1px] border-button-bg bg-bg p-2">
    <VersionChannelIndicator :channel="version.version_type" />
    <div class="flex min-w-0 flex-col gap-1">
      <h1 class="my-0 truncate text-nowrap text-base font-extrabold leading-none text-contrast">
        {{ version.version_number }}
      </h1>
      <p class="m-0 truncate text-nowrap text-xs font-semibold text-secondary">
        {{ version.name }}
      </p>
    </div>
    <ButtonStyled color="brand">
      <a :href="downloadUrl" class="min-w-0" @click="emit('onDownload')" v-if="downloadUrl.includes('cdn.bbsmc.net')">
        <DownloadIcon aria-hidden="true" />
      </a>
      <a :href="downloadUrl" target="_blank" class="min-w-0" v-else>
        <DownloadIcon aria-hidden="true" />
      </a>
    </ButtonStyled>
    <ButtonStyled circular>
      <nuxt-link :to="`/project/${props.version.project_id}/version/${props.version.id}`" class="min-w-0"
        aria-label="Open project page" @click="emit('onNavigate')">
        <ExternalIcon aria-hidden="true" />
      </nuxt-link>
    </ButtonStyled>
  </div>
  <div v-else>
    <div v-for="(u, index) in props.version.disk_urls" :key="index"
      class="grid grid-cols-[min-content_auto_min-content_min-content] items-center gap-2 rounded-2xl border-[1px] border-button-bg bg-bg p-2">
      <VersionChannelIndicator :channel="version.version_type" />
      <div class="flex min-w-0 flex-col gap-1">
        <h1 class="my-0 truncate text-nowrap text-base font-extrabold leading-none text-contrast">
          [{{ u.platform === 'quark' ? '夸克云盘' : u.platform === 'baidu' ? '百度云盘' : u.platform === 'xunlei' ? '迅雷' :
            '第三方云盘' }}] {{ version.version_number }}
        </h1>
        <p class="m-0 truncate text-nowrap text-xs font-semibold text-secondary">
          {{ version.name }}
        </p>
      </div>
      <ButtonStyled color="brand">

        <a :href="u.url" @click="emit('onDownload', props.version.id)" target="_blank" class="min-w-0">
          <DownloadIcon aria-hidden="true" />
        </a>
      </ButtonStyled>
      <ButtonStyled circular>
        <nuxt-link :to="`/project/${props.version.project_id}/version/${props.version.id}`" class="min-w-0"
          aria-label="Open project page" @click="emit('onNavigate')">
          <ExternalIcon aria-hidden="true" />
        </nuxt-link>
      </ButtonStyled>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ButtonStyled, VersionChannelIndicator } from "@modrinth/ui";
import { DownloadIcon, ExternalIcon } from "@modrinth/assets";

const props = defineProps<{
  version: Version;
}>();

const downloadUrl = computed(() => {
  const primary: VersionFile = props.version.files.find((x) => x.primary) || props.version.files[0];
  return primary.url;
});

const emit = defineEmits(["onDownload", "onNavigate"]);
</script>
