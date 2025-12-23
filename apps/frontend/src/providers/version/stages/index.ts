import { stageConfig as addChangelogStageConfig } from "./add-changelog";
import { stageConfig as addDependenciesStageConfig } from "./add-dependencies";
import { stageConfig as addDetailsStageConfig } from "./add-details";
import {
  fromDetailsStageConfig as fromDetailsEnvironmentStageConfig,
  stageConfig as addEnvironmentStageConfig,
} from "./add-environment";
import { stageConfig as addFilesStageConfig } from "./add-files";
import {
  fromDetailsStageConfig as fromDetailsLoadersStageConfig,
  stageConfig as addLoadersStageConfig,
} from "./add-loaders";
import {
  fromDetailsStageConfig as fromDetailsMcVersionsStageConfig,
  stageConfig as addMcVersionsStageConfig,
} from "./add-mc-versions";

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
