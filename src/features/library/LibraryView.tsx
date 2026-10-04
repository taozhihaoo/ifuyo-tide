import { useCallback, useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { EmptyState } from "../../components/EmptyState";
import {
  deleteItem,
  listItems,
  listRoots,
  onScanProgress,
  refreshItem,
  removeItem,
  revealItem,
  scan,
  type ItemPageDto,
  type LibraryItemDto,
  type LibraryRootDto,
  type ScanProgressDto,
} from "./libraryApi";

const FILTERS = [
  { key: "all", label: "All" },
  { key: "video", label: "Video" },
  { key: "audio", label: "Audio" },
  { key: "image", label: "Image" },
  { key: "missing", label: "Missing" },
] as const;

const PAGE_SIZE = 50;

/**
 * M6(上) §54/§55: the Library view — search, filter, paged list,
 * per-item actions. Downloads answer "what tasks are running?";
 * the Library answers "what can Tide manage and play right now".
 * All state is a projection of the backend (§88): every action
 * re-queries.
 */
export function LibraryView({
  onPlay,
}: {
  onPlay: (source: { kind: "library_item"; item_id: string }) => void;
}) {
  const [roots, setRoots] = useState<LibraryRootDto[]>([]);
  const [page, setPage] = useState<ItemPageDto | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<(typeof FILTERS)[number]["key"]>("all");
  const [pageIndex, setPageIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [scanProgress, setScanProgress] = useState<ScanProgressDto | null>(null);
  const [scanning, setScanning] = useState(false);

  const query = useCallback(
    (idx: number) => ({
      search,
      category: filter === "all" || filter === "missing" ? undefined : filter,
      availability: filter === "missing" ? "missing" : undefined,
      sort: "name",
      ascending: true,
      page: idx,
      pageSize: PAGE_SIZE,
    }),
    [search, filter],
  );

  const refresh = useCallback(
    async (idx = pageIndex) => {
      try {
        const [rootsResult, pageResult] = await Promise.all([
          listRoots(),
          listItems(query(idx)),
        ]);
        setRoots(rootsResult);
        setPage(pageResult);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    },
    [pageIndex, query],
  );

  useEffect(() => {
    void refresh(0);
    setPageIndex(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, filter]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    void onScanProgress((progress) => {
      setScanProgress(progress);
      if (progress.status !== "running") {
        setScanning(false);
        setNotice(
          `Scan ${progress.status}: ${progress.filesSeen} files, +${progress.added} added, ${progress.missing} missing${
            progress.errors > 0 ? `, ${progress.errors} errors` : ""
          }.`,
        );
        void refresh();
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refresh]);

  const startScan = useCallback(async () => {
    setError(null);
    setNotice(null);
    setScanning(true);
    setScanProgress(null);
    try {
      await scan();
    } catch (err) {
      setScanning(false);
      setError(String(err));
    }
  }, []);

  const act = useCallback(
    async (action: () => Promise<unknown>, done: string) => {
      setError(null);
      setNotice(null);
      try {
        await action();
        if (done) setNotice(done);
        await refresh();
      } catch (err) {
        setError(String(err));
      }
    },
    [refresh],
  );

  const anyRoot = roots.length > 0;

  return (
    <section className="view" aria-label="Library">
      <div className="view-header">
        <h2>Library</h2>
        <span className="dim" data-testid="library-roots">
          {anyRoot
            ? `${roots.filter((r) => r.enabled).length} of ${roots.length} folders enabled`
            : "No library folders yet"}
        </span>
        <span className="task-spacer" />
        <button
          type="button"
          className="btn btn-small"
          disabled={scanning || !anyRoot}
          title={anyRoot ? "Rescan all enabled folders" : "Add a folder in Settings first"}
          onClick={() => void startScan()}
        >
          {scanning ? "Scanning…" : "Scan"}
        </button>
      </div>

      {error && <div className="banner banner-error" role="alert">{error}</div>}
      {notice && <div className="banner banner-ok" role="status">{notice}</div>}
      {scanProgress?.status === "running" && (
        <div className="banner" role="status" data-testid="scan-progress">
          Scanning… {scanProgress.filesSeen.toLocaleString()} files
        </div>
      )}

      <input
        className="library-search"
        value={search}
        placeholder="Search by name or path…"
        onChange={(e) => setSearch(e.target.value)}
        aria-label="Library search"
      />

      <div className="library-filters" role="tablist" aria-label="Library filters">
        {FILTERS.map((f) => (
          <button
            key={f.key}
            type="button"
            className={`btn btn-small${filter === f.key ? " btn-primary" : ""}`}
            onClick={() => setFilter(f.key)}
          >
            {f.label}
          </button>
        ))}
      </div>

      {!anyRoot ? (
        <EmptyState
          title="No library folders"
          hint="Add a folder under Settings → Library Folders, or complete a download — finished downloads enter the library automatically."
        />
      ) : !page || page.items.length === 0 ? (
        <EmptyState
          title="Nothing here yet"
          hint="Press Scan to index the folder, or complete a download."
        />
      ) : (
        <>
          <table className="library-table" data-testid="library-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
                <th>Size</th>
                <th>State</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {page.items.map((item) => (
                <LibraryRow
                  key={item.id}
                  item={item}
                  onPlay={onPlay}
                  onRefresh={() => void act(() => refreshItem(item.id), "Item refreshed.")}
                  onRevealFile={() => void act(() => revealItem(item.id, false), "")}
                  onRevealFolder={() => void act(() => revealItem(item.id, true), "")}
                  onRemove={() => void act(() => removeItem(item.id), "Removed from library (file kept).")}
                  onDelete={() => {
                    if (
                      window.confirm(
                        `Delete "${item.displayName}" from DISK? This cannot be undone. (Remove only takes it out of the library and keeps the file.)`,
                      )
                    ) {
                      void act(() => deleteItem(item.id), "File deleted.");
                    }
                  }}
                />
              ))}
            </tbody>
          </table>
          <div className="library-pager">
            <span className="dim">
              {page.total.toLocaleString()} items · page {page.page + 1} of{" "}
              {Math.max(1, Math.ceil(page.total / page.pageSize))}
            </span>
            <button
              type="button"
              className="btn btn-small"
              disabled={page.page === 0}
              onClick={() => {
                const idx = page.page - 1;
                setPageIndex(idx);
                void refresh(idx);
              }}
            >
              ← Prev
            </button>
            <button
              type="button"
              className="btn btn-small"
              disabled={(page.page + 1) * page.pageSize >= page.total}
              onClick={() => {
                const idx = page.page + 1;
                setPageIndex(idx);
                void refresh(idx);
              }}
            >
              Next →
            </button>
          </div>
        </>
      )}
    </section>
  );
}

function LibraryRow({
  item,
  onPlay,
  onRefresh,
  onRevealFile,
  onRevealFolder,
  onRemove,
  onDelete,
}: {
  item: LibraryItemDto;
  onPlay: (source: { kind: "library_item"; item_id: string }) => void;
  onRefresh: () => void;
  onRevealFile: () => void;
  onRevealFolder: () => void;
  onRemove: () => void;
  onDelete: () => void;
}) {
  const playable = item.availability === "available";
  return (
    <tr className={`library-row availability-${item.availability}`} data-testid="library-row">
      <td className="library-name" title={item.relativePath}>
        {item.displayName}
      </td>
      <td>{item.category}</td>
      <td>{formatBytes(item.sizeBytes)}</td>
      <td>
        <span className={`library-state state-${item.availability}`}>{item.availability}</span>
      </td>
      <td className="library-actions">
        <button
          type="button"
          className="btn btn-small"
          disabled={!playable}
          title={playable ? "Play through the Tide gateway" : "The file is not available"}
          onClick={() => onPlay({ kind: "library_item", item_id: item.id })}
        >
          Play
        </button>
        <button type="button" className="btn btn-small" disabled={!playable} onClick={onRevealFile}>
          Open
        </button>
        <button type="button" className="btn btn-small" disabled={!playable} onClick={onRevealFolder}>
          Folder
        </button>
        <button type="button" className="btn btn-small" onClick={onRefresh}>
          Refresh
        </button>
        <button type="button" className="btn btn-small" onClick={onRemove}>
          Remove
        </button>
        <button type="button" className="btn btn-small btn-danger" onClick={onDelete}>
          Delete file
        </button>
      </td>
    </tr>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
