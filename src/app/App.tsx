import { useEffect, useState } from "react";
import { Sidebar, type ViewId } from "../components/Sidebar";
import { LibraryView } from "../features/library/LibraryView";
import { PlayerView } from "../features/player/PlayerView";
import { SettingsView } from "../features/settings/SettingsView";
import { SpikeView } from "../features/spike/SpikeView";
import { DownloadsView } from "../features/tasks/DownloadsView";
import { spikeAutorunEnabled } from "../lib/spikeApi";

export default function App() {
  const [view, setView] = useState<ViewId>("downloads");

  // M0.5 SPIKE: hands-free experiment mode jumps straight to the spike
  // view so the autorun sequence mounts and executes.
  useEffect(() => {
    void spikeAutorunEnabled().then((enabled) => {
      if (enabled) setView("spike");
    });
  }, []);

  return (
    <div className="app">
      <Sidebar active={view} onNavigate={setView} />

      <div className="main-col">
        <header className="topbar">
          <span className="topbar-title">Tide</span>
          <span className="topbar-note">M0 · skeleton — downloads and playback are not implemented yet</span>
        </header>

        <main className="content">
          {view === "downloads" && <DownloadsView />}
          {view === "library" && <LibraryView />}
          {view === "player" && <PlayerView />}
          {view === "settings" && <SettingsView />}
          {view === "spike" && <SpikeView />}
        </main>
      </div>
    </div>
  );
}
