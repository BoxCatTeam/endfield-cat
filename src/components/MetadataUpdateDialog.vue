<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import { Snackbar } from '@varlet/ui'

const appStore = useAppStore()
const { t } = useI18n()

const currentVersion = computed(() => appStore.metadataStatus?.currentVersion || t('common.unknown'))
const remoteVersion = computed(() => appStore.metadataStatus?.remote?.packageVersion || '')

// 显示版本信息（如果有变化则显示箭头，否则只显示当前版本）
const dialogSubtitle = computed(() => {
  if (remoteVersion.value && currentVersion.value !== remoteVersion.value) {
    return `v${currentVersion.value} → v${remoteVersion.value}`
  }
  return `v${currentVersion.value}`
})

const progressLabel = computed(() => {
  const progress = appStore.metadataUpdateProgress
  if (!progress) return ''
  
  const phaseKey = `settings.metadata.phases.${progress.phase}`
  return `${t(phaseKey)} (${progress.current}/${progress.total})`
})

const progressPercent = computed(() => {
  const progress = appStore.metadataUpdateProgress
  if (!progress || progress.total === 0) return 0
  return Math.round((progress.current / progress.total) * 100)
})

const handleVerify = async () => {
  try {
    await appStore.performMetadataUpdate()
    Snackbar.success(t('settings.metadata.verifySuccess'))
  } catch (error) {
    console.error('Metadata verify failed:', error)
    Snackbar.error(t('settings.metadata.verifyFailed'))
  }
}
</script>

<template>
  <var-overlay
    v-model:show="appStore.showMetadataUpdateDialog"
    :lock-scroll="false"
    @click.self="appStore.dismissMetadataUpdateDialog"
  >
    <var-card
        :elevation="3"
        :title="t('settings.metadata.needsUpdate')"
        :subtitle="dialogSubtitle"
        style="min-width: 320px;"
    >
      <template #extra>
        <var-space direction="column" align="stretch" style="width: 100%;">
          <var-space direction="column" align="center" v-if="appStore.isMetadataUpdating" style="width: 100%;">
              <var-progress 
                :value="progressPercent" 
                :line-width="6"
                :label="true"
                :track="true"
                mode="circle"
                type="primary"
              />
            <p style="margin: 0 0 8px 0; font-size: 14px;">{{ progressLabel || t('settings.metadata.verifying') }}</p>
          </var-space>
          <var-space justify="flex-end">
            <var-button text @click="appStore.dismissMetadataUpdateDialog" :disabled="appStore.isMetadataUpdating">
              {{ t('settings.update.later') }}
            </var-button>
            <var-button
                type="primary"
                @click="handleVerify"
                :loading="appStore.isMetadataUpdating"
            >
              {{ t('settings.metadata.startVerify') }}
            </var-button>
          </var-space>
        </var-space>
      </template>
    </var-card>
  </var-overlay>
</template>

<style scoped>
</style>
