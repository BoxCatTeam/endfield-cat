<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Snackbar } from "@varlet/ui";
import { listen } from "@tauri-apps/api/event";
import { isSqliteAvailable } from "../../db/db";
import { upsertAccount } from "../../db/accountDb";
import { useI18n } from 'vue-i18n';
import { exchangeHgUserToken, getHgU8TokenByUid, openHgTokenWebview } from "../../api/tauriCommands";

const { t } = useI18n();

type SelectOption = { label: string; value: string };

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
  // 重置对话框状态
  addAccountInput.value = "";
  addAccountStep.value = "token";
  addAccountOauthToken.value = "";
  addAccountUidOptions.value = [];
  addAccountSelectedUid.value = "";
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
    await openHgTokenWebview();
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
      const res = await exchangeHgUserToken(userToken);

      addAccountOauthToken.value = res.oauth_token;
      const uids = res.uids ?? [];
      const roleIds = res.role_ids ?? [];
      const nickNames = res.nick_names ?? [];

      if (uids.length === 0) {
        throw new Error(t('gacha.addAccount.noUid'));
      }

      // 填充账号选项
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

      // 仅有一个 UID 时直接完成
      await finalizeAddAccount(userToken, uids[0], roleIds[0] || uids[0], nickNames[0] || "", res.oauth_token);
      handleClose();
    } catch (err) {
      Snackbar.error((err as Error)?.message ?? String(err));
    } finally {
      addAccountLoading.value = false;
    }
  } else {
    // UID 选择步骤
    addAccountLoading.value = true;
    try {
      const userToken = normalizeUserToken(addAccountInput.value)!;
      // 再次换取 token，确保取到最新元数据
      const res = await exchangeHgUserToken(userToken);

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
  const u8Token = await getHgU8TokenByUid({
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
