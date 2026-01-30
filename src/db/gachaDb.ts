import { isSqliteAvailable } from "./db";
import { dbDeleteInvalidGachaRecords, dbListGachaPulls, dbSaveGachaRecords } from "../api/tauriCommands";

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
  await dbDeleteInvalidGachaRecords(uid);
}

export async function saveGachaRecords(uid: string, records: any[]) {
  if (records.length === 0) return;
  // 后端期望 snake_case，与 hg_api 原始字段保持一致
  await dbSaveGachaRecords(uid, records);
}

/**
 * @deprecated Helper is now internal to backend
 */
export async function saveGachaPulls(_pulls: GachaPull[]) {
  console.warn("saveGachaPulls 已弃用，前端不再实现");
}

export async function listGachaPulls(uid: string, limit = 200): Promise<GachaPull[]> {
  return await dbListGachaPulls<GachaPull[]>(uid, limit);
}
