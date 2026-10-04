import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { call } from "../../lib/ipc";

// M6(上) library API — thin typed layer over the library IPC commands
// and the throttled library://scan progress event (§60). The renderer
// only ever handles item IDs; no path string is trusted or needed.

export interface LibraryRootDto {
  id: string;
  displayName: string;
  enabled: boolean;
  scanStatus: string;
  lastScanAt: number | null;
}

export interface LibraryItemDto {
  id: string;
  rootId: string;
  displayName: string;
  relativePath: string;
  extension: string | null;
  category: string;
  mimeType: string | null;
  sizeBytes: number;
  modifiedAt: number;
  availability: "available" | "missing" | "inaccessible";
  origin: "download" | "imported_local" | "unknown";
  taskId: string | null;
}

export interface ItemPageDto {
  items: LibraryItemDto[];
  total: number;
  page: number;
  pageSize: number;
}

export interface ItemQueryInput {
  search?: string;
  category?: string;
  availability?: string;
  origin?: string;
  sort?: string;
  ascending?: boolean;
  page?: number;
  pageSize?: number;
}

export interface ScanProgressDto {
  rootId: string;
  filesSeen: number;
  added: number;
  missing: number;
  errors: number;
  status: "running" | "ok" | "partial" | "failed";
}

export function listRoots(): Promise<LibraryRootDto[]> {
  return call<LibraryRootDto[]>("library_list_roots");
}

/** The picked directory string is validated server-side (canonicalized,
 * containment-checked) — never trusted here. */
export function addRoot(path: string): Promise<LibraryRootDto> {
  return call<LibraryRootDto>("library_add_root", { path });
}

export function removeRoot(rootId: string): Promise<boolean> {
  return call<boolean>("library_remove_root", { rootId });
}

export function setRootEnabled(rootId: string, enabled: boolean): Promise<void> {
  return call<void>("library_set_root_enabled", { rootId, enabled });
}

/** Starts a background scan; progress arrives via onScanProgress. */
export function scan(rootId?: string): Promise<void> {
  return call<void>("library_scan", { rootId: rootId ?? null });
}

export function listItems(query: ItemQueryInput): Promise<ItemPageDto> {
  return call<ItemPageDto>("library_list_items", {
    query: {
      search: query.search ?? null,
      category: query.category ?? null,
      availability: query.availability ?? null,
      origin: query.origin ?? null,
      sort: query.sort ?? null,
      ascending: query.ascending ?? true,
      page: query.page ?? 0,
      pageSize: query.pageSize ?? 50,
    },
  });
}

export function refreshItem(itemId: string): Promise<LibraryItemDto> {
  return call<LibraryItemDto>("library_refresh_item", { itemId });
}

/** §24: removes the index entry only — the file stays on disk. */
export function removeItem(itemId: string): Promise<boolean> {
  return call<boolean>("library_remove_item", { itemId });
}

/** §25: DESTRUCTIVE — deletes the file behind the item (backend
 * re-validates everything). */
export function deleteItem(itemId: string): Promise<void> {
  return call<void>("library_delete_item", { itemId });
}

export function revealItem(itemId: string, folder: boolean): Promise<void> {
  return call<void>("library_reveal", { itemId, folder });
}

/** Throttled scan progress (§40) — raw counts, never percentages. */
export function onScanProgress(
  handler: (progress: ScanProgressDto) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgressDto>("library://scan", (event) => {
    handler(event.payload);
  });
}
