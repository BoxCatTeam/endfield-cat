import { invoke } from "@tauri-apps/api/core";

export type FetchMetadataArgs = {
  baseUrl: string;
  version: string;
};

export type HgExchangeResult = {
  oauth_token: string;
  uids: string[];
  role_ids: string[];
  nick_names: string[];
};

// 应用生命周期
export function quitApp() {
  return invoke("quit");
}

// 版本与元数据
export function getAppVersion() {
  return invoke<string>("get_app_version");
}

export function fetchLatestRelease<T = unknown>() {
  return invoke<T>("fetch_latest_release");
}

export function downloadAndApplyUpdate(downloadUrl: string) {
  return invoke("download_and_apply_update", { downloadUrl });
}

export function readConfig<T = any>() {
  return invoke<T>("read_config");
}

export function saveConfig(config: any) {
  return invoke("save_config", { config });
}

export function fetchMetadataManifest<T = unknown>(args: FetchMetadataArgs) {
  return invoke<T>("fetch_metadata_manifest", args);
}

export function resetMetadata(args: FetchMetadataArgs) {
  return invoke("reset_metadata", args);
}

export function checkMetadata<T = unknown>() {
  return invoke<T>("check_metadata");
}

// 明日方舟终末地相关命令
export function openHgTokenWebview() {
  return invoke("hg_open_token_webview");
}

export function exchangeHgUserToken(token: string) {
  return invoke<HgExchangeResult>("hg_exchange_user_token", { token });
}

export function getHgU8TokenByUid(params: { uid: string; oauthToken: string }) {
  return invoke<string>("hg_u8_token_by_uid", params);
}

// 数据库相关命令
export function dbListAccounts<T = unknown>() {
  return invoke<T>("db_list_accounts");
}

export function dbUpsertAccount(args: {
  uid: string;
  roleId?: string | null;
  nickName?: string | null;
  userToken: string;
  oauthToken: string;
  u8Token: string;
}) {
  return invoke("db_upsert_account", {
    uid: args.uid,
    roleId: args.roleId ?? null,
    nickName: args.nickName ?? null,
    userToken: args.userToken,
    oauthToken: args.oauthToken,
    u8Token: args.u8Token,
  });
}

export function dbDeleteAccount(uid: string) {
  return invoke("db_delete_account", { uid });
}

export function dbGetAccountTokens<T = unknown>(uid: string) {
  return invoke<T>("db_get_account_tokens", { uid });
}

export function dbDeleteInvalidGachaRecords(uid: string) {
  return invoke("db_delete_invalid_gacha_records", { uid });
}

export function dbSaveGachaRecords(uid: string, records: any[]) {
  return invoke("db_save_gacha_records", { uid, records });
}

export function dbListGachaPulls<T = unknown>(uid: string, limit = 200) {
  return invoke<T>("db_list_gacha_pulls", { uid, limit });
}
