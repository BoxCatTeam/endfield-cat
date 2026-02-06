<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { StyleProvider, Themes } from "@varlet/ui";
import { nekoTheme } from "./theme";
import TitleBar from "./components/TitleBar.vue";
import UpdateDialog from "./components/UpdateDialog.vue";
import MetadataUpdateDialog from "./components/MetadataUpdateDialog.vue";
import { useAppStore } from "./stores/app";
import { useUpdaterStore } from "./stores/updater";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

const appStore = useAppStore();
const updaterStore = useUpdaterStore();
const { locale } = useI18n();
const router = useRouter();

// 跟随系统主题检测
const systemDarkMode = ref(false);

function syncModeFromSystem() {
  systemDarkMode.value = window.matchMedia?.("(prefers-color-scheme: dark)")?.matches;
}

onMounted(async () => {
  syncModeFromSystem();
  window.matchMedia?.("(prefers-color-scheme: dark)")?.addEventListener?.("change", syncModeFromSystem);
  await appStore.loadConfig();
  try {
    await appStore.syncAppVersion();
    const status = await appStore.checkMetadata();
    const missingMetadata = status && !status.hasManifest;
    
    // 首次启动或缺少元数据时跳转指引流程
    if (appStore.firstRun || missingMetadata) {
      router.push('/guide');
    } else if (appStore.needsPostUpdateGuide) {
      router.push('/guide/update');
    } else {
      // 元数据正常后静默检查更新
      updaterStore.checkForUpdate(true);
      
      // 检查元数据是否有更新，如有则弹窗
      if (appStore.isMetadataOutdated) {
        appStore.showMetadataUpdateDialog = true;
      }
    }
  } catch (error) {
    console.error("Failed to check metadata status:", error);
  }
});

onBeforeUnmount(() => {
  window.matchMedia?.("(prefers-color-scheme: dark)")?.removeEventListener?.("change", syncModeFromSystem);
});

// 计算最终主题（亮/暗）
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

// 监听主题变化并应用到 Varlet
watch(currentTheme, (v) => {
  StyleProvider(v);
}, { immediate: true });

// 监听语言变化同步 i18n
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
    
    <UpdateDialog />
    <MetadataUpdateDialog />
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

/* 统一 Varlet 选中态与侧边栏高亮 */
.var-option--selected {
  background-color: var(--color-primary-container) !important;
  color: var(--color-on-primary-container) !important;
}
</style>
