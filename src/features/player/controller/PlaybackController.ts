/**
 * M5 §11/§12/§13 — PlaybackController + MediaEventAdapter.
 *
 * The ONE object allowed to drive the HTMLMediaElement. UI components
 * never touch `video.play()`/`video.currentTime` directly; they call
 * the controller and render its snapshot. Media events are observed
 * HERE (not in React components) and mapped through the M5 §9 state
 * machine — a `waiting` event becomes "buffering", and with backend
 * availability context the UI can say "waiting for download" instead
 * of guessing "network error" (§13).
 */
import {
  classifyMediaError,
  isTransient,
  type PlaybackError,
} from "../errorModel";
import { transition, type PlayerEvent, type PlayerState } from "../state/playbackState";

/** Structural media-element surface (satisfied by HTMLMediaElement;
 * kept narrow so tests can drive a mock). */
export interface MediaElementLike {
  addEventListener(type: string, listener: () => void): void;
  removeEventListener(type: string, listener: () => void): void;
  src: string;
  load(): void;
  removeAttribute(name: string): void;
  play(): Promise<void>;
  pause(): void;
  readonly paused: boolean;
  currentTime: number;
  readonly duration: number;
  readonly buffered: TimeRanges;
  readonly seekable: TimeRanges;
  volume: number;
  muted: boolean;
  playbackRate: number;
  readonly error: { code: number } | null;
  canPlayType(type: string): string;
}

export interface PlayerSnapshot {
  state: PlayerState;
  currentTime: number | null;
  duration: number | null;
  /** Buffered-ahead seconds covering currentTime (browser truth). */
  bufferedAhead: number | null;
  seekableDuration: number | null;
  volume: number;
  muted: boolean;
  playbackRate: number;
  error: PlaybackError | null;
}

export interface PlaybackMetrics {
  openStartedAt: number | null;
  timeToMetadataMs: number | null;
  timeToFirstFrameMs: number | null;
  bufferingCount: number;
  stallDurationMs: number;
  seekCount: number;
  seekWaitMs: number;
  errorCount: number;
  reloadCount: number;
}

const MEDIA_EVENTS = [
  "loadstart",
  "loadedmetadata",
  "loadeddata",
  "canplay",
  "play",
  "playing",
  "pause",
  "timeupdate",
  "progress",
  "durationchange",
  "seeking",
  "seeked",
  "waiting",
  "stalled",
  "suspend",
  "ended",
  "error",
  "volumechange",
  "ratechange",
  "emptied",
] as const;

const MAX_RELOADS = 2;
const RELOAD_BACKOFF_MS = [1_000, 3_000];
const POSITION_REPORT_INTERVAL_MS = 5_000;

function bufferedAheadOf(buffered: TimeRanges, time: number): number | null {
  for (let i = 0; i < buffered.length; i += 1) {
    if (buffered.start(i) <= time && time <= buffered.end(i)) {
      return buffered.end(i) - time;
    }
  }
  return null;
}

export class PlaybackController {
  private readonly element: MediaElementLike;
  private readonly listeners = new Set<(snapshot: PlayerSnapshot) => void>();
  private readonly mediaListeners: Array<[string, () => void]> = [];
  private state: PlayerState = "idle";
  private error: PlaybackError | null = null;
  private sessionId: string | null = null;
  private reloadCount = 0;
  private reloadTimer: ReturnType<typeof setTimeout> | null = null;
  private positionTimer: ReturnType<typeof setInterval> | null = null;
  private seekStartedAt: number | null = null;
  private bufferingStartedAt: number | null = null;
  private firstFrameAt: number | null = null;
  private lastReportedPositionMs = 0;
  private readonly reportPosition:
    | ((
          sessionId: string,
          positionMs: number,
          durationMs: number | null,
          bufferedAheadSecs: number | null,
      ) => void)
    | null;
  readonly metrics: PlaybackMetrics = {
    openStartedAt: null,
    timeToMetadataMs: null,
    timeToFirstFrameMs: null,
    bufferingCount: 0,
    stallDurationMs: 0,
    seekCount: 0,
    seekWaitMs: 0,
    errorCount: 0,
    reloadCount: 0,
  };

  constructor(
    element: MediaElementLike,
    hooks?: {
      reportPosition?: (
        sessionId: string,
        positionMs: number,
        durationMs: number | null,
        bufferedAheadSecs: number | null,
      ) => void;
    },
  ) {
    this.element = element;
    this.reportPosition = hooks?.reportPosition ?? null;
    for (const event of MEDIA_EVENTS) {
      const listener = () => this.onMediaEvent(event);
      this.mediaListeners.push([event, listener]);
      element.addEventListener(event, listener);
    }
  }

  // ----- subscription -------------------------------------------------

  subscribe(listener: (snapshot: PlayerSnapshot) => void): () => void {
    this.listeners.add(listener);
    listener(this.snapshot());
    return () => this.listeners.delete(listener);
  }

  snapshot(): PlayerSnapshot {
    const el = this.element;
    const duration = Number.isFinite(el.duration) ? el.duration : null;
    return {
      state: this.state,
      currentTime: Number.isFinite(el.currentTime) ? el.currentTime : null,
      duration,
      bufferedAhead:
        this.state === "idle" || this.state === "disposed"
          ? null
          : bufferedAheadOf(el.buffered, el.currentTime || 0),
      seekableDuration:
        el.seekable.length > 0 ? el.seekable.end(el.seekable.length - 1) : duration,
      volume: el.volume,
      muted: el.muted,
      playbackRate: el.playbackRate,
      error: this.error,
    };
  }

  getState(): PlayerState {
    return this.state;
  }

  // ----- commands (M5 §11) --------------------------------------------

  /** Opens a playback session URL (the ONLY way media enters the
   * element — backend-created capability URL, §14/§52). */
  load(session: { sessionId: string; url: string; lastPositionMs?: number }): void {
    if (this.state === "disposed") return;
    this.sessionId = session.sessionId;
    this.error = null;
    this.reloadCount = 0;
    this.metrics.openStartedAt = Date.now();
    this.metrics.timeToMetadataMs = null;
    this.metrics.timeToFirstFrameMs = null;
    this.firstFrameAt = null;
    this.transit("load");
    this.element.src = session.url;
    this.element.load();
    this.startPositionReporting();
    // Session-scoped resume (M5 §36): applied once metadata is known —
    // see onMediaEvent(loadedmetadata).
  }

  play(): void {
    if (this.state === "disposed" || this.state === "error") return;
    // Ended → replay from the start (§10: Ended → Playing).
    if (this.state === "ended") {
      this.element.currentTime = 0;
    }
    this.transit("playRequested");
    this.element.play().catch(() => {
      // Autoplay-policy rejection (M5 §28): a NOT-unhandled rejection,
      // surfaced as a paused state — the user presses play explicitly.
      this.transit("paused");
      this.emit();
    });
  }

  pause(): void {
    if (this.state === "disposed") return;
    this.element.pause();
  }

  toggle(): void {
    if (this.state === "playing" || this.state === "buffering" || this.state === "seeking") {
      this.pause();
    } else if (this.state === "ready" || this.state === "paused" || this.state === "ended") {
      this.play();
    }
  }

  /** Time-domain seek (M5 §19): the element+browser derive the byte
   * Range — the player never computes byte offsets. */
  seek(targetSeconds: number): void {
    if (this.state === "disposed" || this.state === "error" || this.state === "idle") return;
    const duration = this.element.duration;
    const target = Math.max(
      0,
      Number.isFinite(duration) ? Math.min(targetSeconds, duration) : targetSeconds,
    );
    // Rapid seeks land here too: a seek during a seek just retargets
    // the element (M5 §86 concurrency), the machine stays `seeking`.
    if (
      this.state === "playing" ||
      this.state === "buffering" ||
      this.state === "paused" ||
      this.state === "seeking" ||
      this.state === "ended"
    ) {
      this.metrics.seekCount += 1;
      this.seekStartedAt = Date.now();
      this.transit("seekRequested");
      this.element.currentTime = target;
      this.emit();
    }
  }

  setVolume(volume: number): void {
    if (this.state === "disposed") return;
    this.element.volume = Math.max(0, Math.min(1, volume));
  }

  setMuted(muted: boolean): void {
    if (this.state === "disposed") return;
    this.element.muted = muted;
  }

  setPlaybackRate(rate: number): void {
    if (this.state === "disposed") return;
    this.element.playbackRate = rate;
  }

  /** M5 §30: stop = pause + release source + dispose session binding.
   * Downloaded DATA is never touched (task deletion is explicit). */
  stop(options?: { onSessionClosed?: (sessionId: string) => void }): void {
    if (this.state === "disposed") return;
    const sessionId = this.sessionId;
    this.stopTimers();
    this.element.pause();
    this.element.removeAttribute("src");
    this.element.load();
    this.state = "disposed";
    this.emit();
    if (sessionId) options?.onSessionClosed?.(sessionId);
  }

  /** Detaches all media listeners and timers (M5 §87: guaranteed
   * cleanup — no zombie listeners, no zombie timers). */
  dispose(): void {
    this.stopTimers();
    for (const [event, listener] of this.mediaListeners) {
      this.element.removeEventListener(event, listener);
    }
    this.mediaListeners.length = 0;
    this.listeners.clear();
    this.state = "disposed";
  }

  /** Frontend reload recovery (M5 §37): adopt the backend's still-
   * alive session and resume at its last known position. */
  adoptSession(session: { sessionId: string; url: string; lastPositionMs: number }): void {
    this.load(session);
    // Resume position is applied on loadedmetadata (see onMediaEvent).
    this.resumePositionMs = session.lastPositionMs;
  }

  // ----- internals ------------------------------------------------------

  private resumePositionMs?: number;

  private startPositionReporting(): void {
    this.stopPositionTimer();
    this.lastReportedPositionMs = 0;
    const report = this.reportPosition;
    if (!report || !this.sessionId) return;
    const sessionId = this.sessionId;
    this.positionTimer = setInterval(() => {
      const t = this.element.currentTime;
      if (!Number.isFinite(t)) return;
      const ms = Math.round(t * 1000);
      if (Math.abs(ms - this.lastReportedPositionMs) < 1) return;
      this.lastReportedPositionMs = ms;
      const dur = this.element.duration;
      try {
        report(
          sessionId,
          ms,
          Number.isFinite(dur) ? Math.round(dur * 1000) : null,
          bufferedAheadOf(this.element.buffered, t),
        );
      } catch {
        // Position reporting is best-effort (M5 §36).
      }
    }, POSITION_REPORT_INTERVAL_MS);
  }

  private stopPositionTimer(): void {
    if (this.positionTimer !== null) {
      clearInterval(this.positionTimer);
      this.positionTimer = null;
    }
  }

  private stopTimers(): void {
    this.stopPositionTimer();
    if (this.reloadTimer !== null) {
      clearTimeout(this.reloadTimer);
      this.reloadTimer = null;
    }
  }

  private transit(event: PlayerEvent): void {
    const next = transition(this.state, event);
    if (next !== null) this.state = next;
  }

  private classifyCurrentError(): PlaybackError {
    return classifyMediaError(this.element.error, {
      sessionId: this.sessionId ?? undefined,
      playerState: this.state,
    });
  }

  private scheduleRecovery(err: PlaybackError): void {
    // M5 §34/§35: bounded retry with backoff — never an infinite
    // reload loop; terminal errors surface immediately.
    if (!isTransient(err.kind) || this.reloadCount >= MAX_RELOADS || !this.sessionId) {
      return;
    }
    const delay = RELOAD_BACKOFF_MS[Math.min(this.reloadCount, RELOAD_BACKOFF_MS.length - 1)];
    this.reloadCount += 1;
    this.metrics.reloadCount += 1;
    const sessionId = this.sessionId;
    const src = this.element.src;
    this.reloadTimer = setTimeout(() => {
      if (this.state === "disposed" || this.sessionId !== sessionId) return;
      this.transit("reload");
      this.error = null;
      this.element.src = src;
      this.element.load();
      this.emit();
    }, delay);
  }

  private onMediaEvent(event: string): void {
    switch (event) {
      case "loadstart":
        break;
      case "loadedmetadata": {
        this.metrics.timeToMetadataMs =
          this.metrics.openStartedAt !== null
            ? Date.now() - this.metrics.openStartedAt
            : null;
        this.transit("loaded");
        const resume = this.resumePositionMs;
        if (resume && resume > 1_000 && Number.isFinite(this.element.duration) && resume < this.element.duration * 1000) {
          this.element.currentTime = resume / 1000;
        }
        this.resumePositionMs = undefined;
        break;
      }
      case "canplay":
      case "loadeddata":
      case "durationchange":
        if (this.state === "loading") this.transit("loaded");
        break;
      case "play":
        break;
      case "playing":
        this.metrics.timeToFirstFrameMs ??=
          this.metrics.openStartedAt !== null ? Date.now() - this.metrics.openStartedAt : null;
        this.firstFrameAt ??= Date.now();
        if (this.bufferingStartedAt !== null) {
          this.metrics.stallDurationMs += Date.now() - this.bufferingStartedAt;
          this.bufferingStartedAt = null;
        }
        this.transit("playing");
        break;
      case "pause":
        if (this.state !== "seeking") this.transit("paused");
        if (this.bufferingStartedAt !== null) {
          this.metrics.stallDurationMs += Date.now() - this.bufferingStartedAt;
          this.bufferingStartedAt = null;
        }
        break;
      case "waiting":
      case "stalled":
        if (this.state === "playing" || this.state === "seeking") {
          this.bufferingStartedAt ??= Date.now();
          this.metrics.bufferingCount += event === "waiting" ? 1 : 0;
          this.transit("waiting");
        }
        break;
      case "seeking":
        break;
      case "seeked":
        if (this.seekStartedAt !== null) {
          this.metrics.seekWaitMs += Date.now() - this.seekStartedAt;
          this.seekStartedAt = null;
        }
        // Land in playing or paused per the element's actual state.
        if (this.state === "seeking") {
          this.transit(this.element.paused ? "paused" : "playing");
        }
        break;
      case "ended":
        this.transit("ended");
        break;
      case "error": {
        const err = this.classifyCurrentError();
        this.metrics.errorCount += 1;
        this.error = err;
        this.transit("error");
        this.emit();
        this.scheduleRecovery(err);
        return;
      }
      case "emptied":
        break;
      case "progress":
      case "timeupdate":
      case "volumechange":
      case "ratechange":
      case "suspend":
        break;
    }
    this.emit();
  }

  private emit(): void {
    const snapshot = this.snapshot();
    for (const listener of this.listeners) listener(snapshot);
  }
}
