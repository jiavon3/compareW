import { invoke } from "@tauri-apps/api/core";
import type { FolderRow, FolderSummary } from "../features/folder-compare/types";
import type { CompareSummary, DiffWindow } from "../features/text-compare/types";
import type { RowFilter } from "../features/text-compare/filterRows";

export function compareTexts(left: string, right: string): Promise<CompareSummary> {
  return invoke("compare_texts", { left, right });
}

export function getDiffRows(
  filter: RowFilter,
  offset: number,
  limit: number,
): Promise<DiffWindow> {
  return invoke("get_diff_rows", { filter, offset, limit });
}

export function readTextFile(path: string): Promise<string> {
  return invoke("read_text_file", { path });
}

export function startFolderCompare(left: string, right: string): Promise<FolderSummary> {
  return invoke("start_folder_compare", { left, right });
}

export function listFolderChildren(path: string[]): Promise<FolderRow[]> {
  return invoke("list_folder_children", { path });
}

export function readFolderEntry(side: "left" | "right", path: string[]): Promise<string> {
  return invoke("read_folder_entry", { side, path });
}

export function decompileClass(side: "left" | "right", path: string[]): Promise<string> {
  return invoke("decompile_class", { side, path });
}

export function javaAvailable(): Promise<boolean> {
  return invoke("java_available");
}

export function pickCompareRoot(): Promise<string | null> {
  return invoke("pick_compare_root");
}
