import type { Labrinth } from "@modrinth/api-client";

export type DiskDisplayMode = Labrinth.Versions.v3.DiskDisplayMode;

/**
 * 网盘链接展示方式（仅夸克/迅雷/百度三个网盘支持）
 * DropdownSelect 的 options 为字符串数组，v-model 直接存 mode 字符串
 */
export const diskUrlDisplayModes: string[] = ["default", "qrcode", "both"];

export function diskUrlDisplayLabel(mode: string): string {
  switch (mode) {
    case "qrcode":
      return "纯二维码";
    case "both":
      return "二维码+链接";
    default:
      return "默认下载";
  }
}

/**
 * 网盘平台中文名（用于扫码弹窗文案）
 */
export function diskPlatformLabel(platform: string): string {
  switch (platform) {
    case "quark":
      return "夸克";
    case "baidu":
      return "百度网盘";
    case "xunlei":
      return "迅雷";
    default:
      return "网盘";
  }
}

/**
 * VersionSummary 里的网盘标签（与原有三元链保持一致）
 */
export function diskPlatformTag(platform: string): string {
  switch (platform) {
    case "quark":
      return "夸克云盘";
    case "baidu":
      return "百度云盘";
    case "xunlei":
      return "迅雷";
    case "modrinth":
      return "Modrinth";
    case "curseforge":
      return "CurseForge";
    default:
      return "第三方云盘";
  }
}

export const DISK_QR_TITLE = "扫码转存下载";

export const DISK_QR_HINT = "请使用手机App「扫一扫」识别二维码，转存后即可下载";

export const DISK_QR_REVENUE_NOTICE = "网盘转存可为作者提供收益，你的支持能保障作者持续创作整合包";
