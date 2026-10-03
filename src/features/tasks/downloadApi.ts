import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { call } from "../../lib/ipc";

// M1 download API — thin typed layer over the download IPC commands and
// the throttled download://progress event.

export interface DownloadDto {
  downloadId: string;
  host: string;
  filename: string | null;
  state: "created" | "running" | "paused" | "completed" | "failed" | "cancelled";
  phase: string;
  downloadedBytes: number;
  totalBytes: number | null;
  percentage: number | null;
  speedBps: number;
  etaSecs: number | null;
  errorCode: string | null;
  errorMessage: string | null;
}

export interface DownloadProgressEvent {
  downloadId: string;
  state: DownloadDto["state"];
  phase: string;
  filename: string | null;
  progress: {
    downloadedBytes: number;
    totalBytes: number | null;
    percentage: number | null;
    speedBps: number;
    etaSecs: number | null;
    elapsedMs: number;
  };
  error: { code: string; message: string } | null;
}

export function startDownload(options: {
  url: string;
  filename?: string | null;
  destinationDir?: string | null;
  overwritePolicy?: string | null;
  expectedChecksum?: string | null;
  allowPrivate?: boolean;
}): Promise<{ downloadId: string }> {
  return call<{ downloadId: string }>("start_http_download", {
    url: options.url,
    filename: options.filename ?? null,
    destinationDir: options.destinationDir ?? null,
    overwritePolicy: options.overwritePolicy ?? null,
    expectedChecksum: options.expectedChecksum ?? null,
    allowPrivate: options.allowPrivate ?? false,
  });
}

export function pauseDownload(downloadId: string): Promise<void> {
  return call<void>("pause_download", { downloadId });
}

export function resumeDownload(downloadId: string): Promise<void> {
  return call<void>("resume_download", { downloadId });
}

export function cancelDownload(downloadId: string): Promise<void> {
  return call<void>("cancel_download", { downloadId });
}

export function listDownloads(): Promise<DownloadDto[]> {
  return call<DownloadDto[]>("list_downloads");
}

export function getDownload(downloadId: string): Promise<DownloadDto> {
  return call<DownloadDto>("get_download", { downloadId });
}

/** Subscribes to the throttled backend progress event stream. */
export function onDownloadProgress(
  handler: (event: DownloadProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>("download://progress", (event) => {
    handler(event.payload);
  });
}
