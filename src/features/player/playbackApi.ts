import { call } from "../../lib/ipc";

// M4(下) playback API — the ONLY way the UI gets media bytes: it asks
// the backend for a session URL and hands it to ONE <video> element
// (M4(上) §69/§73). URLs are runtime-only loopback links; nothing is
// persisted and no filesystem path ever reaches the DOM (§59).

export interface PlaybackCapability {
  supportsRange: boolean;
  supportsRandomAccess: boolean;
  supportsProgressive: boolean;
  knownSize: boolean;
}

export interface PlaybackSessionInfo {
  sessionId: string;
  url: string;
  size: number;
  contentType: string;
  sourceKind: "file" | "part" | "torrent" | "http";
  /** Backend-available bytes right now (M4(下) §149). */
  availableBytes: number;
  capability: PlaybackCapability;
  createdAtMs: number;
  /** Session-scoped last position (M5 §36) for reload resume. */
  lastPositionMs: number;
}

export type PlaybackSourceRequest =
  | { kind: "download_task"; task_id: string }
  | { kind: "torrent_file"; task_id: string; file_index: number }
  | { kind: "library_item"; item_id: string };

export function createPlaybackSession(
  source: PlaybackSourceRequest,
): Promise<PlaybackSessionInfo> {
  return call<PlaybackSessionInfo>("create_playback_session", { source });
}

export function stopPlaybackSession(sessionId: string): Promise<boolean> {
  return call<boolean>("stop_playback_session", { sessionId });
}

export function getPlaybackInfo(sessionId: string): Promise<PlaybackSessionInfo | null> {
  return call<PlaybackSessionInfo | null>("get_playback_info", { sessionId });
}

/** Seek happened: the backend drops demands queued for the old position. */
export function notifySeek(sessionId: string): Promise<void> {
  return call<void>("notify_seek", { sessionId });
}

/** Most recent still-alive session (M5 §37 reload recovery). */
export function getActivePlaybackSession(): Promise<PlaybackSessionInfo | null> {
  return call<PlaybackSessionInfo | null>("get_active_playback_session");
}

/** M5 §36: throttled position report for session-scoped resume. */
export function reportPlaybackPosition(sessionId: string, positionMs: number): Promise<boolean> {
  return call<boolean>("report_playback_position", { sessionId, positionMs });
}

/**
 * M4(下) §99: structured backend codes → honest, human-safe playback
 * messages. Raw internals are never shown.
 */
export function describePlaybackError(err: unknown): string {
  const code = typeof err === "object" && err !== null && "code" in err
    ? String((err as { code: unknown }).code)
    : null;
  switch (code) {
    case "RESOURCE_UNAVAILABLE":
      return "Resource unavailable.";
    case "NETWORK_TIMEOUT":
    case "WAIT_TIMEOUT":
      return "Network timeout while waiting for data.";
    case "NETWORK_CONNECT":
      return "Could not reach the source.";
    case "RESOURCE_RANGE_UNSUPPORTED":
      return "This source does not support seeking — progressive playback only.";
    case "HTTP_STATUS":
      return "The source returned an error.";
    case "NOT_FOUND":
      return "Playback session expired or does not exist.";
    case "PLAYBACK_ERROR":
    case "GATEWAY_ERROR":
    case "DEMAND_ERROR":
    case "SCHEDULER_ERROR":
    case "RANGE_ERROR":
      return "Unable to play this resource.";
    default:
      return "Unable to play this resource.";
  }
}
