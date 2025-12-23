<template>
  <div class="project-versions">
    <div class="versions-header">
      <div class="versions-title">
        <slot name="header">
          <span class="font-bold">Versions</span>
        </slot>
      </div>
    </div>
    <div class="versions-list">
      <div
        v-for="version in versions"
        :key="version.id"
        class="version-row"
      >
        <div class="version-info">
          <NuxtLink
            v-if="versionLink"
            :to="versionLink(version)"
            class="version-name"
          >
            {{ version.name }}
          </NuxtLink>
          <span v-else class="version-name">{{ version.name }}</span>
          <span class="version-number">{{ version.version_number }}</span>
          <Badge
            :type="version.version_type === 'release' ? 'approved-general' : version.version_type === 'beta' ? 'pending' : 'processing'"
            :color="version.version_type === 'release' ? 'green' : version.version_type === 'beta' ? 'orange' : 'red'"
          />
        </div>
        <div class="version-meta">
          <span v-if="version.downloads !== undefined" class="downloads">
            {{ formatNumber(version.downloads) }} downloads
          </span>
          <span class="date">
            {{ formatDate(version.date_published) }}
          </span>
        </div>
        <div class="version-actions">
          <slot name="actions" :version="version" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NuxtLink } from '#components'
import Badge from './Badge.vue'

interface Version {
  id: string
  name: string
  version_number: string
  version_type?: string
  downloads?: number
  date_published?: string
  files?: any[]
  [key: string]: unknown
}

const props = defineProps<{
  project: { id: string; slug?: string; project_type?: string }
  versions: Version[]
  showFiles?: boolean
  currentMember?: boolean
  loaders?: any[]
  gameVersions?: any[]
  baseId?: string
  versionLink?: (version: Version) => string
  openModal?: () => void
}>()

const formatNumber = (num: number): string => {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M'
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K'
  }
  return num.toString()
}

const formatDate = (dateStr?: string): string => {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  return date.toLocaleDateString()
}
</script>

<style lang="scss" scoped>
.project-versions {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.versions-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.versions-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background-color: var(--color-raised-bg);
  border-radius: var(--radius-md);
  transition: background-color 0.15s ease;

  &:hover {
    background-color: var(--color-button-bg);
  }
}

.version-info {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1;
  min-width: 0;
}

.version-name {
  font-weight: 600;
  color: var(--color-text);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  &:hover {
    color: var(--color-brand);
  }
}

.version-number {
  color: var(--color-text-secondary);
  font-size: 0.875rem;
}

.version-meta {
  display: flex;
  align-items: center;
  gap: 1rem;
  color: var(--color-text-secondary);
  font-size: 0.875rem;
}

.version-actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}
</style>
