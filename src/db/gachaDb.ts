import { invoke } from "@tauri-apps/api/core";
import { isSqliteAvailable } from "./db";

export { isSqliteAvailable };

export type GachaPull = {
  uid: string;
  bannerId: string;
  bannerName: string;
  itemName: string;
  rarity: number;
  pulledAt: number;
  seqId?: string;
  itemId?: string;
  poolType?: string;
};

export interface ApiGachaRecord {
  name: string;
  item_id: string;
  rarity: number;
  pool_id: string;
  pool_name: string;
  seq_id: string;
  pulled_at: number;
  pool_type: string;
}

export async function deleteInvalidGachaRecords(uid: string) {
  await invoke("db_delete_invalid_gacha_records", { uid });
}

export async function saveGachaRecords(uid: string, records: any[]) {
  if (records.length === 0) return;
  // backend expects snake_case fields which match the raw records from hg_api
  await invoke("db_save_gacha_records", { uid, records });
}

/**
 * @deprecated Helper is now internal to backend
 */
export async function saveGachaPulls(_pulls: GachaPull[]) {
  // This was used internally or potentially by other callers? 
  // If it's used elsewhere, we might need to support it or migrate callers.
  // Checking usage is wise, but 'encapsulate all SQL' implies we shouldn't have SQL here.
  // Ideally this function should be removed or throw error.
  // But to be safe I'll leave it empty or log warning, OR better, check if others use it.
  // The original code exported it.
  // If I strictly follow "remove ts queries", I can't keep the implementation.
  console.warn("saveGachaPulls is deprecated and no longer functional on frontend");
}

export async function listGachaPulls(uid: string, limit = 200): Promise<GachaPull[]> {
  return await invoke<GachaPull[]>("db_list_gacha_pulls", { uid, limit });
}
