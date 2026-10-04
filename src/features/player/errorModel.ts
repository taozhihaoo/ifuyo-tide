/**
 * M5 §32/§33/§56 — the player error model: a typed classification
 * with user-safe messages and diagnostic context. Raw internals
 * (decoder numbers, Rust debug strings, paths) never reach the UI.
 */
import type { PlaybackSessionInfo } from "./playbackApi";

export type PlaybackErrorKind =
  | "UnsupportedMedia"
  | "DecodeError"
  | "NetworkError"
  | "ResourceUnavailable"
  | "PlaybackTimeout"
  | "SessionExpired"
  | "GatewayError"
  | "ResourceClosed"
  | "UnknownMediaError";

export interface PlaybackError {
  kind: PlaybackErrorKind;
  /** Human-safe, honest message for the UI. */
  message: string;
  /** Whether a bounded retry/reload makes sense (M5 §34). */
  recoverable: boolean;
  /** Diagnostic context (M5 §33) — safe fields only. */
  mediaErrorCode?: number;
  backendCode?: string;
  sessionId?: string;
  contentType?: string;
  playerState?: string;
  timestamp: number;
}

const USER_MESSAGES: Record<PlaybackErrorKind, string> = {
  UnsupportedMedia: "此媒体格式当前环境无法播放。",
  DecodeError: "此媒体解码失败 — 文件可能已损坏或编码不受支持。",
  NetworkError: "网络中断 — 正在尝试恢复…",
  ResourceUnavailable: "资源不可用 — 下载任务可能已被删除。",
  PlaybackTimeout: "等待数据超时 — 网络或来源没有响应。",
  SessionExpired: "播放会话已过期 — 请重新开始播放。",
  GatewayError: "播放网关出现问题。",
  ResourceClosed: "资源已被关闭。",
  UnknownMediaError: "无法播放此资源。",
};

/** HTMLMediaElement MediaError codes (§12 error model input). */
const MEDIA_ERROR_KINDS: Record<number, PlaybackErrorKind> = {
  1: "UnknownMediaError", // MEDIA_ERR_ABORTED
  2: "NetworkError", // MEDIA_ERR_NETWORK
  3: "DecodeError", // MEDIA_ERR_DECODE
  4: "UnsupportedMedia", // MEDIA_ERR_SRC_NOT_SUPPORTED
};

/** Backend streaming error codes (M4(下) §98) → player error kinds. */
const BACKEND_ERROR_KINDS: Record<string, PlaybackErrorKind> = {
  RESOURCE_UNAVAILABLE: "ResourceUnavailable",
  RESOURCE_RANGE_UNSUPPORTED: "UnsupportedMedia",
  NETWORK_TIMEOUT: "PlaybackTimeout",
  WAIT_TIMEOUT: "PlaybackTimeout",
  NETWORK_CONNECT: "NetworkError",
  HTTP_STATUS: "NetworkError",
  GATEWAY_ERROR: "GatewayError",
  PLAYBACK_ERROR: "GatewayError",
  NOT_FOUND: "SessionExpired",
  RANGE_ERROR: "GatewayError",
  DEMAND_ERROR: "GatewayError",
  SCHEDULER_ERROR: "GatewayError",
};

/** Classifies a backend/IPC failure into the player error model. */
export function classifyBackendError(err: unknown, context?: Partial<PlaybackError>): PlaybackError {
  const code = typeof err === "object" && err !== null && "code" in err
    ? String((err as { code: unknown }).code).toUpperCase()
    : null;
  const kind =
    (code && BACKEND_ERROR_KINDS[code]) ||
    (code === "NOT_FOUND" ? "SessionExpired" : "UnknownMediaError");
  return {
    kind,
    message: USER_MESSAGES[kind],
    recoverable: kind === "NetworkError",
    backendCode: code ?? undefined,
    timestamp: Date.now(),
    ...context,
  };
}

/** Classifies a media element error event (M5 §12/§32). */
export function classifyMediaError(
  mediaError: { code?: number } | null | undefined,
  context?: Partial<PlaybackError>,
): PlaybackError {
  const code = mediaError?.code ?? 0;
  const kind = MEDIA_ERROR_KINDS[code] ?? "UnknownMediaError";
  return {
    kind,
    message: USER_MESSAGES[kind],
    recoverable: kind === "NetworkError",
    mediaErrorCode: code,
    timestamp: Date.now(),
    ...context,
  };
}

/** Error kinds that a bounded retry may recover from (M5 §34/§35). */
export function isTransient(kind: PlaybackErrorKind): boolean {
  return kind === "NetworkError" || kind === "PlaybackTimeout";
}

export type { PlaybackSessionInfo };
