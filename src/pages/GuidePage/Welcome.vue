<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../../stores/app'
import logo from '../../assets/icon.webp'

const router = useRouter()
const { t } = useI18n()
const appStore = useAppStore()

const language = computed({
  get: () => appStore.language,
  set: (val) => appStore.language = val
})

const languages = [
  { value: 'zh-CN', native: '简体中文' },
  { value: 'en-US', native: 'English' },
]

const selectLanguage = (lang: string) => {
  language.value = lang
}

const nextStep = () => {
  router.push('/guide/disclaimer')
}
</script>

<template>
  <div class="welcome-card glass-panel">
    <var-space direction="column" align="center" class="welcome-header">
      <var-image :src="logo" width="120" class="logo animate-float" radius="24" />
      <h1 class="welcome-title">{{ t('guide.welcomeTitle') }}</h1>
      <p class="subtitle">{{ t('guide.welcomeSubtitle') }}</p>
    </var-space>

    <div class="language-section">
      <p class="section-label">{{ t('guide.selectLanguage') }}</p>
      <div class="language-list">
        <div
          v-for="lang in languages"
          :key="lang.value"
          class="language-option"
          :class="{ active: language === lang.value }"
          @click="selectLanguage(lang.value)"
          v-ripple
        >
          <span class="lang-native">{{ lang.native }}</span>
          <var-icon v-if="language === lang.value" name="check" class="check-icon" />
        </div>
      </div>
    </div>
    
    <div class="info-block">
      <p>{{ t('guide.welcomeIntro1') }}</p>
      <p>{{ t('guide.welcomeIntro2') }}</p>
    </div>

    <div class="action-footer">
      <var-button type="primary" block size="large" class="glow-btn" @click="nextStep" :elevation="true">
        {{ t('common.next') }}
      </var-button>
    </div>
  </div>
</template>

<style scoped>
.welcome-card{
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

.welcome-header {
  margin-bottom: 24px;
}

.logo {
  margin-bottom: 24px;
  box-shadow: 0 12px 24px rgba(0,0,0,0.12);
}

.animate-float {
  animation: float 6s ease-in-out infinite;
}

@keyframes float {
  0% { transform: translateY(0px); }
  50% { transform: translateY(-10px); }
  100% { transform: translateY(0px); }
}

.welcome-title {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 8px;
  background: linear-gradient(45deg, var(--color-primary), #ff8aae);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  color: var(--color-primary); 
}

.subtitle {
  font-size: 15px;
  color: var(--color-text);
  opacity: 0.7;
  margin: 0;
}

.language-section {
  margin-bottom: 24px;
}

.section-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text);
  opacity: 0.7;
  margin: 0 0 12px 0;
}

.language-list {
  display: flex;
  gap: 12px;
}

.language-option {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 14px 16px;
  background: var(--color-surface-container-low, rgba(255,255,255,0.5));
  border: 2px solid transparent;
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.language-option:hover {
  background: var(--color-surface-container-high, rgba(255,255,255,0.8));
}

.language-option.active {
  border-color: var(--color-primary);
  background: rgba(var(--hsl-primary), 0.08);
}

.lang-native {
  font-size: 15px;
  font-weight: 500;
  color: var(--color-text);
}

.check-icon {
  color: var(--color-primary);
  font-size: 18px;
}

.info-block {
  background: var(--color-card-bg);
  padding: 20px;
  border-radius: 16px;
  margin-bottom: 24px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--color-text);
  border: 1px solid var(--color-border-subtle);
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
