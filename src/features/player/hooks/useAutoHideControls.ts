import { useEffect, useRef, useState } from "react";

/**
 * M5 §45 — controls auto-hide: hide after 3 s of mouse idle WHILE
 * playing; always visible while paused/buffering/error. One timer,
 * cleaned up on unmount.
 */
export function useAutoHideControls(playing: boolean, enabled: boolean): {
  controlsVisible: boolean;
  notifyActivity: () => void;
} {
  const [controlsVisible, setControlsVisible] = useState(true);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    if (!enabled || !playing) {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      timerRef.current = null;
      setControlsVisible(true);
      return;
    }
    const arm = () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      timerRef.current = window.setTimeout(() => setControlsVisible(false), 3000);
    };
    arm();
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    };
  }, [enabled, playing]);

  const notifyActivity = () => {
    setControlsVisible(true);
    if (enabled && playing) {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      timerRef.current = window.setTimeout(() => setControlsVisible(false), 3000);
    }
  };

  return { controlsVisible, notifyActivity };
}
