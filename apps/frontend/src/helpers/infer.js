import { parse as parseTOML } from "@ltd/j-toml";
import JSZip from "jszip";
import { load as yamlLoad } from "js-yaml";
import { satisfies } from "semver";

export const inferVersionInfo = async function (rawFile, project, gameVersions) {
  function versionType(number) {
    if (number.includes("alpha")) {
      return "alpha";
    } else if (
      number.includes("beta") ||
      number.match(/[^A-z](rc)[^A-z]/) || // includes `rc`
      number.match(/[^A-z](pre)[^A-z]/) // includes `pre`
    ) {
      return "beta";
    } else {
      return "release";
    }
  }

  function getGameVersionsMatchingSemverRange(range, gameVersions) {
    if (!range) {
      return [];
    }
    const ranges = Array.isArray(range) ? range : [range];
    return gameVersions.filter((version) => {
      const semverVersion = version.split(".").length === 2 ? `${version}.0` : version; // add patch version if missing (e.g. 1.16 -> 1.16.0)
      return ranges.some((v) => satisfies(semverVersion, v));
    });
  }

  function getGameVersionsMatchingMavenRange(range, gameVersions) {
    if (!range) {
      return [];
    }
    const ranges = [];

    while (range.startsWith("[") || range.startsWith("(")) {
      let index = range.indexOf(")");
      const index2 = range.indexOf("]");
      if (index === -1 || (index2 !== -1 && index2 < index)) {
        index = index2;
      }
      if (index === -1) break;
      ranges.push(range.substring(0, index + 1));
      range = range.substring(index + 1).trim();
      if (range.startsWith(",")) {
        range = range.substring(1).trim();
      }
    }

    if (range) {
      ranges.push(range);
    }

    const LESS_THAN_EQUAL = /^\(,(.*)]$/;
    const LESS_THAN = /^\(,(.*)\)$/;
    const EQUAL = /^\[(.*)]$/;
    const GREATER_THAN_EQUAL = /^\[(.*),\)$/;
    const GREATER_THAN = /^\((.*),\)$/;
    const BETWEEN = /^\((.*),(.*)\)$/;
    const BETWEEN_EQUAL = /^\[(.*),(.*)]$/;
    const BETWEEN_LESS_THAN_EQUAL = /^\((.*),(.*)]$/;
    const BETWEEN_GREATER_THAN_EQUAL = /^\[(.*),(.*)\)$/;

    const semverRanges = [];

    for (const range of ranges) {
      let result;
      if ((result = range.match(LESS_THAN_EQUAL))) {
        semverRanges.push(`<=${result[1]}`);
      } else if ((result = range.match(LESS_THAN))) {
        semverRanges.push(`<${result[1]}`);
      } else if ((result = range.match(EQUAL))) {
        semverRanges.push(`${result[1]}`);
      } else if ((result = range.match(GREATER_THAN_EQUAL))) {
        semverRanges.push(`>=${result[1]}`);
      } else if ((result = range.match(GREATER_THAN))) {
        semverRanges.push(`>${result[1]}`);
      } else if ((result = range.match(BETWEEN))) {
        semverRanges.push(`>${result[1]} <${result[2]}`);
      } else if ((result = range.match(BETWEEN_EQUAL))) {
        semverRanges.push(`>=${result[1]} <=${result[2]}`);
      } else if ((result = range.match(BETWEEN_LESS_THAN_EQUAL))) {
        semverRanges.push(`>${result[1]} <=${result[2]}`);
      } else if ((result = range.match(BETWEEN_GREATER_THAN_EQUAL))) {
        semverRanges.push(`>=${result[1]} <${result[2]}`);
      }
    }
    return getGameVersionsMatchingSemverRange(semverRanges, gameVersions);
  }

  const simplifiedGameVersions = gameVersions
    .filter((it) => it.version_type === "release")
    .map((it) => it.version);

  // @ts-ignore
  const inferFunctions = {
    // NeoForge
    "META-INF/neoforge.mods.toml": (file) => {
      const metadata = parseTOML(file, { joiner: "\n" });
      if (!metadata.mods || metadata.mods.length === 0) {
        return {};
      }

      const neoForgeDependency = Object.values(metadata.dependencies)
        .flat()
        .find((dependency) => dependency.modId === "neoforge");
      if (!neoForgeDependency) {
        return {};
      }

      // https://docs.neoforged.net/docs/gettingstarted/versioning/#neoforge
      const mcVersionRange = neoForgeDependency.versionRange
        .replace("-beta", "")
        .replace(/(\d+)(?:\.(\d+))?(?:\.(\d+)?)?/g, (_match, major, minor) => {
          return `1.${major}${minor ? "." + minor : ""}`;
        });
      const gameVersions = getGameVersionsMatchingMavenRange(
        mcVersionRange,
        simplifiedGameVersions,
      );

      const versionNum = metadata.mods[0].version;
      return {
        name: `${project.title} ${versionNum}`,
        version_number: versionNum,
        loaders: ["neoforge"],
        version_type: versionType(versionNum),
        game_versions: gameVersions,
      };
    },
    // Forge 1.13+
    "META-INF/mods.toml": async (file, zip) => {
      const metadata = parseTOML(file, { joiner: "\n" });

      if (metadata.mods && metadata.mods.length > 0) {
        let versionNum = metadata.mods[0].version;

        // ${file.jarVersion} -> Implementation-Version from manifest
        const manifestFile = zip.file("META-INF/MANIFEST.MF");
        if (
          // eslint-disable-next-line no-template-curly-in-string
          metadata.mods[0].version.includes("${file.jarVersion}") &&
          manifestFile !== null
        ) {
          const manifestText = await manifestFile.async("text");
          const regex = /Implementation-Version: (.*)$/m;
          const match = manifestText.match(regex);
          if (match) {
            // eslint-disable-next-line no-template-curly-in-string
            versionNum = versionNum.replace("${file.jarVersion}", match[1]);
          }
        }

        let gameVersions = [];
        const mcDependencies = Object.values(metadata.dependencies)
          .flat()
          .filter((dependency) => dependency.modId === "minecraft");

        if (mcDependencies.length > 0) {
          gameVersions = getGameVersionsMatchingMavenRange(
            mcDependencies[0].versionRange,
            simplifiedGameVersions,
          );
        }

        return {
          name: `${project.title} ${versionNum}`,
          version_number: versionNum,
          version_type: versionType(versionNum),
          loaders: ["forge"],
          game_versions: gameVersions,
        };
      } else {
        return {};
      }
    },
    // Old Forge
    "mcmod.info": (file) => {
      const metadata = JSON.parse(file);

      return {
        name: metadata.version ? `${project.title} ${metadata.version}` : "",
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        loaders: ["forge"],
        game_versions: simplifiedGameVersions.filter((version) =>
          version.startsWith(metadata.mcversion),
        ),
      };
    },
    // Fabric
    "fabric.mod.json": (file) => {
      const metadata = JSON.parse(file);

      return {
        name: `${project.title} ${metadata.version}`,
        version_number: metadata.version,
        loaders: ["fabric"],
        version_type: versionType(metadata.version),
        game_versions: metadata.depends
          ? getGameVersionsMatchingSemverRange(metadata.depends.minecraft, simplifiedGameVersions)
          : [],
      };
    },
    // Quilt
    "quilt.mod.json": (file) => {
      const metadata = JSON.parse(file);

      return {
        name: `${project.title} ${metadata.quilt_loader.version}`,
        version_number: metadata.quilt_loader.version,
        loaders: ["quilt"],
        version_type: versionType(metadata.quilt_loader.version),
        game_versions: metadata.quilt_loader.depends
          ? getGameVersionsMatchingSemverRange(
              metadata.quilt_loader.depends.find((x) => x.id === "minecraft")
                ? metadata.quilt_loader.depends.find((x) => x.id === "minecraft").versions
                : [],
              simplifiedGameVersions,
            )
          : [],
      };
    },
    // Bukkit + Other Forks
    "plugin.yml": (file) => {
      const metadata = yamlLoad(file);

      return {
        name: `${project.title} ${metadata.version}`,
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        // We don't know which fork of Bukkit users are using
        loaders: [],
        game_versions: gameVersions
          .filter(
            (x) => x.version.startsWith(metadata["api-version"]) && x.version_type === "release",
          )
          .map((x) => x.version),
      };
    },
    // Paper 1.19.3+
    "paper-plugin.yml": (file) => {
      const metadata = yamlLoad(file);

      return {
        name: `${project.title} ${metadata.version}`,
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        loaders: ["paper"],
        game_versions: gameVersions
          .filter(
            (x) => x.version.startsWith(metadata["api-version"]) && x.version_type === "release",
          )
          .map((x) => x.version),
      };
    },
    // Bungeecord + Waterfall
    "bungee.yml": (file) => {
      const metadata = yamlLoad(file);

      return {
        name: `${project.title} ${metadata.version}`,
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        loaders: ["bungeecord"],
      };
    },
    // Velocity
    "velocity-plugin.json": (file) => {
      const metadata = JSON.parse(file);

      return {
        name: `${project.title} ${metadata.version}`,
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        loaders: ["velocity"],
      };
    },
    // Modpacks - Modrinth .mrpack / .zip 格式
    "modrinth.index.json": (file) => {
      const metadata = JSON.parse(file);

      const loaders = [];
      if ("forge" in metadata.dependencies) {
        loaders.push("forge");
      }
      if ("neoforge" in metadata.dependencies) {
        loaders.push("neoforge");
      }
      if ("fabric-loader" in metadata.dependencies) {
        loaders.push("fabric");
      }
      if ("quilt-loader" in metadata.dependencies) {
        loaders.push("quilt");
      }

      return {
        name: `${project.title} ${metadata.versionId}`,
        version_number: metadata.versionId,
        version_type: versionType(metadata.versionId),
        loaders,
        game_versions: gameVersions
          .filter((x) => x.version === metadata.dependencies.minecraft)
          .map((x) => x.version),
        // BBSMC: 标识整合包（含 modrinth.index.json 的 .mrpack 或 .zip）
        is_modpack: true,
        project_type: "modpack",
      };
    },
    // Modpacks - CurseForge .zip 格式
    "manifest.json": (file) => {
      const metadata = JSON.parse(file);
      const loaders = [];
      metadata.minecraft.modLoaders.forEach((modLoader) => {
        if (modLoader.id.includes("neoforge")) {
          loaders.push("neoforge");
        }
        if (modLoader.id.includes("fabric")) {
          loaders.push("fabric");
        }
        if (modLoader.id.includes("forge-")) {
          loaders.push("forge");
        }
        if (modLoader.id.includes("quilt")) {
          loaders.push("quilt");
        }
      });
      return {
        name: `${metadata.name} ${metadata.version}`,
        version_number: metadata.version,
        version_type: versionType(metadata.version),
        loaders,
        game_versions: gameVersions
          .filter((x) => x.version === metadata.minecraft.version)
          .map((x) => x.version),
        // BBSMC: 标识整合包（含 manifest.json 的 CurseForge .zip）
        is_modpack: true,
        project_type: "modpack",
      };
    },

    // Resource Packs + Data Packs
    "pack.mcmeta": (file) => {
      const metadata = JSON.parse(file);

      function getRange(versionA, versionB) {
        const startingIndex = gameVersions.findIndex((x) => x.version === versionA);
        const endingIndex = gameVersions.findIndex((x) => x.version === versionB);

        const final = [];
        const filterOnlyRelease = gameVersions[startingIndex].version_type === "release";

        for (let i = startingIndex; i >= endingIndex; i--) {
          if (gameVersions[i].version_type === "release" || !filterOnlyRelease) {
            final.push(gameVersions[i].version);
          }
        }

        return final;
      }

      const loaders = [];
      let newGameVersions = [];

      if (project.actualProjectType === "mod") {
        loaders.push("datapack");

        switch (metadata.pack.pack_format) {
          case 4:
            newGameVersions = getRange("1.13", "1.14.4");
            break;
          case 5:
            newGameVersions = getRange("1.15", "1.16.1");
            break;
          case 6:
            newGameVersions = getRange("1.16.2", "1.16.5");
            break;
          case 7:
            newGameVersions = getRange("1.17", "1.17.1");
            break;
          case 8:
            newGameVersions = getRange("1.18", "1.18.1");
            break;
          case 9:
            newGameVersions.push("1.18.2");
            break;
          case 10:
            newGameVersions = getRange("1.19", "1.19.3");
            break;
          case 11:
            newGameVersions = getRange("23w03a", "23w05a");
            break;
          case 12:
            newGameVersions.push("1.19.4");
            break;
          default:
        }
      }

      if (project.actualProjectType === "resourcepack") {
        loaders.push("minecraft");

        switch (metadata.pack.pack_format) {
          case 1:
            newGameVersions = getRange("1.6.1", "1.8.9");
            break;
          case 2:
            newGameVersions = getRange("1.9", "1.10.2");
            break;
          case 3:
            newGameVersions = getRange("1.11", "1.12.2");
            break;
          case 4:
            newGameVersions = getRange("1.13", "1.14.4");
            break;
          case 5:
            newGameVersions = getRange("1.15", "1.16.1");
            break;
          case 6:
            newGameVersions = getRange("1.16.2", "1.16.5");
            break;
          case 7:
            newGameVersions = getRange("1.17", "1.17.1");
            break;
          case 8:
            newGameVersions = getRange("1.18", "1.18.2");
            break;
          case 9:
            newGameVersions = getRange("1.19", "1.19.2");
            break;
          case 11:
            newGameVersions = getRange("22w42a", "22w44a");
            break;
          case 12:
            newGameVersions.push("1.19.3");
            break;
          case 13:
            newGameVersions.push("1.19.4");
            break;
          default:
        }
      }
      return {
        loaders,
        game_versions: newGameVersions,
      };
    },
    "language.json": (file) => {
      const metadata = JSON.parse(file);
      const loaders = ["language"];
      const result = {
        name: metadata.name
          ? `${metadata.name} ${metadata.version || ""}`
          : `Language Pack ${metadata.version || "1.0.0"}`,
        version_number: metadata.version || "1.0.0",
        version_type: versionType(metadata.version || "1.0.0"),
        loaders,
      };

      // 如果有目标版本信息，添加到结果中
      if (metadata.targetVersion) {
        result.targetVersion = metadata.targetVersion;
        result.languageCode = metadata.languageCode || "zh_CN";
        result.linkDescription = metadata.linkDescription || "";
      }

      return result;
    },
  };

  // BBSMC 地图板块: 单文件二进制（litematic / schem / nbt / mcstructure）不是 zip 容器，
  // 直接按后缀返回，跳过 JSZip 解析（否则 loadAsync 会抛错中断整个推断）
  const rawName = (rawFile.name || "").toLowerCase();
  const SINGLE_FILE_MAP_EXTS = [".litematic", ".schem", ".schematic", ".nbt", ".mcstructure"];
  if (SINGLE_FILE_MAP_EXTS.some((ext) => rawName.endsWith(ext))) {
    return {
      loaders: ["map"],
      project_type: "map",
    };
  }

  const zipReader = new JSZip();

  let zip;
  try {
    zip = await zipReader.loadAsync(rawFile);
  } catch (err) {
    // BBSMC 兜底: 当前项目就是地图，zip 损坏 / 实为 7z 或 rar 改后缀 / 加密压缩
    // 仍把它当成地图，让用户能继续走完版本创建流程（后端无校验，文件直传 CDN）
    if (project.actualProjectType === "map") {
      return { loaders: ["map"], project_type: "map" };
    }
    throw err;
  }

  for (const fileName in inferFunctions) {
    const file = zip.file(fileName);

    if (file !== null) {
      const text = await file.async("text");
      return inferFunctions[fileName](text, zip);
    }
  }

  // BBSMC: 外层 zip 打包了 .mrpack 整合包（根目录存在 .mrpack 文件）。
  // .mrpack 本身就是 zip，递归解开读内层 modrinth.index.json，复用现有解析推断
  // loaders / 游戏版本 / 版本号；内层损坏时仍按整合包标识返回，加载器让用户自选
  const mrpackEntry = Object.keys(zip.files).find(
    (f) => !zip.files[f].dir && /^[^/]+\.mrpack(-primary)?$/i.test(f),
  );
  if (mrpackEntry) {
    try {
      const innerZip = await JSZip.loadAsync(await zip.file(mrpackEntry).async("blob"));
      const innerIndex = innerZip.file("modrinth.index.json");
      if (innerIndex) {
        return inferFunctions["modrinth.index.json"](await innerIndex.async("text"), zip);
      }
    } catch (err) {
      console.warn("Failed to parse nested .mrpack:", err);
    }
    return { is_modpack: true, project_type: "modpack" };
  }

  // BBSMC 地图板块: zip 容器内的地图特征
  // - 含 level.dat → 完整地图存档
  // - 含 .schem/.litematic/.nbt/.mcstructure → 建筑/结构模板压缩包
  const allEntries = Object.keys(zip.files);
  const hasLevelDat = allEntries.some((f) => f === "level.dat" || f.endsWith("/level.dat"));
  if (hasLevelDat) {
    return { loaders: ["map"], project_type: "map" };
  }
  const hasSchemEntries = allEntries.some((f) =>
    /\.(schem|schematic|litematic|nbt|mcstructure)$/i.test(f),
  );
  if (hasSchemEntries) {
    return { loaders: ["map"], project_type: "map" };
  }

  // BBSMC 兜底: zip 解析成功但内部无任何已知特征文件
  // 旧版地图（仅 region/、playerdata/ 等而无 level.dat）/ 文件夹结构散乱的存档
  // 当前项目是地图 → 仍按地图处理
  if (project.actualProjectType === "map") {
    return { loaders: ["map"], project_type: "map" };
  }
};
