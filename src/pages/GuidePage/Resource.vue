<script setup lang="ts">
import { ref, computed, onUnmounted, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../../stores/app'
import { Snackbar } from '@varlet/ui'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import type { GithubMirrorSourceType, MetadataSourceType } from '../../stores/app'
import { GITHUB_MIRROR_TEMPLATES } from '../../stores/app'
import { fetchMetadataManifest, resetMetadata as resetMetadataCommand, testGithubMirror } from '../../api/tauriCommands'
import SplitButtonSelect from '../../components/SplitButtonSelect.vue'

const router = useRouter()
const appStore = useAppStore()
const { t } = useI18n()

const loading = ref(false)
const checking = ref(false)
const progress = ref(0)
const progressText = ref('')
let unlisten: UnlistenFn | null = null

const metadataValid = computed(() => {
  const s = appStore.metadataStatus
  return s && s.hasManifest && !s.isEmpty
})

// 进度事件负载
type DownloadProgress = {
  current: number;
  total: number;
  filename: string;
}

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
  return appStore.getMetadataBaseUrlFor(source, appStore.metadataCustomBase)
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
    await fetchMetadataManifest({
      baseUrl,
      version: appStore.metadataVersion
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
    // 顺序测试以减少并发请求
    // eslint-disable-next-line no-await-in-loop
    await testSourceConnection(s)
  }
}

const selectSource = async (source: MetadataSourceType) => {
  if (source === 'custom') {
    Snackbar.info(t('settings.messages.devPlaceholder'))
    return
  }
  appStore.metadataSourceType = source
  await testSourceConnection(source)
}

const TAURI_OK = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as any)

// GitHub 镜像配置（引导页中也可设置）
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
  if (!TAURI_OK || !githubMirrorEnabled.value) {
    githubMirrorConnectivity.value = { status: 'idle', latency: 0, error: '' }
    return
  }

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

const checkMetadataState = async () => {
  checking.value = true
  progressText.value = t('guide.checkingMetadata')
  try {
    await appStore.checkMetadata()
    if (metadataValid.value) {
       // 加一点延时便于显示检测过程
      setTimeout(() => {
         router.push('/guide/ready')
      }, 800)
    } else {
      // 无效则自动测试当前源连通性
      testAllConnections()
    }
  } finally {
    checking.value = false
  }
}

const initializeMetadata = async () => {

  loading.value = true
  progress.value = 0
  progressText.value = t('guide.preparing')
  
  try {
    // 监听下载进度
    unlisten = await listen<DownloadProgress>('metadata-progress', (event) => {
      const p = event.payload
      if (p.total > 0) {
        progress.value = Math.floor((p.current / p.total) * 100)
        progressText.value = t('guide.downloading', {
          filename: p.filename,
          current: p.current,
          total: p.total
        })
      }
    })

    await resetMetadataCommand({
      baseUrl: appStore.metadataBaseUrl,
      version: appStore.metadataVersion
    })
    
    await appStore.checkMetadata()
    if (metadataValid.value) {
      router.push('/guide/ready')
    } else {
      Snackbar.warning(t('guide.initPartial'))
    }
  } catch (e) {
    console.error(e)
    Snackbar.error(t('guide.initFailed'))
  } finally {
    loading.value = false
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }
}

onMounted(() => {
  checkMetadataState()
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>

<template>
  <div class="resource-card glass-panel">
    <var-space direction="column" align="center" class="step-header">
      <var-space :size="8" class="step-indicator">
        <span class="step-dot active"></span>
        <span class="step-dot"></span>
      </var-space>
      <h2 class="step-title">{{ t('guide.resourceTitle') }}</h2>
    </var-space>

    <div class="resource-body">
      <!-- 加载/检查状态 -->
      <div v-if="loading || checking" class="state-loading">
        <var-loading type="wave" size="large" color="var(--color-primary)" />
        <p class="status-text-large">{{ progressText }}</p>
        <var-progress v-if="loading" :value="progress" class="download-progress" track-color="rgba(0,0,0,0.1)" />
      </div>

      <!-- 主选择区 -->
      <div v-else class="source-selection">
        <var-space :size="12" class="alert-box warning" align="start">
           <var-icon name="alert-circle-outline" style="margin-top: 2px;" />
           <div>
             <div class="alert-title">{{ t('guide.missingTitle') }}</div>
             <div class="alert-desc">{{ t('guide.missingDesc1') }}</div>
           </div>
        </var-space>

        <div class="section-label">{{ t('settings.metadata.source') }}</div>
        
        <div class="source-list">
          <div
            v-for="src in (['cdn', 'mirror', 'custom'] as const)"
            :key="src"
            class="source-option"
            :class="{ active: appStore.metadataSourceType === src, disabled: src === 'custom' }"
            v-ripple
            @click="selectSource(src)"
          >
            <!-- 1. 左侧复选框 -->
            <div class="source-check">
              <var-checkbox :model-value="appStore.metadataSourceType === src" readonly />
            </div>

            <var-space justify="space-between" class="source-item">
              <!-- 2. 数据源名称 -->
              <div class="source-name">
                <span v-if="src === 'cdn'">{{ t('settings.metadata.sourceCdn') }}</span>
                <span v-else-if="src === 'mirror'">{{ t('settings.metadata.sourceMirror') }}</span>
                <span v-else>{{ t('settings.metadata.sourceCustom') }}</span>
              </div>

              <!-- 3. 右侧连通性状态 -->
              <div class="source-status">
              <span v-if="connectivity[src].status === 'testing'" class="status-badge testing">
                  <var-loading type="cube" size="small" :radius="2" class="inline-loading" />
              </span>
                <span v-else-if="connectivity[src].status === 'success'" class="status-badge success">
                  {{ connectivity[src].latency }}ms
              </span>
                <span v-else-if="connectivity[src].status === 'failed'" class="status-badge failed">
                  [{{ t('guide.connectionFailed') }}]
              </span>
              </div>
            </var-space>

          </div>
        </div>

        <!-- 自定义地址输入区 -->
        <transition name="expand">
          <div v-if="appStore.metadataSourceType === 'custom'" class="custom-input-box">
             <div class="input-label">{{ t('settings.metadata.sourceCustom') }}:</div>
             <var-input
                v-model="appStore.metadataCustomBase"
                size="small"
                variant="outlined"
                :placeholder="t('settings.metadata.customPlaceholder')"
                @change="testSourceConnection('custom')"
              />
          </div>
        </transition>

        <div class="section-label" style="margin-top: 20px;">{{ t('settings.githubMirror.title') }}</div>
        <var-paper :elevation="false" radius="12" style="padding: 14px 16px;">
          <var-space direction="column" :size="12">
            <var-space justify="space-between" align="center">
              <div>
                <div class="source-name">{{ t('settings.githubMirror.enable') }}</div>
                <div class="alert-desc">{{ t('settings.githubMirror.enableDesc') }}</div>
              </div>
              <var-switch v-model="githubMirrorEnabled" />
            </var-space>

            <var-space justify="space-between" align="center">
              <div>
                <div class="source-name">{{ t('settings.githubMirror.source') }}</div>
                <div class="alert-desc">{{ t('settings.githubMirror.sourceDesc') }}</div>
              </div>
              <SplitButtonSelect
                v-model="githubMirrorSource"
                :options="githubMirrorSourceOptions"
                :disabled="!githubMirrorEnabled"
                @update:model-value="selectGithubMirrorSource"
              />
            </var-space>

            <div v-if="githubMirrorEnabled && githubMirrorSource === 'custom'">
              <div class="input-label">{{ t('settings.githubMirror.customUrl') }}</div>
              <var-input
                v-model="githubMirrorCustomTemplate"
                size="small"
                variant="outlined"
                :placeholder="t('settings.githubMirror.customPlaceholder')"
                @change="testGithubMirrorConnection"
              />
            </div>

            <var-space justify="space-between" align="center">
              <div class="source-status">
                <span v-if="githubMirrorConnectivity.status === 'testing'" class="status-badge testing">
                  <var-loading type="cube" size="small" :radius="2" class="inline-loading" />
                </span>
                <span v-else-if="githubMirrorConnectivity.status === 'success'" class="status-badge success">
                  {{ githubMirrorConnectivity.latency }}ms
                </span>
                <span v-else-if="githubMirrorConnectivity.status === 'failed'" class="status-badge failed">
                  [{{ t('guide.connectionFailed') }}]
                </span>
                <span v-else class="status-badge">--ms</span>
              </div>

              <var-button
                round
                text
                size="mini"
                type="primary"
                :disabled="!githubMirrorEnabled || githubMirrorConnectivity.status === 'testing'"
                @click="testGithubMirrorConnection"
              >
                <var-icon name="refresh" size="16" />
              </var-button>
            </var-space>
          </var-space>
        </var-paper>
      </div>
    </div>

    <div class="action-footer" v-if="!loading && !checking">
      <var-space justify="space-between" class="footer-row">
         <var-button text type="primary" size="small" :disabled="anyTesting" @click="testAllConnections">
           <var-icon name="refresh" /> {{ t('common.retry') }}
         </var-button>
         <var-button text type="default" size="small" @click="router.push('/settings')">
           {{ t('guide.customSettings') }}
         </var-button>
      </var-space>
      <var-button 
        type="primary" 
        block 
        size="large" 
        class="glow-btn" 
        @click="initializeMetadata"
        :loading="anyTesting"
      >
        {{ t('guide.download') }}
      </var-button>
    </div>
  </div>
</template>

<style scoped>
/* 玻璃质感基础样式 */
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
.source-item{
  flex: 1;
}
.step-header {
  margin-bottom: 24px;
}

.step-indicator {
  margin-bottom: 12px;
}

.step-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-outline);
  opacity: 0.3;
  transition: all 0.3s;
}

.step-dot.active {
  background: var(--color-primary);
  opacity: 1;
  width: 24px;
  border-radius: 4px;
}

.step-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: var(--color-text);
}

.resource-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  margin-bottom: 24px;
}

.state-loading {
  text-align: center;
  padding: 40px 0;
}

.status-text-large {
  font-size: 16px;
  margin-top: 16px;
  font-weight: 500;
  color: var(--color-text);
  margin-bottom: 20px;
}

.download-progress {
  width: 80%;
  margin: 0 auto;
}

.alert-box {
  background: rgba(255, 152, 0, 0.1);
  padding: 16px;
  border-radius: 12px;
  color: var(--color-warning);
  margin-bottom: 24px;
  border: 1px solid rgba(255, 152, 0, 0.2);
}

.alert-title {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 4px;
}

.alert-desc {
  font-size: 13px;
  opacity: 0.9;
}

.section-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  opacity: 0.6;
  margin-bottom: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.source-list {
  display: grid;
  gap: 12px;
}

.source-option {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  background: var(--color-surface-container-low, rgba(255,255,255,0.5));
  border: 2px solid transparent;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
  text-align: left;
}

.source-option:hover {
  background: var(--color-surface-container-high, rgba(255,255,255,0.8));
}

.source-option.active {
  border-color: var(--color-primary);
  background: rgba(var(--hsl-primary), 0.05);
}

.source-option.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.source-check {
  margin-right: 12px;
  display: flex;
  align-items: center;
  pointer-events: none; /* 点击事件交给整行处理 */
}

.source-name {
  font-weight: 600;
  color: var(--color-text);
  font-size: 15px;
  white-space: nowrap;
}


.source-status {
  font-size: 13px;
  font-weight: 500;
  margin-left: 12px;
  white-space: nowrap;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.status-badge.success { color: var(--color-success); }
.status-badge.failed { color: var(--color-danger); }
.status-badge.testing { opacity: 0.6; }

.inline-loading {
  display: inline-block;
  vertical-align: middle;
}

.custom-input-box {
  margin-top: 16px;
  padding-left: 4px;
}

.input-label {
  font-size: 13px;
  margin-bottom: 8px;
  font-weight: 500;
  opacity: 0.8;
}

.action-footer {
  margin-top: auto;
}

.footer-row {
  margin-bottom: 16px;
}

.glow-btn {
  box-shadow: 0 4px 14px 0 rgba(var(--hsl-primary), 0.39) !important;
  font-weight: 600;
  border-radius: 14px;
  height: 52px;
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  max-height: 100px;
  opacity: 1;
}

.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
  margin-top: 0;
}
</style>
