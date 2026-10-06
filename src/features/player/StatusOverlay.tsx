import { useStrings } from "../../lib/useStrings";
import type { PlayerSnapshot } from "./controller/PlaybackController";

/**
 * M5 §13/§57 — status overlay driven by REAL state only. A buffering
 * wait is explained: with backend availability the UI can distinguish
 * "buffering" (browser) from "waiting for download" (backend behind),
 * and never invents percentages.
 */
export function StatusOverlay({
  snapshot,
  backendAvailableBytes,
  sizeBytes,
  waitingForDownload,
}: {
  snapshot: PlayerSnapshot;
  backendAvailableBytes: number | null;
  sizeBytes: number | null;
  waitingForDownload: boolean;
}) {
  const strings = useStrings();
  switch (snapshot.state) {
    case "idle":
      return null;
    case "loading":
      return (
        <div className="status-overlay" data-testid="status-overlay">
          <span>{strings.player.loading}</span>
        </div>
      );
    case "buffering":
    case "seeking": {
      const behind =
        backendAvailableBytes != null &&
        sizeBytes != null &&
        backendAvailableBytes < sizeBytes;
      return (
        <div className="status-overlay" data-testid="status-overlay">
          <span>{waitingForDownload && behind ? strings.player.waitingForDownload : strings.player.buffering}</span>
        </div>
      );
    }
    case "ended":
      return (
        <div className="status-overlay" data-testid="status-overlay">
          <span>{strings.player.ended}</span>
        </div>
      );
    default:
      return null;
  }
}
