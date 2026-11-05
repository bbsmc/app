export interface FeaturedItem {
  id: number;
  title: string;
  description: string;
  badge: string;
  image_url: string;
  link: string;
  project_id: string | null;
}

export type FeaturedSections = Record<string, FeaturedItem[]>;

interface FeaturedPayload {
  fetchedAt: number;
  sections: FeaturedSections;
}

export const FEATURED_DATA_KEY = "featured";

const FEATURED_DATA_TTL = 5 * 60 * 1000;

export const FEATURED_SECTION_HOME = "home";

export const featuredSearchSection = (projectTypeId: string) => `search_${projectTypeId}`;

export function useFeatured() {
  return useAsyncData<FeaturedPayload>(
    FEATURED_DATA_KEY,
    async () => {
      const sections = (await useBaseFetch(
        "featured",
        { apiVersion: 3 },
        true,
      )) as FeaturedSections | null;
      return { fetchedAt: Date.now(), sections: sections ?? {} };
    },
    {
      default: () => ({ fetchedAt: 0, sections: {} }),
      getCachedData(key, nuxtApp) {
        const cached = nuxtApp.payload.data[key] as FeaturedPayload | undefined;
        if (!cached) return undefined;
        if (nuxtApp.isHydrating) {
          cached.fetchedAt = Date.now();
          return cached;
        }
        return Date.now() - (cached.fetchedAt ?? 0) < FEATURED_DATA_TTL ? cached : undefined;
      },
    },
  );
}

export function clearFeaturedCache() {
  clearNuxtData(FEATURED_DATA_KEY);
}

export function openFeaturedLink(link: string) {
  if (!link) return;
  if (/^https?:\/\//i.test(link)) {
    window.open(link, "_blank", "noopener");
    return;
  }
  navigateTo(link);
}
