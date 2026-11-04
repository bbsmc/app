export const getProjectTypeForUrl = (type, categories) => {
  return getProjectTypeForUrlShorthand(type, categories);
};

export const getProjectTypeForUrlShorthand = (type, categories, overrideTags) => {
  const tags = overrideTags ?? useTags().value;

  if (type === "mod") {
    const isMod = categories.some((category) => {
      return tags.loaderData.modLoaders.includes(category);
    });

    const isPlugin = categories.some((category) => {
      return tags.loaderData.allPluginLoaders.includes(category);
    });

    const isDataPack = categories.some((category) => {
      return tags.loaderData.dataPackLoaders.includes(category);
    });

    if (isDataPack) {
      return "datapack";
    } else if (isPlugin) {
      return "plugin";
    } else if (isMod) {
      return "mod";
    } else {
      return "mod";
    }
  } else {
    return type;
  }
};

export const getProjectLink = (project) => {
  return `/${getProjectTypeForUrl(project.project_type, project.loaders)}/${
    project.slug ? project.slug : project.id
  }`;
};

export const getVersionLink = (project, version) => {
  if (version) {
    return getProjectLink(project) + "/version/" + version.id;
  } else {
    return getProjectLink(project);
  }
};

export const isApproved = (project) => {
  return project && APPROVED_PROJECT_STATUSES.includes(project.status);
};

export const isListed = (project) => {
  return project && LISTED_PROJECT_STATUSES.includes(project.status);
};

export const isUnlisted = (project) => {
  return project && UNLISTED_PROJECT_STATUSES.includes(project.status);
};

export const isPrivate = (project) => {
  return project && PRIVATE_PROJECT_STATUSES.includes(project.status);
};

export const isRejected = (project) => {
  return project && REJECTED_PROJECT_STATUSES.includes(project.status);
};

export const isUnderReview = (project) => {
  return project && UNDER_REVIEW_PROJECT_STATUSES.includes(project.status);
};

export const isDraft = (project) => {
  return project && DRAFT_PROJECT_STATUSES.includes(project.status);
};

export const APPROVED_PROJECT_STATUSES = ["approved", "archived", "unlisted", "private"];
export const LISTED_PROJECT_STATUSES = ["approved", "archived"];
export const UNLISTED_PROJECT_STATUSES = ["unlisted", "withheld"];
export const PRIVATE_PROJECT_STATUSES = ["private", "rejected", "processing"];
export const REJECTED_PROJECT_STATUSES = ["rejected", "withheld"];
export const UNDER_REVIEW_PROJECT_STATUSES = ["processing"];
export const DRAFT_PROJECT_STATUSES = ["draft"];

export function getVersionsToDisplay(project) {
  return formatVersionsForDisplay(project.game_versions.slice());
}

export function formatVersionsForDisplay(gameVersions, overrideTags) {
  return getVersionGroupsForDisplay(gameVersions, overrideTags).map((group) => group.label);
}

// 上游 3487523f7 (#6967): 在按范围分组展示的同时，返回每个显示范围实际包含的版本列表，
// 使版本标签点击时只筛选它自己代表的那些版本（而非该发行支持的全部版本），
// 同时让筛选可以一次性批量应用、避免逐版本触发重渲染导致页面卡死。
export function getVersionGroupsForDisplay(gameVersions, overrideTags) {
  const tags = overrideTags ?? useTags().value;

  const inputVersions = gameVersions.slice();
  const allVersions = tags.gameVersions.slice();

  const allSnapshots = allVersions.filter((version) => version.version_type === "snapshot");
  const allReleases = allVersions.filter((version) => version.version_type === "release");
  const allLegacy = allVersions.filter(
    (version) => version.version_type !== "snapshot" && version.version_type !== "release",
  );

  {
    const indices = allVersions.reduce((map, gameVersion, index) => {
      map[gameVersion.version] = index;
      return map;
    }, {});
    inputVersions.sort((a, b) => indices[a] - indices[b]);
  }

  const releaseVersions = inputVersions.filter((projVer) =>
    allReleases.some((gameVer) => gameVer.version === projVer),
  );

  const latestReleaseVersionDate = Date.parse(
    allReleases.find((version) => version.version === releaseVersions[0])?.date,
  );
  const latestSnapshot = inputVersions.find((projVer) =>
    allSnapshots.some(
      (gameVer) =>
        gameVer.version === projVer &&
        (!latestReleaseVersionDate || latestReleaseVersionDate < Date.parse(gameVer.date)),
    ),
  );

  const allReleasesGrouped = groupVersions(
    allReleases.map((release) => release.version),
    false,
  );
  const projectVersionsGrouped = groupVersions(releaseVersions, true);

  const releaseVersionsAsRanges = projectVersionsGrouped.map(({ major, minor }) => {
    const versions = minor.map((minorVersion) => formatVersion(major, minorVersion));

    if (minor.length === 1) {
      return { label: versions[0], versions };
    }

    if (
      allReleasesGrouped
        .find((x) => x.major === major)
        .minor.every((value, index) => value === minor[index])
    ) {
      return { label: `${major}.x`, versions };
    }

    return {
      label: `${formatVersion(major, minor[0])}–${formatVersion(major, minor[minor.length - 1])}`,
      versions,
    };
  });

  const legacyVersionsAsRanges = groupConsecutiveIndices(
    inputVersions.filter((projVer) => allLegacy.some((gameVer) => gameVer.version === projVer)),
    allLegacy,
  );

  let output = [...legacyVersionsAsRanges];

  // show all snapshots if there's no release versions
  if (releaseVersionsAsRanges.length === 0) {
    const snapshotVersionsAsRanges = groupConsecutiveIndices(
      inputVersions.filter((projVer) =>
        allSnapshots.some((gameVer) => gameVer.version === projVer),
      ),
      allSnapshots,
    );
    output = [...snapshotVersionsAsRanges, ...output];
  } else {
    output = [...releaseVersionsAsRanges, ...output];
  }

  if (latestSnapshot && !output.some((group) => group.label === latestSnapshot)) {
    output = [{ label: latestSnapshot, versions: [latestSnapshot] }, ...output];
  }
  return output;
}

const mcVersionRegex = /^([0-9]+.[0-9]+)(.[0-9]+)?$/;

function groupVersions(versions, consecutive = false) {
  return versions
    .slice()
    .reverse()
    .reduce((ranges, version) => {
      const matchesVersion = version.match(mcVersionRegex);

      if (matchesVersion) {
        const majorVersion = matchesVersion[1];
        const minorVersion = matchesVersion[2];
        const minorNumeric = minorVersion ? parseInt(minorVersion.replace(".", "")) : 0;

        let prevInRange;
        if (
          (prevInRange = ranges.find(
            (x) =>
              x.major === majorVersion &&
              (!consecutive || x.minor[x.minor.length - 1] === minorNumeric - 1),
          ))
        ) {
          prevInRange.minor.push(minorNumeric);
          return ranges;
        }

        return [...ranges, { major: majorVersion, minor: [minorNumeric] }];
      }

      return ranges;
    }, [])
    .reverse();
}

function groupConsecutiveIndices(versions, referenceList) {
  if (!versions || versions.length === 0) {
    return [];
  }

  const referenceMap = new Map();
  referenceList.forEach((item, index) => {
    referenceMap.set(item.version, index);
  });

  const sortedList = versions.slice().sort((a, b) => referenceMap.get(a) - referenceMap.get(b));

  // 上游 3487523f7 (#6967): 返回 { label, versions }，versions 为该连续区间包含的具体版本
  const ranges = [];
  let rangeStartIndex = 0;

  for (let i = 1; i <= sortedList.length; i++) {
    if (
      i === sortedList.length ||
      referenceMap.get(sortedList[i]) !== referenceMap.get(sortedList[i - 1]) + 1
    ) {
      const members = sortedList.slice(rangeStartIndex, i);
      ranges.push({
        label: validateRange(`${members[members.length - 1]}–${members[0]}`),
        versions: members,
      });
      rangeStartIndex = i;
    }
  }

  return ranges;
}

function validateRange(range) {
  switch (range) {
    case "rd-132211–b1.8.1":
      return "All legacy versions";
    case "a1.0.4–b1.8.1":
      return "All alpha and beta versions";
    case "a1.0.4–a1.2.6":
      return "All alpha versions";
    case "b1.0–b1.8.1":
      return "All beta versions";
    case "rd-132211–inf20100618":
      return "All pre-alpha versions";
  }
  const splitRange = range.split("–");
  if (splitRange && splitRange[0] === splitRange[1]) {
    return splitRange[0];
  }
  return range;
}

function formatVersion(major, minor) {
  return minor === 0 ? major : `${major}.${minor}`;
}
