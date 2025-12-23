import type { ComputedRef, Ref } from 'vue'
import { createContext } from './index'

export interface ProjectPageContext {
  projectV2: ComputedRef<{
    id: string
    project_type: string
    slug?: string
    [key: string]: unknown
  }>
  refreshVersions: () => Promise<void>
}

export const [injectProjectPageContext, provideProjectPageContext] =
  createContext<ProjectPageContext>('ProjectPage')
