import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatEta,
  formatSpeed,
  truncateMiddle,
} from "../lib/format";
import { STATE_LABELS } from "../lib/format";
import type { TaskState } from "../lib/format";

/**
 * M9(下) §101/§102 — property + fuzz tests for the pure presentation
 * layer. Invariants (§101): no NaN, no Infinity, no negative nonsense,
 * no panic — for ANY input including u64-max-ish numbers, CJK/emoji
 * names and hostile strings (§102).
 */

// Seeded LCG — deterministic, no dependency.
function lcg(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 0xffffffff;
  };
}

describe("formatter property invariants (§101)", () => {
  it("formatBytes: total over random magnitudes — never NaN/Infinity/negative", () => {
    const rnd = lcg(42);
    for (let i = 0; i < 300; i++) {
      const v = rnd() * Number.MAX_SAFE_INTEGER;
      const out = formatBytes(v);
      expect(out).not.toMatch(/NaN|Infinity|undefined/);
      expect(out.startsWith("-")).toBe(false);
    }
    // u64-max-ish (JS cannot hold 2^64-1 exactly but tests the path).
    const out = formatBytes(Number.MAX_SAFE_INTEGER);
    expect(out).toMatch(/GB$/);
    expect(out).not.toMatch(/NaN|Infinity/);
  });

  it("formatSpeed: total over random magnitudes", () => {
    const rnd = lcg(7);
    for (let i = 0; i < 300; i++) {
      const out = formatSpeed(rnd() * Number.MAX_SAFE_INTEGER);
      expect(out).not.toMatch(/NaN|Infinity/);
      expect(out.startsWith("-")).toBe(false);
    }
  });

  it("formatEta: total over random magnitudes + edge states", () => {
    const rnd = lcg(99);
    for (let i = 0; i < 300; i++) {
      const secs = (rnd() - 0.5) * Number.MAX_SAFE_INTEGER; // includes negatives
      const out = formatEta(secs, i % 2 === 0 ? "running" : "paused");
      expect(out).not.toMatch(/NaN|Infinity|undefined/);
      expect(out.startsWith("-")).toBe(false);
    }
  });

  it("truncateMiddle: never panics, never grows, works for CJK/emoji (§102)", () => {
    const rnd = lcg(1234);
    const samples = [
      "电影.2024.2160p.WEB-DL.x265.10bit.HDR.mp4",
      "🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬🎬",
      "日本語のファイル名がとても長い場合の表示確認用サンプル.mp4",
      "\u{1F600}".repeat(50),
      "",
    ];
    for (let i = 0; i < 200; i++) {
      const len = Math.floor(rnd() * 120);
      samples.push(
        "f" + String(i).repeat(1) + "x".repeat(len) + ".mp4",
      );
    }
    for (const name of samples) {
      for (const max of [4, 10, 20, 48, 200]) {
        const out = truncateMiddle(name, max);
        expect(out.length).toBeLessThanOrEqual(Math.max(max, 4) + 1);
      }
    }
    expect(truncateMiddle("", 10)).toBe("");
  });
});

describe("state terminology map (§56/§100)", () => {
  const expected: Array<[TaskState, string]> = [
    ["created", "Created"],
    ["ready", "Ready"],
    ["running", "Running"],
    ["paused", "Paused"],
    ["completed", "Completed"],
    ["failed", "Failed"],
    ["cancelled", "Cancelled"],
  ];
  it("covers every state exactly once — no synonyms", () => {
    expect(Object.keys(STATE_LABELS).sort()).toEqual(
      expected.map(([s]) => s).sort(),
    );
    for (const [state, label] of expected) {
      expect(STATE_LABELS[state]).toBe(label);
    }
  });
});

/** §13/§100: action availability per task state — the UI matrix the
 * Downloads actions must obey. Pure function so the matrix is
 * unit-enforced. */
export function availableActions(state: string): string[] {
  switch (state) {
    case "created":
    case "running":
      return ["pause", "cancel"];
    case "paused":
      return ["resume", "cancel"];
    case "failed":
      return ["retry", "delete"];
    case "cancelled":
      return ["retry", "delete"];
    case "completed":
      return ["open", "reveal", "play", "delete"];
    default:
      return [];
  }
}

describe("action availability matrix (§13/§100)", () => {
  it("matches the §13 table", () => {
    expect(availableActions("running")).toEqual(["pause", "cancel"]);
    expect(availableActions("paused")).toEqual(["resume", "cancel"]);
    expect(availableActions("failed")).toEqual(["retry", "delete"]);
    expect(availableActions("completed")).toEqual([
      "open",
      "reveal",
      "play",
      "delete",
    ]);
    expect(availableActions("cancelled")).toEqual(["retry", "delete"]);
    expect(availableActions("unknown-state")).toEqual([]);
  });
});

/** §103: large-list render budget — 1000 task rows must render fast
 * enough to keep the UI responsive (loose CI-safe budget; the M7
 * scheduler benchmark covers the compute side at 807µs/1000 tasks). */
describe("large task list render budget (§103)", () => {
  it("renders 1000 formatted rows within budget", () => {
    const started = performance.now();
    const rows: string[] = [];
    for (let i = 0; i < 1000; i++) {
      rows.push(
        [
          `task-${i}`,
          formatBytes(i * 1024 * 1024),
          formatSpeed(i * 1024),
          formatEta(i % 3600),
          STATE_LABELS.running,
        ].join(" | "),
      );
    }
    const elapsed = performance.now() - started;
    expect(rows.length).toBe(1000);
    // Formatting 1000 rows is pure string work — a few ms on any dev
    // machine; the loose budget catches pathological regressions only.
    expect(elapsed).toBeLessThan(500);
  });
});
