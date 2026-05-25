import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AnalysisReport,
  FlatCategory,
  ScanConfig,
  ScanEvent,
} from "./types";

/** Load the default bundled taxonomy. */
export async function loadTaxonomy(): Promise<FlatCategory[]> {
  const res = await invoke<{ categories: FlatCategory[] }>("load_taxonomy");
  return res.categories;
}

export async function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}

export async function validateConfig(config: ScanConfig): Promise<void> {
  await invoke("validate_config", { config });
}

export async function startScan(config: ScanConfig): Promise<AnalysisReport> {
  return invoke<AnalysisReport>("start_scan", { config });
}

export async function cancelScan(): Promise<void> {
  await invoke("cancel_scan");
}

export async function revealPath(path: string): Promise<void> {
  await invoke("reveal_path", { path });
}

/** Subscribe to scan progress events. Returns an unlisten function. */
export async function onScanProgress(
  callback: (event: ScanEvent) => void,
): Promise<UnlistenFn> {
  return listen<ScanEvent>("scan-progress", (e) => callback(e.payload));
}

/** Native folder picker. Returns null on cancel. */
export async function pickFolder(opts?: {
  title?: string;
  multiple?: boolean;
  defaultPath?: string;
}): Promise<string[] | string | null> {
  const result = await open({
    directory: true,
    multiple: !!opts?.multiple,
    title: opts?.title,
    defaultPath: opts?.defaultPath,
  });
  return result ?? null;
}
