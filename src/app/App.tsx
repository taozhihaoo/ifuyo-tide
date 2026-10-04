import { useCallback, useEffect, useRef, useState } from "react";
import { Sidebar, type ViewId } from "../components/Sidebar";
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
  const sessionRef = useRef<PlaybackSessionInfo | null>(null);

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

        <main className="content">
          {view === "downloads" && <DownloadsView onPlay={playSource} />}
          {view === "torrents" && <TorrentsView onPlay={playSource} />}
          {view === "library" && <LibraryView onPlay={playSource} />}
          {view === "player" && <PlayerView session={session} onClose={closeSession} />}
          {view === "settings" && <SettingsView />}
          {view === "spike" && <SpikeView />}
        </main>
      </div>
    </div>
  );
}
