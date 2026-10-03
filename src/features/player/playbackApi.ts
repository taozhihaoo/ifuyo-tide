import { call } from "../../lib/ipc";

// M4(上) playback API — the ONLY way the UI gets media bytes: it asks
// the backend for a session URL and hands it to ONE <video> element
// (M4(上) §69/§73). URLs are runtime-only loopback links; nothing is
// persisted and no filesystem path ever reaches the DOM (§59).

export interface PlaybackSessionInfo {
  sessionId: string;
  url: string;
  size: number;
  contentType: string;
  sourceKind: "file" | "torrent" | "http";
  createdAtMs: number;
}

export type PlaybackSourceRequest =
  | { kind: "download_task"; task_id: string }
  | { kind: "torrent_file"; task_id: string; file_index: number };

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
