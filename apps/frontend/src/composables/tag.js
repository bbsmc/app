import tags from "~/generated/state.json";

// 上游 b8d97268c (#7015) + bcf469ef9 + 17aaf0659：标签等元数据是构建期常量，
// 客户端已通过上面的静态 import 从 state.json 拿到，无需再塞进每个页面的 SSR payload。
// 原来用 useState("tags")，会把整个对象（categories+loaders+gameVersions 约 160KB）序列化进
// 每个登录/未登录页面的 payload。改为模块级构建一次：
//   - 客户端用普通 shallowRef（数据来自打包进 bundle 的 state.json，无需 payload 水合）；
//   - 服务端用不 track 的 customRef——否则每个请求的 computed 读取时都会在这个长寿命 ref 上
//     挂订阅链，而 SSR 永不卸载、永不释放，累积成内存泄漏（上游 bcf469ef9/17aaf0659 的教训）。
// 本 fork 从不在运行时改动标签数组（无 update-game-versions 插件），故无需 setGameVersions。
const tagsValue = Object.freeze({
  categories: tags.categories,
  loaders: tags.loaders,
  gameVersions: tags.gameVersions,
  donationPlatforms: tags.donationPlatforms,
  reportTypes: tags.reportTypes,
  projectTypes: [
    {
      actual: "mod",
      id: "mod",
      display: "模组",
    },
    {
      actual: "project",
      id: "project",
      display: "资源",
    },
    {
      actual: "mod",
      id: "plugin",
      display: "插件",
    },
    {
      actual: "mod",
      id: "datapack",
      display: "原版数据包",
    },
    {
      actual: "shader",
      id: "shader",
      display: "光影包",
    },
    {
      actual: "resourcepack",
      id: "resourcepack",
      display: "资源包",
    },
    {
      actual: "modpack",
      id: "modpack",
      display: "模组包/整合包",
    },
    {
      actual: "software",
      id: "software",
      display: "软件资源",
    },
    {
      actual: "language",
      id: "language",
      display: "汉化资源",
    },
    {
      actual: "map",
      id: "map",
      display: "地图",
    },
  ],
  forumTypes: [
    {
      actual: "chat",
      id: "chat",
      display: "矿工茶馆",
    },
    {
      actual: "announcement",
      id: "announcement",
      display: "公告",
    },
  ],
  loaderData: {
    pluginLoaders: ["bukkit", "spigot", "paper", "purpur", "sponge", "folia"],
    pluginPlatformLoaders: ["bungeecord", "waterfall", "velocity"],
    allPluginLoaders: [
      "bukkit",
      "spigot",
      "paper",
      "purpur",
      "sponge",
      "bungeecord",
      "waterfall",
      "velocity",
      "folia",
    ],
    allLoaders: [
      "forge",
      "fabric",
      "quilt",
      "liteloader",
      "modloader",
      "rift",
      "neoforge",
      "bukkit",
      "spigot",
      "paper",
      "purpur",
      "sponge",
      "bungeecord",
      "waterfall",
      "velocity",
      "folia",
    ],
    dataPackLoaders: ["datapack"],
    dataSoftwareLoaders: ["windows", "linux", "macos"],
    modLoaders: ["forge", "fabric", "quilt", "liteloader", "modloader", "rift", "neoforge"],
    hiddenModLoaders: ["liteloader", "modloader", "rift"],
  },
  projectViewModes: ["list", "grid", "gallery"],
  approvedStatuses: ["approved", "archived", "unlisted", "private"],
  rejectedStatuses: ["rejected", "withheld"],
  staffRoles: ["moderator", "admin"],
});

// 客户端：普通响应式 ref；数据来自打包进 bundle 的 state.json，无需 payload 水合
const clientTags = shallowRef(tagsValue);

// 服务端：不 track 的 ref，读取不注册订阅，避免 SSR 常驻 isolate 上的订阅链泄漏
const serverTags = customRef(() => ({
  get: () => tagsValue,
  set: () => {},
}));

export const useTags = () => (import.meta.server ? serverTags : clientTags);
