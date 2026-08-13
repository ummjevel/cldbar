import type { LimitDisplay, LimitWindowKey, RateLimitStatus, RateLimitWindow } from "./types";

/** Limit windows in the order they are shown, most immediate first. */
export const limitWindowKeys: LimitWindowKey[] = ["fiveHour", "sevenDay", "sevenDayOpus"];

export const limitWindowLabels: Record<LimitWindowKey, string> = {
  fiveHour: "5-Hour",
  sevenDay: "7-Day",
  sevenDayOpus: "7-Day Opus",
};

/**
 * What each window means to the user, rather than how long it is. "5-Hour" is
 * the mechanism; "Session" is what they are actually spending against.
 */
export const limitWindowRoles: Record<LimitWindowKey, string> = {
  fiveHour: "Session",
  sevenDay: "Weekly",
  sevenDayOpus: "Weekly · Opus",
};

export interface LimitWindowEntry {
  key: LimitWindowKey;
  window: RateLimitWindow;
}

/** Flatten a status into the windows the provider actually reported. */
export function listLimitWindows(status: RateLimitStatus | null): LimitWindowEntry[] {
  if (!status || !status.available) return [];
  return limitWindowKeys
    .map((key) => ({ key, window: status[key] }))
    .filter((e): e is LimitWindowEntry => e.window != null);
}

export interface LimitTone {
  label: string;
  color: string;
}

export interface LimitReading {
  /** The number shown in the headline, per the user's preference. */
  value: number;
  /** Word under it: "remaining" or "used". */
  caption: string;
  /** The other side of the same fact, for the hover explainer. */
  inverseText: string;
  /** Compact form for the smaller rows, e.g. "67% left". */
  shortText: string;
}

/**
 * Split a window's utilization into whichever direction the user prefers to
 * read. The colour is always keyed on what is left, so switching the display
 * never changes what counts as healthy.
 */
export function limitReading(utilization: number, display: LimitDisplay): LimitReading {
  const used = Math.min(100, Math.max(0, utilization));
  const remaining = 100 - used;

  return display === "used"
    ? {
        value: used,
        caption: "used",
        inverseText: `${remaining.toFixed(0)}% still available this window`,
        shortText: `${used.toFixed(0)}% used`,
      }
    : {
        value: remaining,
        caption: "remaining",
        inverseText: `${used.toFixed(0)}% used so far this window`,
        shortText: `${remaining.toFixed(0)}% left`,
      };
}

/**
 * Status wording and colour for how much of a window is **left**.
 *
 * The turns sit at 50% and 20% remaining, i.e. 50% and 80% used, which is
 * where the default alert thresholds fire — so the colour changes at the same
 * points the app decides the usage is worth mentioning.
 *
 * The label rides along with the colour so the state is never carried by hue
 * alone.
 */
export function limitTone(remaining: number): LimitTone {
  if (remaining <= 20) return { label: "Almost out", color: "var(--color-danger)" };
  if (remaining <= 50) return { label: "Running low", color: "var(--color-warning)" };
  return { label: "On track", color: "var(--color-success)" };
}

/** "2h 14m" / "8m" / "45s" — the time left before a reset. */
export function formatCountdown(resetsAt: string | null, now: number = Date.now()): string {
  if (!resetsAt) return "—";
  const target = new Date(resetsAt).getTime();
  if (isNaN(target)) return "—";

  const diff = target - now;
  if (diff <= 0) return "resetting";

  const totalMins = Math.floor(diff / 60000);
  const days = Math.floor(totalMins / 1440);
  const hours = Math.floor((totalMins % 1440) / 60);
  const mins = totalMins % 60;

  if (days > 0) return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
  if (hours > 0) return `${hours}h ${mins}m`;
  if (totalMins > 0) return `${totalMins}m`;
  return `${Math.floor(diff / 1000)}s`;
}

/**
 * Absolute wall-clock time of the reset, e.g. "4:30 PM" or "Tue 4:30 PM"
 * once it is far enough out that the day matters.
 */
export function formatResetClock(resetsAt: string | null, now: number = Date.now()): string {
  if (!resetsAt) return "";
  const target = new Date(resetsAt);
  if (isNaN(target.getTime())) return "";

  const time = target.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const sameDay = target.toDateString() === new Date(now).toDateString();
  if (sameDay) return time;

  const day = target.toLocaleDateString([], { weekday: "short" });
  return `${day} ${time}`;
}
