import { describe, expect, it } from "vitest";
import {
  describeDownloadError,
  formatBytes,
  formatEta,
  formatSpeed,
  truncateMiddle,
} from "../lib/format";

/** M9(上) §11/§12/§60: ONE formatting implementation for every view. */
describe("format utilities", () => {
  it("formats speeds with a real zero-state (§11)", () => {
    expect(formatSpeed(0)).toBe("—");
    expect(formatSpeed(null)).toBe("—");
    expect(formatSpeed(NaN)).toBe("—");
    expect(formatSpeed(512)).toBe("512 B/s");
    expect(formatSpeed(128 * 1024)).toBe("128.0 KB/s");
    expect(formatSpeed(3.4 * 1024 * 1024)).toBe("3.4 MB/s");
  });

  it("never shows absurd or negative ETA (§12)", () => {
    expect(formatEta(null)).toBe("—");
    expect(formatEta(-12)).toBe("—");
    expect(formatEta(NaN)).toBe("—");
    expect(formatEta(99999 * 3600)).toBe("—",); // absurd → hidden
    expect(formatEta(0.4)).toBe("<1s");
    expect(formatEta(45)).toBe("45s");
    expect(formatEta(125)).toBe("2m 5s");
    expect(formatEta(3 * 3600 + 1200)).toBe("3h 20m");
    expect(formatEta(30, "paused")).toBe("Paused");
  });

  it("formats bytes consistently (§60)", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(4 * 1024 * 1024)).toBe("4.0 MB");
    expect(formatBytes(5 * 1024 * 1024 * 1024)).toBe("5.00 GB");
    expect(formatBytes(-5)).toBe("—");
  });

  it("truncates long names in the middle, keeping both ends (§59)", () => {
    const long = "a-very-long-file-name-that-exceeds-the-limit.mp4";
    const shown = truncateMiddle(long, 20);
    expect(shown.length).toBeLessThanOrEqual(20);
    expect(shown).toMatch(/a-very-lo…\.mp4$|…/);
    expect(truncateMiddle("short.mp4", 20)).toBe("short.mp4");
  });
});

/** §85/§16: domain error → explanation + recovery, no raw codes. */
describe("download error copy", () => {
  it("maps known codes to explanation + recovery", () => {
    const disk = describeDownloadError("DOWNLOAD_DISK_FULL");
    expect(disk.message).toMatch(/disk is full/i);
    expect(disk.recovery).toMatch(/free up space|destination/i);

    const checksum = describeDownloadError("DOWNLOAD_CHECKSUM_MISMATCH");
    expect(checksum.message).toMatch(/damaged|checksum/i);
    expect(checksum.recovery).toMatch(/retry/i);
  });

  it("has a neutral default that still offers recovery", () => {
    const unknown = describeDownloadError("SOMETHING_NEW");
    expect(unknown.message).toMatch(/failed/i);
    expect(unknown.recovery).toMatch(/retry/i);
  });
});
