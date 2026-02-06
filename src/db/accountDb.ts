import { isSqliteAvailable } from "./db";
import { dbDeleteAccount, dbGetAccountTokens, dbListAccounts, dbUpsertAccount } from "../api/tauriCommands";

export type Account = {
  uid: string;
  roleId: string | null;  // 用于展示的角色 ID
  nickName: string | null;
  serverId: string | null;
  channelId: number | null;
  updatedAt: number;
};

export type AccountWithTokens = {
  uid: string;
  roleId: string | null;
  nickName: string | null;
  serverId: string | null;
  channelId: number | null;
  userToken: string | null;
  oauthToken: string | null;
  u8Token: string | null;
};

export async function listAccounts(): Promise<Account[]> {
  if (!isSqliteAvailable()) return [];
  return await dbListAccounts<Account[]>();
}

export async function upsertAccount(args: {
  uid: string;
  roleId?: string | null;
  nickName?: string | null;
  serverId?: string | null;
  channelId?: number | null;
  userToken?: string | null;
  oauthToken?: string | null;
  u8Token?: string | null;
}) {
  await dbUpsertAccount(args);
}

export async function deleteAccount(uid: string) {
  await dbDeleteAccount(uid);
}

export async function getAccountTokens(uid: string): Promise<AccountWithTokens | null> {
  return await dbGetAccountTokens<AccountWithTokens | null>(uid);
}
