import { invoke } from "@tauri-apps/api/core";
import { isSqliteAvailable } from "./db";

export type Account = {
  uid: string;
  roleId: string | null;  // For display (game character ID)
  nickName: string | null;
  updatedAt: number;
};

export type AccountWithTokens = {
  uid: string;
  roleId: string | null;
  nickName: string | null;
  userToken: string;
  oauthToken: string;
  u8Token: string;
};

export async function listAccounts(): Promise<Account[]> {
  if (!isSqliteAvailable) return [];
  return await invoke<Account[]>("db_list_accounts");
}

export async function upsertAccount(args: {
  uid: string;
  roleId?: string | null;
  nickName?: string | null;
  userToken: string;
  oauthToken: string;
  u8Token: string;
}) {
  await invoke("db_upsert_account", {
    uid: args.uid,
    roleId: args.roleId ?? null,
    nickName: args.nickName ?? null,
    userToken: args.userToken,
    oauthToken: args.oauthToken,
    u8Token: args.u8Token,
  });
}

export async function deleteAccount(uid: string) {
  await invoke("db_delete_account", { uid });
}

export async function getAccountTokens(uid: string): Promise<AccountWithTokens | null> {
  return await invoke<AccountWithTokens | null>("db_get_account_tokens", { uid });
}
