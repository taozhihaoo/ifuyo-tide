import { useCallback, useEffect, useRef, useState } from "react";
import { Sidebar, type ViewId } from "../components/Sidebar";
import { ViewErrorBoundary } from "../components/ViewErrorBoundary";
import { LibraryView } from "../features/library/LibraryView";
import { PlayerView } from "../features/player/PlayerView";
import {
  createPlaybackSession,
  describePlaybackError,
  getActivePlaybackSession,
  stopPlaybackSession,
  type PlaybackSessionInfo,
} from "../features/player/playbackApi";
import type { PlaybackSourceRequest } from "../features/player/playbackApi";
import { SettingsView } from "../features/settings/SettingsView";
import {
  drainLaunchRequests,
  onLaunchRequest,
  type LaunchRequestKind,
} from "../features/settings/windowsApi";
import { SpikeView } from "../features/spike/SpikeView";
import { DownloadsView } from "../features/tasks/DownloadsView";
import { TorrentsView } from "../features/torrents/TorrentsView";
import { spikeAutorunEnabled } from "../lib/spikeApi";

export default function App() {
  const [view, setView] = useState<ViewId>("downloads");
  // M4(上): at most one playback session for the whole app — ONE
  // gateway, ONE player (§73/§74). Views request playback through
  // playSource; the Player view renders it.
  const [session, setSession] = useState<PlaybackSessionInfo | null>(null);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  // M8 §45/§46: external sources enter the Add-Source flow — a
  // LaunchRequest is NOT a task; the user confirms before creation.
  const [notice, setNotice] = useState<string | null>(null);
  const [pendingSource, setPendingSource] = useState<{
    kind: string;
    value: string;
  } | null>(null);
  const sessionRef = useRef<PlaybackSessionInfo | null>(null);

  const intakeSource = useCallback((request: LaunchRequestKind) => {
    if (request.kind === "focus") return;
    const value = typeof request.value === "string" ? request.value : "";
    if (!value) return;
    setPendingSource({ kind: request.kind, value });
    setView("downloads");
  }, []);

  // M0.5 SPIKE: hands-free experiment mode jumps straight to the spike
  // view so the autorun sequence mounts and executes.
  useEffect(() => {
    spikeAutorunEnabled()
      .then((enabled) => {
        if (enabled) setView("spike");
      })
      .catch(() => {});
  }, []);

  // M5 §37: frontend reload recovery — React state is NOT the source
  // of truth. If the backend still has a playback session (e.g. the
  // window reloaded mid-play), adopt its projection.
  useEffect(() => {
    getActivePlaybackSession()
      .then((active) => {
        if (active && sessionRef.current == null) {
          sessionRef.current = active;
          setSession(active);
        }
      })
      .catch(() => {});
  }, []);

  const playSource = useCallback(async (source: PlaybackSourceRequest) => {
    setPlaybackError(null);
    try {
      // One session at a time: opening a new one stops the previous
      // (gateway access revoked, demands cancelled — M4(上) §55).
      const prev = sessionRef.current;
      if (prev) void stopPlaybackSession(prev.sessionId).catch(() => {});
      const info = await createPlaybackSession(source);
      sessionRef.current = info;
      setSession(info);
      setView("player");
    } catch (err) {
      // M4(下) §99: honest human-safe message from the structured code.
      setPlaybackError(describePlaybackError(err));
    }
  }, []);

  const closeSession = useCallback(() => {
    sessionRef.current = null;
    setSession(null);
  }, []);

  // M8 §73/§74: requests that arrived BEFORE the frontend was ready are
  // drained exactly once; live requests arrive via app://launch-request.
  useEffect(() => {
    drainLaunchRequests()
      .then((requests) => {
        for (const request of requests) intakeSource(request.kind);
      })
      .catch(() => {});
    let unlisten: import("@tauri-apps/api/event").UnlistenFn | null = null;
    onLaunchRequest(intakeSource)
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (unlisten) unlisten();
    };
  }, [intakeSource]);

  const confirmSource = useCallback(async () => {
    const source = pendingSource;
    if (!source) return;
    setPendingSource(null);
    try {
      if (source.kind === "open_http_url") {
        const downloads = await import("../features/tasks/downloadApi");
        const { downloadId } = await downloads.startDownload({
          url: source.value,
          allowPrivate: true,
        });
        setNotice("Download started (" + downloadId.slice(0, 8) + "\u2026).");
        setView("downloads");
      } else if (source.kind === "open_magnet" || source.kind === "open_torrent_file") {
        const torrents = await import("../features/torrents/torrentApi");
        const created = await torrents.createTorrentTask(
          source.kind === "open_magnet"
            ? { sourceKind: "magnet", magnetUri: source.value, destinationDir: "C:\\Users\\Gray\\Downloads" }
            : { sourceKind: "torrent_file", torrentPath: source.value, destinationDir: "C:\\Users\\Gray\\Downloads" },
        );
        setNotice("Torrent added (" + created.id.slice(0, 8) + "\u2026).");
        setView("torrents");
      } else if (source.kind === "open_media_file") {
        // Local media → Library intake (M6); playback via Library item.
        setNotice("Media file received — open it from the Library after import.");
        setView("library");
      }
    } catch (err) {
      setPlaybackError(String(err));
    }
  }, [pendingSource]);

  return (
    <div className="app">
      <Sidebar active={view} onNavigate={setView} />

      <div className="main-col">
        <header className="topbar">
          <span className="topbar-title">Tide</span>
          <span className="topbar-note">M4 · progressive streaming through the Tide gateway</span>
        </header>

        {playbackError && (
          <div className="banner banner-error" role="alert">
            {playbackError}
          </div>
        )}
        {notice && <div className="banner banner-ok" role="status">{notice}</div>}
        {pendingSource && (
          <div className="banner banner-ok" role="status">
            External source received.{" "}
            <button type="button" className="btn btn-small" onClick={() => void confirmSource()}>
              Add
            </button>{" "}
            <button type="button" className="btn btn-small" onClick={() => setPendingSource(null)}>
              Dismiss
            </button>
          </div>
        )}

        <main className="content">
          <ViewErrorBoundary viewName="Downloads">
            {view === "downloads" && <DownloadsView onPlay={playSource} />}
          </ViewErrorBoundary>
          <ViewErrorBoundary viewName="Torrents">
            {view === "torrents" && <TorrentsView onPlay={playSource} />}
          </ViewErrorBoundary>
          <ViewErrorBoundary viewName="Library">
            {view === "library" && <LibraryView onPlay={playSource} />}
          </ViewErrorBoundary>
          <ViewErrorBoundary viewName="Player">
            {view === "player" && (
              <PlayerView session={session} onClose={closeSession} />
            )}
          </ViewErrorBoundary>
          <ViewErrorBoundary viewName="Settings">
            {view === "settings" && <SettingsView />}
          </ViewErrorBoundary>
          {view === "spike" && <SpikeView />}
        </main>
      </div>
    </div>
  );
}
