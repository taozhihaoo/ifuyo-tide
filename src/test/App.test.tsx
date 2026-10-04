import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../app/App";
import * as api from "../features/tasks/downloadApi";
import { TideApiError } from "../lib/ipc";
import type { DownloadDto } from "../features/tasks/downloadApi";

/**
 * Frontend tests mock the download API layer (the Rust IPC boundary) and
 * verify real rendering: main page, empty list, download card, and
 * structured errors surfacing in the UI.
 */

vi.mock("../features/tasks/downloadApi", () => ({
  startDownload: vi.fn(),
  pauseDownload: vi.fn(),
  resumeDownload: vi.fn(),
  cancelDownload: vi.fn(),
  listDownloads: vi.fn(),
  getDownload: vi.fn(),
  onDownloadProgress: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("../features/settings/windowsApi", () => ({
  drainLaunchRequests: vi.fn().mockResolvedValue([]),
  onLaunchRequest: vi.fn().mockResolvedValue(() => {}),
}));

const listDownloadsMock = vi.mocked(api.listDownloads);
const startDownloadMock = vi.mocked(api.startDownload);
const onProgressMock = vi.mocked(api.onDownloadProgress);

function makeDownload(overrides: Partial<DownloadDto> = {}): DownloadDto {
  return {
    downloadId: "5a0e6d1e-6f5e-4b7e-9d3a-1a2b3c4d5e6f",
    host: "127.0.0.1",
    filename: "sample.bin",
    state: "running",
    phase: "downloading",
    downloadedBytes: 512,
    totalBytes: 1024,
    percentage: 50,
    speedBps: 2048,
    etaSecs: 1,
    errorCode: null,
    errorMessage: null,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  onProgressMock.mockResolvedValue(() => {});
});

afterEach(() => {
  cleanup();
});

describe("App (M1 downloads)", () => {
  it("renders the main page with an empty downloads list", async () => {
    listDownloadsMock.mockResolvedValue([]);

    render(<App />);

    expect(
      screen.getByRole("navigation", { name: "Main navigation" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Downloads" })).toBeInTheDocument();
    expect(await screen.findByText("No downloads yet")).toBeInTheDocument();
  });

  it("shows a real download card with progress", async () => {
    listDownloadsMock.mockResolvedValue([makeDownload()]);

    render(<App />);

    expect(await screen.findByTestId("download-card")).toBeInTheDocument();
    expect(screen.getByText("sample.bin")).toBeInTheDocument();
    expect(screen.getByText("running")).toBeInTheDocument();
    // honest total, speed and ETA from the engine
    expect(screen.getByText(/512 B \/ 1.0 KB/)).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Pause" })).toBeInTheDocument();
  });

  it("surfaces structured backend errors when starting fails", async () => {
    listDownloadsMock.mockResolvedValue([]);
    startDownloadMock.mockRejectedValue(
      new TideApiError(
        "DOWNLOAD_PRIVATE_ADDRESS_BLOCKED",
        "private address blocked: 127.0.0.1",
      ),
    );

    render(<App />);
    const input = await screen.findByLabelText("Download URL");
    fireEvent.input(input, { target: { value: "http://127.0.0.1/f.bin" } });
    fireEvent.click(screen.getByRole("button", { name: "Download" }));

    const banner = await screen.findByRole("alert");
    expect(banner).toHaveTextContent("private address blocked");
  });
});
