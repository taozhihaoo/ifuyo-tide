import { call } from "../../lib/ipc";

// M3 torrent API — typed layer over the torrent IPC commands.

export interface TorrentFileDto {
  index: number;
  path: string;
  size: number;
  selected: boolean;
}

export interface TorrentDto {
  id: string;
  infoHash: string | null;
  sourceKind: string;
  displayName: string;
  state: string;
  phase: string;
  totalBytes: number | null;
  downloadedBytes: number;
  uploadedBytes: number;
  selectedCount: number | null;
  files: TorrentFileDto[];
  downloadSpeedBps: number | null;
  errorCode: string | null;
  errorMessage: string | null;
}

export interface TorrentInspection {
  infoHash: string;
  name: string;
  totalSize: number;
  pieceLength: number;
  files: TorrentFileDto[];
}

export function inspectTorrentSource(torrentPath: string): Promise<TorrentInspection> {
  return call<TorrentInspection>("inspect_torrent_source", { torrentPath });
}

export function createTorrentTask(options: {
  sourceKind: "magnet" | "torrent_file";
  magnetUri?: string | null;
  torrentPath?: string | null;
  destinationDir: string;
}): Promise<{ id: string; infoHash: string }> {
  return call<{ id: string; infoHash: string }>("create_torrent_task", {
    sourceKind: options.sourceKind,
    magnetUri: options.magnetUri ?? null,
    torrentPath: options.torrentPath ?? null,
    destinationDir: options.destinationDir,
  });
}

export function listTorrents(): Promise<TorrentDto[]> {
  return call<TorrentDto[]>("list_torrents");
}

export function setTorrentFileSelection(
  id: string,
  selected: number[],
): Promise<void> {
  return call<void>("set_torrent_file_selection", { id, selected });
}

export function pauseTorrent(id: string): Promise<void> {
  return call<void>("pause_torrent", { id });
}

export function resumeTorrent(id: string): Promise<void> {
  return call<void>("resume_torrent", { id });
}

export function removeTorrent(id: string): Promise<void> {
  return call<void>("remove_torrent", { id });
}

export function deleteTorrentData(id: string): Promise<void> {
  return call<void>("delete_torrent_data", { id });
}
