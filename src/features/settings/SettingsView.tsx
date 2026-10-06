import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getLocale, setLocale, type Locale } from "../../lib/i18n";
import { useStrings } from "../../lib/useStrings";
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
  const strings = useStrings();
  const [locale, setLocaleState] = useState<Locale>(getLocale());
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
        bps === 0
          ? strings.settingsView.bandwidthRemoved
          : strings.settingsView.bandwidthSet(bandwidthInput),
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
      title: strings.settingsView.chooseDirectoryTitle,
    });
    if (picked === null) {
      return; // user cancelled — not an error
    }
    try {
      const settings = await setDownloadDirectory(picked);
      setDownloadDir(settings.downloadDir);
      setNotice(strings.settingsView.directorySaved);
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
      title: strings.settingsView.addLibraryFolderTitle,
    });
    if (picked === null) return;
    try {
      const root = await addRoot(picked);
      setRoots(await listRoots());
      setNotice(strings.settingsView.libraryFolderAdded(root.displayName));
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
      setNotice(strings.settingsView.libraryFolderRemoved);
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
    <section className="view" aria-label={strings.settingsView.title}>
      <div className="view-header">
        <h2>{strings.settingsView.title}</h2>
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
            <span className="setting-label">{strings.settingsView.downloadDirectory}</span>
            <span className="setting-help" data-testid="download-dir">
              {loading
                ? strings.settingsView.downloadDirectoryLoading
                : (downloadDir ?? strings.settingsView.downloadDirectoryUnset)}
            </span>
          </div>
          <button type="button" className="btn" disabled={loading} onClick={() => void chooseDirectory()}>
            {strings.common.choose}
          </button>
        </div>

        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">{strings.settingsView.language}</span>
            <span className="setting-help">{strings.settingsView.languageHelp}</span>
          </div>
          <select
            value={locale}
            onChange={(e) => {
              const next = e.target.value as Locale;
              setLocaleState(next);
              setLocale(next);
            }}
            aria-label={strings.settingsView.language}
          >
            <option value="en">English</option>
            <option value="zh-CN">简体中文</option>
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
        <div className="settings-group-title">{strings.settingsView.windowsGroup}</div>
        {win && (
          <>
            <div className="setting-row">
              <div className="setting-text">
                <span className="setting-label">{strings.settingsView.startOnBoot}</span>
                <span className="setting-help">{strings.settingsView.startOnBootHelp}</span>
              </div>
              <input
                type="checkbox"
                aria-label={strings.settingsView.startOnBoot}
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
                <span className="setting-label">{strings.settingsView.closeToTray}</span>
                <span className="setting-help">{strings.settingsView.closeToTrayHelp}</span>
              </div>
              <input
                type="checkbox"
                aria-label={strings.settingsView.closeToTray}
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
                <span className="setting-label">{strings.settingsView.notifyDownloadCompleted}</span>
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
                <span className="setting-label">{strings.settingsView.notifyDownloadFailed}</span>
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
        <div className="settings-group-title">{strings.settingsView.bandwidthGroup}</div>
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">{strings.settingsView.globalLimit}</span>
            <span className="setting-help">{strings.settingsView.globalLimitHelp}</span>
          </div>
          <div className="task-actions">
            <input
              type="number"
              min={0}
              step={1}
              className="bandwidth-input"
              aria-label={strings.settingsView.globalLimitAria}
              value={bandwidthInput}
              onChange={(e) => setBandwidthInput(Number(e.target.value) || 0)}
            />
            <button type="button" className="btn btn-small" onClick={() => void saveBandwidth()}>
              {strings.common.apply}
            </button>
          </div>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-group-title">{strings.settingsView.libraryGroup}</div>
        {roots.length === 0 ? (
          <div className="setting-row">
            <div className="setting-text">
              <span className="setting-help" data-testid="library-roots-empty">
                {strings.settingsView.noLibraryFolders}
              </span>
            </div>
          </div>
        ) : (
          roots.map((root) => (
            <div className="setting-row" key={root.id} data-testid="library-root-row">
              <div className="setting-text">
                <span className="setting-label">{root.displayName}</span>
                <span className="setting-help">
                  {root.enabled
                    ? strings.settingsView.enabled
                    : strings.settingsView.disabled}{" "}
                  · {strings.settingsView.lastScan}{" "}
                  {root.lastScanAt != null
                    ? new Date(root.lastScanAt).toLocaleString()
                    : strings.settingsView.never}{" "}
                  · {root.scanStatus}
                </span>
              </div>
              <div className="task-actions">
                <button
                  type="button"
                  className="btn btn-small"
                  onClick={() => void toggleLibraryFolder(root.id, !root.enabled)}
                >
                  {root.enabled ? strings.settingsView.disable : strings.settingsView.enable}
                </button>
                <button
                  type="button"
                  className="btn btn-small btn-danger"
                  onClick={() => void removeLibraryFolder(root.id)}
                >
                  {strings.common.remove}
                </button>
              </div>
            </div>
          ))
        )}
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-help">{strings.settingsView.libraryFoldersHelp}</span>
          </div>
          <button type="button" className="btn" onClick={() => void addLibraryFolder()}>
            {strings.settingsView.addFolder}
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
