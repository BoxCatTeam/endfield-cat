<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Snackbar } from "@varlet/ui";
import { listen } from "@tauri-apps/api/event";
import { isSqliteAvailable } from "../../db/db";
import { openHgTokenWebview } from "../../api/tauriCommands";
import { useI18n } from "vue-i18n";
// import type { HgProvider } from "../../api/tauriCommands"; // HgProvider is used in props, keep it?
import { addAccountByToken } from "../../api/tauriCommands";
import type { HgProvider } from "../../api/tauriCommands";

const { t } = useI18n();

const props = defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "success", uid: string): void;
}>();

const addAccountInput = ref("");
const addAccountLoading = ref(false);
const addAccountWebviewLoading = ref(false);
const provider = ref<HgProvider>("hypergryph");


const providerHelpText = computed(() =>
  provider.value === "gryphline"
    ? t("gacha.addAccount.helpGryphline")
    : t("gacha.addAccount.helpHypergryph")
);

const providerLoginBtnText = computed(() =>
  provider.value === "gryphline"
    ? t("gacha.addAccount.loginBtnGryphline")
    : t("gacha.addAccount.loginBtnHypergryph")
);

const addAccountConfirmDisabled = computed(
  () => addAccountLoading.value || !addAccountInput.value.trim()
);

function handleClose() {
  emit("update:show", false);
  // 重置对话框状态
  addAccountInput.value = "";
  addAccountLoading.value = false;
  addAccountWebviewLoading.value = false;
}

function normalizeUserToken(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  // JSON 字符串时尝试提取 user_token
  if (trimmed.startsWith("{")) {
    try {
      const data = JSON.parse(trimmed);
      return data.user_token || data.token || trimmed;
    } catch {
      return trimmed;
    }
  }
  return trimmed;
}

async function startWebviewTokenFlow() {
  addAccountWebviewLoading.value = true;
  try {
    await openHgTokenWebview(provider.value);
  } catch (err) {
    Snackbar.error((err as Error)?.message ?? t("gacha.addAccount.openLoginError"));
  } finally {
    addAccountWebviewLoading.value = false;
  }
}

async function onAddAccountConfirm() {
  const userToken = normalizeUserToken(addAccountInput.value);
  if (!userToken) {
    Snackbar.warning(t("gacha.addAccount.invalidToken"));
    return;
  }

  addAccountLoading.value = true;
  try {
    const res = await addAccountByToken({ userToken, provider: provider.value });
    
    if (res.accounts && res.accounts.length > 0) {
        const added = res.accounts;
        const names = added.map(a => a.serverId ? `${a.nickName}(${a.roleId}) · ${a.serverId}` : `${a.nickName}(${a.roleId})`).join("、");
        const count = added.length;
        
        Snackbar.success(
            count === 1
            ? t("gacha.addAccount.success", { name: names })
            : t("gacha.addAccount.successMultiple", { count, names })
        );
        emit("success", added[0].uid);
        handleClose();
    } else {
        throw new Error(t("gacha.addAccount.noUid"));
    }
  } catch (err) {
    Snackbar.error((err as Error)?.message ?? String(err));
  } finally {
    addAccountLoading.value = false;
  }
}

let unlistenAutoToken: null | (() => void) = null;

onMounted(async () => {
  if (!isSqliteAvailable()) return;

  unlistenAutoToken = await listen<string>("hg:auto-token", (event) => {
    addAccountInput.value = event.payload;
    if (!props.show) {
      emit("update:show", true);
    }
  });
});

onUnmounted(() => {
  if (unlistenAutoToken) unlistenAutoToken();
});
</script>

<template>
  <var-dialog
    :show="show"
    :title="t('gacha.addAccount.title')"
    :width="480"
    :confirm-button-text="t('gacha.addAccount.add')"
    :confirm-button-loading="addAccountLoading"
    :confirm-button-disabled="addAccountConfirmDisabled"
    :cancel-button-disabled="addAccountLoading"
    @confirm="onAddAccountConfirm"
    @closed="handleClose"
    @update:show="emit('update:show', $event)"
    style="--dialog-border-radius: 8px"
  >
    <var-space direction="column" :size="12">
      <var-button-group type="primary" size="large" :elevation="0" class="server-switcher">
        <var-button
          :type="provider === 'hypergryph' ? 'primary' : 'default'"
          :class="{ 'is-active': provider === 'hypergryph' }"
          @click="provider = 'hypergryph'"
        >
          {{ t("gacha.addAccount.providers.hypergryph") }}
        </var-button>
        <var-button
          :type="provider === 'gryphline' ? 'primary' : 'default'"
          :class="{ 'is-active': provider === 'gryphline' }"
          @click="provider = 'gryphline'"
        >
          {{ t("gacha.addAccount.providers.gryphline") }}
        </var-button>
      </var-button-group>

      <div class="add-help">
        {{ providerHelpText }}
      </div>

      <var-button type="primary" :loading="addAccountWebviewLoading" @click="startWebviewTokenFlow" block style="--button-border-radius: 4px; margin-top: 4px;">
        {{ providerLoginBtnText }}
      </var-button>

      <var-input
        v-model="addAccountInput"
        variant="outlined"
        :placeholder="t('gacha.addAccount.tokenPlaceholder')"
        size="small"
      />
    </var-space>
  </var-dialog>
</template>

<style scoped>
.add-help {
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-on-surface-variant);
  margin-top: 4px;
}

.server-switcher {
  --button-border-radius: 20px;
  display: flex;
}

.server-switcher :deep(.var-button) {
  flex: 1;
}

.server-switcher :deep(.is-active) {
  background-color: var(--color-primary-container) !important;
  color: var(--color-on-primary-container) !important;
  border-color: var(--color-primary) !important;
}

:deep(.var-dialog__confirm-button),
:deep(.var-dialog__cancel-button) {
    --button-border-radius: 4px;
}

:deep(.var-input) {
    --input-border-radius: 4px;
}
</style>
