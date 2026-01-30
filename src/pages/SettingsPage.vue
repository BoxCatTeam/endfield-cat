<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke } from '@tauri-apps/api/core'
import { Snackbar } from '@varlet/ui'
import { useI18n } from 'vue-i18n'
import logo from '../assets/icon.webp'
import { useAppStore } from '../stores/app'
import type { MetadataSourceType } from '../stores/app'

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

  void testAllConnections()
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

type ConnectivityState = {
  status: 'idle' | 'testing' | 'success' | 'failed'
  latency: number
  error: string
}

const connectivity = ref<Record<MetadataSourceType, ConnectivityState>>({
  cdn: { status: 'idle', latency: 0, error: '' },
  mirror: { status: 'idle', latency: 0, error: '' },
  custom: { status: 'idle', latency: 0, error: '' },
})

const anyTesting = computed(() => {
  return Object.values(connectivity.value).some((s) => s.status === 'testing')
})

const getBaseUrlFor = (source: MetadataSourceType) => {
  return appStore.getMetadataBaseUrlFor(source, metadataCustomBase.value)
}

const testSourceConnection = async (source: MetadataSourceType) => {
  const baseUrl = getBaseUrlFor(source)
  if (!baseUrl) {
    connectivity.value[source] = { status: 'idle', latency: 0, error: '' }
    return
  }

  connectivity.value[source] = { status: 'testing', latency: 0, error: '' }
  const start = performance.now()
  try {
    await invoke('fetch_metadata_manifest', {
      baseUrl,
      version: metadataVersion.value
    })
    connectivity.value[source] = {
      status: 'success',
      latency: Math.round(performance.now() - start),
      error: ''
    }
  } catch (e: any) {
    console.error(e)
    connectivity.value[source] = {
      status: 'failed',
      latency: 0,
      error: typeof e === 'string' ? e : t('guide.connectionFailed')
    }
  }
}

const testAllConnections = async () => {
  for (const s of ['cdn', 'mirror', 'custom'] as const) {
    // eslint-disable-next-line no-await-in-loop
    await testSourceConnection(s)
  }
}

const selectSource = async (source: MetadataSourceType) => {
  metadataSourceType.value = source
  await testSourceConnection(source)
}

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

watch(metadataVersion, () => {
  connectivity.value = {
    cdn: { status: 'idle', latency: 0, error: '' },
    mirror: { status: 'idle', latency: 0, error: '' },
    custom: { status: 'idle', latency: 0, error: '' },
  }
})

watch(metadataCustomBase, () => {
  connectivity.value.custom = { status: 'idle', latency: 0, error: '' }
})

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
                  <var-button text type="primary" size="small" :disabled="anyTesting" @click="testAllConnections">{{ t('common.retry') }}</var-button>
                </template>
              </var-cell>

              <var-cell
                v-for="src in (['cdn', 'mirror', 'custom'] as const)"
                :key="src"
                ripple
                @click="selectSource(src)"
              >
                <template #icon>
                  <var-icon name="server" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">
                    <span v-if="src === 'cdn'">{{ t('settings.metadata.sourceCdn') }}</span>
                    <span v-else-if="src === 'mirror'">{{ t('settings.metadata.sourceMirror') }}</span>
                    <span v-else>{{ t('settings.metadata.sourceCustom') }}</span>
                  </div>
                </template>
                <template #description>
                  <div v-if="src === 'custom'" class="metadata-url">{{ getBaseUrlFor(src) || t('settings.metadata.customPlaceholder') }}</div>
                  <div class="metadata-conn">
                    <span class="metadata-conn-label">{{ t('guide.connectivity') }}</span>
                    <span v-if="connectivity[src].status === 'testing'" class="metadata-conn-testing">
                      <var-loading type="cube" size="small" :radius="2" class="metadata-conn-loading" /> {{ t('guide.testing') }}
                    </span>
                    <span v-else-if="connectivity[src].status === 'success'" class="metadata-conn-success">
                      <var-icon name="check-circle-outline" size="14" /> {{ connectivity[src].latency }}ms
                    </span>
                    <span v-else-if="connectivity[src].status === 'failed'" class="metadata-conn-failed">
                      <var-icon name="close-circle-outline" size="14" /> {{ connectivity[src].error }}
                    </span>
                    <span v-else class="metadata-conn-idle">--</span>
                  </div>
                </template>
                <template #extra>
                  <var-icon v-if="metadataSourceType === src" name="check" />
                </template>
              </var-cell>
            </var-paper>

            <var-paper v-if="metadataSourceType === 'custom'" :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="link-variant" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.metadata.sourceCustom') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.metadata.customDesc') }}</div>
                </template>
                <template #extra>
                  <var-input
                    v-model="metadataCustomBase"
                    size="small"
                    variant="outlined"
                    class="metadata-input"
                    :placeholder="t('settings.metadata.customPlaceholder')"
                  />
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

.metadata-conn {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
  flex-wrap: wrap;
}

.metadata-conn-label {
  opacity: 0.75;
}

.metadata-conn-idle {
  opacity: 0.6;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
}

.metadata-conn-loading {
  display: inline-block;
  margin-right: 6px;
  vertical-align: middle;
}

.metadata-conn-success {
  color: var(--color-success);
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
}

.metadata-conn-failed {
  color: var(--color-danger);
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metadata-conn-testing {
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
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
