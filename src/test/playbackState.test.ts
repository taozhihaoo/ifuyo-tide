import { describe, expect, it } from "vitest";
import { canTransition, transition } from "../features/player/state/playbackState";

/**
 * M5 §10: legal transitions work; ILLEGAL transitions are rejected —
 * especially anything out of `disposed`.
 */
describe("player state machine", () => {
  it("follows the legal happy path", () => {
    let s = "idle" as ReturnType<typeof transition> extends null ? never : Parameters<typeof transition>[0];
    s = transition(s, "load")!;
    expect(s).toBe("loading");
    s = transition(s, "loaded")!;
    expect(s).toBe("ready");
    s = transition(s, "playRequested")!;
    expect(s).toBe("playing");
    s = transition(s, "waiting")!;
    expect(s).toBe("buffering");
    s = transition(s, "playing")!;
    s = transition(s, "seekRequested")!;
    expect(s).toBe("seeking");
    s = transition(s, "playing")!;
    s = transition(s, "ended")!;
    expect(s).toBe("ended");
    s = transition(s, "playRequested")!;
    expect(s).toBe("playing");
  });

  it("rejects illegal transitions", () => {
    expect(transition("idle", "playing")).toBeNull();
    expect(transition("idle", "loaded")).toBeNull();
    expect(transition("loading", "playing")).toBeNull();
    expect(transition("ready", "seekRequested")).toBeNull();
    expect(transition("paused", "waiting")).toBeNull();
    expect(transition("ended", "seekRequested")).toBeNull();
    expect(transition("error", "playRequested")).toBeNull();
  });

  it("disposed is terminal — nothing comes back", () => {
    for (const event of [
      "load",
      "loaded",
      "playRequested",
      "playing",
      "paused",
      "waiting",
      "seekRequested",
      "seeked",
      "ended",
      "error",
      "reload",
    ] as const) {
      expect(transition("disposed", event)).toBeNull();
      expect(canTransition("disposed", event)).toBe(false);
    }
    expect(transition("disposed", "dispose")).toBeNull();
  });

  it("everything can be disposed", () => {
    const states = [
      "idle",
      "loading",
      "ready",
      "playing",
      "paused",
      "buffering",
      "seeking",
      "ended",
      "error",
    ] as const;
    for (const state of states) {
      expect(transition(state, "dispose")).toBe("disposed");
    }
  });

  it("error recovers only through an explicit reload", () => {
    expect(transition("error", "reload")).toBe("loading");
    expect(transition("error", "error")).toBeNull();
  });

  it("buffering can be paused (user pauses during a stall)", () => {
    expect(transition("buffering", "paused")).toBe("paused");
    expect(transition("buffering", "error")).toBe("error");
  });
});
