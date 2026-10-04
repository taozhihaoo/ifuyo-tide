import { useEffect } from "react";

/**
 * M5 §46 — keyboard shortcuts. Skips events originating from inputs,
 * textareas, selects, content-editables and sliders so it never
 * fights with the volume control or the timeline.
 */
export function useKeyboardShortcuts(handlers: {
  onToggle: () => void;
  onStep: (seconds: number) => void;
  onVolumeStep: (delta: number) => void;
  onMute: () => void;
  onFullscreen: () => void;
}): void {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable ||
          target.getAttribute("role") === "slider")
      ) {
        return;
      }
      switch (e.key) {
        case " ":
        case "k":
          e.preventDefault();
          handlers.onToggle();
          break;
        case "ArrowLeft":
          e.preventDefault();
          handlers.onStep(-5);
          break;
        case "ArrowRight":
          e.preventDefault();
          handlers.onStep(5);
          break;
        case "ArrowUp":
          e.preventDefault();
          handlers.onVolumeStep(0.1);
          break;
        case "ArrowDown":
          e.preventDefault();
          handlers.onVolumeStep(-0.1);
          break;
        case "m":
        case "M":
          handlers.onMute();
          break;
        case "f":
        case "F":
          handlers.onFullscreen();
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handlers]);
}
