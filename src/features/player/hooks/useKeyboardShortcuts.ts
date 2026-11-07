import { useEffect, useRef } from "react";

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
  // Callers pass a fresh handlers object on every render (the player
  // snapshot updates continuously). Keep the listener installed once
  // and read the latest handlers through a ref instead of re-binding
  // the global keydown listener on every timeupdate.
  const handlersRef = useRef(handlers);
  useEffect(() => {
    handlersRef.current = handlers;
  }, [handlers]);

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
          handlersRef.current.onToggle();
          break;
        case "ArrowLeft":
          e.preventDefault();
          handlersRef.current.onStep(-5);
          break;
        case "ArrowRight":
          e.preventDefault();
          handlersRef.current.onStep(5);
          break;
        case "ArrowUp":
          e.preventDefault();
          handlersRef.current.onVolumeStep(0.1);
          break;
        case "ArrowDown":
          e.preventDefault();
          handlersRef.current.onVolumeStep(-0.1);
          break;
        case "m":
        case "M":
          handlersRef.current.onMute();
          break;
        case "f":
        case "F":
          handlersRef.current.onFullscreen();
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
