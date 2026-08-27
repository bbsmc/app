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
          [{{ diskPlatformTag(u.platform) }}] {{ version.name }}
        </span>
        <p class="m-0 truncate text-nowrap text-xs font-semibold text-secondary">
          {{ version.version_number }}
        </p>
      </div>
      <ButtonStyled color="brand">
        <a :href="u.url" target="_blank" class="min-w-0" @click="handleDiskDownload(u, $event)">
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

  <!--  网盘扫码弹窗（纯二维码 / 二维码+链接 模式） -->
  <NewModal ref="diskQrModal" :header="DISK_QR_TITLE">
    <div v-if="qrTarget" class="flex w-full flex-col items-center gap-4">
      <QrcodeVue :value="qrTarget.url" :size="220" margin="3" class="rounded-xl bg-white p-2" />
      <p class="m-0 text-center text-sm text-secondary">
        请使用{{ qrTarget.platformLabel }}手机App「扫一扫」识别二维码，转存后即可下载
      </p>
      <Admonition type="info" class="w-full">
        {{ DISK_QR_REVENUE_NOTICE }}
      </Admonition>
      <ButtonStyled v-if="qrTarget.mode === 'both'" color="brand" class="w-full">
        <a :href="qrTarget.url" target="_blank" rel="noopener" class="w-full justify-center">
          打开{{ qrTarget.platformLabel }}页面
          <ExternalIcon aria-hidden="true" />
        </a>
      </ButtonStyled>
    </div>
  </NewModal>
</template>

<script setup lang="ts">
import { ButtonStyled, Admonition, NewModal, VersionChannelIndicator } from "@modrinth/ui";
import { DownloadIcon, ExternalIcon } from "@modrinth/assets";
import QrcodeVue from "qrcode.vue";
import { usePrivateDownload, isPrivateUrl } from "~/composables/usePrivateDownload";
import {
  diskPlatformLabel,
  diskPlatformTag,
  DISK_QR_REVENUE_NOTICE,
  DISK_QR_TITLE,
} from "~/utils/disk-urls";

const props = defineProps<{
  version: Version;
}>();

const emit = defineEmits(["onDownload", "onNavigate", "onDiskQr"]);

const { isDownloading, download, getHref } = usePrivateDownload();

// 网盘扫码弹窗状态
const diskQrModal = ref();
const qrTarget = ref<{ url: string; platformLabel: string; mode: string } | null>(null);

// 网盘下载点击：default 直接跳转；qrcode/both 弹出扫码窗口并统计
const handleDiskDownload = (
  u: { url: string; platform: string; display?: string },
  event: MouseEvent,
) => {
  const mode = u.display ?? "default";
  if (mode === "default") {
    emit("onDownload", props.version.id);
    return; // 默认模式：a 标签自身新标签页跳转
  }
  event.preventDefault();
  qrTarget.value = { url: u.url, platformLabel: diskPlatformLabel(u.platform), mode };
  diskQrModal.value?.show();
  emit("onDiskQr", props.version.id);
};

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
