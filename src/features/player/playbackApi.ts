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

/** M5 §36 + M7 §8: throttled telemetry report — position for resume,
 * duration + REAL browser-buffered-ahead for the scheduler's buffer
 * model. All values come from the media element. */
export function reportPlaybackPosition(
  sessionId: string,
  positionMs: number,
  durationMs: number | null,
  bufferedAheadSecs: number | null,
): Promise<boolean> {
  return call<boolean>("report_playback_position", {
    sessionId,
    positionMs,
    durationMs,
    bufferedAheadSecs: bufferedAheadSecs == null ? null : Number(bufferedAheadSecs.toFixed(2)),
  });
}

/** M7 §78: scheduler intent + snapshot DTO (§79). */
export type UserPriority = "highest" | "high" | "normal" | "low" | "background";

export function setTaskPriority(taskId: string, priority: UserPriority): Promise<void> {
  return call<void>("scheduler_set_task_priority", { taskId, priority });
}

export function setGlobalBandwidthLimit(bps: number): Promise<void> {
  return call<void>("scheduler_set_global_bandwidth_limit", { bps });
}

export interface SchedulerTaskDto {
  taskId: string;
  engine: string;
  state: string;
  userPriority: string | null;
  smoothedBps: number;
}

export interface SchedulerPlaybackDto {
  sessionId: string;
  taskId: string | null;
  state: string;
  bufferedAheadSecs: number | null;
}

export interface SchedulerDecisionDto {
  taskId: string;
  priorityClass: string;
  allocatedBps: number;
  reasonCode: string;
  reasonDetail: string;
}

export interface SchedulerSnapshotDto {
  globalBandwidthBps: number | null;
  tasks: SchedulerTaskDto[];
  playback: SchedulerPlaybackDto[];
  decisions: SchedulerDecisionDto[];
}

export function getSchedulerSnapshot(): Promise<SchedulerSnapshotDto> {
  return call<SchedulerSnapshotDto>("scheduler_get_snapshot");
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
