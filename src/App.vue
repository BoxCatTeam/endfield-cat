<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { StyleProvider, Themes, Snackbar } from "@varlet/ui";
import { nekoTheme } from "./theme";
import TitleBar from "./components/TitleBar.vue";
import { useAppStore } from "./stores/app";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

const appStore = useAppStore();
const { locale, t } = useI18n();
const router = useRouter();

// System theme detection
const systemDarkMode = ref(false);

// Update state
type LatestRelease = {
  tag_name: string;
  name?: string;
  html_url?: string;
  download_url?: string;
  body?: string;
};

const updateInfo = ref<LatestRelease | null>(null);
const showUpdateDialog = ref(false);
const isUpdating = ref(false);

function syncModeFromSystem() {
  systemDarkMode.value = window.matchMedia?.("(prefers-color-scheme: dark)")?.matches;
}

// Compare versions (returns true if remote > local)
function isNewerVersion(local: string, remote: string): boolean {
  const parseVersion = (v: string) => v.replace(/^v/, '').split('.').map(Number);
  const localParts = parseVersion(local);
  const remoteParts = parseVersion(remote);
  
  for (let i = 0; i < Math.max(localParts.length, remoteParts.length); i++) {
    const l = localParts[i] || 0;
    const r = remoteParts[i] || 0;
    if (r > l) return true;
    if (r < l) return false;
  }
  return false;
}

async function checkForUpdate() {
  try {
    const [localVersion, release] = await Promise.all([
      invoke<string>('get_app_version'),
      invoke<LatestRelease>('fetch_latest_release')
    ]);
    
    if (release && isNewerVersion(localVersion, release.tag_name)) {
      updateInfo.value = release;
      showUpdateDialog.value = true;
    }
  } catch (error) {
    console.error("Failed to check for updates:", error);
  }
}

async function handleInstallNow() {
  if (!updateInfo.value?.download_url) {
    Snackbar.error(t('settings.update.installFailed'));
    return;
  }
  
  isUpdating.value = true;
  try {
    await invoke('download_and_apply_update', {
      downloadUrl: updateInfo.value.download_url
    });
  } catch (error) {
    console.error("Update failed:", error);
    Snackbar.error(t('settings.update.installFailed'));
    isUpdating.value = false;
  }
}

async function handleManualDownload() {
  if (updateInfo.value?.html_url) {
    await openUrl(updateInfo.value.html_url);
  }
  showUpdateDialog.value = false;
}

function handleLater() {
  showUpdateDialog.value = false;
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
    } else {
      // Check for updates after metadata check
      checkForUpdate();
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
    
    <!-- Update Dialog -->
    <var-dialog
      v-model:show="showUpdateDialog"
      :title="t('settings.update.available')"
      :cancel-button="false"
      :confirm-button="false"
    >
      <div class="update-dialog-content">
        <p class="update-version">{{ updateInfo?.tag_name }}</p>
        <p class="update-name" v-if="updateInfo?.name">{{ updateInfo.name }}</p>
        <div class="update-body" v-if="updateInfo?.body">
          <pre>{{ updateInfo.body }}</pre>
        </div>
        
        <div v-if="isUpdating" class="update-progress">
          <var-loading type="wave" />
          <p>{{ t('settings.update.downloading') }}</p>
        </div>
      </div>
      
      <template #actions>
        <var-space justify="flex-end">
          <var-button text @click="handleLater" :disabled="isUpdating">
            {{ t('settings.update.later') }}
          </var-button>
          <var-button text type="primary" @click="handleManualDownload" :disabled="isUpdating">
            {{ t('settings.update.manualDownload') }}
          </var-button>
          <var-button
            type="primary"
            @click="handleInstallNow"
            :loading="isUpdating"
            :disabled="!updateInfo?.download_url"
          >
            {{ t('settings.update.installNow') }}
          </var-button>
        </var-space>
      </template>
    </var-dialog>
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

.update-dialog-content {
  max-height: 300px;
  overflow-y: auto;
}

.update-version {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: var(--color-primary);
}

.update-name {
  font-size: 14px;
  margin: 0 0 16px 0;
  opacity: 0.8;
}

.update-body {
  background: var(--color-surface-container-low);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 16px;
}

.update-body pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  line-height: 1.5;
  max-height: 150px;
  overflow-y: auto;
}

.update-progress {
  text-align: center;
  padding: 16px 0;
}

.update-progress p {
  margin-top: 12px;
  font-size: 14px;
}

</style>

/* Override Varlet Option selected style to match Sidebar active state */
.var-option--selected {
  background-color: var(--color-primary-container) !important;
  color: var(--color-on-primary-container) !important;
}
