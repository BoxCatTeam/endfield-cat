<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const router = useRouter()
const { t, tm } = useI18n()
const disclaimerItems = computed(() => tm('common.disclaimer.items') as string[])

const nextStep = () => {
  router.push('/guide/resource')
}
</script>

<template>
  <div class="disclaimer-card glass-panel">
    <div class="disclaimer-header">
      <var-icon name="information-outline" size="48" class="disclaimer-icon" />
      <h1 class="disclaimer-title">{{ t('common.disclaimer.title') }}</h1>
    </div>

    <div class="disclaimer-content">
      <ul class="disclaimer-list">
        <li v-for="(line, idx) in disclaimerItems" :key="idx">{{ line }}</li>
      </ul>
    </div>

    <div class="action-footer">
      <var-button type="primary" block size="large" class="glow-btn" @click="nextStep">
        {{ t('guide.agreeAndContinue') }}
      </var-button>
    </div>
  </div>
</template>

<style scoped>
.disclaimer-card {
  margin-top: -25px;
  margin-bottom: 50px;
}

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

.disclaimer-header {
  text-align: center;
  margin-bottom: 24px;
}

.disclaimer-icon {
  color: var(--color-primary);
  margin-bottom: 12px;
}

.disclaimer-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0;
  color: var(--color-text);
}

.disclaimer-content {
  background: var(--color-surface-container-low, rgba(255,255,255,0.5));
  border-radius: 16px;
  padding: 20px;
  margin-bottom: 24px;
  max-height: 300px;
  overflow-y: auto;
}

.disclaimer-list {
  padding-left: 20px;
  margin: 0;
  font-size: 13px;
  line-height: 1.8;
  color: var(--color-text);
  opacity: 0.85;
}

.disclaimer-list li {
  margin-bottom: 12px;
}

.disclaimer-list li:last-child {
  margin-bottom: 0;
}

.action-footer {
  margin-top: auto;
}

.glow-btn {
  box-shadow: 0 4px 14px 0 rgba(var(--hsl-primary), 0.39) !important;
  font-weight: 600;
  border-radius: 14px;
  height: 52px;
}
</style>
