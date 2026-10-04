/**
 * M9(上) §55/§56/§57 — centralized product language. ONE module holds
 * every user-facing string so no copy is scattered across components
 * and the eventual i18n framework has a single extraction point.
 *
 * Locale: English is the product language today (§57 audit — the UI
 * was already English-dominant; the previous mixed-Chinese copy was
 * an inconsistency). Structure is locale-ready: swap this object for
 * a locale lookup when i18n lands (M9下/later).
 */
export const strings = {
  player: {
    loading: "Loading…",
    buffering: "Buffering…",
    waitingForDownload: "Waiting for download data…",
    ended: "Ended",
    unsupportedTitle: "This media format cannot be played in this environment.",
    decodeTitle: "Decoding failed — the file may be damaged or the codec unsupported.",
    openInDefaultApp: "Open in default app",
    retry: "Retry",
    closePlayer: "Close player",
  },
  downloads: {
    emptyTitle: "No downloads yet",
    emptyHint: "Paste a link to start your first download.",
    startedNotice: (id: string) => `Download started (${id.slice(0, 8)}…).`,
  },
  torrents: {
    addedNotice: (id: string) => `Torrent added (${id.slice(0, 8)}…).`,
  },
  library: {
    nothingPlaying: "Nothing playing",
    nothingPlayingHint:
      "Press Play on a download or a torrent file — playback streams through the Tide gateway.",
    fileUnavailable: "File unavailable",
    recheck: "Recheck",
    removedNotice: "Removed from library (file kept).",
  },
  welcome: {
    title: "Tide",
    tagline: "Download. Watch. Flow.",
    intro:
      "Tide downloads files over HTTP and BitTorrent and lets you watch them while they download. Everything stays on your machine.",
    downloadDirectory: "Download directory",
    downloadDirectoryHelp: "Where finished downloads are placed.",
    choose: "Choose…",
    getStarted: "Get started",
  },
  tray: {
    show: "Show Tide",
    pauseAll: "Pause all downloads",
    resumeAll: "Resume all downloads",
    quit: "Quit Tide",
  },
  errors: {
    viewCrashed: "This view hit an unexpected error. The rest of Tide still works.",
    tryAgain: "Try again",
    playerCrashedTitle: "This view hit an unexpected error.",
  },
} as const;
