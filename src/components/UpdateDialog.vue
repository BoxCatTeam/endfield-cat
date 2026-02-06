<script setup lang="ts">
import { computed } from "vue";
import { useUpdaterStore } from "../stores/updater";
import { useI18n } from "vue-i18n";

const updater = useUpdaterStore();
const { t } = useI18n();

const version = computed(() => updater.localVersion || "");
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
            <var-button text type="primary" @click="updater.manualDownload()" :disabled="updater.isUpdating">
              {{ t('settings.update.manualDownload') }}
            </var-button>
            <template v-if="updater.altUpdateInfo">
              <var-button
                  type="primary"
                  @click="updater.installUpdate('primary')"
                  :loading="updater.isUpdating"
                  :disabled="updater.isUpdating || !updater.updateInfo?.download_url"
              >
                {{ t('settings.update.installStable') }}
              </var-button>
              <var-button
                  type="primary"
                  @click="updater.installUpdate('alt')"
                  :loading="updater.isUpdating"
                  :disabled="updater.isUpdating || !updater.altUpdateInfo?.download_url"
              >
                {{ t('settings.update.installPreview') }}
              </var-button>
            </template>
            <var-button
                v-else
                type="primary"
                @click="updater.installUpdate()"
                :loading="updater.isUpdating"
                :disabled="updater.isUpdating || !updater.updateInfo?.download_url"
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
