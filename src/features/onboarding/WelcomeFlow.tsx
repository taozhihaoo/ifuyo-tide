import { useState } from "react";
import { setDownloadDirectory } from "../../lib/api";
import {
  getOnboardingCompleted,
  setOnboardingCompleted,
} from "./onboardingApi";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import { useStrings } from "../../lib/useStrings";

/**
 * M9(上) §6 — first-launch welcome flow: confirm the download
 * directory in one step, then enter the main interface. No accounts,
 * no tutorial, no marketing (§6 forbidden list). Shown exactly once —
 * `onboarding_completed` persists in the settings KV, and the flow is
 * skipped when settings already exist (§6 "已有设置则跳过").
 */
export function WelcomeFlow({ onDone }: { onDone: () => void }) {
  const strings = useStrings();
  const [downloadDir, setDownloadDir] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const finish = async (confirmedDir: string | null) => {
    setBusy(true);
    setError(null);
    try {
      if (confirmedDir) {
        await setDownloadDirectory(confirmedDir);
      }
      await setOnboardingCompleted();
      onDone();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };

  const choose = async () => {
    const picked = await pickDirectory({
      directory: true,
      multiple: false,
      title: "Choose download directory",
    });
    if (picked !== null) setDownloadDir(picked);
  };

  return (
    <div className="app welcome" role="dialog" aria-label="Welcome to Tide">
      <div className="welcome-card">
        <h1>{strings.welcome.title}</h1>
        <p className="welcome-tagline">{strings.welcome.tagline}</p>

        <p>
          Tide downloads files over HTTP and BitTorrent and lets you watch
          them while they download. Everything stays on your machine.
        </p>

        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">{strings.welcome.downloadDirectory}</span>
            <span className="setting-help">
              {downloadDir ?? strings.welcome.downloadDirectoryHelp}
            </span>
          </div>
          <button type="button" className="btn" onClick={() => void choose()}>
            Choose…
          </button>
        </div>

        {error && (
          <div className="banner banner-error" role="alert">
            {error}
          </div>
        )}

        <div className="task-actions">
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void finish(downloadDir)}
          >
            Get started
          </button>
        </div>
      </div>
    </div>
  );
}

/** Shared helpers for the shell: has onboarding been completed? */
export async function loadOnboardingState(): Promise<boolean> {
  return getOnboardingCompleted();
}
