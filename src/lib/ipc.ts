import { invoke } from "@tauri-apps/api/core";
import type { TideErrorPayload } from "./types";

/**
 * Structured error surfaced by the Rust core: `{ code, message }`.
 * The frontend never receives panics or raw internals.
 */
export class TideApiError extends Error {
  readonly code: string;

  constructor(code: string, message: string, cause?: unknown) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = "TideApiError";
    this.code = code;
  }

  /** Views often do `String(err)`; never leak the JS class name into
   *  user-facing banners (§15 — raw internals stay in the console). */
  override toString(): string {
    return this.message;
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
    // Unstructured failure = the IPC bridge itself is broken. Keep the
    // raw error as `cause` for devtools, but never show it in the UI
    // (§15). No console noise on every failed call either.
    throw new TideApiError(
      "IPC_FAILED",
      "Tide could not reach its local backend. Please restart the app.",
      raw,
    );
  }
}
