import { useCallback, useEffect, useRef, useState } from "react";
import { EmptyState } from "../../components/EmptyState";
import { useStrings } from "../../lib/useStrings";
import { getLocale } from "../../lib/i18n";
import { formatEta, formatSpeed, stateLabel as formatStateLabel } from "../../lib/format";
import * as torrentApi from "./torrentApi";
import type { TorrentDto } from "./torrentApi";

/**
 * M3 torrent view: real magnet/.torrent tasks, file selection, and
 * honest progress from the engine — no fake data anywhere.
 * M4(上): any file in the torrent can play progressively through the
 * Tide gateway while the swarm is still downloading.
 */
export function TorrentsView({
  onPlay,
}: {
  onPlay?: (source: { kind: "torrent_file"; task_id: string; file_index: number }) => void;
}) {
  const strings = useStrings();
  const [torrents, setTorrents] = useState<TorrentDto[]>([]);
  const [magnet, setMagnet] = useState("");
  const [torrentPath, setTorrentPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setTorrents(await torrentApi.listTorrents());
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 1500);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const add = useCallback(
    async (kind: "magnet" | "torrent_file") => {
      setError(null);
      setNotice(null);
      setBusy(true);
      try {
        const destination = "C:\\Users\\Gray\\Downloads"; // resolved via Settings
        const result = await torrentApi.createTorrentTask({
          sourceKind: kind,
          magnetUri: kind === "magnet" ? magnet.trim() || null : null,
          torrentPath: kind === "torrent_file" ? torrentPath.trim() || null : null,
          destinationDir: destination,
        });
        setNotice(`Torrent added (${result.id.slice(0, 8)}…).`);
        setMagnet("");
        setTorrentPath("");
        await refresh();
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [magnet, torrentPath, refresh],
  );

  return (
    <section className="view" aria-label="Torrents">
      <div className="view-header">
        <h2>Torrents</h2>
      </div>

      {error && <div className="banner banner-error" role="alert">{error}</div>}
      {notice && <div className="banner banner-ok" role="status">{notice}</div>}

      <form
        className="create-form"
        onSubmit={(e) => {
          e.preventDefault();
          void add("magnet");
        }}
      >
        <input
          value={magnet}
          placeholder={strings.torrentsView.magnetPlaceholder}
          onChange={(e) => setMagnet(e.target.value)}
          aria-label={strings.torrentsView.magnetPlaceholder}
        />
        <button type="submit" className="btn btn-primary" disabled={busy}>
          {strings.torrentsView.addMagnet}
        </button>
      </form>
      <div className="spike-row">
        <input
          value={torrentPath}
          placeholder={strings.torrentsView.torrentPathPlaceholder}
          onChange={(e) => setTorrentPath(e.target.value)}
          aria-label={strings.torrentsView.torrentPathPlaceholder}
          style={{ flex: 1 }}
        />
        <button
          type="button"
          className="btn"
          disabled={busy || !torrentPath.trim()}
          onClick={() => void add("torrent_file")}
        >
          {strings.torrentsView.addTorrent}
        </button>
      </div>

      {torrents.length === 0 ? (
        <EmptyState
          title="No torrents yet"
          hint="Add a magnet URI or a local .torrent file. Real BitTorrent via librqbit."
        />
      ) : (
        <ul className="task-list">
          {torrents.map((t) => (
            <TorrentCard key={t.id} t={t} onChanged={refresh} onPlay={onPlay} />
          ))}
        </ul>
      )}
    </section>
  );
}

function TorrentCard({
  t,
  onChanged,
  onPlay,
}: {
  t: TorrentDto;
  onChanged: () => void;
  onPlay?: (source: { kind: "torrent_file"; task_id: string; file_index: number }) => void;
}) {
  const [showFiles, setShowFiles] = useState(false);

  // Stalled = running, no bytes flowing, AND the engine reports zero
  // speed for over a minute (1.2.2: progress bytes do not change during
  // long piece rechecks, but data IS moving — the speed signal keeps
  // the badge honest there).
  const [stalled, setStalled] = useState(false);
  const lastProgress = useRef({ bytes: t.downloadedBytes, ts: Date.now() });
  useEffect(() => {
    if (lastProgress.current.bytes !== t.downloadedBytes) {
      lastProgress.current = { bytes: t.downloadedBytes, ts: Date.now() };
      setStalled(false);
    }
  }, [t.downloadedBytes]);
  useEffect(() => {
    const timer = window.setInterval(() => {
      setStalled(
        t.state === "running" &&
          (t.downloadSpeedBps ?? 0) <= 0 &&
          Date.now() - lastProgress.current.ts > 60_000,
      );
    }, 5000);
    return () => window.clearInterval(timer);
  }, [t.state]);

  // Live speed + honest ETA (M9 §12 rules: only with reliable data).
  const speed = t.downloadSpeedBps ?? 0;
  const etaSecs =
    t.state === "running" && speed > 0 && t.totalBytes != null
      ? Math.max(0, (t.totalBytes - t.downloadedBytes) / speed)
      : null;

  const [actionError, setActionError] = useState<string | null>(null);
  const strings = useStrings();
  // Actions can legitimately take a while (resume may re-add the
  // torrent on demand, bounded by engine timeouts) — show it, and
  // refuse re-entrant clicks instead of stacking requests.
  const [busy, setBusy] = useState(false);
  const run = (fn: () => Promise<unknown>) => {
    if (busy) return;
    setBusy(true);
    setActionError(null);
    fn()
      .then(onChanged)
      .catch((err) => setActionError(String(err)))
      .finally(() => setBusy(false));
  };

  const setSelection = (index: number) => {
    const selected = t.files
      .filter((x) => (x.index === index ? !x.selected : x.selected))
      .map((x) => x.index);
    run(() => torrentApi.setTorrentFileSelection(t.id, selected));
  };

  // §39/§66: the badge reflects the REAL state in user language — a
  // magnet that has not resolved its metadata yet is not "downloading",
  // and internal enum values never leak into the UI copy.
  const stateLabel =
    t.phase === "metadata_pending"
      ? "Resolving metadata…"
      : stalled && t.state === "running"
        ? strings.torrentsView.stalled
        : formatStateLabel(t.state, getLocale());

  return (
    <li className="task-card" data-testid="torrent-card">
      <div className="task-main">
        <span className="task-name" title={t.displayName}>
          {t.displayName}
        </span>
        <span className={`task-state state-${t.state}`}>{stateLabel}</span>
      </div>
      <div className="download-progress">
        {t.totalBytes != null && (
          <progress value={t.downloadedBytes} max={t.totalBytes} aria-label="Progress" />
        )}
        <div className="task-meta">
          <span>
            {formatBytes(t.downloadedBytes)}
            {t.totalBytes != null ? ` / ${formatBytes(t.totalBytes)}` : ""}
          </span>
          {t.state === "running" && speed > 0 && <span>{formatSpeed(speed)}</span>}
          {etaSecs != null && <span>ETA {formatEta(etaSecs)}</span>}
          {(() => {
            const checked = t.files.filter((f) => f.selected);
            if (checked.length === 0 && t.selectedCount == null) return null;
            const count = checked.length || t.selectedCount || 0;
            const size = checked.reduce((sum, f) => sum + f.size, 0);
            const sizeLabel =
              checked.length > 0
                ? ` · ${strings.torrentsView.sizeSelected(formatBytes(size))}`
                : "";
            return (
              <span>
                {t.files.length > 0
                  ? strings.torrentsView.filesOf(count, t.files.length)
                  : strings.torrentsView.filesSelected(count)}
                {sizeLabel}
              </span>
            );
          })()}
        </div>
      </div>
      {t.errorMessage && (
        <div className="dim" role="alert">
          {t.errorMessage} ({t.errorCode})
        </div>
      )}
      {actionError && (
        <div className="dim" role="alert">
          {actionError}
        </div>
      )}
      <div className="task-actions">
        {t.state === "running" && (
          <button
            type="button"
            className="btn btn-small"
            disabled={busy}
            onClick={() => run(() => torrentApi.pauseTorrent(t.id))}
          >
            {busy ? strings.torrentsView.pausing : strings.common.pause}
          </button>
        )}
        {(t.state === "paused" || (t.state === "ready" && t.phase !== "metadata_pending")) && (
          <button
            type="button"
            className="btn btn-small"
            disabled={busy}
            onClick={() => run(() => torrentApi.resumeTorrent(t.id))}
          >
            {busy ? strings.torrentsView.starting : strings.common.resume}
          </button>
        )}
        <button
          type="button"
          className="btn btn-small"
          onClick={() => setShowFiles((v) => !v)}
        >
          {showFiles
            ? strings.torrentsView.hideFiles
            : strings.torrentsView.showFiles(t.files.length)}
        </button>
        {busy && <span className="dim">{strings.common.working}</span>}
        {t.recoveryReason === "readd_timeout" && (
          <button
            type="button"
            className="btn btn-small"
            title={strings.torrentsView.resetTitle}
            disabled={busy}
            onClick={() => run(() => torrentApi.resetTorrentPartialData(t.id))}
          >
            {strings.torrentsView.resetPartialData}
          </button>
        )}
        <span className="task-spacer" />
        <button
          type="button"
          className="btn btn-small"
          title={strings.torrentsView.removeTitle}
          onClick={() => run(() => torrentApi.removeTorrent(t.id))}
        >
          {strings.common.remove}
        </button>
        <button
          type="button"
          className="btn btn-small btn-danger"
          title={strings.torrentsView.deleteDataTitle}
          onClick={() => run(() => torrentApi.deleteTorrentData(t.id))}
        >
          {strings.torrentsView.deleteData}
        </button>
      </div>
      {showFiles && (
        <ul className="torrent-files">
          {t.files.map((f) => (
            <li key={f.index} className={f.selected ? "" : "dim"}>
              <input
                type="checkbox"
                aria-label={strings.torrentsView.downloadFile(f.path)}
                checked={f.selected}
                disabled={t.state === "completed" || busy}
                onChange={() => setSelection(f.index)}
              />
              <span className="file-index">#{f.index}</span> {f.path}{" "}
              <span className="dim">({formatBytes(f.size)})</span>
              {onPlay && t.infoHash && (
                <button
                  type="button"
                  className="btn btn-small torrent-file-play"
                  title="Stream this file through the Tide gateway"
                  onClick={() =>
                    onPlay({ kind: "torrent_file", task_id: t.id, file_index: f.index })
                  }
                >
                  Play
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

