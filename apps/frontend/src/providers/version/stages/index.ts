import { stageConfig as addChangelogStageConfig } from "./add-changelog.ts";
import { stageConfig as addDependenciesStageConfig } from "./add-dependencies.ts";
import { stageConfig as addDetailsStageConfig } from "./add-details.ts";
import {
  fromDetailsStageConfig as fromDetailsEnvironmentStageConfig,
  stageConfig as addEnvironmentStageConfig,
} from "./add-environment.ts";
import { stageConfig as addFilesStageConfig } from "./add-files.ts";
import {
  fromDetailsStageConfig as fromDetailsLoadersStageConfig,
  stageConfig as addLoadersStageConfig,
} from "./add-loaders.ts";
import {
  fromDetailsStageConfig as fromDetailsMcVersionsStageConfig,
  stageConfig as addMcVersionsStageConfig,
} from "./add-mc-versions.ts";

export const stageConfigs = [
  addFilesStageConfig,
  addDetailsStageConfig,
  addLoadersStageConfig,
  addMcVersionsStageConfig,
  addEnvironmentStageConfig,
  addDependenciesStageConfig,
  addChangelogStageConfig,
  // Non-progress stages for editing from details page
  fromDetailsLoadersStageConfig,
  fromDetailsMcVersionsStageConfig,
  fromDetailsEnvironmentStageConfig,
];
