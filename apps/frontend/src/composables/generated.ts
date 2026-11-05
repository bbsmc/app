import { useTags } from "./tag.js";

/**
 * 上游 stages/components 用 useGeneratedState() 读取 categories/loaders/gameVersions 等元数据。
 * 本地架构使用 useTags() 从 ~/generated/state.json 读取。
 *
 * 此 wrapper 让上游代码可直接 import { useGeneratedState } from '~/composables/generated'，
 * 不必在 stage UI 中改写为 useTags。
 *
 * @see ~/composables/tag.ts
 */
export const useGeneratedState = useTags;
