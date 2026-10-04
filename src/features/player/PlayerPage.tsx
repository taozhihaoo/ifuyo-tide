import { useCallback, useEffect, useRef, useState } from "react";
import { PlaybackController, type PlayerSnapshot } from "./controller/PlaybackController";
import type { PlaybackError } from "./errorModel";
import { strings } from "../../lib/strings";
import {
  getPlaybackInfo,
  reportPlaybackPosition,
  stopPlaybackSession,
  type PlaybackSessionInfo,
} from "./playbackApi";
import { PlayerControls } from "./controls/PlayerControls";
import { StatusOverlay } from "./StatusOverlay";
import { useAutoHideControls } from "./hooks/useAutoHideControls";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";

/**
 * M5 §42 PlayerPage — the composition root of the player UI:
 * VideoSurface + StatusOverlay + ErrorState + Controls + ResourceInfo
 * (+ dev-only diagnostics). Owns the controller instance (created
 * once the <video> mounts); components render snapshots and delegate
 * every media action to the controller — they never touch the
 * element directly (§11).
 */
export function PlayerPage({
  session,
  onClose,
}: {
  session: PlaybackSessionInfo;
  onClose: () => void;
}) {
  const surfaceRef = useRef<HTMLDivElement | null>(null);
  const controllerRef = useRef<PlaybackController | null>(null);
  const [controller, setController] = useState<PlaybackController | null>(null);
  const [snapshot, setSnapshot] = useState<PlayerSnapshot | null>(null);
  const [fullscreen, setFullscreen] = useState(false);
  const [backendAvailableBytes, setBackendAvailableBytes] = useState<number | null>(
    session.availableBytes,
  );
  const adoptTargetRef = useRef<number>(session.lastPositionMs);

  // Create the controller exactly once, when the element mounts.
  const bindVideo = useCallback((el: HTMLVideoElement | null) => {
    if (el == null || controllerRef.current != null) return;
    const instance = new PlaybackController(el, {
      reportPosition: (sessionId, positionMs, durationMs, bufferedAheadSecs) => {
        void reportPlaybackPosition(sessionId, positionMs, durationMs, bufferedAheadSecs).catch(
          () => {},
        );
      },
    });
    controllerRef.current = instance;
    instance.subscribe(setSnapshot);
    setController(instance);
  }, []);

  // Open (or re-adopt after navigation/reload) the session exactly
  // once per session id. Fresh info gives the last reported position
  // for the §36 session-scoped resume.
  useEffect(() => {
    if (controller == null) return;
    let cancelled = false;
    getPlaybackInfo(session.sessionId)
      .then((fresh) => {
        if (cancelled || controller.getState() === "disposed") return;
        if (fresh && fresh.lastPositionMs > 0) adoptTargetRef.current = fresh.lastPositionMs;
        controller.adoptSession({
          sessionId: session.sessionId,
          url: fresh?.url ?? session.url,
          lastPositionMs: adoptTargetRef.current,
        });
      })
      .catch(() => {
        if (!cancelled && controller.getState() !== "disposed") {
          controller.adoptSession({
            sessionId: session.sessionId,
            url: session.url,
            lastPositionMs: adoptTargetRef.current,
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [controller, session.sessionId, session.url]);

  // Live backend availability (M4(下) §149), 1 s cadence — coarse by
  // design (§41/§73: no per-chunk UI churn).
  useEffect(() => {
    let cancelled = false;
    const timer = window.setInterval(() => {
      getPlaybackInfo(session.sessionId)
        .then((info) => {
          if (!cancelled && info) setBackendAvailableBytes(info.availableBytes);
        })
        .catch(() => {});
    }, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [session.sessionId]);

  const stop = useCallback(async () => {
    try {
      await stopPlaybackSession(session.sessionId);
    } finally {
      onClose();
    }
  }, [session.sessionId, onClose]);

  // Unmount cleanup: listeners/timers die with the controller (§87).
  useEffect(() => {
    return () => {
      controllerRef.current?.dispose();
      controllerRef.current = null;
    };
  }, []);

  const toggleFullscreen = useCallback(() => {
    const surface = surfaceRef.current;
    if (!surface) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    } else {
      void surface.requestFullscreen().catch(() => {
        setFullscreen(false); // refused — stay honest (M5 §47)
      });
    }
  }, []);

  useEffect(() => {
    const handler = () => setFullscreen(document.fullscreenElement != null);
    document.addEventListener("fullscreenchange", handler);
    return () => document.removeEventListener("fullscreenchange", handler);
  }, []);

  const playing = snapshot?.state === "playing";
  const { controlsVisible, notifyActivity } = useAutoHideControls(playing, true);

  useKeyboardShortcuts({
    onToggle: () => controller?.toggle(),
    onStep: (s) => controller?.seek((snapshot?.currentTime ?? 0) + s),
    onVolumeStep: (d) =>
      controller?.setVolume((snapshot?.muted ? 0 : (snapshot?.volume ?? 1)) + d),
    onMute: () => controller?.setMuted(!(snapshot?.muted ?? false)),
    onFullscreen: toggleFullscreen,
  });

  if (snapshot == null || controller == null) {
    return (
      <section className="view player-page" aria-label="Player">
        <div className="view-header">
          <h2>Player</h2>
        </div>
        <div className="video-shell">
          <div className="video-surface" ref={surfaceRef}>
            <video ref={bindVideo} data-testid="video-element" style={{ width: "100%", height: "100%", background: "#000" }} />
          </div>
        </div>
      </section>
    );
  }

  const error: PlaybackError | null = snapshot.error;

  return (
    <section className="view player-page" aria-label="Player">
      <div className="view-header">
        <h2>Player</h2>
        <span className="dim resource-info" data-testid="resource-info">
          {session.sourceKind} · {session.contentType} ·{" "}
          {session.size > 0 ? formatBytes(session.size) : "size unavailable"} · backend{" "}
          {formatBytes(backendAvailableBytes ?? session.availableBytes)} / browser buffered{" "}
          {snapshot.bufferedAhead != null ? `${snapshot.bufferedAhead.toFixed(1)}s` : "unavailable"}
        </span>
      </div>

      <div
        className={`video-shell${playing && !controlsVisible ? " controls-hidden" : ""}`}
        onMouseMove={notifyActivity}
        onMouseLeave={notifyActivity}
      >
        <div className="video-surface" ref={surfaceRef} data-testid="video-surface">
          <video
            ref={bindVideo}
            data-testid="video-element"
            controls={false}
            style={{ width: "100%", height: "100%", objectFit: "contain", background: "#000" }}
            onClick={() => controller.toggle()}
            onDoubleClick={toggleFullscreen}
          />
        </div>

        <StatusOverlay
          snapshot={snapshot}
          backendAvailableBytes={backendAvailableBytes}
          sizeBytes={session.size}
          waitingForDownload={session.sourceKind === "part" || session.sourceKind === "torrent"}
        />

        {error && (
          <div className="player-error" data-testid="player-error" role="alert">
            <p>{error.message}</p>
            <div className="task-actions">
              {error.recoverable && (
                <button type="button" className="btn btn-small" onClick={() => controller.play()}>
                  {strings.player.retry}
                </button>
              )}
              {/* §33: unsupported media → offer the OS default player as
                  the escape path (M8 opener capability already granted
                  through the backend's validated reveal command). */}
              {(error.kind === "UnsupportedMedia" || error.kind === "DecodeError") && (
                <button
                  type="button"
                  className="btn btn-small"
                  title="Open this file with the Windows default player"
                  onClick={() => {
                    void import("../../features/library/libraryApi").then((m) =>
                      // The library item id equals the playback owner for
                      // library-sourced sessions; downloads keep playing
                      // through Tide only.
                      m.revealItem(session.sessionId, false).catch(() => {}),
                    );
                  }}
                >
                  {strings.player.openInDefaultApp}
                </button>
              )}
              <button type="button" className="btn btn-small btn-danger" onClick={() => void stop()}>
                {strings.player.closePlayer}
              </button>
            </div>
          </div>
        )}

        <div className={`controls-host${controlsVisible || !playing ? "" : " hidden"}`}>
          <PlayerControls
            snapshot={snapshot}
            onToggle={() => controller.toggle()}
            onSeek={(t) => controller.seek(t)}
            onVolume={(v) => controller.setVolume(v)}
            onMute={() => controller.setMuted(!snapshot.muted)}
            onRate={(r) => controller.setPlaybackRate(r)}
            onFullscreen={toggleFullscreen}
            fullscreen={fullscreen}
          />
        </div>
      </div>

      {import.meta.env.DEV && (
        <DiagnosticsPanel
          session={session}
          snapshot={snapshot}
          backendAvailableBytes={backendAvailableBytes}
          controller={controller}
        />
      )}

      <div className="task-actions">
        <button type="button" className="btn btn-small btn-danger" onClick={() => void stop()}>
          Stop
        </button>
      </div>
    </section>
  );
}

/** Dev-only diagnostics (M5 §83): safe fields only — no tokens, no
 * paths, no magnets. Production builds render nothing. */
function DiagnosticsPanel({
  session,
  snapshot,
  backendAvailableBytes,
  controller,
}: {
  session: PlaybackSessionInfo;
  snapshot: PlayerSnapshot;
  backendAvailableBytes: number | null;
  controller: PlaybackController;
}) {
  const m = controller.metrics;
  return (
    <div className="player-diagnostics" data-testid="diagnostics-panel">
      <div>state: {snapshot.state}</div>
      <div>session: {session.sessionId.slice(0, 8)}…</div>
      <div>source kind: {session.sourceKind}</div>
      <div>
        time: {snapshot.currentTime?.toFixed(1) ?? "–"}s / {snapshot.duration?.toFixed(1) ?? "–"}s
      </div>
      <div>
        buffered ahead: {snapshot.bufferedAhead?.toFixed(1) ?? "–"}s · seekable:{" "}
        {snapshot.seekableDuration?.toFixed(1) ?? "–"}s
      </div>
      <div>backend available: {backendAvailableBytes ?? session.availableBytes} B</div>
      <div>
        TTFF: {m.timeToFirstFrameMs ?? "–"}ms · TTM: {m.timeToMetadataMs ?? "–"}ms
      </div>
      <div>
        buffering: {m.bufferingCount}× {m.stallDurationMs}ms · seeks: {m.seekCount} ({m.seekWaitMs}
        ms)
      </div>
      <div>
        errors: {m.errorCount} · reloads: {m.reloadCount}
      </div>
      <div>
        volume: {Math.round((snapshot.muted ? 0 : snapshot.volume) * 100)}% · rate:{" "}
        {snapshot.playbackRate}×
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
