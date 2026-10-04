import { useCallback, useEffect, useState } from "react";
import { EmptyState } from "../../components/EmptyState";
import * as api from "./downloadApi";
import {
  getSchedulerSnapshot,
  setTaskPriority,
  type SchedulerSnapshotDto,
  type UserPriority,
} from "../player/playbackApi";
import { getWindowsSettings } from "../settings/windowsApi";
import { notify } from "../settings/notify";
import {
  describeDownloadError,
  formatBytes as sharedFormatBytes,
  formatEta as sharedFormatEta,
  formatSpeed as sharedFormatSpeed,
} from "../../lib/format";
import type { DownloadDto } from "./downloadApi";

/**
 * M1 real-download view: URL → engine → live progress → pause/resume/
 * cancel. Every number comes from the Rust engine via the throttled
 * download://progress event — no fake progress anywhere.
 * M4(上): tasks with bytes on disk can play progressively through the
 * Tide gateway (the .part while downloading, the final file when done).
 */
export function DownloadsView({
  onPlay,
}: {
  onPlay?: (source: { kind: "download_task"; task_id: string }) => void;
}) {
  const [downloads, setDownloads] = useState<Map<string, DownloadDto>>(new Map());
  const [url, setUrl] = useState("");
  const [checksum, setChecksum] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  // M7 §76: dev-only scheduler debug panel — REAL snapshot data only.
  const [sched, setSched] = useState<SchedulerSnapshotDto | null>(null);

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

  // M8 §15: Windows notification on completion/failure — only when the
  // user's setting allows; a notification failure never fails the task
  // (§39). The download lifecycle itself is untouched.
  useEffect(() => {
    let unlisten: UnlistenLike | null = null;
    void api.onDownloadProgress((event) => {
      if (event.state === "completed" || event.state === "failed") {
        void getWindowsSettings()
          .then((settings) => {
            const name = event.filename ?? "Download";
            if (event.state === "completed" && settings.notifyDownloadCompleted) {
              void notify("Download completed", name);
            } else if (event.state === "failed" && settings.notifyDownloadFailed) {
              void notify("Download failed", name);
            }
          })
          .catch(() => {});
      }
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

  // M7 §76/§78: dev diagnostics at 1 s — throttled, real data only.
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const timer = window.setInterval(() => {
      getSchedulerSnapshot()
        .then(setSched)
        .catch(() => {});
    }, 1000);
    return () => window.clearInterval(timer);
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
            <DownloadCard key={d.downloadId} d={d} onPlay={onPlay} />
          ))}
        </ul>
      )}

      {import.meta.env.DEV && sched && <SchedulerDebug sched={sched} />}
    </section>
  );
}

/// M7 §76: dev-only scheduler debug — allocations + REAL decision
/// reasons from the backend snapshot. Never rendered in release.
function SchedulerDebug({ sched }: { sched: SchedulerSnapshotDto }) {
  if (sched.tasks.length === 0 && sched.decisions.length === 0) return null;
  return (
    <div className="player-diagnostics" data-testid="scheduler-debug">
      <div>global cap: {sched.globalBandwidthBps != null ? formatBytes(sched.globalBandwidthBps) + "/s" : "unlimited"}</div>
      {sched.tasks.map((t) => {
        const d = sched.decisions.find((x) => x.taskId === t.taskId);
        return (
          <div key={t.taskId}>
            {t.taskId.slice(0, 8)}… · {t.engine} · {t.state} · prio {t.userPriority ?? "—"} ·{" "}
            {formatBytes(t.smoothedBps)}/s
            {d != null && (
              <>
                {" "}
                → {d.priorityClass} {formatBytes(d.allocatedBps)}/s ({d.reasonCode}: {d.reasonDetail})
              </>
            )}
          </div>
        );
      })}
    </div>
  );
}

interface UnlistenLike {
  (): void;
}

const PRIORITIES: Array<{ value: UserPriority; label: string }> = [
  { value: "highest", label: "Highest" },
  { value: "high", label: "High" },
  { value: "normal", label: "Normal" },
  { value: "low", label: "Low" },
  { value: "background", label: "Background" },
];

function DownloadCard({
  d,
  onPlay,
}: {
  d: DownloadDto;
  onPlay?: (source: { kind: "download_task"; task_id: string }) => void;
}) {
  const active = d.state === "running" || d.state === "created";
  const paused = d.state === "paused";
  const playable = d.downloadedBytes > 0;
  const [priority, setPriority] = useState<UserPriority>("normal");
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
      {d.state === "failed" && d.errorCode && (
        <div className="dim" role="alert">
          {describeDownloadError(d.errorCode).message} — {describeDownloadError(d.errorCode).recovery}
          <details className="error-details">
            <summary>Technical details</summary>
            {d.errorMessage} ({d.errorCode})
          </details>
        </div>
      )}
      <div className="task-actions">
        {playable && onPlay && (
          <button
            type="button"
            className="btn btn-small"
            title="Play what has downloaded so far — streams through the Tide gateway"
            onClick={() => onPlay({ kind: "download_task", task_id: d.downloadId })}
          >
            Play
          </button>
        )}
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
        {active && (
          <select
            className="priority-select"
            aria-label="Download priority"
            value={priority}
            onChange={(e) => {
              const value = e.target.value as UserPriority;
              setPriority(value);
              void setTaskPriority(d.downloadId, value).catch(() => {});
            }}
          >
            {PRIORITIES.map((p) => (
              <option key={p.value} value={p.value}>
                {p.label}
              </option>
            ))}
          </select>
        )}
        {d.state === "failed" && (
          <button
            type="button"
            className="btn btn-small"
            onClick={() => void api.retryDownload(d.downloadId)}
          >
            Retry
          </button>
        )}
        {(d.state === "completed" || d.state === "failed" || d.state === "cancelled") && (
          <button
            type="button"
            className="btn btn-small btn-danger"
            onClick={() => void api.deleteDownload(d.downloadId)}
          >
            Delete
          </button>
        )}
        {d.state === "completed" && <span className="dim">done ✓</span>}
      </div>
    </li>
  );
}

// M9(上) §11/§12/§60: formatting moved to the shared lib (lib/format).
const formatBytes = (bytes: number) => sharedFormatBytes(bytes);
const formatSpeed = (bps: number) => sharedFormatSpeed(bps);
const formatEta = (secs: number) => sharedFormatEta(secs);
