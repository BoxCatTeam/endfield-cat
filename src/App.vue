<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { StyleProvider, Themes } from "@varlet/ui";
import { nekoTheme } from "./theme";
import TitleBar from "./components/TitleBar.vue";
import { useAppStore } from "./stores/app";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

const appStore = useAppStore();
const { locale } = useI18n();
const router = useRouter();

// System theme detection
const systemDarkMode = ref(false);

function syncModeFromSystem() {
  systemDarkMode.value = window.matchMedia?.("(prefers-color-scheme: dark)")?.matches;
}

onMounted(async () => {
  syncModeFromSystem();
  window.matchMedia?.("(prefers-color-scheme: dark)")?.addEventListener?.("change", syncModeFromSystem);
  await appStore.loadConfig();
  try {
    const status = await appStore.checkMetadata();
    const missingMetadata = status && !status.hasManifest;
    
    // Redirect to guide if first run or metadata missing
    if (appStore.firstRun || missingMetadata) {
      router.push('/guide');
    }
  } catch (error) {
    console.error("Failed to check metadata status:", error);
  }
});

onBeforeUnmount(() => {
  window.matchMedia?.("(prefers-color-scheme: dark)")?.removeEventListener?.("change", syncModeFromSystem);
});

// Calculate effective mode (light or dark)
const effectiveMode = computed(() => {
  if (appStore.theme === 'system') {
    return systemDarkMode.value ? 'dark' : 'light';
  }
  return appStore.theme;
});

const currentTheme = computed(() => {
  const mode = effectiveMode.value;
  const base = mode === "dark" ? Themes.md3Dark : Themes.md3Light;
  return { ...base, ...nekoTheme[mode] };
});

// Watch effective mode to apply theme
watch(currentTheme, (v) => {
  StyleProvider(v);
}, { immediate: true });

// Watch language changes
watch(() => appStore.language, (ver) => {
  locale.value = ver;
}, { immediate: true });
</script>

<template>
  <div class="app">
    <TitleBar />
    <div class="app-body">
      <router-view />
    </div>
  </div>
</template>

<style>
@font-face {
  font-family: 'Noto Sans';
  src: url('/NotoSans.ttf') format('truetype');
  font-weight: 100 900;
  font-style: normal;
  font-display: swap;
}

:root {
  font-family: 'Noto Sans', Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

html,
body,
#app {
  height: 100%;
}

.var-style-provider {
  height: 100%;
}

body {
  margin: 0;
  color: var(--color-text, #0f0f0f);
  background: var(--color-body, #f6f6f6);
  overflow: hidden;
}

.app {
  height: 100%;
  color: var(--color-text, #0f0f0f);
  background: var(--color-body, #f6f6f6);
  --titlebar-height: 44px;
}

.app-body {
  height: 100%;
  padding-top: var(--titlebar-height);
  box-sizing: border-box;
}

</style>

/* Override Varlet Option selected style to match Sidebar active state */
.var-option--selected {
  background-color: var(--color-primary-container) !important;
  color: var(--color-on-primary-container) !important;
}
