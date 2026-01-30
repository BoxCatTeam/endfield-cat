<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import { Snackbar } from '@varlet/ui'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import logo from '../assets/icon.webp'
import { useI18n } from 'vue-i18n'

const router = useRouter()
const appStore = useAppStore()
const { t, tm } = useI18n()
const disclaimerItems = computed(() => tm('common.disclaimer.items') as string[])

const step = ref(0)
const loading = ref(false)
const checking = ref(false)
const progress = ref(0)
const progressText = ref('')
let unlisten: UnlistenFn | null = null

const showWelcome = computed(() => appStore.firstRun)
const metadataValid = computed(() => {
  const s = appStore.metadataStatus
  return s && s.hasManifest && !s.isEmpty
})

// Progress Event Payload
type DownloadProgress = {
  current: number;
  total: number;
  filename: string;
}

const nextStep = () => {
  step.value++
  if (step.value === 1) {
    checkMetadataState()
  }
}

const connectionStatus = ref<'idle' | 'testing' | 'success' | 'failed'>('idle')
const latency = ref(0)
const testError = ref('')

const testConnection = async () => {
  connectionStatus.value = 'testing'
  testError.value = ''
  const start = performance.now()
  try {
    await invoke('fetch_metadata_manifest', {
      baseUrl: appStore.metadataBaseUrl,
      version: appStore.metadataVersion
    })
    latency.value = Math.round(performance.now() - start)
    connectionStatus.value = 'success'
  } catch (e: any) {
    console.error(e)
    connectionStatus.value = 'failed'
    testError.value = typeof e === 'string' ? e : t('guide.connectionFailed')
  }
}

const checkMetadataState = async () => {
  checking.value = true
  progressText.value = t('guide.checkingMetadata')
  try {
    await appStore.checkMetadata()
    if (metadataValid.value) {
       // Using small delay to show the check happening
      setTimeout(() => {
         step.value = 2 // Go to Ready
      }, 800)
    } else {
      // If not valid, auto-test connectivity to current source
      testConnection()
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
    // Setup listener
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

    await invoke('reset_metadata', {
      baseUrl: appStore.metadataBaseUrl,
      version: appStore.metadataVersion
    })
    
    await appStore.checkMetadata()
    if (metadataValid.value) {
      step.value = 2 // Ready
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

const finishSetup = async () => {
  await appStore.completeSetup()
  router.replace('/')
}

onMounted(() => {
  if (showWelcome.value) {
    step.value = 0
  } else {
    step.value = 1
    checkMetadataState()
  }
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>

<template>
  <div class="guide-page-bg">
    <div class="guide-card glass-effect">
      <!-- Steps Header -->
      <var-steps :active="step" class="steps-header" active-color="var(--color-primary)">
        <var-step>{{ t('guide.stepWelcome') }}</var-step>
        <var-step>{{ t('guide.stepResource') }}</var-step>
        <var-step>{{ t('guide.stepFinish') }}</var-step>
      </var-steps>

      <div class="card-content">
        <transition name="fade-slide" mode="out-in">
          
          <!-- Step 0: Welcome -->
          <div v-if="step === 0" class="step-content" key="welcome">
            <var-image :src="logo" width="100" class="logo shadow-lg" radius="16" />
            <h1 class="welcome-title">{{ t('guide.welcomeTitle') }}</h1>
            <p class="subtitle">{{ t('guide.welcomeSubtitle') }}</p>
            
            <div class="info-block">
              <p>{{ t('guide.welcomeIntro1') }}</p>
              <p>{{ t('guide.welcomeIntro2') }}</p>
            </div>

            <div class="disclaimer">
              <div class="disclaimer-title">{{ t('common.disclaimer.title') }}</div>
              <ul class="disclaimer-list">
                <li v-for="(line, idx) in disclaimerItems" :key="idx">{{ line }}</li>
              </ul>
            </div>

            <var-button type="primary" size="large" class="action-btn" @click="nextStep" :elevation="true">
              {{ t('guide.startSetup') }}
            </var-button>
          </div>

          <!-- Step 1: Resource Check -->
          <div v-else-if="step === 1" class="step-content" key="resources">
            <div class="icon-container">
               <var-icon name="cloud-download-outline" size="64" color="var(--color-primary)" />
            </div>
            
            <h2 class="step-title">{{ t('guide.resourceTitle') }}</h2>

            <!-- Status / Progress Area -->
            <div class="status-area">
              <template v-if="loading">
                <var-progress :value="progress" label />
                <p class="status-text">{{ progressText }}</p>
              </template>
              
              <template v-else-if="checking">
                <var-loading type="wave" />
                <p class="status-text">{{ t('guide.checkingEnv') }}</p>
              </template>

              <template v-else>
                 <div class="missing-alert">
                    <var-icon name="alert-circle-outline" />
                    <span>{{ t('guide.missingTitle') }}</span>
                 </div>
                 <p class="status-desc">
                   {{ t('guide.missingDesc1') }}<br>
                   {{ t('guide.missingDesc2') }}
                 </p>
                 
                 <!-- Connectivity Status -->
                 <div class="connectivity-box">
                   <div class="conn-row">
                      <span>{{ t('guide.currentSource') }}</span>
                      <span class="source-url">{{ appStore.metadataBaseUrl }}</span>
                   </div>
                   <div class="conn-row status-row">
                      <span>{{ t('guide.connectivity') }}</span>
                      <span v-if="connectionStatus === 'testing'">
                        <var-loading type="cube" size="small" :radius="2" class="inline-loading" /> {{ t('guide.testing') }}
                      </span>
                      <span v-else-if="connectionStatus === 'success'" class="conn-success">
                        <var-icon name="check-circle-outline" size="14" /> {{ latency }}ms
                      </span>
                      <span v-else-if="connectionStatus === 'failed'" class="conn-failed">
                        <var-icon name="close-circle-outline" size="14" /> {{ testError }}
                      </span>
                      <var-button v-if="connectionStatus !== 'testing'" text type="primary" size="mini" @click="testConnection">{{ t('common.retry') }}</var-button>
                   </div>
                 </div>
              </template>
            </div>

            <var-button 
              v-if="!loading && !checking" 
              type="primary" 
              size="large" 
              class="action-btn" 
              @click="initializeMetadata"
              :elevation="true"
            >
              {{ t('guide.download') }}
            </var-button>

             <div class="advanced-link" v-if="!loading && !checking">
               <var-button text type="primary" size="small" @click="router.push('/settings')">
                 {{ t('guide.customSettings') }}
               </var-button>
             </div>
          </div>

          <!-- Step 2: Ready -->
           <div v-else-if="step === 2" class="step-content" key="ready">
             <div class="success-anim">
               <var-icon name="check-circle-outline" size="80" color="var(--color-success)" />
             </div>
             
             <h2 class="step-title">{{ t('guide.readyTitle') }}</h2>
             <p class="subtitle">{{ t('guide.readySubtitle') }}</p>

             <var-button 
               type="success" 
               size="large" 
               class="action-btn finish-btn" 
               @click="finishSetup"
               :elevation="true"
             >
               {{ t('guide.enterApp') }}
             </var-button>
           </div>

        </transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.guide-page-bg {
  height: 100vh;
  width: 100vw;
  display: flex;
  align-items: center;
  justify-content: center;
  background-image: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
  /* Dark mode background override needed globally or here via vars if supported properly */
  background-color: var(--color-body);
  background-size: cover;
}

/* Glassmorphism Card */
.guide-card {
  width: 90%;
  max-width: 460px;
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 24px;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.18);
  padding: 32px;
  overflow: hidden;
  position: relative;
  transition: all 0.3s ease;
}

/* Dark mode adjustment for card */
@media (prefers-color-scheme: dark) {
  .guide-card {
    background: rgba(30, 30, 30, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.05);
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.3);
  }
  .guide-page-bg {
    background-image: linear-gradient(135deg, #2c3e50 0%, #000000 100%);
  }
}

.steps-header {
  margin-bottom: 24px;
}

.step-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  min-height: 320px;
}

.logo {
  margin-top: 16px;
  margin-bottom: 24px;
}

.shadow-lg {
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
}

.welcome-title {
  font-size: 26px;
  font-weight: 600;
  margin: 0 0 8px;
  background: -webkit-linear-gradient(45deg, var(--color-primary), #a29bfe);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent; 
  /* Fallback color if needed */
  color: var(--color-primary);
}

.step-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 16px;
  color: var(--color-text);
}

.subtitle {
  font-size: 15px;
  color: var(--color-text);
  opacity: 0.7;
  margin: 0 0 32px;
}

.info-block {
  background: var(--color-surface-container-low);
  padding: 20px;
  border-radius: 12px;
  width: 100%;
  margin-bottom: 32px;
  text-align: left;
  font-size: 14px;
  opacity: 0.9;
  color: var(--color-text);
}
.disclaimer {
  width: 100%;
  text-align: left;
  margin-bottom: 24px;
  font-size: 12px;
  color: var(--color-text);
  opacity: 0.8;
}
.disclaimer-title {
  font-weight: 600;
  margin-bottom: 6px;
}
.disclaimer-list {
  padding-left: 18px;
  margin: 0;
  display: grid;
  gap: 4px;
}

.status-area {
  width: 100%;
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--color-surface-container-low);
  border-radius: 16px;
  padding: 24px;
  margin-bottom: 32px;
}

.status-text {
  margin-top: 16px;
  font-size: 13px;
  opacity: 0.7;
  color: var(--color-text);
}

.missing-alert {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--color-warning);
  font-weight: 500;
  margin-bottom: 12px;
  font-size: 18px;
}

.status-desc {
  font-size: 13px;
  opacity: 0.6;
  color: var(--color-text);
  line-height: 1.5;
}

.action-btn {
  border-radius: 12px;
  height: 48px;
  font-size: 16px;
  width: 100%;
  font-weight: 600;
}

.finish-btn {
  margin-top: auto;
}

.advanced-link {
  margin-top: 16px;
}

.icon-container {
  margin-bottom: 16px;
}

.success-anim {
  margin-top: 40px;
  margin-bottom: 24px;
  animation: popIn 0.6s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

/* Transitions */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.4s ease;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

@keyframes popIn {
  from { opacity: 0; transform: scale(0.5); }
  to { opacity: 1; transform: scale(1); }
}

.connectivity-box {
  margin-top: 20px;
  padding: 12px;
  background: rgba(255, 255, 255, 0.5);
  border-radius: 8px;
  width: 100%;
  font-size: 13px;
  color: var(--color-text);
  border: 1px solid rgba(0,0,0,0.05);
}

.conn-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
  gap: 8px;
}
.conn-row:last-child {
  margin-bottom: 0;
}

.source-url {
  opacity: 0.6;
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}

.status-row {
  justify-content: start;
}

.inline-loading {
  display: inline-block;
  margin-right: 6px;
  vertical-align: middle;
}

.conn-success {
  color: var(--color-success);
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
  margin-right: auto;
}

.conn-failed {
  color: var(--color-danger);
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
  margin-right: auto;
}
</style>
