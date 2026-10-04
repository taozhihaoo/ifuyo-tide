import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LibraryView } from "../features/library/LibraryView";
import * as api from "../features/library/libraryApi";
import type { ItemPageDto, LibraryRootDto } from "../features/library/libraryApi";

/**
 * M6(上) frontend tests: the Library view renders real backend rows,
 * filter tabs re-query, Play routes the ITEM ID (never a path), and
 * Delete-from-disk requires confirmation (§25).
 */

vi.mock("../features/library/libraryApi", () => ({
  listRoots: vi.fn(),
  listItems: vi.fn(),
  addRoot: vi.fn(),
  removeRoot: vi.fn(),
  setRootEnabled: vi.fn(),
  scan: vi.fn(),
  refreshItem: vi.fn(),
  removeItem: vi.fn(),
  deleteItem: vi.fn(),
  revealItem: vi.fn(),
  onScanProgress: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const listRootsMock = vi.mocked(api.listRoots);
const listItemsMock = vi.mocked(api.listItems);
const onScanProgressMock = vi.mocked(api.onScanProgress);
const removeItemMock = vi.mocked(api.removeItem);

function makeRoot(overrides: Partial<LibraryRootDto> = {}): LibraryRootDto {
  return {
    id: "root-1",
    displayName: "Media",
    enabled: true,
    scanStatus: "ok",
    lastScanAt: 1_700_000_000_000,
    ...overrides,
  };
}

function makePage(overrides: Partial<ItemPageDto> = {}): ItemPageDto {
  return {
    items: [
      {
        id: "item-1",
        rootId: "root-1",
        displayName: "Movie A.mp4",
        relativePath: "Movie A.mp4",
        extension: "mp4",
        category: "video",
        mimeType: "video/mp4",
        sizeBytes: 4_000_000_000,
        modifiedAt: 1_700_000_000_000,
        availability: "available",
        origin: "download",
        taskId: "task-1",
      },
      {
        id: "item-2",
        rootId: "root-1",
        displayName: "Gone.mkv",
        relativePath: "Gone.mkv",
        extension: "mkv",
        category: "video",
        mimeType: "video/x-matroska",
        sizeBytes: 8_000_000,
        modifiedAt: 1_700_000_000_000,
        availability: "missing",
        origin: "imported_local",
        taskId: null,
      },
    ],
    total: 2,
    page: 0,
    pageSize: 50,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  listRootsMock.mockResolvedValue([makeRoot()]);
  listItemsMock.mockResolvedValue(makePage());
  onScanProgressMock.mockResolvedValue(() => {});
});

afterEach(() => cleanup());

describe("LibraryView (M6上)", () => {
  it("renders items with honest availability and roots summary", async () => {
    render(<LibraryView onPlay={() => {}} />);

    expect(await screen.findByTestId("library-table")).toBeInTheDocument();
    expect(screen.getByText("Movie A.mp4")).toBeInTheDocument();
    expect(screen.getByText("Gone.mkv")).toBeInTheDocument();
    expect(screen.getByText("1 of 1 folders enabled")).toBeInTheDocument();
    // §18: missing items have Play disabled honestly.
    const goneRow = screen.getByText("Gone.mkv").closest("tr")!;
    expect(goneRow.querySelector('button[title*="not available"]')).not.toBeNull();
  });

  it("routes the ITEM ID to onPlay — never a path (§46/§59)", async () => {
    const onPlay = vi.fn();
    render(<LibraryView onPlay={onPlay} />);
    const row = (await screen.findByText("Movie A.mp4")).closest("tr")!;
    fireEvent.click(row.querySelector('button[title*="gateway"]')!);
    await waitFor(() => {
      expect(onPlay).toHaveBeenCalledWith({ kind: "library_item", item_id: "item-1" });
    });
  });

  it("re-queries with the category filter (backend-side, §42)", async () => {
    render(<LibraryView onPlay={() => {}} />);
    await screen.findByTestId("library-table");
    fireEvent.click(screen.getByRole("button", { name: "Video" }));
    await waitFor(() => {
      expect(listItemsMock).toHaveBeenLastCalledWith(
        expect.objectContaining({ category: "video", availability: undefined }),
      );
    });
    fireEvent.click(screen.getByRole("button", { name: "Missing" }));
    await waitFor(() => {
      expect(listItemsMock).toHaveBeenLastCalledWith(
        expect.objectContaining({ availability: "missing", category: undefined }),
      );
    });
  });

  it("remove goes straight through; delete-from-disk requires confirmation (§24/§25)", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    removeItemMock.mockResolvedValue(true);
    render(<LibraryView onPlay={() => {} } />);
    await screen.findByTestId("library-table");

    const row = screen.getByText("Movie A.mp4").closest("tr")!;
    const buttons = [...row.querySelectorAll("button")];
    const removeBtn = buttons.find((b) => b.textContent === "Remove")!;
    const deleteBtn = buttons.find((b) => b.textContent === "Delete file")!;

    fireEvent.click(removeBtn);
    await waitFor(() => expect(removeItemMock).toHaveBeenCalledWith("item-1"));
    expect(confirmSpy).not.toHaveBeenCalled();

    fireEvent.click(deleteBtn);
    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(api.deleteItem).not.toHaveBeenCalled(); // confirm returned false
    confirmSpy.mockRestore();
  });
});
