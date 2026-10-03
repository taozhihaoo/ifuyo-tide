import { useRef, useState } from "react";
import { notifySeek, stopPlaybackSession } from "./playbackApi";
import type { PlaybackSessionInfo } from "./playbackApi";

/**
 * The ONE real player (M4(上) §64/§73): every source (HTTP download,
 * torrent file, local file) plays through the same element and the same
 * gateway URL. All displayed values come from the media element's real
 * events — when the browser cannot provide a value we show "unavailable"
 * and never invent one (§65/§66).
 */
export function VideoPlayer({
  session,
  onClose,
}: {
  session: PlaybackSessionInfo;
  onClose: () => void;
}) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [state, setState] = useState({
    ready: false,
    playing: false,
    waiting: false,
    seeking: false,
    ended: false,
    error: null as string | null,
    currentTime: null as number | null,
    duration: null as number | null,
    // Real buffered-end of the range covering currentTime, in seconds.
    bufferedAhead: null as number | null,
  });

  const read = () => {
    const el = videoRef.current;
    if (!el) return;
    let bufferedAhead: number | null = null;
    for (let i = 0; i < el.buffered.length; i += 1) {
      if (el.buffered.start(i) <= el.currentTime && el.currentTime <= el.buffered.end(i)) {
        bufferedAhead = el.buffered.end(i) - el.currentTime;
        break;
      }
    }
    setState((prev) => ({
      ...prev,
      ready: el.readyState >= 1,
      currentTime: Number.isFinite(el.currentTime) ? el.currentTime : null,
      duration: Number.isFinite(el.duration) ? el.duration : null,
      bufferedAhead,
    }));
  };

  const stop = async () => {
    try {
      await stopPlaybackSession(session.sessionId);
    } finally {
      onClose();
    }
  };

  return (
    <section className="view" aria-label="Player">
      <div className="view-header">
        <h2>Player</h2>
        <span className="dim">
          {session.sourceKind} · {session.contentType} · {session.size >= 0 ? formatBytes(session.size) : "unavailable"}
        </span>
      </div>

      {state.error && (
        <div className="banner banner-error" role="alert">
          {state.error}
        </div>
      )}

      {/* Controls are the element's native UI; the buttons drive the same
          element. No simulated play state anywhere (M4(上) §64). */}
      <video
        ref={videoRef}
        data-testid="video-element"
        src={session.url}
        controls
        autoPlay
        style={{ width: "100%", maxHeight: "60vh", background: "#000" }}
        onLoadedMetadata={read}
        onCanPlay={read}
        onPlay={() => setState((p) => ({ ...p, playing: true, ended: false }))}
        onPause={() => setState((p) => ({ ...p, playing: false }))}
        onWaiting={() => setState((p) => ({ ...p, waiting: true }))}
        onPlaying={() => setState((p) => ({ ...p, waiting: false }))}
        onSeeking={() => setState((p) => ({ ...p, seeking: true }))}
        onSeeked={() => {
          setState((p) => ({ ...p, seeking: false }));
          // Obsolete demands for the old position are dropped (§80).
          void notifySeek(session.sessionId).catch(() => {});
          read();
        }}
        onTimeUpdate={read}
        onProgress={read}
        onEnded={() => setState((p) => ({ ...p, playing: false, ended: true }))}
        onError={() =>
          setState((p) => ({
            ...p,
            playing: false,
            error: "The browser cannot decode this resource (format/container unsupported).",
          }))
        }
        onStalled={read}
      />

      <div className="task-meta player-state" data-testid="player-state">
        <span>{state.playing ? "playing" : state.ended ? "ended" : "paused"}</span>
        {state.waiting && <span>buffering…</span>}
        {state.seeking && <span>seeking…</span>}
        <span>
          time:{" "}
          {state.currentTime != null ? formatTime(state.currentTime) : "unavailable"}
          {state.duration != null ? ` / ${formatTime(state.duration)}` : ""}
        </span>
        <span>
          buffered ahead:{" "}
          {state.bufferedAhead != null ? `${state.bufferedAhead.toFixed(1)}s` : "unavailable"}
        </span>
      </div>

      <div className="task-actions">
        <button type="button" className="btn btn-small" onClick={() => void videoRef.current?.play().catch(() => {})}>
          Play
        </button>
        <button type="button" className="btn btn-small" onClick={() => videoRef.current?.pause()}>
          Pause
        </button>
        <button
          type="button"
          className="btn btn-small"
          onClick={() => {
            const el = videoRef.current;
            if (el && Number.isFinite(el.duration)) {
              el.currentTime = el.duration / 2;
            }
          }}
        >
          Seek 50%
        </button>
        <span className="task-spacer" />
        <button type="button" className="btn btn-small btn-danger" onClick={() => void stop()}>
          Stop
        </button>
      </div>
    </section>
  );
}

function formatTime(secs: number): string {
  const s = Math.floor(secs % 60);
  const m = Math.floor((secs / 60) % 60);
  const h = Math.floor(secs / 3600);
  const mm = String(m).padStart(2, "0");
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${m}:${ss}`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
