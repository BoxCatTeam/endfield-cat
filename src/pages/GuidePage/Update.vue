<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Snackbar } from '@varlet/ui'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { updateMetadata as updateMetadataCommand } from '../../api/tauriCommands'
import { useAppStore } from '../../stores/app'

type DownloadProgress = {
  current: number;
  total: number;
  filename: string;
}

const appStore = useAppStore()
const router = useRouter()
const { t } = useI18n()

const loading = ref(false)
const checking = ref(false)
const progress = ref(0)
const progressText = ref('')
const totalFiles = ref(0)
let unlisten: UnlistenFn | null = null

const previousVersion = computed(() => appStore.acknowledgedAppVersion || t('common.unknown'))
const currentVersion = computed(() => appStore.currentAppVersion || t('common.unknown'))
const metadataLocalVersion = computed(() => appStore.metadataStatus?.currentVersion || t('common.unknown'))
const metadataRemoteVersion = computed(() => appStore.metadataStatus?.remote?.packageVersion || t('common.unknown'))
const pendingSizeMB = computed(() => {
  const size = appStore.metadataStatus?.remote?.totalSize
  if (!size || size <= 0) return null
  const mb = size / (1024 * 1024)
  return mb >= 5 ? mb.toFixed(1) : mb.toFixed(2)
})

const checkMetadataState = async () => {
  checking.value = true
  progressText.value = t('guide.checkingMetadata')
  try {
    totalFiles.value = 0
    await appStore.checkMetadata()
  } finally {
    checking.value = false
  }
}

const startUpdate = async () => {
  if (loading.value) return
  loading.value = true
  progress.value = 0
  progressText.value = t('guide.preparing')

  try {
    totalFiles.value = 0
    unlisten = await listen<DownloadProgress>('metadata-progress', (event) => {
      const p = event.payload
      if (p.total > 0) {
        totalFiles.value = p.total
        progress.value = Math.floor((p.current / p.total) * 100)
        progressText.value = t('guide.downloading', {
          filename: p.filename,
          current: p.current,
          total: p.total
        })
      }
    })

    await updateMetadataCommand({
      baseUrl: appStore.metadataBaseUrl,
      version: appStore.metadataVersion
    })

    await appStore.checkMetadata()
    await appStore.completePostUpdateGuide()
    Snackbar.success(t('guide.postUpdateSuccess'))
    router.replace('/')
  } catch (error) {
    console.error(error)
    Snackbar.error(t('guide.postUpdateFailed'))
  } finally {
    loading.value = false
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }
}

onMounted(() => {
  void checkMetadataState()
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>

<template>
  <div class="update-card glass-panel">
    <var-space direction="column" align="center" class="header">
      <div class="version-pill">v{{ previousVersion }} → v{{ currentVersion }}</div>
      <h2 class="title">{{ t('guide.postUpdateTitle') }}</h2>
      <p class="subtitle">{{ t('guide.postUpdateSubtitle') }}</p>
    </var-space>

    <div class="info-block">
      <div class="info-row">
        <span class="info-label">{{ t('guide.postUpdateDataTitle') }}</span>
        <span class="info-value">
          {{ metadataLocalVersion }} → {{ metadataRemoteVersion }}
        </span>
      </div>
      <p class="desc">{{ t('guide.postUpdateDesc') }}</p>
      <p v-if="pendingSizeMB" class="size-hint">待下载约 {{ pendingSizeMB }} MB</p>
    </div>

    <div v-if="loading || checking" class="state-loading">
      <var-loading type="wave" size="large" color="var(--color-primary)" />
      <p class="status-text">{{ progressText }}</p>
      <p v-if="totalFiles" class="status-subtext">待下载文件：{{ totalFiles }}</p>
      <var-progress
        v-if="loading"
        class="progress-bar"
        :value="progress"
        track-color="rgba(0,0,0,0.08)"
      />
    </div>

    <div v-else class="action-footer">
      <var-space justify="end" class="footer-row">
        <var-button text type="primary" size="small" @click="checkMetadataState">
          {{ t('guide.postUpdateRetry') }}
        </var-button>
      </var-space>
      <var-button
        type="primary"
        block
        size="large"
        class="glow-btn"
        @click="startUpdate"
      >
        {{ t('guide.postUpdateAction') }}
      </var-button>
    </div>
  </div>
</template>

<style scoped>
.glass-panel {
  background: rgba(255, 255, 255, 0.65);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border-radius: 28px;
  box-shadow:
    0 20px 40px -8px rgba(0, 0, 0, 0.1),
    0 0 0 1px rgba(255, 255, 255, 0.4) inset;
  padding: 32px;
  display: flex;
  flex-direction: column;
  transition: all 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

@media (prefers-color-scheme: dark) {
  .glass-panel {
    background: rgba(30, 30, 30, 0.6);
    box-shadow:
      0 20px 40px -8px rgba(0, 0, 0, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.08) inset;
  }
}

.header {
  text-align: center;
  margin-bottom: 20px;
}

.version-pill {
  padding: 8px 12px;
  border-radius: 999px;
  background: rgba(var(--hsl-primary), 0.12);
  color: var(--color-primary);
  font-weight: 600;
  font-size: 14px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.title {
  margin: 12px 0 4px;
  font-size: 22px;
  color: var(--color-text);
}

.subtitle {
  margin: 0;
  color: var(--color-text);
  opacity: 0.7;
  font-size: 14px;
}

.info-block {
  background: var(--color-surface-container-low, rgba(255,255,255,0.6));
  border-radius: 18px;
  padding: 16px;
  border: 1px solid rgba(0,0,0,0.05);
  margin-bottom: 24px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
  color: var(--color-text);
}

.info-label {
  opacity: 0.75;
}

.info-value {
  background: rgba(var(--hsl-primary), 0.08);
  color: var(--color-primary);
  padding: 6px 10px;
  border-radius: 10px;
  font-size: 13px;
}

.desc {
  margin: 10px 0 0;
  color: var(--color-text);
  opacity: 0.75;
  line-height: 1.5;
  font-size: 14px;
}

.state-loading {
  text-align: center;
  padding: 40px 0;
}

.status-text {
  margin-top: 16px;
  margin-bottom: 16px;
  color: var(--color-text);
  font-weight: 500;
}

.status-subtext,
.size-hint {
  margin: 0 0 12px;
  color: var(--color-text);
  opacity: 0.7;
  font-size: 13px;
}


.progress-bar {
  width: 80%;
  margin: 0 auto;
}

.action-footer {
  margin-top: auto;
}

.footer-row {
  margin-bottom: 12px;
}

.glow-btn {
  box-shadow: 0 4px 14px 0 rgba(var(--hsl-primary), 0.39) !important;
  font-weight: 600;
  border-radius: 14px;
  height: 52px;
}
</style>
















