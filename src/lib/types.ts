/**
 * Wire types mirroring the Rust DTO boundary (src-tauri/src/dto.rs).
 * Domain structs may evolve; these shapes are the stable IPC contract.
 */

export type TaskState =
  | "created"
  | "ready"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface TaskDto {
  taskId: string;
  sourceId: string;
  name: string;
  state: TaskState;
  createdAt: number;
  updatedAt: number;
}

export interface AppInfoDto {
  appName: string;
  version: string;
  milestone: string;
}

export interface SettingsDto {
  downloadDir: string | null;
}

export interface TideErrorPayload {
  code: string;
  message: string;
}
