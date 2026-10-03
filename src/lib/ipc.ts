import { invoke } from "@tauri-apps/api/core";
import type { TideErrorPayload } from "./types";

/**
 * Structured error surfaced by the Rust core: `{ code, message }`.
 * The frontend never receives panics or raw internals.
 */
export class TideApiError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "TideApiError";
    this.code = code;
  }
}

export function isTideApiError(err: unknown): err is TideApiError {
  return err instanceof TideApiError;
}

/**
 * Minimal typed invoke wrapper. All UI calls go through this layer so
 * error shaping stays in one place.
 */
export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    if (
      raw &&
      typeof raw === "object" &&
      "code" in raw &&
      "message" in raw
    ) {
      const payload = raw as TideErrorPayload;
      throw new TideApiError(payload.code, payload.message);
    }
    throw new TideApiError("IPC_FAILED", String(raw));
  }
}
