import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertToast } from "./AlertToast";
import { useSettings } from "../../hooks/useProviderData";
import { applyTheme } from "../../lib/theme";
import type { Alert } from "../../lib/types";

/** Toasts visible at once; older ones are dropped rather than stacked forever. */
const MAX_VISIBLE = 4;
const TOAST_WIDTH = 380;
const TICK_MS = 500;

interface QueuedAlert extends Alert {
  /** Epoch ms at which this toast disappears; Infinity when it must be dismissed manually. */
  expiresAt: number;
}

export function AlertOverlay() {
  const { settings } = useSettings();
  const [alerts, setAlerts] = useState<QueuedAlert[]>([]);
  const contentRef = useRef<HTMLDivElement>(null);
  const hoveringRef = useRef(false);

  // The overlay window has its own webview, so it applies the theme itself.
  useEffect(() => {
    const theme = settings?.theme || "system";
    applyTheme(theme);
    if (theme !== "system") return;

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [settings?.theme]);

  const durationMs = (settings?.alerts?.durationSecs ?? 12) * 1000;

  const pull = useCallback(async () => {
    try {
      const incoming = await invoke<Alert[]>("drain_alerts");
      if (incoming.length === 0) return;

      setAlerts((prev) => {
        const known = new Set(prev.map((a) => a.id));
        const fresh = incoming
          .filter((a) => !known.has(a.id))
          .map((a) => ({
            ...a,
            expiresAt: durationMs > 0 ? Date.now() + durationMs : Infinity,
          }));
        return [...prev, ...fresh].slice(-MAX_VISIBLE);
      });
    } catch (e) {
      console.error("Failed to drain alerts:", e);
    }
  }, [durationMs]);

  // Pick up anything queued before this window finished loading, then follow events.
  useEffect(() => {
    pull();
    const unlisten = listen("alert://push", () => pull());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [pull]);

  // Expire toasts, holding the countdown while the pointer is over the stack.
  useEffect(() => {
    const id = setInterval(() => {
      if (hoveringRef.current) {
        setAlerts((prev) =>
          prev.map((a) => (a.expiresAt === Infinity ? a : { ...a, expiresAt: a.expiresAt + TICK_MS })),
        );
        return;
      }
      const now = Date.now();
      setAlerts((prev) => {
        const next = prev.filter((a) => a.expiresAt > now);
        return next.length === prev.length ? prev : next;
      });
    }, TICK_MS);
    return () => clearInterval(id);
  }, []);

  // Keep the native window exactly as tall as the stack.
  useLayoutEffect(() => {
    const el = contentRef.current;
    if (!el) return;

    const sync = () => {
      const scale = window.devicePixelRatio || 1;
      const height = Math.max(1, Math.ceil(el.offsetHeight));
      invoke("resize_alert_window", {
        width: Math.round(TOAST_WIDTH * scale),
        height: Math.round(height * scale),
      }).catch((e) => console.error("Failed to resize alert window:", e));
    };

    sync();
    const observer = new ResizeObserver(sync);
    observer.observe(el);
    return () => observer.disconnect();
  }, [alerts.length]);

  const dismiss = useCallback((id: string) => {
    setAlerts((prev) => prev.filter((a) => a.id !== id));
  }, []);

  const openMain = useCallback((id: string) => {
    invoke("show_main_window").catch((e) => console.error("Failed to open main window:", e));
    setAlerts((prev) => prev.filter((a) => a.id !== id));
  }, []);

  // Nothing left to show: give the window back to the desktop.
  const handleExitComplete = useCallback(() => {
    if (alerts.length === 0) {
      invoke("hide_alert_window").catch((e) => console.error("Failed to hide alert window:", e));
    }
  }, [alerts.length]);

  return (
    <div
      className="min-h-screen flex flex-col justify-end bg-transparent"
      onMouseEnter={() => (hoveringRef.current = true)}
      onMouseLeave={() => (hoveringRef.current = false)}
    >
      <div ref={contentRef} className="flex flex-col gap-2 p-2">
        <AnimatePresence initial={false} mode="popLayout" onExitComplete={handleExitComplete}>
          {alerts.map((alert) => (
            <AlertToast
              key={alert.id}
              alert={alert}
              onDismiss={() => dismiss(alert.id)}
              onOpen={() => openMain(alert.id)}
            />
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}
