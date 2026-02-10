import { open } from "@tauri-apps/plugin-dialog";

const TAURI_OK = typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);

export async function pickDirectory(options?: { title?: string; defaultPath?: string }) {
  if (!TAURI_OK) return null;

  const selected = await open({
    title: options?.title,
    defaultPath: options?.defaultPath,
    directory: true,
    multiple: false,
  });

  if (Array.isArray(selected)) return selected[0] ?? null;
  return selected ?? null;
}

