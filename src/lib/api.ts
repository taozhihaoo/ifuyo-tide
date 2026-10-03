import { call } from "./ipc";
import type { AppInfoDto, SettingsDto, TaskDto, TaskState } from "./types";

/**
 * Small API layer between React components and the Rust core.
 * Components never call `invoke` directly.
 */

export function getAppInfo(): Promise<AppInfoDto> {
  return call<AppInfoDto>("get_app_info");
}

export function listTasks(): Promise<TaskDto[]> {
  return call<TaskDto[]>("list_tasks");
}

export function createTestTask(name?: string): Promise<TaskDto> {
  return call<TaskDto>("create_test_task", { name: name ?? null });
}

export function deleteTask(taskId: string): Promise<void> {
  return call<void>("delete_task", { taskId });
}

export function updateTaskState(taskId: string, state: TaskState): Promise<TaskDto> {
  return call<TaskDto>("update_task_state", {
    taskId,
    update: { state },
  });
}

export function getSettings(): Promise<SettingsDto> {
  return call<SettingsDto>("get_settings");
}

export function setDownloadDirectory(path: string): Promise<SettingsDto> {
  return call<SettingsDto>("set_download_directory", { path });
}
