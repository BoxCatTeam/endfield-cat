import Database from "@tauri-apps/plugin-sql";
import i18n from "../i18n";

const { t } = i18n.global;

export const DB_URL = "sqlite:endcat.db";
export const STORAGE_OK = typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);

let dbPromise: Promise<Database> | null = null;

export function isSqliteAvailable() {
  return STORAGE_OK;
}

export async function getDb() {
  if (!STORAGE_OK) throw new Error(t("errors.sqliteUnavailable"));
  if (!dbPromise) dbPromise = Database.load(DB_URL);
  return dbPromise;
}
