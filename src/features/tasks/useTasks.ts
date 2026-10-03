import { useCallback, useEffect, useState } from "react";
import {
  createTestTask,
  deleteTask,
  listTasks,
  updateTaskState,
} from "../../lib/api";
import { isTideApiError } from "../../lib/ipc";
import type { TaskDto, TaskState } from "../../lib/types";

export interface TasksApi {
  /** null = still loading, [] = loaded and empty */
  tasks: TaskDto[] | null;
  error: string | null;
  busy: boolean;
  refresh: () => Promise<void>;
  create: (name?: string) => Promise<TaskDto | null>;
  remove: (taskId: string) => Promise<void>;
  changeState: (taskId: string, next: TaskState) => Promise<void>;
}

/**
 * Real state only: every value comes from the Rust core via IPC.
 * M0 has no optimistic updates, no fake progress, no fake data.
 */
export function useTasks(): TasksApi {
  const [tasks, setTasks] = useState<TaskDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const list = await listTasks();
      setTasks(list);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const create = useCallback(
    async (name?: string): Promise<TaskDto | null> => {
      setBusy(true);
      try {
        const task = await createTestTask(name);
        setError(null);
        await refresh();
        return task;
      } catch (err) {
        setError(errorMessage(err));
        return null;
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  const remove = useCallback(
    async (taskId: string): Promise<void> => {
      setBusy(true);
      try {
        await deleteTask(taskId);
        setError(null);
        await refresh();
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  const changeState = useCallback(
    async (taskId: string, next: TaskState): Promise<void> => {
      setBusy(true);
      try {
        await updateTaskState(taskId, next);
        setError(null);
        await refresh();
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  return { tasks, error, busy, refresh, create, remove, changeState };
}

function errorMessage(err: unknown): string {
  if (isTideApiError(err)) {
    return `${err.message} (${err.code})`;
  }
  return String(err);
}
