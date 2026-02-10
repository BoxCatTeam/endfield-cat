<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Snackbar } from '@varlet/ui'
import { useI18n } from 'vue-i18n'
import logo from '../assets/icon.webp'
import { useAppStore } from '../stores/app'
import { useUpdaterStore } from '../stores/updater'
import type { MetadataSourceType, GithubMirrorSourceType } from '../stores/app'
import { GITHUB_MIRROR_TEMPLATES } from '../stores/app'
import { fetchMetadataManifest, getAppVersion, getStoragePaths, openDataDir as openDataDirCommand, resetMetadata as resetMetadataCommand, testGithubMirror } from '../api/tauriCommands'
import type { StoragePaths } from '../api/tauriCommands'
import { pickDirectory } from '../api/systemDialog'
import SplitButtonSelect from '../components/SplitButtonSelect.vue'

const { t, tm } = useI18n()
const disclaimerItems = computed(() => tm('common.disclaimer.items') as string[])
const appStore = useAppStore()
const updaterStore = useUpdaterStore()

// 展示所需的本地状态
const appVersion = ref('')
const latestVersion = computed(() => updaterStore.updateInfo?.tag_name ? normalizeVersion(updaterStore.updateInfo.tag_name) : '')
const latestReleaseUrl = computed(() => updaterStore.updateInfo?.html_url || '')
const checkingUpdate = computed(() => updaterStore.isChecking)

onMounted(async () => {
  try {
    appVersion.value = await getAppVersion()
    // 首次进入时静默拉取最新版本信息，保持“当前/最新”展示可用
  } catch (error) {
    console.error('Failed to get version:', error)
    appVersion.value = 'Unknown'
  }

  void testAllConnections()
  void refreshStoragePaths()
})

// 与 store 双向绑定
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
    const version = metadataVersion.value.trim() || 'latest'
    await fetchMetadataManifest({
      baseUrl,
      version
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

const normalizeVersion = (v: string) => v.replace(/^v/i, '').trim()

const checkUpdate = async () => {
   await updaterStore.checkForUpdate(false)
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
  void (async () => {
    try {
      await openDataDirCommand()
    } catch (error) {
      console.error('Failed to open data dir:', error)
      Snackbar.error(t('settings.messages.openDirFailed'))
    }
  })()
}

const storagePaths = ref<StoragePaths | null>(null)
const resolvedDataDir = computed(() => storagePaths.value?.dataDir || '')

const refreshStoragePaths = async () => {
  try {
    storagePaths.value = await getStoragePaths()
  } catch (error) {
    console.error('Failed to get storage paths:', error)
  }
}

const showDataDirDialog = ref(false)
const dataDirDraft = ref('')

const openDataDirDialog = async () => {
  await refreshStoragePaths()
  dataDirDraft.value = appStore.dataDir || ''
  showDataDirDialog.value = true
}

const pickDataDir = async () => {
  try {
    const defaultPath = dataDirDraft.value.trim() || resolvedDataDir.value.trim() || undefined
    const selected = await pickDirectory({
      title: t('settings.dataDir.pickTitle'),
      defaultPath,
    })
    if (selected) dataDirDraft.value = selected
  } catch (error) {
    console.error('Failed to pick directory:', error)
    Snackbar.error(t('settings.messages.openDirFailed'))
  }
}

const resetDataDirToDefault = async () => {
  dataDirDraft.value = ''
  appStore.dataDir = ''
  await appStore.saveConfig()
  await refreshStoragePaths()
  Snackbar.info(t('settings.dataDir.resetOk'))
}

const confirmDataDir = async () => {
  appStore.dataDir = dataDirDraft.value.trim()
  await appStore.saveConfig()
  await refreshStoragePaths()
  showDataDirDialog.value = false
  Snackbar.info(t('settings.dataDir.restartHint'))
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
  { label: t('settings.themeSystem'), value: 'system' as const },
  { label: t('settings.themeLight'), value: 'light' as const },
  { label: t('settings.themeDark'), value: 'dark' as const },
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

// GitHub 镜像相关
const githubMirrorEnabled = computed({
  get: () => appStore.githubMirrorEnabled,
  set: (val) => appStore.githubMirrorEnabled = val
})

const githubMirrorSource = computed({
  get: () => appStore.githubMirrorSource,
  set: (val) => appStore.githubMirrorSource = val
})

const githubMirrorCustomTemplate = computed({
  get: () => appStore.githubMirrorCustomTemplate,
  set: (val) => appStore.githubMirrorCustomTemplate = val
})

const githubMirrorSourceOptions = computed(() => [
  { label: t('settings.githubMirror.sources.gh-proxy-cf'), value: 'gh-proxy-cf' as const },
  { label: t('settings.githubMirror.sources.gh-proxy-fastly'), value: 'gh-proxy-fastly' as const },
  { label: t('settings.githubMirror.sources.gh-proxy-edgeone'), value: 'gh-proxy-edgeone' as const },
  { label: t('settings.githubMirror.sources.ghfast'), value: 'ghfast' as const },
  { label: t('settings.githubMirror.sources.custom'), value: 'custom' as const },
])

const githubMirrorConnectivity = ref<{ status: 'idle' | 'testing' | 'success' | 'failed'; latency: number; error: string }>({
  status: 'idle',
  latency: 0,
  error: ''
})

const getGithubMirrorTemplate = () => {
  if (githubMirrorSource.value === 'custom') {
    return githubMirrorCustomTemplate.value || '{url}'
  }
  return GITHUB_MIRROR_TEMPLATES[githubMirrorSource.value]
}

const testGithubMirrorConnection = async () => {
  const template = getGithubMirrorTemplate()
  if (!template || template === '{url}') {
    githubMirrorConnectivity.value = { status: 'idle', latency: 0, error: '' }
    return
  }

  githubMirrorConnectivity.value = { status: 'testing', latency: 0, error: '' }
  try {
    const latency = await testGithubMirror(template)
    githubMirrorConnectivity.value = { status: 'success', latency, error: '' }
  } catch (e: any) {
    console.error('GitHub mirror test failed:', e)
    githubMirrorConnectivity.value = {
      status: 'failed',
      latency: 0,
      error: typeof e === 'string' ? e : t('guide.connectionFailed')
    }
  }
}

const selectGithubMirrorSource = async (source: GithubMirrorSourceType) => {
  githubMirrorSource.value = source
  await testGithubMirrorConnection()
}

watch(githubMirrorEnabled, (enabled) => {
  if (enabled) {
    void testGithubMirrorConnection()
  } else {
    githubMirrorConnectivity.value = { status: 'idle', latency: 0, error: '' }
  }
})

watch(githubMirrorCustomTemplate, () => {
  if (githubMirrorSource.value === 'custom') {
    githubMirrorConnectivity.value = { status: 'idle', latency: 0, error: '' }
  }
})



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
const checkingMetadataUpdate = ref(false)

const metadataCurrentVersion = computed(() => appStore.metadataStatus?.currentVersion || t('common.unknown'))
const isMetadataOutdated = computed(() => appStore.isMetadataOutdated)
const remoteVersion = computed(() => appStore.metadataStatus?.remote?.packageVersion)

const verifyMetadataFiles = async () => {
    checkingMetadataUpdate.value = true
    try {
        await appStore.performMetadataUpdate()
        Snackbar.success(t('settings.metadata.verifySuccess'))
    } catch (error) {
        console.error('Failed to verify metadata files:', error)
        Snackbar.error(t('settings.metadata.verifyFailed'))
    } finally {
        checkingMetadataUpdate.value = false
    }
}

const resetMetadata = async () => {
  resetMetadataLoading.value = true
  try {
    const version = metadataVersion.value.trim() || 'latest'
    await resetMetadataCommand({
      baseUrl: metadataBaseUrl.value,
      version
    })
    // 重新检查以更新状态
    await appStore.checkMetadata()
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
          <var-button
              type="default"
              size="small"
              variant="text"
              :elevation="false"
              @click="openLink('settings.buttons.github')"
          >
            {{ t('settings.buttons.github') }}
          </var-button>
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

        <!-- 关于 -->
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

        <!-- 外观 -->
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
                  <SplitButtonSelect v-model="theme" :options="themeOptions" />
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
                     <SplitButtonSelect disabled v-model="background" :options="bgOptions" />
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
                  <SplitButtonSelect v-model="language" :options="langOptions" />
                </template>
              </var-cell>
            </var-paper>
          </var-space>
        </section>

        <!-- 游戏 -->
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

        <!-- 用户数据 -->
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
                 <div v-if="resolvedDataDir" class="cell-desc text-ellipsis">{{ resolvedDataDir }}</div>
                 <div class="cell-desc">{{ t('settings.userDataDesc') }}</div>
              </template>
               <template #extra>
                <var-icon name="chevron-right" />
              </template>
            </var-cell>

            <var-cell ripple @click="openDataDirDialog">
              <template #icon>
                <var-icon name="folder" size="24px" class="section-icon" />
              </template>
              <template #default>
                <div class="cell-title">{{ t('settings.dataDir.title') }}</div>
              </template>
              <template #description>
                <div class="cell-desc">{{ t('settings.dataDir.desc') }}</div>
              </template>
              <template #extra>
                <var-icon name="chevron-right" />
              </template>
            </var-cell>
          </var-paper>
        </section>

        <!-- 元数据 -->
        <section>
          <div class="section-title">{{ t('settings.metadata.title') }}</div>
          <var-space direction="column" size="small">
            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="update" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.metadata.update') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.metadata.currentVersion') }}: {{ metadataCurrentVersion }}</div>
                  <div v-if="isMetadataOutdated && remoteVersion" class="cell-desc" style="color: var(--color-warning);">
                      {{ t('settings.metadata.newVersionAvailable', { version: remoteVersion }) }}
                  </div>
                </template>
                <template #extra>
                   <var-space :size="8">
                       <var-button
                        type="primary"
                        size="small"
                        variant="text"
                        :loading="checkingMetadataUpdate"
                        :elevation="false"
                        @click="verifyMetadataFiles"
                      >
                        {{ t('settings.metadata.verify') }}
                      </var-button>
                      <var-button
                        v-if="isMetadataOutdated"
                        type="primary"
                        size="small"
                        :elevation="false"
                        :loading="resetMetadataLoading"
                        @click="resetMetadata"
                      >
                        {{ t('settings.update.action') }}
                      </var-button>
                   </var-space>
                </template>
              </var-cell>
            </var-paper>

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
                  <SplitButtonSelect v-model="metadataSourceType" :options="metadataSourceOptions" @update:model-value="selectSource" />
                </template>
              </var-cell>
            </var-paper>

            <!-- 仅展示当前选中源的状态 -->
            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                   <var-icon name="access-point-network" size="24px" class="section-icon" />
                </template>
                <template #default>
                   <div class="cell-title">{{ t('guide.connectivity') }}</div>
                </template>
                <template #description>
                  <div class="metadata-conn">
                    <span class="metadata-conn-label" style="font-weight: 500;">
                      {{ metadataSourceOptions.find(o => o.value === metadataSourceType)?.label || metadataSourceType }}
                    </span>
                  </div>
                  <div
                    v-if="metadataSourceType === 'custom'"
                    class="metadata-inline-input"
                  >
                    <div class="inline-input-label">{{ t('settings.metadata.sourceCustom') }}</div>
                    <var-input
                      v-model="metadataCustomBase"
                      size="small"
                      variant="outlined"
                      class="metadata-input"
                      :placeholder="t('settings.metadata.customPlaceholder')"
                      @change="testSourceConnection('custom')"
                    />
                  </div>
                </template>

                <template #extra>
                   <div style="display: flex; align-items: center; gap: 8px;">
                     <span v-if="connectivity[metadataSourceType].status === 'testing'" class="metadata-conn-testing">
                        <var-loading type="cube" size="small" :radius="2" class="metadata-conn-loading" />
                      </span>
                      <span v-else-if="connectivity[metadataSourceType].status === 'success'" class="metadata-conn-success">
                        <var-icon name="check-circle-outline" size="14" style="margin-right: 4px" />{{ connectivity[metadataSourceType].latency }}ms
                      </span>
                      <span v-else-if="connectivity[metadataSourceType].status === 'failed'" class="metadata-conn-failed">
                        <var-icon name="close-circle-outline" size="14" style="margin-right: 4px" />{{ t('guide.connectionFailed') }}
                      </span>
                      <span v-else class="metadata-conn-idle">--ms</span>

                      <var-button
                        round
                        text
                        size="mini"
                        type="primary"
                        :disabled="connectivity[metadataSourceType].status === 'testing'"
                        @click="testSourceConnection(metadataSourceType)"
                      >
                        <var-icon name="refresh" size="16" />
                      </var-button>
                   </div>
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

        <!-- Github 镜像 -->
        <section>
          <div class="section-title">{{ t('settings.githubMirror.title') }}</div>
          <var-space direction="column" size="small">
            <!-- 启用开关 -->
            <var-paper :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="github" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.githubMirror.enable') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.githubMirror.enableDesc') }}</div>
                </template>
                <template #extra>
                  <var-switch v-model="githubMirrorEnabled" />
                </template>
              </var-cell>
            </var-paper>

            <!-- 镜像源选择（仅启用时显示） -->
            <var-paper v-if="githubMirrorEnabled" :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="source-branch" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.githubMirror.source') }}</div>
                </template>
                <template #description>
                  <div class="cell-desc">{{ t('settings.githubMirror.sourceDesc') }}</div>
                </template>
                <template #extra>
                  <SplitButtonSelect v-model="githubMirrorSource" :options="githubMirrorSourceOptions" @update:model-value="selectGithubMirrorSource" />
                </template>
              </var-cell>
            </var-paper>

            <!-- 当前源连通性展示 -->
            <var-paper v-if="githubMirrorEnabled" :elevation="false" radius="12">
              <var-cell>
                <template #icon>
                  <var-icon name="access-point-network" size="24px" class="section-icon" />
                </template>
                <template #default>
                  <div class="cell-title">{{ t('settings.githubMirror.currentSource') }}</div>
                </template>
                <template #description>
                  <div class="metadata-conn">
                    <span class="metadata-conn-label" style="font-weight: 500;">
                      {{ githubMirrorSourceOptions.find(o => o.value === githubMirrorSource)?.label }}
                    </span>
                  </div>
                  <!-- 自定义输入框 -->
                  <div v-if="githubMirrorSource === 'custom'" class="metadata-inline-input">
                    <div class="inline-input-label">{{ t('settings.githubMirror.customUrl') }}</div>
                    <var-input
                      v-model="githubMirrorCustomTemplate"
                      size="small"
                      variant="outlined"
                      class="metadata-input"
                      :placeholder="t('settings.githubMirror.customPlaceholder')"
                      @change="testGithubMirrorConnection"
                    />
                  </div>
                </template>
                <template #extra>
                  <!-- 连通性状态与重测按钮 -->
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span v-if="githubMirrorConnectivity.status === 'testing'" class="metadata-conn-testing">
                      <var-loading type="cube" size="small" :radius="2" class="metadata-conn-loading" />
                    </span>
                    <span v-else-if="githubMirrorConnectivity.status === 'success'" class="metadata-conn-success">
                      <var-icon name="check-circle-outline" size="14" style="margin-right: 4px" />{{ githubMirrorConnectivity.latency }}ms
                    </span>
                    <span v-else-if="githubMirrorConnectivity.status === 'failed'" class="metadata-conn-failed">
                      <var-icon name="close-circle-outline" size="14" style="margin-right: 4px" />{{ t('guide.connectionFailed') }}
                    </span>
                    <span v-else class="metadata-conn-idle">--ms</span>

                    <var-button
                      round
                      text
                      size="mini"
                      type="primary"
                      :disabled="githubMirrorConnectivity.status === 'testing'"
                      @click="testGithubMirrorConnection"
                    >
                      <var-icon name="refresh" size="16" />
                    </var-button>
                  </div>
                </template>
              </var-cell>
            </var-paper>
          </var-space>
        </section>

      </var-space>

      <var-dialog
        :show="showDataDirDialog"
        :title="t('settings.dataDir.dialogTitle')"
        :width="560"
        :confirm-button-text="t('settings.dataDir.confirm')"
        :cancel-button-text="t('settings.dataDir.cancel')"
        @confirm="confirmDataDir"
        @closed="showDataDirDialog = false"
        @update:show="showDataDirDialog = $event"
        style="--dialog-border-radius: 8px"
      >
        <var-space direction="column" :size="12">
          <div class="cell-desc">{{ t('settings.dataDir.hint') }}</div>
          <var-input
            v-model="dataDirDraft"
            variant="outlined"
            size="small"
            :placeholder="t('settings.dataDir.placeholder')"
          />
          <var-space :size="8">
            <var-button type="primary" size="small" :elevation="false" @click="pickDataDir">
              {{ t('settings.dataDir.pick') }}
            </var-button>
            <var-button type="default" size="small" :elevation="false" @click="resetDataDirToDefault">
              {{ t('settings.dataDir.reset') }}
            </var-button>
          </var-space>
        </var-space>
      </var-dialog>
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

.metadata-option-disabled {
  opacity: 0.5;
  pointer-events: none;
}

.disclaimer-block {
  padding: 14px 16px;
  font-size: 12px;
  color: var(--color-text);
  opacity: 0.85;
}

.metadata-inline-input {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.inline-input-label {
  font-size: 12px;
  opacity: 0.75;
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
