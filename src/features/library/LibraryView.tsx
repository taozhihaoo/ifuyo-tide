import { useCallback, useEffect, useState } from "react";
import { useStrings } from "../../lib/useStrings";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { EmptyState } from "../../components/EmptyState";
import {
  cancelScan,
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
  const strings = useStrings();
  const [roots, setRoots] = useState<LibraryRootDto[]>([]);
  const [page, setPage] = useState<ItemPageDto | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<(typeof FILTERS)[number]["key"]>("all");
  const [pageIndex, setPageIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [scanProgress, setScanProgress] = useState<ScanProgressDto | null>(null);
  const [scanning, setScanning] = useState(false);
  // M9(下) §110: honest restore state — Loading until first response.
  const [listLoaded, setListLoaded] = useState(false);

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
        setListLoaded(true);
        setError(null);
      } catch (err) {
        setListLoaded(true);
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
          progress.status === "cancelled"
            ? strings.libraryView.scanCancelled
            : strings.libraryView.scanFinished(
                progress.status,
                progress.filesSeen,
                progress.added,
                progress.missing,
                progress.errors,
              ),
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
    <section className="view" aria-label={strings.libraryView.title}>
      <div className="view-header">
        <h2>{strings.libraryView.title}</h2>
        <span className="dim" data-testid="library-roots">
          {anyRoot
            ? strings.libraryView.foldersEnabled(
                roots.filter((r) => r.enabled).length,
                roots.length,
              )
            : strings.libraryView.noFolders}
        </span>
        <span className="task-spacer" />
        <button
          type="button"
          className="btn btn-small"
          disabled={scanning || !anyRoot}
          title={anyRoot ? strings.libraryView.rescanTitle : strings.libraryView.addInSettingsTitle}
          onClick={() => void startScan()}
        >
          {scanning ? strings.libraryView.scanning : strings.common.scan}
        </button>
        {scanning && scanProgress && (
          <button
            type="button"
            className="btn btn-small btn-danger"
            aria-label={strings.libraryView.cancelScan}
            onClick={() => void cancelScan(scanProgress.rootId).catch(() => {})}
          >
            {strings.libraryView.cancel}
          </button>
        )}
      </div>

      {error && <div className="banner banner-error" role="alert">{error}</div>}
      {notice && <div className="banner banner-ok" role="status">{notice}</div>}
      {scanProgress?.status === "running" && (
        <div className="banner" role="status" data-testid="scan-progress">
          {strings.libraryView.scanningBanner(scanProgress.filesSeen)}
        </div>
      )}

      <input
        className="library-search"
        value={search}
        placeholder={strings.libraryView.searchPlaceholder}
        onChange={(e) => setSearch(e.target.value)}
        aria-label={strings.libraryView.searchAria}
      />

      <div className="library-filters" role="tablist" aria-label={strings.libraryView.filtersAria}>
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

      {!listLoaded ? (
        <EmptyState title={strings.libraryView.loadingTitle} hint={strings.libraryView.loadingHint} />
      ) : !anyRoot ? (
        <EmptyState
          title={strings.libraryView.noFoldersTitle}
          hint={strings.libraryView.noFoldersHint}
        />
      ) : !page || page.items.length === 0 ? (
        <EmptyState title={strings.libraryView.emptyTitle} hint={strings.libraryView.emptyHint} />
      ) : (
        <>
          <table className="library-table" data-testid="library-table">
            <thead>
              <tr>
                <th>{strings.libraryView.colName}</th>
                <th>{strings.libraryView.colType}</th>
                <th>{strings.libraryView.colSize}</th>
                <th>{strings.libraryView.colState}</th>
                <th>{strings.libraryView.colActions}</th>
              </tr>
            </thead>
            <tbody>
              {page.items.map((item) => (
                <LibraryRow
                  key={item.id}
                  item={item}
                  onPlay={onPlay}
                  onRefresh={() => void act(() => refreshItem(item.id), strings.libraryView.itemRefreshed)}
                  onRevealFile={() => void act(() => revealItem(item.id, false), "")}
                  onRevealFolder={() => void act(() => revealItem(item.id, true), "")}
                  onRemove={() => void act(() => removeItem(item.id), strings.library.removedNotice)}
                  onDelete={() => {
                    if (window.confirm(strings.libraryView.deleteConfirm(item.displayName))) {
                      void act(() => deleteItem(item.id), strings.libraryView.fileDeleted);
                    }
                  }}
                />
              ))}
            </tbody>
          </table>
          <div className="library-pager">
            <span className="dim">
              {strings.libraryView.itemsPaged(
                page.total,
                page.page + 1,
                Math.max(1, Math.ceil(page.total / page.pageSize)),
              )}
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
              {strings.common.prev}
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
              {strings.common.next}
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
  const strings = useStrings();
  const playable = item.availability === "available";
  const missing = item.availability === "missing";
  return (
    <tr className={`library-row availability-${item.availability}`} data-testid="library-row">
      <td className="library-name" title={item.relativePath}>
        {item.displayName}
      </td>
      <td>{item.category}</td>
      <td>{formatBytes(item.sizeBytes)}</td>
      <td>
        {/* §38: state text, never color-only; missing explains and offers
            actions rather than a bare "broken". */}
        <span className={`library-state state-${item.availability}`}>
          {missing ? strings.library.fileUnavailable : item.availability}
        </span>
        {missing && (
          <button type="button" className="btn btn-small" onClick={onRefresh} title={strings.libraryView.recheckTitle}>
            {strings.library.recheck}
          </button>
        )}
      </td>
      <td className="library-actions">
        <button
          type="button"
          className="btn btn-small"
          disabled={!playable}
          title={playable ? strings.libraryView.playTitle : strings.libraryView.notAvailableTitle}
          onClick={() => onPlay({ kind: "library_item", item_id: item.id })}
        >
          {strings.common.play}
        </button>
        <button type="button" className="btn btn-small" disabled={!playable} onClick={onRevealFile}>
          {strings.common.open}
        </button>
        <button type="button" className="btn btn-small" disabled={!playable} onClick={onRevealFolder}>
          {strings.common.folder}
        </button>
        <button type="button" className="btn btn-small" onClick={onRefresh}>
          {strings.common.refresh}
        </button>
        <button type="button" className="btn btn-small" onClick={onRemove}>
          {strings.common.remove}
        </button>
        <button type="button" className="btn btn-small btn-danger" onClick={onDelete}>
          {strings.libraryView.deleteFile}
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
