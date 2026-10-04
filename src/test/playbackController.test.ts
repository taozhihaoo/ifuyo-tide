import { describe, expect, it, vi } from "vitest";
import { PlaybackController, type MediaElementLike } from "../features/player/controller/PlaybackController";
import { classifyBackendError, classifyMediaError, isTransient } from "../features/player/errorModel";

/**
 * M5 §84 unit tests: the PlaybackController drives a MOCK media
 * element, so state transitions, error classification, bounded
 * reload and cleanup are verified without a real WebView2.
 */

type Handler = () => void;

class MockMediaElement implements MediaElementLike {
  readonly listeners = new Map<string, Set<Handler>>();
  src = "";
  paused = true;
  currentTime = 0;
  duration = 100;
  volume = 1;
  muted = false;
  playbackRate = 1;
  mediaError: { code: number } | null = null;
  loadCalls = 0;
  playPromise: Promise<void> = Promise.resolve();
  readonly buffered = makeRanges([]);
  readonly seekable = makeRanges([[0, 100]]);

  get error() {
    return this.mediaError;
  }

  addEventListener(type: string, listener: Handler) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type)!.add(listener);
  }

  removeEventListener(type: string, listener: Handler) {
    this.listeners.get(type)?.delete(listener);
  }

  fire(type: string) {
    for (const listener of [...(this.listeners.get(type) ?? [])]) listener();
  }

  load() {
    this.loadCalls += 1;
  }

  removeAttribute(name: string) {
    if (name === "src") this.src = "";
  }

  play() {
    this.paused = false;
    return this.playPromise;
  }

  pause() {
    this.paused = true;
  }

  canPlayType() {
    return "probably";
  }
}

function makeRanges(ranges: Array<[number, number]>): TimeRanges {
  return {
    length: ranges.length,
    start: (i: number) => ranges[i][0],
    end: (i: number) => ranges[i][1],
  } as TimeRanges;
}

function makeController(element = new MockMediaElement()) {
  const controller = new PlaybackController(element);
  const snapshots: Array<ReturnType<PlaybackController["snapshot"]>> = [];
  controller.subscribe((s) => snapshots.push(s));
  return { controller, element, snapshots };
}

const SESSION = { sessionId: "s1", url: "http://127.0.0.1:9/stream/tok", lastPositionMs: 0 };

describe("PlaybackController", () => {
  it("walks load → ready → playing through element events", () => {
    const { controller, element } = makeController();
    controller.load(SESSION);
    expect(controller.getState()).toBe("loading");
    element.fire("loadedmetadata");
    expect(controller.getState()).toBe("ready");
    controller.play();
    expect(controller.getState()).toBe("playing");
  });

  it("waiting becomes buffering, playing resumes it (§13)", () => {
    const { controller, element } = makeController();
    controller.load(SESSION);
    element.fire("loadedmetadata");
    controller.play();
    element.fire("waiting");
    expect(controller.getState()).toBe("buffering");
    element.fire("playing");
    expect(controller.getState()).toBe("playing");
  });

  it("classifies media errors and does NOT auto-reload terminal kinds", () => {
    vi.useFakeTimers();
    const { controller, element } = makeController();
    controller.load(SESSION);
    element.fire("loadedmetadata");
    element.mediaError = { code: 4 }; // SRC_NOT_SUPPORTED
    element.fire("error");
    expect(controller.getState()).toBe("error");
    const snap = controller.snapshot();
    expect(snap.error?.kind).toBe("UnsupportedMedia");
    expect(snap.error?.recoverable).toBe(false);
    vi.advanceTimersByTime(10_000);
    expect(element.loadCalls).toBe(1); // no reload loop for terminal errors
    vi.useRealTimers();
  });

  it("recovers a network error with a bounded reload (§34/§35)", () => {
    vi.useFakeTimers();
    const { controller, element } = makeController();
    controller.load(SESSION);
    element.fire("loadedmetadata");

    element.mediaError = { code: 2 }; // NETWORK
    element.fire("error");
    expect(controller.getState()).toBe("error");
    expect(controller.snapshot().error?.kind).toBe("NetworkError");

    vi.advanceTimersByTime(2_000);
    expect(element.loadCalls).toBe(2); // one reload attempted
    expect(controller.getState()).toBe("loading");

    // Second failure → second reload; third failure → terminal error
    // state, no further reloads (bounded).
    element.mediaError = { code: 2 };
    element.fire("error");
    vi.advanceTimersByTime(5_000);
    expect(element.loadCalls).toBe(3);
    element.mediaError = { code: 2 };
    element.fire("error");
    vi.advanceTimersByTime(10_000);
    expect(element.loadCalls).toBe(3); // cap reached
    expect(controller.getState()).toBe("error");
    vi.useRealTimers();
  });

  it("seek is time-domain and clamped to duration (§19)", () => {
    const { controller, element } = makeController();
    controller.load(SESSION);
    element.fire("loadedmetadata");
    controller.play();
    controller.seek(9999);
    expect(element.currentTime).toBe(100);
    controller.seek(42);
    expect(element.currentTime).toBe(42);
    expect(controller.getState()).toBe("seeking");
    element.fire("seeked");
    expect(controller.getState()).toBe("playing");
  });

  it("stop releases the source and disposes; nothing plays afterwards", () => {
    const { controller, element } = makeController();
    controller.load(SESSION);
    element.fire("loadedmetadata");
    let closed: string | null = null;
    controller.stop({ onSessionClosed: (id) => (closed = id) });
    expect(closed).toBe("s1");
    expect(controller.getState()).toBe("disposed");
    controller.play();
    expect(controller.getState()).toBe("disposed");
    expect(element.paused).toBe(true);
  });

  it("dispose removes every media listener (§87 no zombies)", () => {
    const { controller, element } = makeController();
    controller.dispose();
    const before = element.loadCalls;
    element.fire("playing");
    element.fire("error");
    expect(controller.getState()).toBe("disposed");
    expect(element.loadCalls).toBe(before);
  });

  it("survives a rejected play() promise without unhandled rejection (§28)", async () => {
    const { controller, element } = makeController();
    element.playPromise = Promise.reject(new Error("autoplay blocked"));
    controller.load(SESSION);
    element.fire("loadedmetadata");
    controller.play();
    expect(controller.getState()).toBe("playing");
    await element.playPromise.catch(() => {});
    expect(controller.getState()).toBe("paused");
  });

  it("reports position + duration + buffered through the hook (§36/M7)", () => {
    vi.useFakeTimers();
    const report = vi.fn();
    const element = new MockMediaElement();
    element.currentTime = 5;
    (element as { buffered: TimeRanges }).buffered = makeRanges([[0, 8]]);
    const controller = new PlaybackController(element, { reportPosition: report });
    controller.load(SESSION);
    vi.advanceTimersByTime(6_000);
    expect(report).toHaveBeenCalledWith("s1", 5000, 100_000, 3);
    vi.useRealTimers();
  });
});

describe("error model mapping (M5 §32/§56)", () => {
  it("maps media error codes to kinds", () => {
    expect(classifyMediaError({ code: 3 }).kind).toBe("DecodeError");
    expect(classifyMediaError({ code: 4 }).kind).toBe("UnsupportedMedia");
    expect(classifyMediaError({ code: 2 }).kind).toBe("NetworkError");
  });

  it("maps backend codes to kinds and keeps diagnostics context", () => {
    const err = classifyBackendError(
      { code: "RESOURCE_UNAVAILABLE", message: "gone" },
      { sessionId: "s1", contentType: "video/mp4" },
    );
    expect(err.kind).toBe("ResourceUnavailable");
    expect(err.backendCode).toBe("RESOURCE_UNAVAILABLE");
    expect(err.sessionId).toBe("s1");
    expect(err.message).not.toContain("gone");
  });

  it("only network-ish kinds are transient", () => {
    expect(isTransient("NetworkError")).toBe(true);
    expect(isTransient("DecodeError")).toBe(false);
    expect(isTransient("UnsupportedMedia")).toBe(false);
  });
});
