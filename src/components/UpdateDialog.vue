<script setup lang="ts">
import { useUpdaterStore } from '../stores/updater'
import { useI18n } from 'vue-i18n'
import {getVersion} from "@tauri-apps/api/app";
import {onMounted, ref} from "vue";

const updater = useUpdaterStore()
const { t } = useI18n()

const version = ref('')

onMounted(async ()=>{
  version.value = await getVersion();
})
</script>

<template>
  <var-overlay
    v-model:show="updater.showUpdateDialog"
    :lock-scroll="false"
    @click.self="updater.dismissDialog"
  >
    <var-card
        :elevation="3"
        :title="t('settings.update.available')"
        :subtitle="'v'+ version+' → '+updater.updateInfo?.tag_name"
        :description="updater.updateInfo?.body"
    >
      <template #extra>
        <var-space direction="column" align="center">
          <var-space direction="column" align="center" v-if="updater.isUpdating">
            <var-loading type="wave" />
            <p>{{ t('settings.update.downloading') }}</p>
          </var-space>
          <var-space justify="flex-end">
            <var-button text @click="updater.dismissDialog" :disabled="updater.isUpdating">
              {{ t('settings.update.later') }}
            </var-button>
            <var-button text type="primary" @click="updater.manualDownload" :disabled="updater.isUpdating">
              {{ t('settings.update.manualDownload') }}
            </var-button>
            <var-button
                type="primary"
                @click="updater.installUpdate"
                :loading="updater.isUpdating"
                :disabled="!updater.updateInfo?.download_url"
            >
              {{ t('settings.update.installNow') }}
            </var-button>
          </var-space>
        </var-space>
      </template>
    </var-card>
  </var-overlay>
</template>

<style scoped>

</style>
