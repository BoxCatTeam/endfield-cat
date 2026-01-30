<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Snackbar } from "@varlet/ui";
import { isSqliteAvailable } from "../../db/db";
import { upsertAccount } from "../../db/accountDb";
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

type SelectOption = { label: string; value: string };
type HgExchangeResult = { oauth_token: string; uids: string[]; role_ids: string[]; nick_names: string[] };

const props = defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "success", uid: string): void;
}>();

const addAccountInput = ref("");
const addAccountStep = ref<"token" | "uid">("token");
const addAccountLoading = ref(false);
const addAccountWebviewLoading = ref(false);
const addAccountOauthToken = ref("");
const addAccountUidOptions = ref<SelectOption[]>([]);
const addAccountSelectedUid = ref("");

const addAccountConfirmDisabled = computed(() => {
  if (addAccountStep.value === "token") return !addAccountInput.value.trim();
  return !addAccountSelectedUid.value;
});

function handleClose() {
  emit("update:show", false);
  // Reset state
  addAccountInput.value = "";
  addAccountStep.value = "token";
  addAccountOauthToken.value = "";
  addAccountUidOptions.value = [];
  addAccountSelectedUid.value = "";
}

function normalizeUserToken(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  // If it's a JSON string, try to extract user_token
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
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("hg_open_token_webview");
  } catch (err) {
    Snackbar.error((err as Error)?.message ?? t('gacha.addAccount.openLoginError'));
  } finally {
    addAccountWebviewLoading.value = false;
  }
}

async function onAddAccountConfirm() {
  if (addAccountStep.value === "token") {
    const userToken = normalizeUserToken(addAccountInput.value);
    if (!userToken) {
      Snackbar.warning(t('gacha.addAccount.invalidToken'));
      return;
    }

    addAccountLoading.value = true;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const res = await invoke<HgExchangeResult>("hg_exchange_user_token", {
        token: userToken,
      });

      addAccountOauthToken.value = res.oauth_token;
      const uids = res.uids ?? [];
      const roleIds = res.role_ids ?? [];
      const nickNames = res.nick_names ?? [];

      if (uids.length === 0) {
        throw new Error(t('gacha.addAccount.noUid'));
      }

      // Populate options
      addAccountUidOptions.value = uids.map((uid, i) => {
        const roleId = roleIds[i] ?? uid;
        const nickName = nickNames[i] ?? "";
        const label = nickName ? `${nickName}(${roleId})` : roleId;
        return { label, value: uid };
      });
      addAccountSelectedUid.value = uids[0];

      if (uids.length > 1) {
        addAccountStep.value = "uid";
        return;
      }

      // Only one UID, finalize directly
      await finalizeAddAccount(userToken, uids[0], roleIds[0] || uids[0], nickNames[0] || "", res.oauth_token);
      handleClose();
    } catch (err) {
      Snackbar.error((err as Error)?.message ?? String(err));
    } finally {
      addAccountLoading.value = false;
    }
  } else {
    // Step 'uid'
    addAccountLoading.value = true;
    try {
      const userToken = normalizeUserToken(addAccountInput.value)!;
      const { invoke } = await import("@tauri-apps/api/core");
      
      // Re-exchange because we don't store everything in refs, ensures accurate metadata
      const res = await invoke<HgExchangeResult>("hg_exchange_user_token", {
        token: userToken,
      });

      const idx = res.uids.findIndex(u => u === addAccountSelectedUid.value);
      if (idx === -1) throw new Error(t('gacha.addAccount.invalidUid'));

      await finalizeAddAccount(
        userToken,
        addAccountSelectedUid.value,
        res.role_ids[idx] || addAccountSelectedUid.value,
        res.nick_names[idx] || "",
        res.oauth_token
      );
      handleClose();
    } catch (err) {
      Snackbar.error((err as Error)?.message ?? String(err));
    } finally {
      addAccountLoading.value = false;
    }
  }
}

async function finalizeAddAccount(userToken: string, uidValue: string, roleId: string, nickName: string, oauthToken: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  const u8Token = await invoke<string>("hg_u8_token_by_uid", {
    uid: uidValue,
    oauthToken: oauthToken,
  });

  await upsertAccount({
    uid: uidValue,
    roleId,
    nickName,
    userToken,
    oauthToken,
    u8Token,
  });

  emit("success", uidValue);
  const display = nickName ? `${nickName}(${roleId})` : roleId;
  Snackbar.success(t('gacha.addAccount.success', { name: display }));
}

let unlistenAutoToken: null | (() => void) = null;

onMounted(async () => {
  if (!isSqliteAvailable()) return;

  const { listen } = await import("@tauri-apps/api/event");
  unlistenAutoToken = await listen<string>("hg:auto-token", (event) => {
    addAccountInput.value = event.payload;
    addAccountStep.value = "token";
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
    :confirm-button-text="addAccountStep === 'token' ? t('gacha.addAccount.next') : t('gacha.addAccount.add')"
    :confirm-button-loading="addAccountLoading"
    :confirm-button-disabled="addAccountConfirmDisabled"
    :cancel-button-disabled="addAccountLoading"
    @confirm="onAddAccountConfirm"
    @closed="handleClose"
    @update:show="emit('update:show', $event)"
    style="--dialog-border-radius: 8px"
  >
    <var-space direction="column" :size="12">
      <div class="add-help">
        {{ t('gacha.addAccount.help') }}
      </div>

      <var-space v-if="addAccountStep === 'token'" align="center" :wrap="false" :size="10">
        <var-button type="primary" :loading="addAccountWebviewLoading" @click="startWebviewTokenFlow" style="--button-border-radius: 4px">
          {{ t('gacha.addAccount.loginBtn') }}
        </var-button>
      </var-space>

      <template v-if="addAccountStep === 'token'">
        <var-input
          v-model="addAccountInput"
          variant="outlined"
          :placeholder="t('gacha.addAccount.tokenPlaceholder')"
          size="small"
        />
      </template>

      <var-select
        v-else
        v-model="addAccountSelectedUid"
        variant="outlined"
        size="small"
        :placeholder="t('gacha.addAccount.selectUid')"
        :options="addAccountUidOptions"
        style="--select-border-radius: 4px"
      />
    </var-space>
  </var-dialog>
</template>

<style scoped>
.add-help {
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-on-surface-variant);
  margin-bottom: 4px;
}

:deep(.var-dialog__confirm-button),
:deep(.var-dialog__cancel-button) {
    --button-border-radius: 4px;
}

:deep(.var-input) {
    --input-border-radius: 4px;
}
</style>
