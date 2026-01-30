<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAppStore } from '../../stores/app'

const appStore = useAppStore()
const router = useRouter()
const { t } = useI18n()

const finishSetup = async () => {
  await appStore.completeSetup()
  router.replace('/')
}
</script>

<template>
  <div class="success-card glass-panel">
      <var-space direction="column" align="center" justify="center" class="success-content">
        <div class="success-icon-wrapper">
          <var-icon name="check" size="64" color="#fff" />
        </div>
        <h2 class="success-title">{{ t('guide.readyTitle') }}</h2>
        <p class="success-desc">{{ t('guide.readySubtitle') }}</p>
      </var-space>
      
      <div class="action-footer">
        <var-button 
          type="primary" 
          block 
          size="large" 
          class="start-app-btn" 
          @click="finishSetup"
          :elevation="true"
        >
          {{ t('guide.enterApp') }}
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

.success-card {
  text-align: center;
  justify-content: center;
  min-height: 480px;
}

.success-content {
  flex: 1;
}

.success-icon-wrapper {
  width: 100px;
  height: 100px;
  border-radius: 50%;
  background: var(--color-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 32px;
  box-shadow: 0 10px 30px rgba(var(--hsl-primary), 0.4);
  animation: popIn 0.8s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

@keyframes popIn {
  0% { transform: scale(0); opacity: 0; }
  100% { transform: scale(1); opacity: 1; }
}

.success-title {
  font-size: 24px;
  margin-bottom: 8px;
  color: var(--color-text);
}

.success-desc {
  font-size: 16px;
  color: var(--color-text);
  opacity: 0.6;
}

.action-footer {
  margin-top: auto;
}

.start-app-btn {
  font-weight: 600;
  height: 52px;
  border-radius: 14px;
  box-shadow: 0 4px 14px 0 rgba(var(--hsl-primary), 0.39) !important;
}
</style>

