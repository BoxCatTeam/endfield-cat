import i18n from "../i18n";

const { t } = i18n.global;
const API_HOST = "https://ef-webview.hypergryph.com";
const CHAR_ENDPOINT = "/api/record/char";
const WEAPON_POOL_ENDPOINT = "/api/record/weapon/pool";
const WEAPON_RECORD_ENDPOINT = "/api/record/weapon";

export type GachaRecord = {
  name: string;
  rarity: number;
  poolId: string;
  poolName: string;
  seqId: string;
  pulledAt: number;
};

async function requestJson<T>(url: string) {
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) throw new Error(t("errors.requestFailed", { status: res.status }));
  return (await res.json()) as T;
}

export async function fetchCharRecords(params: {
  token: string;
  serverId: string;
  poolType: string;
  seqId?: string;
}) {
  const { token, serverId, poolType, seqId } = params;
  const url = new URL(API_HOST + CHAR_ENDPOINT);
  url.searchParams.set("token", token);
  url.searchParams.set("server_id", serverId);
  url.searchParams.set("lang", "zh-cn");
  url.searchParams.set("pool_type", poolType);
  if (seqId) url.searchParams.set("seq_id", seqId);

  type Resp = { code: number; data?: { list: any[]; hasMore: boolean }; msg?: string };
  const json = await requestJson<Resp>(url.toString());
  if (json.code !== 0) throw new Error(json.msg || t("errors.fetchGachaFailed"));
  const list = json.data?.list ?? [];
  return list.map<GachaRecord>((item: any) => ({
    name: item.charName || item.charId || "",
    rarity: item.rarity || 0,
    poolId: item.poolId || "",
    poolName: item.poolName || "",
    seqId: item.seqId || "",
    pulledAt: Number(item.gachaTs || 0),
  }));
}

export async function fetchWeaponPools(params: { token: string; serverId: string }) {
  const { token, serverId } = params;
  const url = new URL(API_HOST + WEAPON_POOL_ENDPOINT);
  url.searchParams.set("token", token);
  url.searchParams.set("server_id", serverId);
  url.searchParams.set("lang", "zh-cn");

  type Resp = { code: number; data?: Array<{ poolId: string; poolName: string }>; msg?: string };
  const json = await requestJson<Resp>(url.toString());
  if (json.code !== 0) throw new Error(json.msg || t("errors.fetchWeaponPoolFailed"));
  return json.data ?? [];
}

export async function fetchWeaponRecords(params: {
  token: string;
  serverId: string;
  poolId: string;
  seqId?: string;
}) {
  const { token, serverId, poolId, seqId } = params;
  const url = new URL(API_HOST + WEAPON_RECORD_ENDPOINT);
  url.searchParams.set("token", token);
  url.searchParams.set("server_id", serverId);
  url.searchParams.set("pool_id", poolId);
  url.searchParams.set("lang", "zh-cn");
  if (seqId) url.searchParams.set("seq_id", seqId);

  type Resp = { code: number; data?: { list: any[]; hasMore: boolean }; msg?: string };
  const json = await requestJson<Resp>(url.toString());
  if (json.code !== 0) throw new Error(json.msg || t("errors.fetchWeaponRecordsFailed"));
  const list = json.data?.list ?? [];
  return list.map<GachaRecord>((item: any) => ({
    name: item.weaponName || item.weaponId || "",
    rarity: item.rarity || 0,
    poolId: item.poolId || poolId,
    poolName: item.poolName || "",
    seqId: item.seqId || "",
    pulledAt: Number(item.gachaTs || 0),
  }));
}
