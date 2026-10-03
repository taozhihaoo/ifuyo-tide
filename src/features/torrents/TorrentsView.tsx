import { useCallback, useEffect, useState } from "react";
import { EmptyState } from "../../components/EmptyState";
import * as torrentApi from "./torrentApi";
import type { TorrentDto } from "./torrentApi";

/**
 * M3 torrent view: real magnet/.torrent tasks, file selection, and
 * honest progress from the engine — no fake data anywhere.
 */
export function TorrentsView() {
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
          placeholder="Magnet URI"
          onChange={(e) => setMagnet(e.target.value)}
          aria-label="Magnet URI"
        />
        <button type="submit" className="btn btn-primary" disabled={busy}>
          Add magnet
        </button>
      </form>
      <div className="spike-row">
        <input
          value={torrentPath}
          placeholder="Path to .torrent file"
          onChange={(e) => setTorrentPath(e.target.value)}
          aria-label="Torrent file path"
          style={{ flex: 1 }}
        />
        <button
          type="button"
          className="btn"
          disabled={busy || !torrentPath.trim()}
          onClick={() => void add("torrent_file")}
        >
          Add .torrent
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
            <TorrentCard key={t.id} t={t} onChanged={refresh} />
          ))}
        </ul>
      )}
    </section>
  );
}

function TorrentCard({ t, onChanged }: { t: TorrentDto; onChanged: () => void }) {
  const [showFiles, setShowFiles] = useState(false);

  return (
    <li className="task-card" data-testid="torrent-card">
      <div className="task-main">
        <span className="task-name" title={t.displayName}>
          {t.displayName}
        </span>
        <span className={`task-state state-${t.state}`}>{t.state}</span>
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
          {t.selectedCount != null && <span>{t.selectedCount} files selected</span>}
          {t.phase !== "downloading" && <span>{t.phase}</span>}
        </div>
      </div>
      {t.errorMessage && (
        <div className="dim" role="alert">
          {t.errorMessage} ({t.errorCode})
        </div>
      )}
      <div className="task-actions">
        {(t.state === "running" || t.state === "ready") && (
          <button
            type="button"
            className="btn btn-small"
            onClick={() => void torrentApi.pauseTorrent(t.id).then(onChanged)}
          >
            Pause
          </button>
        )}
        {(t.state === "paused" || t.state === "ready") && (
          <button
            type="button"
            className="btn btn-small"
            onClick={() => void torrentApi.resumeTorrent(t.id).then(onChanged)}
          >
            Resume
          </button>
        )}
        <button
          type="button"
          className="btn btn-small"
          onClick={() => setShowFiles((v) => !v)}
        >
          {showFiles ? "Hide files" : `Files (${t.files.length})`}
        </button>
        <span className="task-spacer" />
        <button
          type="button"
          className="btn btn-small"
          title="Remove from Tide — downloaded data is preserved"
          onClick={() => void torrentApi.removeTorrent(t.id).then(onChanged)}
        >
          Remove
        </button>
        <button
          type="button"
          className="btn btn-small btn-danger"
          title="Remove and delete downloaded data"
          onClick={() => void torrentApi.deleteTorrentData(t.id).then(onChanged)}
        >
          Delete data
        </button>
      </div>
      {showFiles && (
        <ul className="torrent-files">
          {t.files.map((f) => (
            <li key={f.index} className={f.selected ? "" : "dim"}>
              <span className="file-index">#{f.index}</span> {f.path}{" "}
              <span className="dim">({formatBytes(f.size)})</span>
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

