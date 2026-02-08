import { listen } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { checkMetadata as checkMetadataCommand, fetchMetadataManifest, getAppVersion, readConfig, saveConfig as saveConfigCommand, updateMetadata } from '../api/tauriCommands'

const METADATA_CDN_TEMPLATE = 'https://cdn.jsdelivr.net/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const METADATA_MIRROR_TEMPLATE = 'https://cdn.jsdmirror.com/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const DEFAULT_METADATA_VERSION = 'latest'

export type MetadataSourceType = 'cdn' | 'mirror' | 'custom'
export type GithubMirrorSourceType = 'gh-proxy-cf' | 'gh-proxy-fastly' | 'gh-proxy-edgeone' | 'ghfast' | 'custom'

export const GITHUB_MIRROR_TEMPLATES: Record<GithubMirrorSourceType, string> = {
  'gh-proxy-cf': 'https://gh-proxy.org/{url}',
  'gh-proxy-fastly': 'https://cdn.gh-proxy.org/{url}',
  'gh-proxy-edgeone': 'https://edgeone.gh-proxy.org/{url}',
  'ghfast': 'https://ghfast.top/{url}',
  'custom': '{url}',
}
type RemoteManifest = {
  packageVersion?: string
  metadataChecksum?: string
  itemCount?: number
  totalSize?: number
}
type MetadataStatus = {
  path: string
  isEmpty: boolean
  fileCount: number
  hasManifest: boolean
  currentVersion?: string
  remote?: RemoteManifest
}

export type MetadataUpdateProgress = {
  phase: 'verifying' | 'downloading' | 'cleaning'
  current: number
  total: number
  path: string
}


export const useAppStore = defineStore('app', () => {
  const theme = ref<'system' | 'light' | 'dark'>('system')
  const background = ref('default')
  const language = ref('zh-CN')

  const metadataSourceType = ref<MetadataSourceType>('cdn')
  const metadataVersion = ref(DEFAULT_METADATA_VERSION)
  const metadataCustomBase = ref('')

  const configCache = ref<Record<string, any>>({})
  const acknowledgedAppVersion = ref<string | null>(null)
  const pendingPostUpdateVersion = ref<string | null>(null)
  const currentAppVersion = ref<string | null>(null)
  const needsPostUpdateGuide = ref(false)

  // GitHub 镜像配置
  const githubMirrorEnabled = ref(false)
  const githubMirrorSource = ref<GithubMirrorSourceType>('gh-proxy-cf')
  const githubMirrorCustomTemplate = ref('')

  // 初次加载时避免写回配置
  const isLoaded = ref(false)
  const metadataStatus = ref<MetadataStatus | null>(null)

  // 元数据更新相关状态
  const showMetadataUpdateDialog = ref(false)
  const isMetadataUpdating = ref(false)
  const metadataUpdateProgress = ref<MetadataUpdateProgress | null>(null)

  const isMetadataOutdated = computed(() => {
    if (!metadataStatus.value || !metadataStatus.value.remote || !metadataStatus.value.currentVersion) return false
    const local = metadataStatus.value.currentVersion
    const remote = metadataStatus.value.remote.packageVersion
    return remote ? remote !== local : false
  })

  const metadataNeedCheck = computed(() => {
    if (!metadataStatus.value) return false
    return metadataStatus.value.isEmpty || !metadataStatus.value.hasManifest || (isMetadataOutdated.value && metadataStatus.value.remote)
  })

  const normalizeBaseUrl = (baseUrl: string) => {
    const trimmed = baseUrl.trim()
    if (!trimmed) return ''
    return trimmed.endsWith('/') ? trimmed : `${trimmed}/`
  }

  const getMetadataBaseUrlFor = (sourceType: MetadataSourceType, customBase?: string) => {
    if (sourceType === 'custom') {
      return normalizeBaseUrl(customBase ?? metadataCustomBase.value)
    }
    // 默认使用 latest，可通过 metadataVersion 覆盖
    const version = metadataVersion.value.trim() || DEFAULT_METADATA_VERSION
    if (sourceType === 'mirror') {
      return METADATA_MIRROR_TEMPLATE.replace('{version}', version)
    }
    return METADATA_CDN_TEMPLATE.replace('{version}', version)
  }

  const metadataBaseUrl = computed(() => getMetadataBaseUrlFor(metadataSourceType.value))

  const firstRun = ref(true)

  // 从后端读取配置
  const loadConfig = async () => {
    try {
      const config: any = await readConfig()
      console.log('Loaded config:', config)
      configCache.value = config || {}

      if (config?.theme) theme.value = config.theme
      if (config?.background) background.value = config.background
      if (config?.language) language.value = config.language
      if (config?.firstRun !== undefined) firstRun.value = config.firstRun
      if (config?.appVersion) acknowledgedAppVersion.value = config.appVersion
      if (config?.pendingPostUpdateVersion) pendingPostUpdateVersion.value = config.pendingPostUpdateVersion
      if (config?.needsPostUpdateGuide !== undefined) needsPostUpdateGuide.value = config.needsPostUpdateGuide

      const metadata = (config?.metadata as Record<string, any>) || {}
      // baseUrl 固定在前端，只持久化自定义地址
      if (metadata.customBase) metadataCustomBase.value = metadata.customBase

      // 加载 GitHub 镜像配置
      if (config?.githubMirror) {
        githubMirrorEnabled.value = config.githubMirror.enabled ?? false
        githubMirrorSource.value = config.githubMirror.source ?? 'gh-proxy-cf'
        githubMirrorCustomTemplate.value = config.githubMirror.customTemplate ?? ''
      }

      isLoaded.value = true
    } catch (error) {
      console.error('Failed to load config:', error)
      isLoaded.value = true // 读取失败也继续使用默认值
    }
  }

  // 将配置写回后端
  const saveConfig = async () => {
    if (!isLoaded.value) return

    try {
      const persistedCustomBase = normalizeBaseUrl(metadataCustomBase.value)
      const existingAppVersion = (configCache.value as any)?.appVersion ?? null
      const nextConfig = {
        ...configCache.value,
        theme: theme.value,
        background: background.value,
        language: language.value,
        firstRun: firstRun.value,
        // appVersion 仅用于“已完成引导/已确认”的版本记录，不作为当前版本来源
        // 当前版本永远从 Tauri 的 get_app_version 获取。
        appVersion: acknowledgedAppVersion.value ?? existingAppVersion,
        pendingPostUpdateVersion: pendingPostUpdateVersion.value,
        needsPostUpdateGuide: needsPostUpdateGuide.value,
        metadata: {
          customBase: persistedCustomBase,
        },
        githubMirror: {
          enabled: githubMirrorEnabled.value,
          source: githubMirrorSource.value,
          customTemplate: githubMirrorCustomTemplate.value,
        }
      }
      configCache.value = nextConfig
      await saveConfigCommand(nextConfig)
    } catch (error) {
      console.error('Failed to save config:', error)
    }
  }

  // 监听变更自动保存
  watch([theme, background, language, metadataCustomBase, firstRun, acknowledgedAppVersion, pendingPostUpdateVersion, needsPostUpdateGuide, githubMirrorEnabled, githubMirrorSource, githubMirrorCustomTemplate], () => {
    void saveConfig()
  })

  // 获取当前镜像URL模板
  const getGithubMirrorTemplate = () => {
    if (!githubMirrorEnabled.value) return '{url}'
    if (githubMirrorSource.value === 'custom') {
      return githubMirrorCustomTemplate.value || '{url}'
    }
    return GITHUB_MIRROR_TEMPLATES[githubMirrorSource.value]
  }

  const completeSetup = async () => {
    firstRun.value = false
    if (!acknowledgedAppVersion.value && currentAppVersion.value) {
      acknowledgedAppVersion.value = currentAppVersion.value
    }
    await saveConfig()
  }

  const syncAppVersion = async () => {
    try {
      const version = await getAppVersion()
      currentAppVersion.value = version

      // 若已处于“升级后引导待处理”状态，避免重复覆盖（比如多次进入 App.vue onMounted）
      if (needsPostUpdateGuide.value) return

      // 仅当已有记录版本且与当前不同，标记需要更新后引导
      if (acknowledgedAppVersion.value && acknowledgedAppVersion.value !== version) {
        pendingPostUpdateVersion.value = version
        needsPostUpdateGuide.value = true
      } else if (!acknowledgedAppVersion.value) {
        // 兼容旧配置：没有 appVersion 但 firstRun 已完成，视为升级后首启，提示一次引导
        if (!firstRun.value) {
          pendingPostUpdateVersion.value = version
          needsPostUpdateGuide.value = true
        } else {
          acknowledgedAppVersion.value = version
        }
      }
    } catch (error) {
      console.error('Failed to sync app version:', error)
    }
  }

  const completePostUpdateGuide = async () => {
    if (currentAppVersion.value) {
      acknowledgedAppVersion.value = currentAppVersion.value
    } else if (pendingPostUpdateVersion.value) {
      acknowledgedAppVersion.value = pendingPostUpdateVersion.value
    }
    pendingPostUpdateVersion.value = null
    needsPostUpdateGuide.value = false
    await saveConfig()
  }

  const checkMetadata = async () => {
    try {
      const status = await checkMetadataCommand<MetadataStatus>()
      let merged: MetadataStatus = status

      if (metadataBaseUrl.value.trim()) {
        try {
          const version = metadataVersion.value.trim() || DEFAULT_METADATA_VERSION
          const remote = await fetchMetadataManifest<RemoteManifest>({ baseUrl: metadataBaseUrl.value, version })
          merged = { ...status, remote }
        } catch (error) {
          console.error('Failed to fetch remote manifest:', error)
        }
      }

      metadataStatus.value = merged
      return merged
    } catch (error) {
      console.error('Failed to check metadata:', error)
      return null
    }
  }

  // 执行元数据差分更新
  const performMetadataUpdate = async () => {
    if (isMetadataUpdating.value) return

    isMetadataUpdating.value = true
    metadataUpdateProgress.value = null
    showMetadataUpdateDialog.value = true  // 显示进度弹窗

    try {
      let unlisten: (() => void) | null = null

      // 监听更新进度事件
      unlisten = await listen<MetadataUpdateProgress>('metadata-update-progress', (event) => {
        metadataUpdateProgress.value = event.payload
      })

      try {
        await updateMetadata(metadataBaseUrl.value)

        // 更新完成后刷新状态
        await checkMetadata()
        showMetadataUpdateDialog.value = false
      } finally {
        if (unlisten) unlisten()
      }
    } catch (error) {
      console.error('Failed to update metadata:', error)
      throw error
    } finally {
      isMetadataUpdating.value = false
      metadataUpdateProgress.value = null
    }
  }

  // 关闭更新弹窗
  const dismissMetadataUpdateDialog = () => {
    showMetadataUpdateDialog.value = false
  }

  return {
    theme,
    background,
    language,
    firstRun,
    metadataSourceType,
    metadataVersion,
    metadataCustomBase,
    metadataBaseUrl,
    getMetadataBaseUrlFor,
    metadataStatus,
    metadataNeedCheck,
    isMetadataOutdated,
    checkMetadata,
    loadConfig,
    syncAppVersion,
    completeSetup,
    completePostUpdateGuide,
    acknowledgedAppVersion,
    currentAppVersion,
    pendingPostUpdateVersion,
    needsPostUpdateGuide,
    githubMirrorEnabled,
    githubMirrorSource,
    githubMirrorCustomTemplate,
    getGithubMirrorTemplate,
    // 元数据更新相关
    showMetadataUpdateDialog,
    isMetadataUpdating,
    metadataUpdateProgress,
    performMetadataUpdate,
    dismissMetadataUpdateDialog
  }
})

