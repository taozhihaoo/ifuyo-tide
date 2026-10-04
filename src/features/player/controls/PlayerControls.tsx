import { useState } from "react";
import type { PlayerSnapshot } from "../controller/PlaybackController";
import { Timeline } from "./Timeline";

const RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

/**
 * M5 §42 PlayerControls: play/pause, time, timeline, volume (with
 * lastNonZeroVolume restore, §27), playback rate, fullscreen.
 */
export function PlayerControls({
  snapshot,
  onToggle,
  onSeek,
  onVolume,
  onMute,
  onRate,
  onFullscreen,
  fullscreen,
}: {
  snapshot: PlayerSnapshot;
  onToggle: () => void;
  onSeek: (seconds: number) => void;
  onVolume: (v: number) => void;
  onMute: () => void;
  onRate: (rate: number) => void;
  onFullscreen: () => void;
  fullscreen: boolean;
}) {
  // M5 §27: keep the last non-zero volume so mute→unmute restores it
  // instead of unmuting into silence.
  const [lastNonZero, setLastNonZero] = useState(1);

  const changeVolume = (v: number) => {
    const clamped = Math.max(0, Math.min(1, v));
    if (clamped > 0) setLastNonZero(clamped);
    onVolume(clamped);
    if (clamped > 0 && snapshot.muted) onMute();
  };

  const toggleMute = () => {
    if (snapshot.muted) {
      onMute();
      if (snapshot.volume === 0) onVolume(lastNonZero);
    } else {
      if (snapshot.volume > 0) setLastNonZero(snapshot.volume);
      onMute();
    }
  };

  return (
    <div className="controls-bar">
      <Timeline snapshot={snapshot} onSeek={onSeek} />

      <div className="controls-row">
        <button
          type="button"
          className="btn btn-small"
          data-testid="play-pause"
          aria-label={snapshot.state === "playing" ? "Pause" : "Play"}
          onClick={onToggle}
        >
          {snapshot.state === "playing" ? "⏸" : "▶"}
        </button>

        <span className="time-display" data-testid="time-display">
          {formatTime(snapshot.currentTime)}
          {" / "}
          {snapshot.duration != null ? formatTime(snapshot.duration) : "--:--"}
        </span>

        <span className="task-spacer" />

        <button
          type="button"
          className="btn btn-small"
          aria-label={snapshot.muted ? "Unmute" : "Mute"}
          onClick={toggleMute}
        >
          {snapshot.muted || snapshot.volume === 0 ? "🔇" : "🔊"}
        </button>
        <input
          type="range"
          className="volume-slider"
          aria-label="Volume"
          min={0}
          max={100}
          value={snapshot.muted ? 0 : Math.round(snapshot.volume * 100)}
          onChange={(e) => changeVolume(Number(e.target.value) / 100)}
        />

        <label className="rate-label">
          <select
            aria-label="Playback rate"
            value={String(snapshot.playbackRate)}
            onChange={(e) => onRate(Number(e.target.value))}
          >
            {RATES.map((r) => (
              <option key={r} value={r}>
                {r}×
              </option>
            ))}
          </select>
        </label>

        <button
          type="button"
          className="btn btn-small"
          aria-label={fullscreen ? "Exit fullscreen" : "Fullscreen"}
          onClick={onFullscreen}
        >
          {fullscreen ? "⤢" : "⛶"}
        </button>
      </div>
    </div>
  );
}

/** M5 §25: HH:MM:SS (MM:SS under an hour); never NaN — "--:--" when
 * the duration is unavailable. */
function formatTime(secs: number | null): string {
  if (secs == null || !Number.isFinite(secs)) return "--:--";
  const s = Math.floor(secs % 60);
  const m = Math.floor((secs / 60) % 60);
  const h = Math.floor(secs / 3600);
  const mm = String(m).padStart(2, "0");
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}
