import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { checkMetadata as checkMetadataCommand, fetchMetadataManifest, readConfig, saveConfig as saveConfigCommand } from '../api/tauriCommands'

const METADATA_CDN_TEMPLATE = 'https://cdn.jsdelivr.net/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const METADATA_MIRROR_TEMPLATE = 'https://cdn.jsdmirror.com/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const DEFAULT_METADATA_VERSION = 'latest'

export type MetadataSourceType = 'cdn' | 'mirror' | 'custom'
type RemoteManifest = {
  packageVersion?: string
  metadataChecksum?: string
  itemCount?: number
}
type MetadataStatus = {
  path: string
  isEmpty: boolean
  fileCount: number
  hasManifest: boolean
  remote?: RemoteManifest
}

export const useAppStore = defineStore('app', () => {
  const theme = ref<'system' | 'light' | 'dark'>('system')
  const background = ref('default')
  const language = ref('zh-CN')

  const metadataSourceType = ref<MetadataSourceType>('cdn')
  const metadataVersion = ref(DEFAULT_METADATA_VERSION)
  const metadataCustomBase = ref('')

  const configCache = ref<Record<string, any>>({})

  // 初次加载时避免写回配置
  const isLoaded = ref(false)
  const metadataStatus = ref<MetadataStatus | null>(null)
  const metadataNeedCheck = computed(() => {
    if (!metadataStatus.value) return false
    return metadataStatus.value.isEmpty || !metadataStatus.value.hasManifest
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

      const metadata = (config?.metadata as Record<string, any>) || {}
      // baseUrl 固定在前端，只持久化自定义地址
      if (metadata.customBase) metadataCustomBase.value = metadata.customBase

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
      const nextConfig = {
        ...configCache.value,
        theme: theme.value,
        background: background.value,
        language: language.value,
        firstRun: firstRun.value,
        metadata: {
          customBase: persistedCustomBase,
        }
      }
      configCache.value = nextConfig
      await saveConfigCommand(nextConfig)
    } catch (error) {
      console.error('Failed to save config:', error)
    }
  }

  // 监听变更自动保存
  watch([theme, background, language, metadataCustomBase, firstRun], () => {
    void saveConfig()
  })

  const completeSetup = async () => {
    firstRun.value = false
    await saveConfig()
  }

  const checkMetadata = async () => {
    try {
      const status = await checkMetadataCommand<MetadataStatus>()
      let merged: MetadataStatus = status

      if ((status.isEmpty || !status.hasManifest) && metadataBaseUrl.value.trim()) {
        try {
          const remote = await fetchMetadataManifest<RemoteManifest>({ baseUrl: metadataBaseUrl.value, version: metadataVersion.value })
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
    checkMetadata,
    loadConfig,
    completeSetup
  }
})
