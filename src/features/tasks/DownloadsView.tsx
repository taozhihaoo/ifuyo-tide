import { useCallback, useEffect, useState } from "react";
import { EmptyState } from "../../components/EmptyState";
import * as api from "./downloadApi";
import type { DownloadDto } from "./downloadApi";

/**
 * M1 real-download view: URL → engine → live progress → pause/resume/
 * cancel. Every number comes from the Rust engine via the throttled
 * download://progress event — no fake progress anywhere.
 */
export function DownloadsView() {
  const [downloads, setDownloads] = useState<Map<string, DownloadDto>>(new Map());
  const [url, setUrl] = useState("");
  const [checksum, setChecksum] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    let unlisten: UnlistenLike | null = null;
    void api.onDownloadProgress((event) => {
      setDownloads((prev) => {
        const next = new Map(prev);
        const existing = next.get(event.downloadId);
        next.set(event.downloadId, {
          downloadId: event.downloadId,
          host: existing?.host ?? "",
          filename: event.filename ?? existing?.filename ?? null,
          state: event.state,
          phase: event.phase,
          downloadedBytes: event.progress.downloadedBytes,
          totalBytes: event.progress.totalBytes,
          percentage: event.progress.percentage,
          speedBps: event.progress.speedBps,
          etaSecs: event.progress.etaSecs,
          errorCode: event.error?.code ?? null,
          errorMessage: event.error?.message ?? null,
        });
        return next;
      });
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Initial snapshot (events may have been missed before mount).
  useEffect(() => {
    void api
      .listDownloads()
      .then((list) => {
        setDownloads((prev) => {
          const next = new Map(prev);
          for (const d of list) next.set(d.downloadId, d);
          return next;
        });
      })
      .catch(() => {});
  }, []);

  const start = useCallback(async () => {
    setError(null);
    setNotice(null);
    if (!url.trim()) {
      setError("Enter an HTTP/HTTPS URL.");
      return;
    }
    setStarting(true);
    try {
      const { downloadId } = await api.startDownload({
        url: url.trim(),
        expectedChecksum: checksum.trim() || null,
      });
      setNotice(`Download started (${downloadId.slice(0, 8)}…).`);
      setUrl("");
      setChecksum("");
      const snapshot = await api.getDownload(downloadId).catch(() => null);
      if (snapshot) {
        setDownloads((prev) => new Map(prev).set(downloadId, snapshot));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setStarting(false);
    }
  }, [url, checksum]);

  return (
    <section className="view" aria-label="Downloads">
      <div className="view-header">
        <h2>Downloads</h2>
      </div>

      {error && <div className="banner banner-error" role="alert">{error}</div>}
      {notice && <div className="banner banner-ok" role="status">{notice}</div>}

      <form
        className="create-form"
        onSubmit={(e) => {
          e.preventDefault();
          void start();
        }}
      >
        <input
          value={url}
          placeholder="HTTP/HTTPS URL"
          onChange={(e) => setUrl(e.target.value)}
          aria-label="Download URL"
        />
        <input
          value={checksum}
          placeholder="SHA-256 (optional)"
          onChange={(e) => setChecksum(e.target.value)}
          aria-label="Expected checksum"
          style={{ maxWidth: 220 }}
        />
        <button type="submit" className="btn btn-primary" disabled={starting}>
          Download
        </button>
      </form>

      {downloads.size === 0 ? (
        <EmptyState
          title="No downloads yet"
          hint="Paste an HTTP/HTTPS URL above. Real M1 downloads with pause/resume/retry."
        />
      ) : (
        <ul className="task-list">
          {[...downloads.values()].map((d) => (
            <DownloadCard key={d.downloadId} d={d} />
          ))}
        </ul>
      )}
    </section>
  );
}

interface UnlistenLike {
  (): void;
}

function DownloadCard({ d }: { d: DownloadDto }) {
  const active = d.state === "running" || d.state === "created";
  const paused = d.state === "paused";
  return (
    <li className="task-card" data-testid="download-card">
      <div className="task-main">
        <span className="task-name" title={d.filename ?? d.downloadId}>
          {d.filename ?? `${d.host}/…`}
        </span>
        <span className={`task-state state-${d.state}`}>{d.state}</span>
      </div>
      <div className="download-progress">
        <progress
          value={d.percentage ?? undefined}
          max={100}
          aria-label="Download progress"
        />
        <div className="task-meta">
          <span>
            {formatBytes(d.downloadedBytes)}
            {d.totalBytes != null ? ` / ${formatBytes(d.totalBytes)}` : " (unknown size)"}
          </span>
          <span>{formatSpeed(d.speedBps)}</span>
          <span>{d.etaSecs != null ? `ETA ${formatEta(d.etaSecs)}` : "ETA —"}</span>
          <span>{d.phase}</span>
        </div>
      </div>
      {d.errorMessage && (
        <div className="dim" role="alert">
          {d.errorMessage} ({d.errorCode})
        </div>
      )}
      <div className="task-actions">
        {active && (
          <button
            type="button"
            className="btn btn-small"
            onClick={() => void api.pauseDownload(d.downloadId)}
          >
            Pause
          </button>
        )}
        {paused && (
          <button
            type="button"
            className="btn btn-small"
            onClick={() => void api.resumeDownload(d.downloadId)}
          >
            Resume
          </button>
        )}
        {(active || paused) && (
          <button
            type="button"
            className="btn btn-small btn-danger"
            onClick={() => void api.cancelDownload(d.downloadId)}
          >
            Cancel
          </button>
        )}
        {d.state === "completed" && <span className="dim">done ✓</span>}
      </div>
    </li>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatSpeed(bps: number): string {
  if (bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

function formatEta(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}
