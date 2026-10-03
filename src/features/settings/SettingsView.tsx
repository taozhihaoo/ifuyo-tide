import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getAppInfo, getSettings, setDownloadDirectory } from "../../lib/api";
import { isTideApiError } from "../../lib/ipc";
import type { AppInfoDto } from "../../lib/types";

/**
 * Settings: M0 implements only the download directory for real. Language
 * and start-on-boot are honest placeholders for M9 / M8 respectively.
 */
export function SettingsView() {
  const [downloadDir, setDownloadDir] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfoDto | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const settings = await getSettings();
        setDownloadDir(settings.downloadDir);
        setAppInfo(await getAppInfo());
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const chooseDirectory = useCallback(async () => {
    setError(null);
    setNotice(null);
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Choose download directory",
    });
    if (picked === null) {
      return; // user cancelled — not an error
    }
    try {
      const settings = await setDownloadDirectory(picked);
      setDownloadDir(settings.downloadDir);
      setNotice("Download directory saved.");
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  return (
    <section className="view" aria-label="Settings">
      <div className="view-header">
        <h2>Settings</h2>
      </div>

      {error && (
        <div className="banner banner-error" role="alert">
          {error}
        </div>
      )}
      {notice && (
        <div className="banner banner-ok" role="status">
          {notice}
        </div>
      )}

      <div className="settings-group">
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">Download directory</span>
            <span className="setting-help" data-testid="download-dir">
              {loading
                ? "Loading…"
                : (downloadDir ?? "Not set — choose where downloaded files will go.")}
            </span>
          </div>
          <button type="button" className="btn" disabled={loading} onClick={() => void chooseDirectory()}>
            Choose…
          </button>
        </div>

        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">Language</span>
            <span className="setting-help">English only in M0; localization arrives in M9.</span>
          </div>
          <select disabled defaultValue="en" aria-label="Language">
            <option value="en">English</option>
          </select>
        </div>

        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">Start on boot</span>
            <span className="setting-help">Disabled placeholder — Windows integration is M8.</span>
          </div>
          <input type="checkbox" disabled aria-label="Start on boot" />
        </div>
      </div>

      {appInfo && (
        <p className="dim app-info">
          {appInfo.appName} {appInfo.version} · {appInfo.milestone}
        </p>
      )}
    </section>
  );
}

function errorMessage(err: unknown): string {
  if (isTideApiError(err)) {
    return `${err.message} (${err.code})`;
  }
  return String(err);
}
