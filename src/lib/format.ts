/**
 * M9(上) — shared presentation utilities: ONE formatter set and ONE
 * state-terminology mapping for every view (§9/§11/§12/§60/§56).
 * All values come from real backend state — nothing invented (§10).
 */

/** Canonical user-facing task state (§9): one term per state across
 * all views — no "Running/Downloading/Active" synonyms. */
export type TaskState =
  | "created"
  | "ready"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export const STATE_LABELS: Record<TaskState, string> = {
  created: "Created",
  ready: "Ready",
  running: "Running",
  paused: "Paused",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
};

/** §11: speed display — one decimal, zero-state is "—", never
 * "0.0 MB/s" noise. Input is real backend bytes/sec. */
export function formatSpeed(bps: number | null | undefined): string {
  if (bps == null || !Number.isFinite(bps) || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

/** §12: ETA only with reliable data; never NaN/Infinity/absurd values
 * (§12 forbidden list). Paused/unknown → "—". */
export function formatEta(
  secs: number | null | undefined,
  state?: string,
): string {
  if (state === "paused") return "Paused";
  if (secs == null || !Number.isFinite(secs) || secs < 0) return "—";
  if (secs > 30 * 24 * 3600) return "—"; // absurd estimate → no ETA
  if (secs < 1) return "<1s";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

/** §60: byte formatting — one implementation everywhere. */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/** §59: long paths/names truncate with an ellipsis; callers render the
 * full value in `title` for accurate copy. */
export function truncateMiddle(value: string, max = 48): string {
  if (value.length <= max) return value;
  const keep = Math.floor((max - 1) / 2);
  return `${value.slice(0, keep)}…${value.slice(value.length - keep)}`;
}

/** §85: domain-error → user explanation + recovery action. Technical
 * codes stay in the collapsible details (§17). */
export function describeDownloadError(
  code: string | null | undefined,
): { message: string; recovery: string } {
  switch (code) {
    case "DOWNLOAD_HTTP_STATUS":
      return {
        message: "The server returned an error for this file.",
        recovery: "Retry, or check the link in your browser.",
      };
    case "DOWNLOAD_NETWORK":
      return {
        message: "The connection was interrupted.",
        recovery: "Retry — Tide resumes from where it stopped.",
      };
    case "DOWNLOAD_TIMEOUT":
      return {
        message: "The server stopped responding.",
        recovery: "Retry, or try again later.",
      };
    case "DOWNLOAD_CHECKSUM_MISMATCH":
      return {
        message: "The downloaded file is damaged (checksum mismatch).",
        recovery: "Retry to re-download; the broken file is kept for diagnosis.",
      };
    case "DOWNLOAD_DISK_FULL":
      return {
        message: "The disk is full.",
        recovery: "Free up space or choose another destination, then retry.",
      };
    case "DOWNLOAD_FILE_WRITE":
    case "DOWNLOAD_PERMISSION_DENIED":
      return {
        message: "The file could not be written.",
        recovery: "Check the destination folder permissions, then retry.",
      };
    case "DOWNLOAD_INVALID_URL":
      return {
        message: "This link is not a valid HTTP/HTTPS download URL.",
        recovery: "Check the link and start a new download.",
      };
    case "DOWNLOAD_PRIVATE_ADDRESS_BLOCKED":
      return {
        message: "This address points to a private network.",
        recovery: "Allow private addresses when creating the task if this is intended.",
      };
    default:
      return {
        message: "The download failed.",
        recovery: "Retry the download.",
      };
  }
}
