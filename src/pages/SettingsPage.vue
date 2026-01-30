<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke } from '@tauri-apps/api/core'
import { Snackbar } from '@varlet/ui'
import { useI18n } from 'vue-i18n'
import logo from '../assets/icon.webp'
import { useAppStore } from '../stores/app'

const { t, tm } = useI18n()
const disclaimerItems = computed(() => tm('common.disclaimer.items') as string[])
const appStore = useAppStore()

// Mock Data
const appVersion = ref('')
const latestVersion = ref('')
const latestReleaseUrl = ref('')
const checkingUpdate = ref(false)

onMounted(async () => {
  try {
    appVersion.value = await invoke<string>('get_app_version')
  } catch (error) {
    console.error('Failed to get version:', error)
    appVersion.value = 'Unknown'
  }
})

// Bind to store
const theme = computed({
  get: () => appStore.theme,
  set: (val) => appStore.theme = val
})

const background = computed({
  get: () => appStore.background,
  set: (val) => appStore.background = val
})

const language = computed({
  get: () => appStore.language,
  set: (val) => appStore.language = val
})

const metadataSourceType = computed({
  get: () => appStore.metadataSourceType,
  set: (val) => appStore.metadataSourceType = val
})

const metadataVersion = computed({
  get: () => appStore.metadataVersion,
  set: (val) => appStore.metadataVersion = val
})

const metadataCustomBase = computed({
  get: () => appStore.metadataCustomBase,
  set: (val) => appStore.metadataCustomBase = val
})

const metadataBaseUrl = computed(() => appStore.metadataBaseUrl)

const gamePath = ref('')

// Mock Functions
const normalizeVersion = (v: string) => v.replace(/^v/i, '').trim()

const compareVersion = (a: string, b: string) => {
  const pa = normalizeVersion(a).split(/[.-]/).map((n) => parseInt(n, 10) || 0)
  const pb = normalizeVersion(b).split(/[.-]/).map((n) => parseInt(n, 10) || 0)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const diff = (pa[i] || 0) - (pb[i] || 0)
    if (diff !== 0) return Math.sign(diff)
  }
  return 0
}

const checkUpdate = async () => {
  checkingUpdate.value = true
  try {
    const release = await invoke<{ tag_name?: string; name?: string; html_url?: string }>('fetch_latest_release')
    const tag = release.tag_name || release.name || ''
    const latest = normalizeVersion(tag)
    latestVersion.value = latest
    latestReleaseUrl.value = release.html_url || ''

    if (!latest || !appVersion.value) {
      Snackbar.info(t('settings.update.latestUnknown'))
      return
    }

    const cmp = compareVersion(latest, appVersion.value)
    if (cmp > 0) {
      Snackbar.success(t('settings.update.found', { version: latest }))
    } else {
      Snackbar.success(t('settings.update.upToDate'))
    }
  } catch (error) {
    console.error('Failed to check update:', error)
    Snackbar.error(`${t('settings.update.failed')}: ${error}`)
  } finally {
    checkingUpdate.value = false
  }
}

const openLatestRelease = async () => {
  if (!latestReleaseUrl.value) return
  try {
    await openUrl(latestReleaseUrl.value)
  } catch (error) {
    Snackbar.error(t('settings.update.failed'))
  }
}

const openDataDir = () => {
  Snackbar.info(t('settings.userData'))
}

const links: Record<string, string> = {
  'settings.buttons.github': 'https://github.com/BoxCatTeam/endfield-cat',
  'settings.buttons.website': 'https://boxcat.org',
  'settings.buttons.feedback': 'https://github.com/BoxCatTeam/endfield-cat/issues',
}

const openLink = async (nameKey: string) => {
  const url = links[nameKey]
  if (url) {
    try {
      await openUrl(url)
    } catch (e) {
      console.error('Failed to open url:', e)
      Snackbar.error(t('settings.messages.openLinkFailed'))
    }
  } else {
    Snackbar.info(t('common.clickTip', { label: t(nameKey) }))
  }
}

const themeOptions = computed(() => [
  { label: t('settings.themeSystem'), value: 'system' },
  { label: t('settings.themeLight'), value: 'light' },
  { label: t('settings.themeDark'), value: 'dark' },
])

const bgOptions = computed(() => [
  { label: t('settings.bgDefault'), value: 'default' },
  { label: t('settings.bgMinimal'), value: 'minimal' },
])

const langOptions = computed(() => [
  { label: t('settings.langZh'), value: 'zh-CN' },
  { label: t('settings.langEn'), value: 'en-US' },
])

const metadataSourceOptions = computed(() => [
  { label: t('settings.metadata.sourceCdn'), value: 'cdn' },
  { label: t('settings.metadata.sourceMirror'), value: 'mirror' },
  { label: t('settings.metadata.sourceCustom'), value: 'custom' },
])

const resetMetadataLoading = ref(false)

const resetMetadata = async () => {
  resetMetadataLoading.value = true
  try {
    await invoke('reset_metadata', {
      baseUrl: metadataBaseUrl.value,
      version: metadataVersion.value
    })
    Snackbar.success(t('settings.metadata.resetSuccess'))
  } catch (error) {
    console.error('Failed to reset metadata:', error)
    Snackbar.error(t('settings.metadata.resetFailed'))
  } finally {
    resetMetadataLoading.value = false
  }
}
const notAvailable = () => {
  Snackbar.info(t('settings.messages.devPlaceholder'))
}
</script>

<template>
  <div class="page-container">
    <div class="content-wrapper">
      <var-space direction="column" align="center" class="settings-header" :size="4">
        <var-image :src="logo" width="128"></var-image>
        <div class="logo-container">
          <span class="app-name-prefix">End</span><span class="app-name-suffix">Cat</span>
        </div>
        <div class="copyright">
          Copyright © 2026 <span class="link" @click="openUrl('https://boxcat.org')">BoxCat.</span> under <span class="link" @click="openUrl('https://opensource.org/licenses/GPL-2.0')">GPLv2 License</span>
        </div>
        <var-space class="header-buttons" justify="center" :size="8">
          <var-button type="default" size="small" variant="text" :elevation="false" @click="openLink('settings.buttons.github')">{{ t('settings.buttons.github') }}</var-button>
          <var-button type="default" size="small" variant="text" :elevation="false" @click="openLink('settings.buttons.website')">{{ t('settings.buttons.website') }}</var-button>
          <var-button type="default" size="small" variant="text" :elevation="false" @click="openLink('settings.buttons.feedback')">{{ t('settings.buttons.feedback') }}</var-button>
          <var-button type="default" size="small" variant="text" :elevation="false" @click="openLink('settings.buttons.privacy')">{{ t('settings.buttons.privacy') }}</var-button>
          <var-button type="default" size="small" variant="text" :elevation="false" @click="openLink('settings.buttons.sponsor')">{{ t('settings.buttons.sponsor') }}</var-button>
          <var-button text round :elevation="false" @click="openLink('EasterEgg')">
               <var-icon name="cat" />
          </var-button>
        </var-space>
      </var-space>

      <var-space direction="column" size="large">
        
        <!-- About Section -->
        <section>
          <div class="section-title">{{ t('settings.about') }}</div>
          <var-paper :elevation="false" radius="12">
            <var-cell>
              <template #icon>
                <var-icon name="gift" size="24px" class="section-icon" />
              </template>
                <template #default>
                  <div class="cell-title">{{ t('common.appName') }}</div>
                </template>
                <template #description>
                <div class="cell-desc">{{ t('settings.update.current') }}: {{ appVersion }}</div>
                <div v-if="latestVersion" class="cell-desc">{{ t('settings.update.latest') }}: {{ latestVersion }}</div>
                </template>
                <template #extra>
                <var-space align="center" :wrap="false" :size="4">
                  <var-button type="primary" size="small" :elevation="false" :loading="checkingUpdate" @click="checkUpdate">{{ t('settings.checkUpdate') }}</var-button>
                  <var-button v-if="latestReleaseUrl" type="default" text size="small" :elevation="false" @click="openLatestRelease">{{ t('settings.update.view') }}</var-button>
                </var-space>
                </template>
              </var-cell>
            </var-paper>
            <var-paper :elevation="false" radius="12" class="disclaimer-paper">
              <div class="disclaimer-block">
                <div class="disclaimer-title">{{ t('common.disclaimer.title') }}</div>
                <ul class="disclaimer-list">
                  <li v-for="(line, idx) in disclaimerItems" :key="idx">{{ line }}</li>
                </ul>
              </div>
            </var-paper>
          </section>

        <!-- Appearance Section -->
        <section>
          <div class="section-title">{{ t('settings.appearance') }}</div>
          <var-space direction="column" size="small">
            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="palette" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.theme') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.themeDesc') }}</div>
                </template>
                <template #extra>
                  <var-select v-model="theme" size="small" variant="outlined" class="select-list-theme">
                    <var-option v-for="opt in themeOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
                  </var-select>
                </template>
              </var-cell>
            </var-paper>

            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="image" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.background') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.bgDesc') }}</div>
                </template>
                <template #extra>
                   <div @click="notAvailable">
                    <var-select disabled v-model="background" size="small" variant="outlined" class="select-list-bg">
                      <var-option v-for="opt in bgOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
                    </var-select>
                   </div>
                </template>
              </var-cell>
            </var-paper>

            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="translate" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.language') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.langDesc') }}</div>
                </template>
                <template #extra>
                  <var-select v-model="language" size="small" variant="outlined" class="select-list-lang">
                    <var-option v-for="opt in langOptions" :key="opt.value" :label="opt.label" :value="opt.value"/>
                  </var-select>
                </template>
              </var-cell>
            </var-paper>
          </var-space>
        </section>

        <!-- Game Section -->
        <section>
          <div class="section-title">{{ t('settings.game') }}</div>
          <var-paper :elevation="false" radius="12">
            <var-cell ripple>
               <template #icon>
                <var-icon name="xml" size="24px" class="section-icon" />
              </template>
              <template #default>
                <div class="cell-title">{{ t('settings.gamePath') }}</div>
              </template>
               <template #description>
                <div class="cell-desc text-ellipsis">{{ gamePath }}</div>
              </template>
              <template #extra>
                <var-icon name="chevron-right" />
              </template>
            </var-cell>
          </var-paper>
        </section>

        <!-- User Data Section -->
        <section>
          <div class="section-title">{{ t('settings.userData') }}</div>
          <var-paper :elevation="false" radius="12">
            <var-cell ripple @click="openDataDir">
              <template #icon>
                <var-icon name="folder-open" size="24px" class="section-icon" />
              </template>
              <template #default>
                <div class="cell-title">{{ t('settings.openUserData') }}</div>
              </template>
              <template #description>
                 <div class="cell-desc">{{ t('settings.userDataDesc') }}</div>
              </template>
               <template #extra>
                <var-icon name="chevron-right" />
              </template>
            </var-cell>
          </var-paper>
        </section>

        <!-- Metadata Section -->
        <section>
          <div class="section-title">{{ t('settings.metadata.title') }}</div>
          <var-space direction="column" size="small">
            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="database" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.metadata.source') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.metadata.sourceDesc') }}</div>
                </template>
                <template #extra>
                  <var-select v-model="metadataSourceType" size="small" variant="outlined" class="select-list-source">
                    <var-option v-for="opt in metadataSourceOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
                  </var-select>
                </template>
              </var-cell>

              <var-cell>
                <template #icon>
                  <var-icon name="server" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.metadata.basePreview') }}</div>
                </template>
                <template #description>
                   <!-- Show desc only if custom, or maybe generic desc? User said 'display box' otherwise. -->
                   <div v-if="metadataSourceType !== 'custom'" class="metadata-url">{{ metadataBaseUrl }}</div>
                   <div v-else class="cell-desc">{{ t('settings.metadata.customDesc') }}</div>
                </template>
                <template #extra>
                  <var-input
                    v-if="metadataSourceType === 'custom'"
                    v-model="metadataCustomBase"
                    size="small"
                    variant="outlined"
                    class="metadata-input"
                    :placeholder="t('settings.metadata.customPlaceholder')"
                  />
                  <!-- If not custom, we rely on the description slot to show the URL text, extra slot is empty or could duplicate? User said 'becomes input box, otherwise is display box'. Displaying as text in description is cleaner for long URLs than fitting in 'extra'. -->
                </template>
              </var-cell>
            </var-paper>

            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="backup-restore" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.metadata.reset') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.metadata.resetDesc') }}</div>
                </template>
                <template #extra>
                  <var-button
                    type="danger"
                    size="small"
                    :loading="resetMetadataLoading"
                    :elevation="false"
                    @click="resetMetadata"
                  >
                    {{ t('settings.metadata.reset') }}
                  </var-button>
                </template>
              </var-cell>
            </var-paper>
          </var-space>
        </section>

      </var-space>
    </div>
  </div>
</template>

<style scoped>
.page-container {
  height: 100%;
  overflow-y: auto;
  width: 100%;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar-thumb) transparent;
}

.page-container::-webkit-scrollbar {
  width: 8px;
}

.page-container::-webkit-scrollbar-track {
  background: transparent;
}

.page-container::-webkit-scrollbar-thumb {
  background-color: var(--color-scrollbar-thumb);
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.page-container::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-scrollbar-thumb-hover);
}

.content-wrapper {
  padding: 24px;
  max-width: 800px;
  margin: 0 auto;
}

.section-title {
  font-size: 14px;
  color: var(--color-text);
  opacity: 0.6;
  margin-bottom: 8px;
  margin-left: 4px;
}

.section-icon {
  margin-right: 16px;
  color: var(--color-primary);
}

.cell-title {
  font-size: 16px;
  font-weight: 500;
  color: var(--color-text);
}

.cell-desc {
  font-size: 12px;
  color: var(--color-text);
  opacity: 0.6;
  margin-top: 4px;
}

.text-ellipsis {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
}

.select-list-bg {
  width: 80px;
}

.select-list-theme {
  width: 120px;
}
.select-list-lang {
  width: 120px;
}
.settings-header {
  margin-bottom: 32px;
  margin-top: 16px;
}

.logo-container {
  font-size: 48px;
  line-height: 1.2;
  margin-bottom: 8px;
  user-select: none;
}

.app-name-prefix {
  color: var(--color-logo-text);
}

.app-name-suffix {
  color: var(--color-logo-accent); 
}

.copyright {
  font-size: 14px;
  color: var(--color-text);
  opacity: 0.6;
  margin-bottom: 16px;
}

.link {
  cursor: pointer;
  color: var(--color-primary);
  font-weight: 500;
}

.link:hover {
  opacity: 0.8;
}

.metadata-url {
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 12px;
  color: var(--color-text);
  opacity: 0.8;
  margin-top: 4px;
  word-break: break-all;
}

.metadata-input {
  width: 220px;
}

.select-list-source {
  width: 180px;
}

.disclaimer-block {
  padding: 14px 16px;
  font-size: 12px;
  color: var(--color-text);
  opacity: 0.85;
}

.disclaimer-list {
  margin: 0;
  padding-left: 18px;
  display: grid;
  gap: 6px;
}

.disclaimer-title {
  font-weight: 600;
  margin-bottom: 6px;
}

.disclaimer-paper {
  margin-top: 8px;
}


</style>
