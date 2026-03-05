import type { AbstractFeature, AuthConfig, NuxtClientConfig } from "@modrinth/api-client";
import {
  AuthFeature,
  CircuitBreakerFeature,
  NuxtCircuitBreakerStorage,
  NuxtModrinthClient,
  VerboseLoggingFeature,
} from "@modrinth/api-client";

// Re-export from @modrinth/ui for convenience
export { injectModrinthClient, provideModrinthClient } from "@modrinth/ui";

export function createModrinthClient(
  auth: { token: string | undefined },
  config: { apiBaseUrl: string; archonBaseUrl: string; rateLimitKey?: string },
): NuxtModrinthClient {
  const optionalFeatures = [
    import.meta.dev ? (new VerboseLoggingFeature() as AbstractFeature) : undefined,
  ].filter(Boolean) as AbstractFeature[];

  const clientConfig: NuxtClientConfig = {
    labrinthBaseUrl: config.apiBaseUrl,
    archonBaseUrl: config.archonBaseUrl,
    rateLimitKey: config.rateLimitKey,
    features: [
      new AuthFeature({
        token: () => auth.token,
      } as AuthConfig),
      new CircuitBreakerFeature({
        storage: new NuxtCircuitBreakerStorage(),
        maxFailures: 3,
        resetTimeout: 30000,
      }),
      ...optionalFeatures,
    ],
  };

  return new NuxtModrinthClient(clientConfig);
}
