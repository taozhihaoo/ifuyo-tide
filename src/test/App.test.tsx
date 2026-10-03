import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../app/App";
import * as api from "../lib/api";
import { TideApiError } from "../lib/ipc";
import type { TaskDto } from "../lib/types";

/**
 * Frontend tests mock the API layer (the Rust IPC boundary) and verify
 * real rendering behaviour: main page, empty list, task appears, and
 * structured errors surfacing in the UI.
 */

vi.mock("../lib/api", () => ({
  getAppInfo: vi.fn(),
  listTasks: vi.fn(),
  createTestTask: vi.fn(),
  deleteTask: vi.fn(),
  updateTaskState: vi.fn(),
  getSettings: vi.fn(),
  setDownloadDirectory: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

const listTasksMock = vi.mocked(api.listTasks);
const updateTaskStateMock = vi.mocked(api.updateTaskState);

function makeTask(overrides: Partial<TaskDto> = {}): TaskDto {
  return {
    taskId: "5a0e6d1e-6f5e-4b7e-9d3a-1a2b3c4d5e6f",
    sourceId: "6a0e6d1e-6f5e-4b7e-9d3a-1a2b3c4d5e6f",
    name: "Integration task",
    state: "ready",
    createdAt: 1_767_225_600_000,
    updatedAt: 1_767_225_600_000,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("App (M0 skeleton)", () => {
  it("renders the main page with an empty task list", async () => {
    listTasksMock.mockResolvedValue([]);

    render(<App />);

    expect(screen.getByRole("navigation", { name: "Main navigation" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Downloads" })).toBeInTheDocument();
    expect(await screen.findByText("No downloads yet")).toBeInTheDocument();
  });

  it("shows a task card after the backend reports one", async () => {
    listTasksMock.mockResolvedValue([makeTask()]);

    render(<App />);

    expect(await screen.findByTestId("task-card")).toBeInTheDocument();
    expect(screen.getByText("Integration task")).toBeInTheDocument();
    expect(screen.getByText("ready")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete" })).toBeInTheDocument();
  });

  it("surfaces structured backend errors instead of crashing", async () => {
    listTasksMock.mockResolvedValue([makeTask()]);
    updateTaskStateMock.mockRejectedValue(
      new TideApiError(
        "STATE_TRANSITION",
        "illegal state transition from ready to completed",
      ),
    );

    render(<App />);
    const startButton = await screen.findByRole("button", { name: "Start" });
    fireEvent.click(startButton);

    const banner = await screen.findByRole("alert");
    expect(banner).toHaveTextContent("illegal state transition from ready to completed");
    expect(banner).toHaveTextContent("STATE_TRANSITION");
  });
});
