import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { testGithubMirror } from "../api/tauriCommands";
import { useAppStore, type GithubMirrorSourceType } from "../stores/app";

type GithubMirrorConnectivityState = {
  status: "idle" | "testing" | "success" | "failed";
  latency: number;
  error: string;
};

type UseGithubMirrorOptions = {
  canTest?: () => boolean;
};

const createIdleState = (): GithubMirrorConnectivityState => ({
  status: "idle",
  latency: 0,
  error: "",
});

export function useGithubMirror(options: UseGithubMirrorOptions = {}) {
  const { t } = useI18n();
  const appStore = useAppStore();
  const canTest = options.canTest ?? (() => true);

  const githubMirrorEnabled = computed({
    get: () => appStore.githubMirrorEnabled,
    set: (val) => appStore.githubMirrorEnabled = val,
  });

  const githubMirrorSource = computed({
    get: () => appStore.githubMirrorSource,
    set: (val) => appStore.githubMirrorSource = val,
  });

  const githubMirrorCustomTemplate = computed({
    get: () => appStore.githubMirrorCustomTemplate,
    set: (val) => appStore.githubMirrorCustomTemplate = val,
  });

  const githubMirrorSourceOptions = computed(() => [
    { label: t("settings.githubMirror.sources.gh-proxy-cf"), value: "gh-proxy-cf" as const },
    { label: t("settings.githubMirror.sources.gh-proxy-fastly"), value: "gh-proxy-fastly" as const },
    { label: t("settings.githubMirror.sources.gh-proxy-edgeone"), value: "gh-proxy-edgeone" as const },
    { label: t("settings.githubMirror.sources.ghfast"), value: "ghfast" as const },
    { label: t("settings.githubMirror.sources.custom"), value: "custom" as const },
  ]);

  const githubMirrorConnectivity = ref<GithubMirrorConnectivityState>(createIdleState());

  const currentGithubMirrorLabel = computed(() => {
    return githubMirrorSourceOptions.value.find((option) => option.value === githubMirrorSource.value)?.label ?? githubMirrorSource.value;
  });

  const resetGithubMirrorConnectivity = () => {
    githubMirrorConnectivity.value = createIdleState();
  };

  const testGithubMirrorConnection = async () => {
    if (!canTest() || !githubMirrorEnabled.value) {
      resetGithubMirrorConnectivity();
      return;
    }

    const template = appStore.getGithubMirrorTemplate();
    if (!template || template === "{url}") {
      resetGithubMirrorConnectivity();
      return;
    }

    githubMirrorConnectivity.value = { status: "testing", latency: 0, error: "" };

    try {
      const latency = await testGithubMirror(template);
      githubMirrorConnectivity.value = { status: "success", latency, error: "" };
    } catch (error: any) {
      console.error("GitHub mirror test failed:", error);
      githubMirrorConnectivity.value = {
        status: "failed",
        latency: 0,
        error: typeof error === "string" ? error : t("guide.connectionFailed"),
      };
    }
  };

  const selectGithubMirrorSource = async (source: GithubMirrorSourceType) => {
    githubMirrorSource.value = source;
    await testGithubMirrorConnection();
  };

  watch(githubMirrorEnabled, (enabled) => {
    if (enabled) {
      void testGithubMirrorConnection();
    } else {
      resetGithubMirrorConnectivity();
    }
  });

  watch(githubMirrorCustomTemplate, () => {
    if (githubMirrorSource.value === "custom") {
      resetGithubMirrorConnectivity();
    }
  });

  return {
    currentGithubMirrorLabel,
    githubMirrorConnectivity,
    githubMirrorCustomTemplate,
    githubMirrorEnabled,
    githubMirrorSource,
    githubMirrorSourceOptions,
    resetGithubMirrorConnectivity,
    selectGithubMirrorSource,
    testGithubMirrorConnection,
  };
}
