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

// §57 audit: user-facing copy centralized in lib/strings (English is
// the product language; locale swap lands with the i18n framework).
const USER_MESSAGES: Record<PlaybackErrorKind, string> = {
  UnsupportedMedia: "This media format cannot be played in this environment.",
  DecodeError: "Decoding failed — the file may be damaged or the codec unsupported.",
  NetworkError: "The connection was interrupted — trying to recover…",
  ResourceUnavailable: "The resource is unavailable — its task may have been deleted.",
  PlaybackTimeout: "Timed out waiting for data — the network or source is not responding.",
  SessionExpired: "This playback session has expired — start it again.",
  GatewayError: "The playback gateway hit a problem.",
  ResourceClosed: "The resource was closed.",
  UnknownMediaError: "This resource cannot be played.",
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
