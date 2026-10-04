/**
 * M5 §9/§10 — the player runtime state machine.
 *
 * These are PLAYBACK UI/RUNTIME states: they never leak into
 * DownloadTask state and the download state machine never touches
 * them (M5 §9). Pure and dependency-free so transitions are
 * unit-testable without a DOM.
 */

export type PlayerState =
  | "idle"
  | "loading"
  | "ready"
  | "playing"
  | "paused"
  | "buffering"
  | "seeking"
  | "ended"
  | "error"
  | "disposed";

export type PlayerEvent =
  | "load"
  | "loaded"
  | "playRequested"
  | "playing"
  | "paused"
  | "waiting"
  | "seekRequested"
  | "seeked"
  | "ended"
  | "error"
  | "reload"
  | "dispose";

/**
 * Legal transitions (M5 §10). Anything absent is ILLEGAL and rejected
 * — e.g. `disposed → playing` must never happen.
 */
const TRANSITIONS: Record<PlayerState, Partial<Record<PlayerEvent, PlayerState>>> = {
  idle: { load: "loading", dispose: "disposed" },
  loading: { loaded: "ready", error: "error", dispose: "disposed" },
  ready: { playRequested: "playing", paused: "paused", error: "error", dispose: "disposed" },
  playing: {
    paused: "paused",
    waiting: "buffering",
    seekRequested: "seeking",
    ended: "ended",
    error: "error",
    dispose: "disposed",
  },
  paused: {
    playRequested: "playing",
    seekRequested: "seeking",
    error: "error",
    dispose: "disposed",
  },
  buffering: {
    playing: "playing",
    paused: "paused",
    error: "error",
    dispose: "disposed",
  },
  seeking: {
    // A seek lands either in playback or in a buffer wait; `seeked`
    // with a still-buffering element surfaces as waiting.
    playing: "playing",
    waiting: "buffering",
    paused: "paused",
    error: "error",
    dispose: "disposed",
  },
  ended: { playRequested: "playing", dispose: "disposed" },
  error: { reload: "loading", dispose: "disposed" },
  disposed: {},
};

export function transition(current: PlayerState, event: PlayerEvent): PlayerState | null {
  const next = TRANSITIONS[current][event];
  return next ?? null;
}

export function isTerminal(state: PlayerState): boolean {
  return state === "disposed";
}

/** True when the state machine would accept this transition. */
export function canTransition(current: PlayerState, event: PlayerEvent): boolean {
  return transition(current, event) !== null;
}
