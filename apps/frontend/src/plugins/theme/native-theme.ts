export type SystemTheme = "unknown" | "light" | "dark";

function useNativeThemeServer() {
  let clientHint;

  switch (useRequestHeader("Sec-CH-Prefers-Color-Scheme")) {
    case "light":
      clientHint = "dark";
      break;
    case "dark":
      clientHint = "dark";
      break;
    default:
      clientHint = "unknown";
  }

  return computed(() => clientHint as SystemTheme);
}

// 默认主题设置位置
function useNativeThemeClient() {
  const lightPreference = window.matchMedia("(prefers-color-scheme: dark)");

  const isLight = ref(lightPreference.matches);

  const onPreferenceChange = ({ matches }: MediaQueryListEvent) => (isLight.value = matches);

  lightPreference.addEventListener("change", onPreferenceChange);

  onScopeDispose(() => lightPreference.removeEventListener("change", onPreferenceChange));

  return computed<SystemTheme>(() => "dark");
  // return computed<SystemTheme>(() => (isLight.value ? "light" : "dark"));
}

export function useNativeTheme() {
  if (import.meta.server) return useNativeThemeServer();
  if (import.meta.client) return useNativeThemeClient();
  throw new Error("Cannot determine the side");
}
