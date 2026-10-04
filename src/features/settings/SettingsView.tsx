import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getAppInfo, getSettings, setDownloadDirectory } from "../../lib/api";
import { isTideApiError } from "../../lib/ipc";
import type { AppInfoDto } from "../../lib/types";
import {
  addRoot,
  listRoots,
  removeRoot,
  setRootEnabled,
  type LibraryRootDto,
} from "../library/libraryApi";
import { setGlobalBandwidthLimit } from "../player/playbackApi";
import {
  autostartDisable,
  autostartEnable,
  autostartIsEnabled,
  getWindowsSettings,
  setWindowsSettings,
  type WindowsSettingsDto,
} from "../settings/windowsApi";

/**
 * Settings: the download directory (M0) and the M6(上) library
 * folders. Language and start-on-boot are honest placeholders for
 * M9 / M8 respectively.
 */
export function SettingsView() {
  const [downloadDir, setDownloadDir] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfoDto | null>(null);
  const [roots, setRoots] = useState<LibraryRootDto[]>([]);
  const [bandwidthInput, setBandwidthInput] = useState(0);
  const [win, setWin] = useState<WindowsSettingsDto | null>(null);
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const settings = await getSettings();
        setDownloadDir(settings.downloadDir);
        setAppInfo(await getAppInfo());
        setRoots(await listRoots());
        setWin(await getWindowsSettings());
        setAutostart(await autostartIsEnabled());
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const saveBandwidth = useCallback(async () => {
    setError(null);
    setNotice(null);
    try {
      // MB/s → bytes/sec; 0 = unlimited.
      const bps = Math.max(0, Math.round(bandwidthInput * 1024 * 1024));
      await setGlobalBandwidthLimit(bps);
      setNotice(
        bps === 0 ? "Bandwidth limit removed (unlimited)." : `Bandwidth limit set to ${bandwidthInput} MB/s.`,
      );
    } catch (err) {
      setError(errorMessage(err));
    }
  }, [bandwidthInput]);

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

  const addLibraryFolder = useCallback(async () => {
    setError(null);
    setNotice(null);
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Add a library folder",
    });
    if (picked === null) return;
    try {
      const root = await addRoot(picked);
      setRoots(await listRoots());
      setNotice(`Library folder added: ${root.displayName}. Press Scan in the Library to index it.`);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  const removeLibraryFolder = useCallback(async (id: string) => {
    setError(null);
    setNotice(null);
    try {
      await removeRoot(id);
      setRoots(await listRoots());
      setNotice("Library folder removed (files on disk are untouched).");
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  const toggleLibraryFolder = useCallback(async (id: string, enabled: boolean) => {
    setError(null);
    setNotice(null);
    try {
      await setRootEnabled(id, enabled);
      setRoots(await listRoots());
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

      <div className="settings-group">
        <div className="settings-group-title">Windows (M8)</div>
        {win && (
          <>
            <div className="setting-row">
              <div className="setting-text">
                <span className="setting-label">Start with Windows</span>
                <span className="setting-help">Off by default (§16.1). Toggles the OS login registration.</span>
              </div>
              <input
                type="checkbox"
                aria-label="Start with Windows"
                checked={autostart}
                onChange={(e) => {
                  const on = e.target.checked;
                  setAutostart(on);
                  void (on ? autostartEnable() : autostartDisable())
                    .then(() => setWin((w) => (w ? { ...w, autostartEnabled: on } : w)))
                    .catch((err) => {
                      setAutostart(!on);
                      setError(errorMessage(err));
                    });
                }}
              />
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <span className="setting-label">Close to tray</span>
                <span className="setting-help">
                  Closing the window hides Tide; downloads continue. Off = closing exits.
                </span>
              </div>
              <input
                type="checkbox"
                aria-label="Close to tray"
                checked={win.closeToTray}
                onChange={(e) => {
                  const next = { ...win, closeToTray: e.target.checked };
                  setWin(next);
                  void setWindowsSettings(next).catch((err) => {
                    setError(errorMessage(err));
                  });
                }}
              />
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <span className="setting-label">Notify: download completed</span>
              </div>
              <input
                type="checkbox"
                aria-label="Notify on download completed"
                checked={win.notifyDownloadCompleted}
                onChange={(e) => {
                  const next = { ...win, notifyDownloadCompleted: e.target.checked };
                  setWin(next);
                  void setWindowsSettings(next).catch(() => {});
                }}
              />
            </div>
            <div className="setting-row">
              <div className="setting-text">
                <span className="setting-label">Notify: download failed</span>
              </div>
              <input
                type="checkbox"
                aria-label="Notify on download failed"
                checked={win.notifyDownloadFailed}
                onChange={(e) => {
                  const next = { ...win, notifyDownloadFailed: e.target.checked };
                  setWin(next);
                  void setWindowsSettings(next).catch(() => {});
                }}
              />
            </div>
          </>
        )}
      </div>

      <div className="settings-group">
        <div className="settings-group-title">Bandwidth (M7)</div>
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">Global download limit</span>
            <span className="setting-help">
              MB/s — playback (emergency) may still use up to this cap. 0 = unlimited.
            </span>
          </div>
          <div className="task-actions">
            <input
              type="number"
              min={0}
              step={1}
              className="bandwidth-input"
              aria-label="Global download limit in megabytes per second"
              value={bandwidthInput}
              onChange={(e) => setBandwidthInput(Number(e.target.value) || 0)}
            />
            <button type="button" className="btn btn-small" onClick={() => void saveBandwidth()}>
              Apply
            </button>
          </div>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-group-title">Library folders (M6)</div>
        {roots.length === 0 ? (
          <div className="setting-row">
            <div className="setting-text">
              <span className="setting-help" data-testid="library-roots-empty">
                No library folders yet — finished downloads still enter the library automatically.
              </span>
            </div>
          </div>
        ) : (
          roots.map((root) => (
            <div className="setting-row" key={root.id} data-testid="library-root-row">
              <div className="setting-text">
                <span className="setting-label">{root.displayName}</span>
                <span className="setting-help">
                  {root.enabled ? "enabled" : "disabled"} · last scan:{" "}
                  {root.lastScanAt != null
                    ? new Date(root.lastScanAt).toLocaleString()
                    : "never"}{" "}
                  · {root.scanStatus}
                </span>
              </div>
              <div className="task-actions">
                <button
                  type="button"
                  className="btn btn-small"
                  onClick={() => void toggleLibraryFolder(root.id, !root.enabled)}
                >
                  {root.enabled ? "Disable" : "Enable"}
                </button>
                <button
                  type="button"
                  className="btn btn-small btn-danger"
                  onClick={() => void removeLibraryFolder(root.id)}
                >
                  Remove
                </button>
              </div>
            </div>
          ))
        )}
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-help">
              Explicit folders only — Tide never scans a whole drive. Download destination and
              library folders are independent concepts.
            </span>
          </div>
          <button type="button" className="btn" onClick={() => void addLibraryFolder()}>
            Add folder…
          </button>
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
