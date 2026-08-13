import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { isDialogOpen, isDragging } from "../lib/windowState";

/** Logical width of the popup window, in CSS pixels. */
export const POPUP_WIDTH = 380;

interface Options {
  /** Called when the popup regains focus after being hidden — the moment the numbers need to be current. */
  onOpen: () => void;
  /** Called on Escape. Return true when consumed (e.g. backed out of a sub-view); false hides the window. */
  onEscape: () => boolean;
  /** Logical (CSS px) height the window should currently have. */
  height: number;
}

/**
 * Native-window behavior for the tray popup:
 *
 * - Hide on blur, debounced so drags and native dialogs don't dismiss it.
 * - Refresh on focus, via `onOpen`. Reopening the popup is when the numbers
 *   actually need to be current; the limit lookup goes through the backend's
 *   shared cache, so this adds no network calls. (This replaced a five-second
 *   poll that was re-reading the logs and re-hitting the usage APIs for
 *   numbers that move far slower.)
 * - Escape backs out of sub-views, then hides the window.
 * - Height follows the active view; the top-left corner stays fixed.
 */
export function useTrayWindow(options: Options) {
  // Listeners are subscribed once; the latest callbacks are reached via a ref
  // so a profile or view change never tears down the native subscriptions.
  const callbacks = useRef(options);
  callbacks.current = options;

  useEffect(() => {
    const win = getCurrentWindow();
    let blurTimeout: ReturnType<typeof setTimeout> | null = null;

    const unlistenBlur = win.listen("tauri://blur", () => {
      if (!isDialogOpen() && !isDragging()) {
        blurTimeout = setTimeout(() => {
          // Clear the handle before hiding: a stale one would make the next
          // focus look like a return from a drag or dialog and swallow the
          // refresh-on-open.
          blurTimeout = null;
          win.hide();
        }, 150);
      }
    });

    const unlistenFocus = win.listen("tauri://focus", () => {
      if (blurTimeout) {
        // Returning from a drag or dialog, not a fresh open.
        clearTimeout(blurTimeout);
        blurTimeout = null;
        return;
      }
      callbacks.current.onOpen();
    });

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (!callbacks.current.onEscape()) {
        win.hide();
      }
    };
    window.addEventListener("keydown", onKeyDown);

    return () => {
      unlistenBlur.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
      window.removeEventListener("keydown", onKeyDown);
      if (blurTimeout) clearTimeout(blurTimeout);
    };
  }, []);

  const { height } = options;
  useEffect(() => {
    const scale = window.devicePixelRatio || 1;
    const size = new PhysicalSize(Math.round(POPUP_WIDTH * scale), Math.round(height * scale));
    getCurrentWindow()
      .setSize(size)
      .catch((e) => console.error("Window resize failed:", e));
  }, [height]);
}
