import { call } from "./ipc";
import type { TaskState } from "./types";

// M0.5 SPIKE — typed API for experiment commands. Mirrors
// src-tauri/src/commands/spike.rs.

export interface MediaFixture {
  name: string;
  size: number;
  contentType: string;
}

export interface SpikeRuntimeInfo {
  fixturePort: number | null;
  gatewayPort: number | null;
  torrentRunning: boolean;
}

export interface SpikeTorrentStatus {
  infoHash: string;
  progressBytes: number;
  totalBytes: number;
  finished: boolean;
  paused: boolean;
  error: string | null;
  fileProgress: number[];
}

export interface SpikeFileInfoDto {
  id: number;
  name: string;
  len: number;
  offsetInTorrent: number;
}

export interface SpikeTorrentStarted {
  infoHash: string;
  files: SpikeFileInfoDto[];
  downloadDir: string;
}

export interface PlaybackSessionDto {
  url: string;
  source: string;
  name: string;
}

export interface SpikeEvent {
  tsUnixMs: number;
  kind: string;
  detail: Record<string, string>;
}

export function spikeStatus(): Promise<SpikeRuntimeInfo> {
  return call<SpikeRuntimeInfo>("spike_status");
}

export function spikeListMedia(): Promise<MediaFixture[]> {
  return call<MediaFixture[]>("spike_list_media");
}

export function spikeStartFixtureServer(): Promise<number> {
  return call<number>("spike_start_fixture_server");
}

export function spikeFixtureSlowMs(delayMs: number): Promise<void> {
  return call<void>("spike_fixture_slow_ms", { delayMs });
}

export function spikeStartTorrent(mode: "single" | "multi"): Promise<SpikeTorrentStatus> {
  return call<SpikeTorrentStatus>("spike_start_torrent", { mode });
}

export function spikeTorrentStatus(): Promise<SpikeTorrentStatus | null> {
  return call<SpikeTorrentStatus | null>("spike_torrent_status");
}

export function spikePauseTorrent(): Promise<void> {
  return call<void>("spike_pause_torrent");
}

export function spikeResumeTorrent(): Promise<void> {
  return call<void>("spike_resume_torrent");
}

export function spikeOpenPlaybackSession(
  kind: "file" | "http" | "torrent",
  fileName?: string,
): Promise<PlaybackSessionDto> {
  return call<PlaybackSessionDto>("spike_open_playback_session", {
    kind,
    fileName: fileName ?? null,
  });
}

export function spikeClosePlaybackSession(token: string): Promise<boolean> {
  return call<boolean>("spike_close_playback_session", { token });
}

export function spikeGatewaySlowMs(delayMs: number): Promise<void> {
  return call<void>("spike_gateway_slow_ms", { delayMs });
}

export function spikeGatewayReadTimeoutMs(timeoutMs: number): Promise<void> {
  return call<void>("spike_gateway_read_timeout_ms", { timeoutMs });
}

export function spikeEvents(kind?: string): Promise<SpikeEvent[]> {
  return call<SpikeEvent[]>("spike_events", { kind: kind ?? null });
}

export function spikeClearEvents(): Promise<void> {
  return call<void>("spike_clear_events");
}

export function spikeReportMediaResult(report: Record<string, unknown>): Promise<void> {
  return call<void>("spike_report_media_result", { report });
}

export function spikeReadMediaResults(): Promise<Record<string, unknown>[]> {
  return call<Record<string, unknown>[]>("spike_read_media_results");
}

export function spikeAutorunEnabled(): Promise<boolean> {
  return call<boolean>("spike_autorun_enabled");
}

export function spikeShutdown(): Promise<SpikeRuntimeInfo> {
  return call<SpikeRuntimeInfo>("spike_shutdown");
}

// keep TaskState import referenced for future spike task integration
export type { TaskState };
