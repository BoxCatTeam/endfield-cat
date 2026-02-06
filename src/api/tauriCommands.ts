import { invoke } from "@tauri-apps/api/core";

export type HgProvider = "hypergryph" | "gryphline";

export type FetchMetadataArgs = {
  baseUrl: string;
  version: string;
};

export type HgBindingEntry = {
  uid: string;
  roleId: string;
  nickName: string;
  serverId: string;
  channelMasterId?: number | null;
};

export type HgExchangeResult = {
  oauth_token: string;
  uids: string[];
  role_ids: string[];
  nick_names: string[];
  server_ids: string[];
  channel_master_ids?: Array<number | null>;
  bindings?: HgBindingEntry[];
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

export function fetchLatestPrerelease<T = unknown>() {
  return invoke<T>("fetch_latest_prerelease");
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

export function updateMetadata(baseUrl: string) {
  return invoke("update_metadata", { baseUrl });
}

export function checkMetadata<T = unknown>() {
  return invoke<T>("check_metadata");
}

// GitHub 镜像连通性测试
export function testGithubMirror(mirrorUrlTemplate: string) {
  return invoke<number>("test_github_mirror", { mirrorUrlTemplate });
}

// 明日方舟终末地相关命令
export function openHgTokenWebview(provider?: HgProvider) {
  return invoke("hg_open_token_webview", provider ? { provider } : {});
}

export function exchangeHgUserToken(token: string, provider?: HgProvider) {
  return invoke<HgExchangeResult>("hg_exchange_user_token", { token, provider });
}

export function getHgU8TokenByUid(params: { uid: string; oauthToken: string; provider?: HgProvider }) {
  return invoke<string>("hg_u8_token_by_uid", {
    uid: params.uid,
    oauthToken: params.oauthToken,
    provider: params.provider,
  });
}

export type HgLogGachaAuth = {
  u8Token: string;
  serverId: string;
  provider: string;
  inferredUid: string;
  channel?: string | null;
  subChannel?: string | null;
  sourcePath: string;
  sourceUrl: string;
};

export function hgGachaAuthFromLog(params?: { logPath?: string }) {
  return invoke<HgLogGachaAuth>("hg_gacha_auth_from_log", {
    logPath: params?.logPath,
  });
}

export type HgRoleListResult = {
  uid: string;
  roles: Array<{ roleId: string; nickName: string; serverName?: string | null }>;
  channelId?: number | null;
};

export function hgQueryRoleList(params: { token: string; serverId: string }) {
  return invoke<HgRoleListResult>("hg_query_role_list", params);
}

export type HgApiGachaRecord = {
  name: string;
  item_id: string;
  rarity: number;
  pool_id: string;
  pool_name: string;
  seq_id: string;
  pulled_at: number;
  pool_type: string;
  is_free: boolean;
  is_new: boolean;
};

export type HgWeaponPool = {
  pool_id: string;
  pool_name: string;
};

export function hgFetchCharRecords(params: {
  token: string;
  serverId: string;
  poolType: string;
  lastSeqIdStop?: string;
  provider?: HgProvider;
}) {
  return invoke<HgApiGachaRecord[]>("hg_fetch_char_records", params);
}

export function hgFetchWeaponPools(params: { token: string; serverId: string; provider?: HgProvider }) {
  return invoke<HgWeaponPool[]>("hg_fetch_weapon_pools", params);
}

export function hgFetchWeaponRecords(params: {
  token: string;
  serverId: string;
  poolId: string;
  lastSeqIdStop?: string;
  provider?: HgProvider;
}) {
  return invoke<HgApiGachaRecord[]>("hg_fetch_weapon_records", params);
}

// 数据库相关命令
export function dbListAccounts<T = unknown>() {
  return invoke<T>("db_list_accounts");
}

export function dbUpsertAccount(args: {
  uid: string;
  roleId?: string | null;
  nickName?: string | null;
  serverId?: string | null;
  channelId?: number | null;
  userToken?: string | null;
  oauthToken?: string | null;
  u8Token?: string | null;
}) {
  return invoke("db_upsert_account", {
    uid: args.uid,
    roleId: args.roleId ?? null,
    nickName: args.nickName ?? null,
    serverId: args.serverId ?? null,
    channelId: args.channelId ?? null,
    userToken: args.userToken ?? null,
    oauthToken: args.oauthToken ?? null,
    u8Token: args.u8Token ?? null,
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

// ───────────────────────────────────────────────────────────────────────────
// 同步相关命令 (后端直接处理 DB)
// ───────────────────────────────────────────────────────────────────────────

export type SyncResult = { count: number; accountUpdated: boolean };

export function syncGachaByToken(params: { uid: string; mode: "incremental" | "full" }) {
  return invoke<SyncResult>("sync_gacha_by_token", params);
}

export type LogSyncResult = { uid: string; count: number };

export function syncGachaFromLog(params: { logPath?: string; mode: "incremental" | "full" }) {
  return invoke<LogSyncResult>("sync_gacha_from_log", params);
}

export type AddedAccount = { uid: string; roleId: string; nickName: string; serverId: string };
export type AddAccountResult = { accounts: AddedAccount[] };

export function addAccountByToken(params: { userToken: string; provider?: HgProvider }) {
  return invoke<AddAccountResult>("add_account_by_token", params);
}
