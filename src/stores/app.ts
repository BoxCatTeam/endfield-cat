import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const METADATA_CDN_TEMPLATE = 'https://cdn.jsdelivr.net/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const METADATA_MIRROR_TEMPLATE = 'https://cdn.jsdmirror.com/gh/BoxCatTeam/endfield-cat-metadata@v{version}/'
const DEFAULT_METADATA_VERSION = 'latest'

type MetadataSourceType = 'cdn' | 'mirror' | 'custom'
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

  // Flag to avoid saving during initial load
  const isLoaded = ref(false)
  const metadataStatus = ref<MetadataStatus | null>(null)
  const metadataNeedCheck = computed(() => {
    if (!metadataStatus.value) return false
    return metadataStatus.value.isEmpty || !metadataStatus.value.hasManifest
  })

  const metadataBaseUrl = computed(() => {
    if (metadataSourceType.value === 'custom') {
      return metadataCustomBase.value.trim()
    }
    const version = metadataVersion.value.trim() || DEFAULT_METADATA_VERSION
    if (metadataSourceType.value === 'mirror') {
      return METADATA_MIRROR_TEMPLATE.replace('{version}', version)
    }
    return METADATA_CDN_TEMPLATE.replace('{version}', version)
  })

  const firstRun = ref(true)

  // Load config from Rust backend
  const loadConfig = async () => {
    try {
      const config: any = await invoke('read_config')
      console.log('Loaded config:', config)
      configCache.value = config || {}

      if (config?.theme) theme.value = config.theme
      if (config?.background) background.value = config.background
      if (config?.language) language.value = config.language
      if (config?.firstRun !== undefined) firstRun.value = config.firstRun

      const metadata = (config?.metadata as Record<string, any>) || {}
      if (metadata.sourceType) metadataSourceType.value = metadata.sourceType as MetadataSourceType
      if (metadata.version) metadataVersion.value = metadata.version
      if (metadata.customBase) metadataCustomBase.value = metadata.customBase

      isLoaded.value = true
    } catch (error) {
      console.error('Failed to load config:', error)
      isLoaded.value = true // valid to proceed with defaults
    }
  }

  // Save config to Rust backend
  const saveConfig = async () => {
    if (!isLoaded.value) return

    try {
      const trimmedCustomBase = metadataCustomBase.value.trim()
      const nextConfig = {
        ...configCache.value,
        theme: theme.value,
        background: background.value,
        language: language.value,
        firstRun: firstRun.value,
        metadata: {
          sourceType: metadataSourceType.value,
          version: metadataVersion.value,
          customBase: trimmedCustomBase,
          baseUrl: metadataBaseUrl.value,
        }
      }
      configCache.value = nextConfig
      await invoke('save_config', { config: nextConfig })
    } catch (error) {
      console.error('Failed to save config:', error)
    }
  }

  // Watch for changes and save
  watch([theme, background, language, metadataSourceType, metadataVersion, metadataCustomBase, firstRun], () => {
    void saveConfig()
  })

  const completeSetup = async () => {
    firstRun.value = false
    await saveConfig()
  }

  const checkMetadata = async () => {
    try {
      const status = await invoke<MetadataStatus>('check_metadata')
      let merged: MetadataStatus = status

      if ((status.isEmpty || !status.hasManifest) && metadataBaseUrl.value.trim()) {
        try {
          const remote = await invoke<RemoteManifest>('fetch_metadata_manifest', { baseUrl: metadataBaseUrl.value, version: metadataVersion.value })
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
    metadataStatus,
    metadataNeedCheck,
    checkMetadata,
    loadConfig,
    completeSetup
  }
})
