import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { call } from "../../lib/ipc";

// M8 Windows-integration API (§80): settings persisted in the existing
// backend settings KV; autostart via the official plugin; launch
// requests arrive as `app://launch-request` events (§61) or via the
// pending-queue drain (§74).

export interface WindowsSettingsDto {
  closeToTray: boolean;
  autostartEnabled: boolean;
  notifyDownloadCompleted: boolean;
  notifyDownloadFailed: boolean;
  notifyTorrentCompleted: boolean;
}

export function getWindowsSettings(): Promise<WindowsSettingsDto> {
  return call<WindowsSettingsDto>("windows_get_settings");
}

export function setWindowsSettings(settings: WindowsSettingsDto): Promise<void> {
  return call<void>("windows_set_settings", { settings });
}

export function autostartEnable(): Promise<void> {
  return call<void>("windows_autostart_enable");
}

export function autostartDisable(): Promise<void> {
  return call<void>("windows_autostart_disable");
}

export function autostartIsEnabled(): Promise<boolean> {
  return call<boolean>("windows_autostart_is_enabled");
}

export function drainLaunchRequests(): Promise<LaunchRequestDto[]> {
  return call<LaunchRequestDto[]>("windows_drain_launch_requests");
}

export interface LaunchRequestDto {
  kind: LaunchRequestKind;
}

export type LaunchRequestKind =
  | { kind: "open_http_url"; value: string }
  | { kind: "open_magnet"; value: string }
  | { kind: "open_torrent_file"; value: string }
  | { kind: "open_media_file"; value: string }
  | { kind: "focus" };

/** §61: validated launch requests from the backend.
 *
 * The Rust router emits the whole `LaunchRequest` wrapper, i.e.
 * `{ kind: { kind: "open_magnet", value: "magnet:…" } }` — the same
 * shape `windows_drain_launch_requests` returns. Unwrap the wrapper
 * here so BOTH the live event and the startup drain deliver the inner
 * tagged union to the same handler. Passing the wrapper through would
 * leave `request.value` undefined and silently drop every real launch
 * (M8 §73/§74). */
export function onLaunchRequest(
  handler: (request: LaunchRequestKind) => void,
): Promise<UnlistenFn> {
  return listen<LaunchRequestDto>("app://launch-request", (event) => {
    handler(event.payload.kind);
  });
}
