<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Snackbar } from "@varlet/ui";
import { isSqliteAvailable } from "../db/db";
import BannerCard from "../components/gacha/BannerCard.vue";
import AddAccountDialog from "../components/gacha/AddAccountDialog.vue";
import SplitButtonSelect from "../components/SplitButtonSelect.vue";
import { useGachaStore } from "../stores/gacha";
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const store = useGachaStore();
const donutSize = 200;

const showAddAccount = ref(false);

function openAddAccountDialog() {
  if (!isSqliteAvailable()) {
    Snackbar.warning(t('gacha.addAccountTauriWarning'));
    return;
  }
  showAddAccount.value = true;
}

async function onAccountAdded(newUid: string) {
  // 新账号添加后选中该 UID 并拉取最新数据
  await store.reloadAccounts(newUid);
  await store.refreshGacha();
}

function notAvailable() {
  Snackbar.info(t('common.notAvailable'));
}

onMounted(() => {
  void store.reloadAccounts();
});
</script>

<template>
  <section class="page" :style="{ '--donut-size': donutSize + 'px' }">
    <header class="toolbar">
      <var-space class="toolbar-space" justify="space-between" align="center" :wrap="false" :size="8">
        <var-space align="center" :wrap="false" :size="6">
          <var-menu placement="bottom">
            <var-button text :loading="store.loading">
              <span class="btn-content">
                <var-icon name="refresh" />
                {{ t('gacha.refresh') }}
                <var-icon name="chevron-down" />
              </span>
            </var-button>
            <template #menu>
              <var-cell ripple @click="store.refreshGacha('incremental')">
                {{ t('gacha.refreshIncremental') }}
              </var-cell>
              <var-cell ripple @click="store.refreshGacha('full')">
                {{ t('gacha.refreshFull') }}
              </var-cell>
              <var-cell ripple @click="store.refreshGachaFromLog('incremental')">
                {{ t('gacha.refreshFromLog') }}
              </var-cell>
            </template>
          </var-menu>
          <var-button text @click="notAvailable">
            <span class="btn-content">
              <var-icon name="upload" />
              {{ t('gacha.importExport') }}
            </span>
          </var-button>
        </var-space>

        <var-space align="center" :wrap="false" :size="6">
          <SplitButtonSelect
            v-model="store.uid"
            :options="store.uidOptions"
            :disabled="store.uidOptions.length === 0"
            :placeholder="t('gacha.noAccount')"
          />
          <var-button text class="danger" :disabled="!store.canDeleteCurrentAccount" @click="store.deleteCurrentAccount">
            <var-icon name="trash-can-outline" />
          </var-button>
          <var-button text @click="openAddAccountDialog">
            <var-icon name="plus-circle-outline" />
          </var-button>
        </var-space>
      </var-space>
    </header>

    <div class="scroller">
      <AddAccountDialog
        v-model:show="showAddAccount"
        @success="onAccountAdded"
      />

      <var-collapse v-if="store.bannerSummary.length > 0" v-model="store.opened" class="collapse" :divider="false" :elevation="false">
        <var-collapse-item v-for="banner in store.bannerSummary" :key="banner.id" :name="banner.id" :title="banner.title">
          <BannerCard :banner="banner" :donut-size="donutSize" />
        </var-collapse-item>
      </var-collapse>

      <var-result
          v-else-if="store.uid && !store.loading"
          type="empty"
          class="empty-state"
          :title="t('gacha.messages.noRecords')"
          :description="t('gacha.messages.noRecordsHint')"
      >
      </var-result>
    </div>
  </section>
</template>

<style scoped>
.page {
  height: 100%;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  overflow: hidden;
}
.collapse{
  max-width: 1080px;
}
.toolbar {
  flex-shrink: 0;
  padding: 10px 14px;
  background: var(--color-toolbar-bg);
  border-bottom: 1px solid var(--color-border-subtle);
  z-index: 1;
}

.scroller {
  flex: 1;
  overflow-y: auto;
  padding: 14px 14px 40px;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar-thumb) transparent;
}

.scroller::-webkit-scrollbar {
  width: 8px;
}

.scroller::-webkit-scrollbar-track {
  background: transparent;
}

.scroller::-webkit-scrollbar-thumb {
  background-color: var(--color-scrollbar-thumb);
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: content-box;
}

.scroller::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-scrollbar-thumb-hover);
}

.toolbar-space {
  width: 100%;
}



.nickname {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text);
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.danger {
  color: var(--color-danger, #e53935);
}

.collapse :deep(.var-collapse-item) {
  border-radius: 12px;
  overflow: hidden;
  margin-bottom: 12px;
  background: var(--color-card-bg);
  border: 1px solid var(--color-border-subtle);
}

.collapse :deep(.var-collapse-item__title) {
  font-weight: 650;
}

.btn-content {
  display: flex;
  align-items: center;
  gap: 6px;
}

.empty-state {
  margin-top: 80px;
}
</style>
