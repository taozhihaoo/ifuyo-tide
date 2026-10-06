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

const STATE_LABELS_ZH: Record<TaskState, string> = {
  created: "已创建",
  ready: "就绪",
  running: "下载中",
  paused: "已暂停",
  completed: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

/** 1.1.0 i18n: user-facing state term for the active locale. Unknown
 * states fall through raw (never invented). */
export function stateLabel(state: string, locale: "en" | "zh-CN"): string {
  if (locale === "zh-CN") {
    const hit = STATE_LABELS_ZH[state as TaskState];
    return hit ?? state;
  }
  return STATE_LABELS[state as TaskState] ?? state;
}

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
  locale: "en" | "zh-CN" = "en",
): { message: string; recovery: string } {
  const zh = locale === "zh-CN";
  switch (code) {
    case "DOWNLOAD_HTTP_STATUS":
      return zh
        ? {
            message: "服务器对该文件返回了错误。",
            recovery: "重试，或在浏览器中检查该链接。",
          }
        : {
            message: "The server returned an error for this file.",
            recovery: "Retry, or check the link in your browser.",
          };
    case "DOWNLOAD_NETWORK":
      return zh
        ? {
            message: "连接被中断。",
            recovery: "重试——Tide 会从停止处续传。",
          }
        : {
            message: "The connection was interrupted.",
            recovery: "Retry — Tide resumes from where it stopped.",
          };
    case "DOWNLOAD_TIMEOUT":
      return zh
        ? {
            message: "服务器停止了响应。",
            recovery: "重试，或稍后再试。",
          }
        : {
            message: "The server stopped responding.",
            recovery: "Retry, or try again later.",
          };
    case "DOWNLOAD_CHECKSUM_MISMATCH":
      return zh
        ? {
            message: "下载的文件已损坏（校验和不匹配）。",
            recovery: "重试以重新下载；损坏的文件会保留用于排查。",
          }
        : {
            message: "The downloaded file is damaged (checksum mismatch).",
            recovery: "Retry to re-download; the broken file is kept for diagnosis.",
          };
    case "DOWNLOAD_DISK_FULL":
      return zh
        ? {
            message: "磁盘空间不足。",
            recovery: "清理空间或更换目标目录后重试。",
          }
        : {
            message: "The disk is full.",
            recovery: "Free up space or choose another destination, then retry.",
          };
    case "DOWNLOAD_FILE_WRITE":
    case "DOWNLOAD_PERMISSION_DENIED":
      return zh
        ? {
            message: "文件写入失败。",
            recovery: "检查目标文件夹的权限后重试。",
          }
        : {
            message: "The file could not be written.",
            recovery: "Check the destination folder permissions, then retry.",
          };
    case "DOWNLOAD_INVALID_URL":
      return zh
        ? {
            message: "该链接不是有效的 HTTP/HTTPS 下载地址。",
            recovery: "检查链接后重新创建下载。",
          }
        : {
            message: "This link is not a valid HTTP/HTTPS download URL.",
            recovery: "Check the link and start a new download.",
          };
    case "DOWNLOAD_PRIVATE_ADDRESS_BLOCKED":
      return zh
        ? {
            message: "该地址指向私有网络。",
            recovery: "如确有需要，请在创建任务时允许私有地址。",
          }
        : {
            message: "This address points to a private network.",
            recovery: "Allow private addresses when creating the task if this is intended.",
          };
    default:
      return zh
        ? {
            message: "下载失败。",
            recovery: "重试该下载。",
          }
        : {
            message: "The download failed.",
            recovery: "Retry the download.",
          };
  }
}
