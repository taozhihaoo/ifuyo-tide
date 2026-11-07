import { useCallback, useRef, useState } from "react";
import type { PlayerSnapshot } from "../controller/PlaybackController";

/**
 * M5 §24 — the timeline: played / buffered / seekable as three
 * DISTINCT visual layers (§22 — never draw downloaded% as buffered%).
 *
 * Storm prevention (§24): pointer MOVE only updates a local preview;
 * the real seek fires on pointerup, Enter, or a keyboard step. Rapid
 * seeks are retargeted by the controller (§86).
 */
export function Timeline({
  snapshot,
  onSeek,
}: {
  snapshot: PlayerSnapshot;
  onSeek: (seconds: number) => void;
}) {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const [previewSeconds, setPreviewSeconds] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);

  const duration =
    snapshot.duration ?? snapshot.seekableDuration ?? null;
  const current = snapshot.currentTime ?? 0;
  const playedPct = duration ? Math.min(100, (current / duration) * 100) : 0;
  const bufferedPct = duration && snapshot.bufferedAhead != null
    ? Math.min(100, ((current + snapshot.bufferedAhead) / duration) * 100)
    : 0;
  const seekablePct = duration
    ? Math.min(100, ((snapshot.seekableDuration ?? duration) / duration) * 100)
    : 0;

  const secondsAt = useCallback(
    (clientX: number): number | null => {
      const track = trackRef.current;
      if (!track || !duration) return null;
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      return ratio * duration;
    },
    [duration],
  );

  const onPointerDown = (e: React.PointerEvent) => {
    if (!duration) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    setDragging(true);
    setPreviewSeconds(secondsAt(e.clientX));
  };

  const onPointerMove = (e: React.PointerEvent) => {
    // Only a drag produces a visible preview; hovering used to
    // re-render the whole timeline on every mouse move for nothing.
    if (!dragging) return;
    setPreviewSeconds(secondsAt(e.clientX));
  };

  const cancelDrag = () => {
    // A cancelled pointer never fires pointerup — leaving `dragging`
    // true would freeze the seek head on the next interaction.
    setDragging(false);
    setPreviewSeconds(null);
  };

  const commit = (e: React.PointerEvent) => {
    if (!dragging) return;
    e.currentTarget.releasePointerCapture(e.pointerId);
    setDragging(false);
    setPreviewSeconds(null);
    const target = secondsAt(e.clientX);
    if (target != null) onSeek(target);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!duration) return;
    const step = duration / 20;
    if (e.key === "Enter") {
      e.preventDefault();
      if (previewSeconds != null) {
        onSeek(previewSeconds);
        setPreviewSeconds(null);
      }
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      onSeek(Math.min(duration, current + step));
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      onSeek(Math.max(0, current - step));
    }
  };

  const shown = dragging && previewSeconds != null ? previewSeconds : current;

  return (
    <div
      className="timeline"
      data-testid="timeline"
      role="slider"
      aria-label="Seek"
      aria-valuemin={0}
      aria-valuemax={duration ?? 0}
      aria-valuenow={Math.round(shown)}
      tabIndex={0}
      ref={trackRef}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={commit}
      onPointerCancel={cancelDrag}
      onPointerLeave={() => {
        if (!dragging) setPreviewSeconds(null);
      }}
      onKeyDown={onKeyDown}
    >
      <div className="timeline-layer timeline-seekable" style={{ width: `${seekablePct}%` }} />
      <div className="timeline-layer timeline-buffered" style={{ width: `${bufferedPct}%` }} />
      <div className="timeline-layer timeline-played" style={{ width: `${playedPct}%` }} />
      {duration != null && (
        <div
          className="timeline-knob"
          style={{ left: `${duration ? Math.min(100, (shown / duration) * 100) : 0}%` }}
        />
      )}
    </div>
  );
}
