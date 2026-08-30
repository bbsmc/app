import type { Labrinth, UploadProgress } from "@modrinth/api-client";
import { SaveIcon, UpdatedIcon } from "@modrinth/assets";
import {
  createContext,
  injectModrinthClient,
  injectNotificationManager,
  injectProjectPageContext,
  type MultiStageModal,
  resolveCtxFn,
  type StageButtonConfig,
  type StageConfigInput,
} from "@modrinth/ui";
import JSZip from "jszip";
import type { ComputedRef, Ref, ShallowRef } from "vue";
import type { ComponentExposed } from "vue-component-type-helpers";

import { useGeneratedState } from "~/composables/generated";
import { inferVersionInfo } from "~/helpers/infer.js";

import { stageConfigs } from "./stages/index.ts";

// this interface should be in infer.js, but gotta refactor that to ts first
export interface InferredVersionInfo {
  name?: string;
  version_number?: string;
  version_type?: "alpha" | "beta" | "release";
  loaders?: string[];
  game_versions?: string[];
  project_type?: Labrinth.Projects.v2.ProjectType;
  environment?: Labrinth.Projects.v3.Environment;
  /** BBSMC 推断（来自 .lang 文件等） */
  languageCode?: string;
  /** BBSMC: 检测到 modrinth.index.json / manifest.json 时为 true（整合包） */
  is_modpack?: boolean;
}

const EMPTY_DRAFT_VERSION: Labrinth.Versions.v3.DraftVersion = {
  project_id: "",
  name: "",
  version_number: "",
  version_type: "release",
  loaders: [],
  game_versions: [],
  featured: false,
  status: "draft",
  changelog: "",
  dependencies: [],
  // BBSMC 自定义字段（必选 type 由 UI 强制让用户先选）
  type: null,
  disk_only: false,
  disk_urls: [],
  version_links: [],
  // is_modpack 不设默认值（undefined）：网盘模式下必须用户显式选"是/否"才能继续
  // 用户主动选择后变为 true 或 false；自动从 manifest.json/modrinth.index.json 检测到也会设 true
  is_modpack: undefined,
  quark_disk: "",
  xunlei_disk: "",
  baidu_disk: "",
  modrinth: "",
  curseforge: "",
  quark_disk_display: "default",
  xunlei_disk_display: "default",
  baidu_disk_display: "default",
};

export type VersionStage =
  | "add-files"
  | "upload-mode"
  | "files-upload"
  | "add-details"
  | "add-loaders"
  | "add-mc-versions"
  | "add-environment"
  | "add-dependencies"
  | "metadata"
  | "from-details-files"
  | "from-details-loaders"
  | "from-details-mc-versions"
  | "from-details-environment"
  | "from-details-dependencies";

/**
 * BBSMC: 上传方式
 * - local: 仅站内文件（不填网盘）
 * - disk: 仅网盘下载（disk_only=true，不上传文件）
 * - both: 站内文件 + 网盘下载（disk_only=false，两者并存）
 */
export type UploadMode = "local" | "disk" | "both";

/**
 * BBSMC: 翻译链接的语言选项（5 种）
 */
export const TRANSLATION_LANGUAGE_OPTIONS = [
  { value: "zh_CN", label: "简体中文" },
  { value: "zh_TW", label: "繁体中文" },
  { value: "en_US", label: "英语" },
  { value: "ja_JP", label: "日语" },
  { value: "ko_KR", label: "韩语" },
] as const;

export interface PrimaryFile {
  name: string;
  fileType?: string;
  existing?: boolean;
}

export interface ManageVersionContextValue {
  // === 状态 ===
  draftVersion: Ref<Labrinth.Versions.v3.DraftVersion>;
  filesToAdd: Ref<Labrinth.Versions.v3.DraftVersionFile[]>;
  existingFilesToDelete: Ref<Labrinth.Versions.v3.VersionFileHash["sha1"][]>;
  inferredVersionData: Ref<InferredVersionInfo | undefined>;
  projectType: ComputedRef<Labrinth.Projects.v2.ProjectType>;
  dependencyProjects: Ref<Record<string, Labrinth.Projects.v3.Project>>;
  dependencyVersions: Ref<Record<string, Labrinth.Versions.v3.Version>>;
  handlingNewFiles: Ref<boolean>;
  primaryFile: ComputedRef<PrimaryFile | null>;

  // === Stage 管理 ===
  stageConfigs: StageConfigInput<ManageVersionContextValue>[];
  isSubmitting: Ref<boolean>;
  isUploading: Ref<boolean>;
  uploadProgress: Ref<UploadProgress>;
  modal: ShallowRef<ComponentExposed<typeof MultiStageModal> | null>;

  // === BBSMC: 上传方式 ===
  uploadMode: Ref<UploadMode | null>;
  /** 上传方式是否需要"网盘"输入（disk / both） */
  needsDiskInputs: ComputedRef<boolean>;
  /** 上传方式是否需要"站内文件"上传（local / both） */
  needsLocalFiles: ComputedRef<boolean>;

  // === 计算状态 ===
  editingVersion: ComputedRef<boolean>;
  noLoadersProject: ComputedRef<boolean>;
  noEnvironmentProject: ComputedRef<boolean>;
  noDependenciesProject: ComputedRef<boolean>;
  /** BBSMC: language 类型版本 */
  isLanguageVersion: ComputedRef<boolean>;
  /** BBSMC: disk_only 模式 */
  isDiskOnly: ComputedRef<boolean>;

  // === Stage helpers ===
  getNextLabel: (currentIndex?: number | null) => string;
  saveButtonConfig: () => StageButtonConfig;

  // === 版本/文件方法 ===
  newDraftVersion: (
    projectId: string,
    version?: Labrinth.Versions.v3.DraftVersion | null,
  ) => void;
  /** 主次切换：把 index 处的文件交换到位置 0（成为主文件） */
  swapPrimaryFile: (index: number) => Promise<void>;
  /** @deprecated 兼容旧 stage UI；新代码请使用 swapPrimaryFile */
  setPrimaryFile: (index: number) => Promise<void>;
  /** 替换主文件：编辑模式下不能用 */
  replacePrimaryFile: (file: File) => Promise<void>;
  /** 处理新上传的文件（可拖放、可点击选择） */
  handleNewFiles: (newFiles: File[]) => Promise<void>;
  setInferredVersionData: (
    file: File,
    project: Labrinth.Projects.v2.Project,
  ) => Promise<InferredVersionInfo>;
  getProject: (projectId: string) => Promise<Labrinth.Projects.v3.Project>;
  getVersion: (versionId: string) => Promise<Labrinth.Versions.v3.Version>;

  // === BBSMC: 翻译链接管理 ===
  addVersionLink: (link: Labrinth.Versions.v3.VersionLink) => void;
  removeVersionLink: (index: number) => void;

  // === 提交 ===
  handleCreateVersion: () => Promise<void>;
  handleSaveVersionEdits: () => Promise<void>;
}

const PROJECT_TYPE_LOADERS: Record<string, readonly string[]> = {
  mod: [
    "fabric",
    "neoforge",
    "forge",
    "quilt",
    "liteloader",
    "rift",
    "ornithe",
    "nilloader",
    "risugami",
    "legacy-fabric",
    "bta-babric",
    "babric",
    "modloader",
    "java-agent",
  ],
  shader: ["optifine", "iris", "canvas", "vanilla"],
  plugin: [
    "paper",
    "purpur",
    "spigot",
    "bukkit",
    "sponge",
    "folia",
    "bungeecord",
    "velocity",
    "waterfall",
    "geyser",
  ],
  datapack: ["datapack"],
  resourcepack: ["minecraft"],
  modpack: ["mrpack"],
} as const;

export const [injectManageVersionContext, provideManageVersionContext] =
  createContext<ManageVersionContextValue>("CreateProjectVersionModal");

export function createManageVersionContext(
  modal: ShallowRef<ComponentExposed<typeof MultiStageModal> | null>,
  onSave?: () => void,
): ManageVersionContextValue {
  const { labrinth } = injectModrinthClient();
  const { addNotification } = injectNotificationManager();
  const { refreshVersions, invalidate, projectV2 } = injectProjectPageContext();

  // === 状态 ===
  const draftVersion = ref<Labrinth.Versions.v3.DraftVersion>(
    structuredClone(EMPTY_DRAFT_VERSION),
  );
  const filesToAdd = ref<Labrinth.Versions.v3.DraftVersionFile[]>([]);
  const existingFilesToDelete = ref<Labrinth.Versions.v3.VersionFileHash["sha1"][]>([]);
  const handlingNewFiles = ref(false);
  const inferredVersionData = ref<InferredVersionInfo>();
  const dependencyProjects = ref<Record<string, Labrinth.Projects.v3.Project>>({});
  const dependencyVersions = ref<Record<string, Labrinth.Versions.v3.Version>>({});

  const isSubmitting = ref(false);
  const isUploading = ref(false);
  const uploadProgress = ref<UploadProgress>({ loaded: 0, total: 0, progress: 0 });

  // BBSMC: 上传方式（在 stage 间共享）
  const uploadMode = ref<UploadMode | null>(null);
  const needsDiskInputs = computed(
    () => uploadMode.value === "disk" || uploadMode.value === "both",
  );
  const needsLocalFiles = computed(
    () => uploadMode.value === "local" || uploadMode.value === "both",
  );

  // === 项目类型推断 ===
  const projectType = computed<Labrinth.Projects.v2.ProjectType>(() => {
    // BBSMC: 用户在"资源类型"步骤选了地图 → 直接定为 map
    // （loaders 永远是 ["map"]，反推不出 map，必须在前面拦截）
    if (draftVersion.value.type === "map") {
      return "map" as Labrinth.Projects.v2.ProjectType;
    }

    // BBSMC: 项目本身就是 map 类型（编辑版本场景）
    if ((projectV2 as any).value?.project_type === "map") {
      return "map" as Labrinth.Projects.v2.ProjectType;
    }

    // BBSMC: 自动识别的整合包（modrinth.index.json / manifest.json）或用户手动勾选
    if (draftVersion.value.is_modpack) {
      return "modpack";
    }

    const primaryFileLocal = filesToAdd.value[0]?.file;
    if (
      (primaryFileLocal && primaryFileLocal.name.toLowerCase().endsWith(".mrpack")) ||
      (primaryFileLocal && primaryFileLocal.name.toLowerCase().endsWith(".mrpack-primary"))
    ) {
      return "modpack";
    }

    const loaders = draftVersion.value.loaders || [];

    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.modpack.includes(loader))) {
      return "modpack";
    }
    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.datapack.includes(loader))) {
      return "datapack";
    }
    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.resourcepack.includes(loader))) {
      return "resourcepack";
    }
    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.shader.includes(loader))) {
      return "shader";
    }
    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.plugin.includes(loader))) {
      return "plugin";
    }
    if (loaders.some((loader: string) => PROJECT_TYPE_LOADERS.mod.includes(loader))) {
      return "mod";
    }

    return "project";
  });

  // === 计算状态 ===
  const editingVersion = computed(() => Boolean(draftVersion.value.version_id));

  /**
   * 是否为"单一 loader"项目（不让用户选 loader，由项目类型决定）
   * - resourcepack → minecraft
   * - modpack → mrpack
   * - datapack → datapack
   * - map → map（地图无运行时加载器，统一打 map 标）
   */
  const noLoadersProject = computed(() => {
    const pt = (projectV2 as any).value?.project_type;
    return (
      pt === "resourcepack" ||
      pt === "modpack" ||
      pt === "datapack" ||
      pt === "map" ||
      projectType.value === "resourcepack" ||
      projectType.value === "map"
    );
  });
  const noEnvironmentProject = computed(
    () => projectType.value !== "mod" && projectType.value !== "modpack",
  );
  const noDependenciesProject = computed(
    () => projectType.value === "modpack" || projectType.value === "map",
  );
  const isLanguageVersion = computed(() => draftVersion.value.type === "language");
  const isDiskOnly = computed(() => !!draftVersion.value.disk_only);

  // === 主文件计算 ===
  const primaryFile = computed<PrimaryFile | null>(() => {
    const existingPrimary = draftVersion.value.existing_files?.[0];
    if (existingPrimary) {
      return {
        name: existingPrimary.filename,
        fileType: existingPrimary.file_type,
        existing: true,
      };
    }

    const addedPrimary = filesToAdd.value[0];
    if (addedPrimary) {
      return {
        name: addedPrimary.file.name,
        fileType: addedPrimary.fileType,
        existing: false,
      };
    }

    return null;
  });

  // === 版本管理 ===
  function newDraftVersion(
    projectId: string,
    version: Labrinth.Versions.v3.DraftVersion | null = null,
  ) {
    draftVersion.value = structuredClone(version ?? EMPTY_DRAFT_VERSION);
    draftVersion.value.project_id = projectId;

    // BBSMC: 仅在新建（非加载现有版本）时根据项目类型预设单一 loader
    if (!version) {
      const pt = (projectV2 as any).value?.project_type;
      if (pt === "resourcepack") {
        draftVersion.value.loaders = ["minecraft"];
      } else if (pt === "modpack") {
        draftVersion.value.loaders = ["mrpack"];
      } else if (pt === "datapack") {
        draftVersion.value.loaders = ["datapack"];
      }
    }

    // BBSMC: 重置上传方式（让用户重新选）；编辑模式根据已有数据回填
    if (version) {
      const hasFiles = (version.existing_files?.length ?? 0) > 0;
      const hasDisks =
        !!version.quark_disk ||
        !!version.xunlei_disk ||
        !!version.baidu_disk ||
        !!version.modrinth ||
        !!version.curseforge ||
        (version.disk_urls?.length ?? 0) > 0;
      if (hasFiles && hasDisks) uploadMode.value = "both";
      else if (hasDisks) uploadMode.value = "disk";
      else if (hasFiles) uploadMode.value = "local";
      else uploadMode.value = null;
    } else {
      uploadMode.value = null;
    }

    filesToAdd.value = [];
    existingFilesToDelete.value = [];
    inferredVersionData.value = undefined;
  }

  // === 文件检测 helpers ===
  const tags = useGeneratedState();

  const hasFile = (entries: string[], name: string) =>
    entries.some((f) => f === name || f.endsWith(`/${name}`));

  const hasDir = (entries: string[], dir: string) => entries.some((f) => f.startsWith(`${dir}/`));

  async function checkIsResourcePack(file: File): Promise<boolean> {
    try {
      const name = file.name.toLowerCase();
      if (!name.endsWith(".zip")) return false;

      const zip = await JSZip.loadAsync(file);
      const entries = Object.keys(zip.files).map((f) => f.toLowerCase());

      return hasFile(entries, "pack.mcmeta") && hasDir(entries, "assets");
    } catch {
      return false;
    }
  }

  async function checkIsDataPack(file: File): Promise<boolean> {
    try {
      const name = file.name.toLowerCase();
      if (!name.endsWith(".zip")) return false;

      const zip = await JSZip.loadAsync(file);
      const entries = Object.keys(zip.files).map((f) => f.toLowerCase());

      return hasFile(entries, "pack.mcmeta") && hasDir(entries, "data");
    } catch {
      return false;
    }
  }

  async function setInferredVersionData(
    file: File,
    project: Labrinth.Projects.v2.Project,
  ): Promise<InferredVersionInfo> {
    const inferred = (await inferVersionInfo(
      file,
      project,
      tags.value.gameVersions,
    )) as InferredVersionInfo;

    try {
      const versions = await labrinth.versions_v3.getProjectVersions(project.id, {
        loaders: inferred.loaders ?? [],
      });

      if (versions.length > 0) {
        const mostRecentVersion = versions[0];
        const version = await labrinth.versions_v3.getVersion(mostRecentVersion.id);
        inferred.environment = version.environment !== "unknown" ? version.environment : undefined;
      }
    } catch (error) {
      console.error("Error fetching versions for environment inference:", error);
    }

    const noLoaders = !inferred.loaders?.length;

    if (noLoaders && (await checkIsResourcePack(file))) {
      inferred.loaders = ["minecraft"];
    }

    if (noLoaders && (await checkIsDataPack(file))) {
      inferred.loaders = ["datapack"];
    }

    if (noLoaders && projectType.value === "modpack") {
      inferred.loaders = ["minecraft"];
    }

    inferredVersionData.value = inferred;

    return inferred;
  }

  // === 文件操作 ===
  function detectPrimaryFileIndex(files: File[]): number {
    const extensionPriority = [".jar", ".zip", ".litemod", ".mrpack", ".mrpack-primary"];

    for (const ext of extensionPriority) {
      const matches = files.filter((file) => file.name.toLowerCase().endsWith(ext));
      if (matches.length > 0) {
        const shortest = matches.reduce((a, b) => (a.name.length < b.name.length ? a : b));
        return files.indexOf(shortest);
      }
    }

    return 0;
  }

  const addDetectedData = async (file?: File) => {
    if (editingVersion.value) return;

    const primaryFileData = file ?? filesToAdd.value[0]?.file;
    if (!primaryFileData) return;

    try {
      // 用顶层 setup 阶段已 inject 的 projectV2（不能在异步回调内重新调 inject）
      const project = projectV2.value;
      const inferredData = await setInferredVersionData(
        primaryFileData,
        project as unknown as Labrinth.Projects.v2.Project,
      );
      const mappedInferredData: Partial<Labrinth.Versions.v3.DraftVersion> = {
        ...inferredData,
        name: inferredData.name || "",
      };

      draftVersion.value = {
        ...draftVersion.value,
        ...mappedInferredData,
      };
    } catch (err) {
      console.error("Error parsing version file data", err);
    }
  };

  async function handleNewFiles(newFiles: File[]) {
    handlingNewFiles.value = true;
    try {
      const primaryFileIndex = primaryFile.value ? null : detectPrimaryFileIndex(newFiles);

      newFiles.forEach((file) => filesToAdd.value.push({ file }));

      if (primaryFileIndex !== null) {
        if (primaryFileIndex) {
          await swapPrimaryFile(primaryFileIndex);
        } else {
          // 第 0 个就是主文件，触发推断
          const primaryFileData = filesToAdd.value[0]?.file;
          if (primaryFileData) await addDetectedData(primaryFileData);
        }
      }
    } finally {
      handlingNewFiles.value = false;
    }
  }

  async function replacePrimaryFile(file: File) {
    if (file && !editingVersion.value) {
      filesToAdd.value[0] = { file };
      await addDetectedData(file);
    }
  }

  async function swapPrimaryFile(index: number) {
    const files = filesToAdd.value;
    if (index <= 0 || index >= files.length) return;
    files[0].fileType = "unknown";
    files[index].fileType = "unknown";
    [files[0], files[index]] = [files[index], files[0]];
    await addDetectedData(files[0].file);
  }

  // === 依赖项目/版本数据 ===
  const getProject = async (projectId: string) => {
    if (dependencyProjects.value[projectId]) {
      return dependencyProjects.value[projectId];
    }
    const proj = await labrinth.projects_v3.get(projectId);
    dependencyProjects.value[projectId] = proj;
    return proj;
  };

  const getVersion = async (versionId: string) => {
    if (dependencyVersions.value[versionId]) {
      return dependencyVersions.value[versionId];
    }
    const version = await labrinth.versions_v3.getVersion(versionId);
    dependencyVersions.value[versionId] = version;
    return version;
  };

  // === BBSMC: 翻译链接管理 ===
  function addVersionLink(link: Labrinth.Versions.v3.VersionLink) {
    if (!draftVersion.value.version_links) draftVersion.value.version_links = [];
    draftVersion.value.version_links.push(link);
  }

  function removeVersionLink(index: number) {
    if (!draftVersion.value.version_links) return;
    draftVersion.value.version_links.splice(index, 1);
  }

  // === 提交：创建版本 ===
  async function handleCreateVersion() {
    const version = toRaw(draftVersion.value);
    const files = toRaw(filesToAdd.value);
    isSubmitting.value = true;
    isUploading.value = true;
    uploadProgress.value = { loaded: 0, total: 0, progress: 0 };

    if (noEnvironmentProject.value) version.environment = undefined;

    // BBSMC: 根据 uploadMode 设置 disk_only 和清理无关字段
    if (uploadMode.value === "disk") {
      version.disk_only = true;
    } else {
      version.disk_only = false;
      if (uploadMode.value === "local") {
        // 纯站内：清空网盘字段
        version.disk_urls = [];
        version.quark_disk = "";
        version.xunlei_disk = "";
        version.baidu_disk = "";
        version.modrinth = "";
        version.curseforge = "";
        version.quark_disk_display = "default";
        version.xunlei_disk_display = "default";
        version.baidu_disk_display = "default";
        // 不强制重置 is_modpack：local 模式下整合包标识可能由文件自动检测得到（modrinth.index.json/manifest.json）
      }
    }

    // 提交前确保 is_modpack 是 boolean（undefined 时默认 false）
    if (typeof version.is_modpack !== "boolean") {
      version.is_modpack = false;
    }

    try {
      const uploadHandle = labrinth.versions_v3.createVersion(
        version,
        files,
        projectType.value ?? null,
      );

      uploadHandle.onProgress((progress) => {
        uploadProgress.value = progress;
      });

      await uploadHandle.promise;

      isUploading.value = false;
      modal.value?.hide();
      addNotification({
        title: "版本已创建",
        text: "新版本已成功添加到项目。",
        type: "success",
      });
      if (invalidate) {
        await invalidate();
      } else {
        await refreshVersions();
      }
      onSave?.();
    } catch (err: any) {
      addNotification({
        title: "创建版本失败",
        text: err?.data ? err.data.description : err?.message ?? String(err),
        type: "error",
      });
    } finally {
      isUploading.value = false;
      isSubmitting.value = false;
    }
  }

  // === 提交：保存编辑 ===
  async function handleSaveVersionEdits() {
    // BBSMC: 先同步 uploadMode → disk_only / 网盘字段 / 已有文件删除标记，
    // 必须放在 toRaw 之前完成，因为切到 disk 模式时需要往 existingFilesToDelete 推 sha1
    if (uploadMode.value === "disk") {
      draftVersion.value.disk_only = true;
      // 切到"仅网盘下载"时，把所有已上传的站内文件加入删除队列，
      // 否则后端不会清掉这些文件，UI 切换形同虚设
      const existingFiles = draftVersion.value.existing_files ?? [];
      for (const file of existingFiles) {
        const sha1 = file.hashes?.sha1;
        if (sha1 && !existingFilesToDelete.value.includes(sha1)) {
          existingFilesToDelete.value.push(sha1);
        }
      }
      // 切到 disk 模式后不应再上传新文件（UI 已隐藏，但兜底清空）
      filesToAdd.value = [];
    } else if (uploadMode.value === "local" || uploadMode.value === "both") {
      draftVersion.value.disk_only = false;
      if (uploadMode.value === "local") {
        draftVersion.value.disk_urls = [];
        draftVersion.value.quark_disk = "";
        draftVersion.value.xunlei_disk = "";
        draftVersion.value.baidu_disk = "";
        draftVersion.value.modrinth = "";
        draftVersion.value.curseforge = "";
        draftVersion.value.quark_disk_display = "default";
        draftVersion.value.xunlei_disk_display = "default";
        draftVersion.value.baidu_disk_display = "default";
      }
    }

    const version = toRaw(draftVersion.value);
    const files = toRaw(filesToAdd.value);
    const filesToDelete = toRaw(existingFilesToDelete.value);

    isSubmitting.value = true;

    if (files.length > 0) {
      isUploading.value = true;
      uploadProgress.value = { loaded: 0, total: 0, progress: 0 };
    }

    if (noEnvironmentProject.value) version.environment = undefined;

    try {
      if (!version.version_id) throw new Error("Version ID is required to save edits.");

      const isLanguageType = version.type === "language";
      const isSoftwareType = version.type === "software";
      const isModpackVersion = !!version.is_modpack;
      // BBSMC: language / software 类型 loader 不绑定 game_versions field
      const skipGameVersions = isLanguageType || isSoftwareType;

      // BBSMC: 同 v3.ts createVersion，编辑时也把自定义类型转换为 v3 标准 loader 格式
      const originalLoadersEdit = version.loaders;
      let editLoaders: string[] = originalLoadersEdit;
      let editMrpackLoaders: string[] | undefined;
      let editSoftwareLoaders: string[] | undefined;

      if (isLanguageType) {
        editLoaders = ["language"];
      } else if (isModpackVersion || projectType.value === "modpack") {
        editMrpackLoaders = originalLoadersEdit.filter((l: string) => l !== "mrpack");
        editLoaders = ["mrpack"];
      } else if (isSoftwareType) {
        editSoftwareLoaders = originalLoadersEdit.filter((l: string) => l !== "software");
        editLoaders = ["software"];
      }

      // 用 any 构造便于条件性 omit 字段（不发的字段直接从对象删除，避免后端把空字段
      // 当 loader_field 查表导致 "加载器字段 'xxx' 对于任何提供的加载器都不存在" 错误）
      const data: any = {
        name: version.name || version.version_number,
        version_number: version.version_number,
        changelog: version.changelog,
        version_type: version.version_type,
        dependencies: version.dependencies || [],
        loaders: editLoaders,
        file_types: version.existing_files
          ?.filter((file: any) => file.file_type)
          .map((file: any) => ({
            algorithm: "sha1",
            hash: file.hashes.sha1,
            file_type: file.file_type ?? null,
          })),
        disk_only: !!version.disk_only,
        // BBSMC: 编辑模式下显式以数组（即使为空）发送，避免 null 时后端跳过更新
        // 导致从"站内+网盘"切到"仅站内"后旧网盘链接残留
        disk_urls: aggregateDiskUrlsFromDraft(version) ?? [],
      };

      if (!skipGameVersions) {
        data.game_versions = version.game_versions;
      }
      if (version.environment) {
        data.environment = version.environment;
      }
      if (isLanguageType && version.version_links && version.version_links.length > 0) {
        data.version_links = version.version_links;
      }
      if (editMrpackLoaders) {
        data.mrpack_loaders = editMrpackLoaders;
      }
      if (editSoftwareLoaders) {
        data.software_loaders = editSoftwareLoaders;
      }

      await labrinth.versions_v3.modifyVersion(version.version_id, data);

      if (files.length > 0) {
        const uploadHandle = labrinth.versions_v3.addFilesToVersion(version.version_id, files);
        uploadHandle.onProgress((progress) => {
          uploadProgress.value = progress;
        });
        await uploadHandle.promise;
      }

      // 删除标记的文件
      for (const hash of filesToDelete) {
        await useBaseFetch(`version_file/${hash}?version_id=${version.version_id}`, {
          method: "DELETE",
        });
      }

      modal.value?.hide();
      addNotification({
        title: "版本已保存",
        text: "版本已成功更新。",
        type: "success",
      });
      if (invalidate) {
        await invalidate();
      } else {
        await refreshVersions();
      }
      onSave?.();
    } catch (err: any) {
      addNotification({
        title: "保存失败",
        text: err?.data ? err.data.description : err?.message ?? String(err),
        type: "error",
      });
    } finally {
      isUploading.value = false;
      isSubmitting.value = false;
    }
  }

  // === Stage 标签 ===
  function getNextLabel(currentIndex: number | null = null) {
    const currentStageIndex = currentIndex ?? modal.value?.currentStageIndex ?? 0;

    let nextIndex = currentStageIndex + 1;
    while (nextIndex < stageConfigs.length) {
      const skip = stageConfigs[nextIndex]?.skip;
      if (!skip || !resolveCtxFn(skip, contextValue)) break;
      nextIndex++;
    }

    const next = stageConfigs[nextIndex];
    if (!next) return "完成";

    switch (next.id) {
      case "add-details":
        return editingVersion.value ? "编辑详情" : "填写详情";
      case "add-files":
        return editingVersion.value ? "编辑版本" : "选择类型";
      case "upload-mode":
        return editingVersion.value ? "编辑上传方式" : "选择上传方式";
      case "files-upload":
        return editingVersion.value ? "编辑上传内容" : "上传内容";
      case "add-loaders":
        return editingVersion.value ? "编辑加载器" : "选择加载器";
      case "add-mc-versions":
        return editingVersion.value ? "编辑游戏版本" : "选择游戏版本";
      case "add-dependencies":
        return editingVersion.value ? "编辑依赖" : "设置依赖";
      case "add-environment":
        return editingVersion.value ? "编辑环境" : "选择环境";
      case "metadata":
        return editingVersion.value ? "编辑元数据" : "确认信息";
      default:
        return "下一步";
    }
  }

  const saveButtonConfig = (): StageButtonConfig => ({
    label: "保存修改",
    icon: isSubmitting.value ? UpdatedIcon : SaveIcon,
    iconPosition: "before",
    iconClass: isSubmitting.value ? "animate-spin" : undefined,
    color: "green",
    disabled: isSubmitting.value,
    onClick: () => handleSaveVersionEdits(),
  });

  const contextValue: ManageVersionContextValue = {
    // 状态
    draftVersion,
    filesToAdd,
    existingFilesToDelete,
    inferredVersionData,
    projectType,
    dependencyProjects,
    dependencyVersions,
    handlingNewFiles,
    primaryFile,

    // Stage
    stageConfigs,
    isSubmitting,
    isUploading,
    uploadProgress,
    modal,

    // BBSMC: 上传方式
    uploadMode,
    needsDiskInputs,
    needsLocalFiles,

    // Computed
    editingVersion,
    noLoadersProject,
    noEnvironmentProject,
    noDependenciesProject,
    isLanguageVersion,
    isDiskOnly,

    // Stage helpers
    getNextLabel,
    saveButtonConfig,

    // Methods
    newDraftVersion,
    swapPrimaryFile,
    setPrimaryFile: swapPrimaryFile, // @deprecated 兼容旧 stage UI
    replacePrimaryFile,
    handleNewFiles,
    setInferredVersionData,
    getProject,
    getVersion,

    // BBSMC: 翻译链接
    addVersionLink,
    removeVersionLink,

    // 提交
    handleCreateVersion,
    handleSaveVersionEdits,
  };

  return contextValue;
}

/**
 * BBSMC: 把 draftVersion 上的 5 个网盘 input 字段聚合为 disk_urls 数组
 * 与 v3.ts 内同名函数语义一致，但在 manage-version-modal.ts 内独立保留一份方便调用
 */
function aggregateDiskUrlsFromDraft(
  draft: Labrinth.Versions.v3.DraftVersion,
): Labrinth.Versions.v3.QueryDisk[] | null {
  const out: Labrinth.Versions.v3.QueryDisk[] = [];
  const push = (platform: string, url?: string, display?: string) => {
    if (url && url.trim() !== "") out.push({ platform, url, display: display ?? "default" });
  };
  push("quark", draft.quark_disk, draft.quark_disk_display);
  push("baidu", draft.baidu_disk, draft.baidu_disk_display);
  push("curseforge", draft.curseforge);
  push("modrinth", draft.modrinth);
  push("xunlei", draft.xunlei_disk, draft.xunlei_disk_display);

  // 如果调用者已经设置了 disk_urls 数组（直接用），优先使用
  if ((!out.length) && draft.disk_urls && draft.disk_urls.length > 0) {
    return draft.disk_urls;
  }

  return out.length > 0 ? out : null;
}
