import { useState } from "react";
import { Sidebar, type ViewId } from "../components/Sidebar";
import { LibraryView } from "../features/library/LibraryView";
import { PlayerView } from "../features/player/PlayerView";
import { TasksView } from "../features/tasks/TasksView";
import { SettingsView } from "../features/settings/SettingsView";
import { SpikeView } from "../features/spike/SpikeView";

export default function App() {
  const [view, setView] = useState<ViewId>("downloads");
  const [createRequest, setCreateRequest] = useState(0);

  return (
    <div className="app">
      <Sidebar active={view} onNavigate={setView} />

      <div className="main-col">
        <header className="topbar">
          <span className="topbar-title">Tide</span>
          <span className="topbar-note">M0 · skeleton — downloads and playback are not implemented yet</span>
          <button
            type="button"
            className="btn btn-primary"
            title="Add a test task (M0)"
            onClick={() => {
              setView("downloads");
              setCreateRequest((n) => n + 1);
            }}
          >
            + Add
          </button>
        </header>

        <main className="content">
          {view === "downloads" && <TasksView createRequest={createRequest} />}
          {view === "library" && <LibraryView />}
          {view === "player" && <PlayerView />}
          {view === "settings" && <SettingsView />}
          {view === "spike" && <SpikeView />}
        </main>
      </div>
    </div>
  );
}
