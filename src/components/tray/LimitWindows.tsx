import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { MinusCircle } from "lucide-react";
import { Tooltip } from "./Tooltip";
import { useNow } from "../../hooks/useNow";
import { formatTimeAgo } from "../../lib/format";
import {
  formatCountdown,
  formatResetClock,
  limitReading,
  limitTone,
  limitWindowRoles,
  listLimitWindows,
  type LimitWindowEntry,
} from "../../lib/limits";
import { useSettings } from "../../hooks/useProviderData";
import type { LimitDisplay, RateLimitStatus, SourceType } from "../../lib/types";

interface Props {
  status: RateLimitStatus | null;
  providerLabel: string;
  loading: boolean;
  sourceType: SourceType;
}

/** Holds the headline's place while the first reading is on its way. */
function HeadlineSkeleton() {
  return (
    <div className="p-3 rounded-xl bg-card border border-border animate-pulse">
      <div className="h-2 w-24 rounded bg-border mb-4" />
      <div className="flex items-start">
        <div className="flex-1 space-y-2">
          <div className="h-6 w-16 rounded bg-border" />
          <div className="h-2 w-12 rounded bg-border" />
        </div>
        <div className="w-px self-stretch bg-border mx-3" />
        <div className="flex-1 space-y-2 flex flex-col items-end">
          <div className="h-6 w-20 rounded bg-border" />
          <div className="h-2 w-16 rounded bg-border" />
        </div>
      </div>
      <div className="mt-3 h-2 rounded-full bg-border" />
    </div>
  );
}

/**
 * The headline never silently disappears: when there is nothing to show, it
 * says so and names the likely reason, so a broken sign-in cannot look like
 * a screen that simply has no limits section.
 */
function HeadlineUnavailable({ sourceType }: { sourceType: SourceType }) {
  const reason =
    sourceType === "api"
      ? "API keys bill per token rather than against a plan window."
      : "Signed out, offline, or the provider is rate limiting the request. Press refresh to retry.";

  return (
    <div className="p-3 rounded-xl bg-card border border-border">
      <div className="flex items-center gap-2">
        <MinusCircle size={13} className="text-muted shrink-0" />
        <span className="text-xs font-medium text-text">No limit data</span>
      </div>
      <p className="text-[10px] text-muted leading-relaxed mt-1.5">{reason}</p>
    </div>
  );
}

/**
 * The window being spent against right now. This is the whole reason the popup
 * gets opened, so it answers both halves of the question at equal weight:
 * how much is left, and how long until it comes back.
 */
function HeadlineWindow({
  entry,
  now,
  display,
}: {
  entry: LimitWindowEntry;
  now: number;
  display: LimitDisplay;
}) {
  const { key, window: w } = entry;
  const [hovered, setHovered] = useState<"amount" | "reset" | null>(null);
  const remaining = Math.max(0, 100 - w.utilization);
  const reading = limitReading(w.utilization, display);
  const tone = limitTone(remaining);
  const countdown = formatCountdown(w.resetsAt, now);
  const clock = formatResetClock(w.resetsAt, now);

  return (
    <div className="p-3 rounded-xl bg-card border border-border">
      <div className="flex items-center justify-between mb-3">
        <span className="text-[10px] font-medium text-muted uppercase tracking-wider">
          {limitWindowRoles[key]} · {w.label}
        </span>
        <span
          className="flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[9px] font-semibold"
          style={{
            color: tone.color,
            backgroundColor: `color-mix(in srgb, ${tone.color} 14%, transparent)`,
          }}
        >
          <span className="w-1 h-1 rounded-full" style={{ backgroundColor: tone.color }} />
          {tone.label}
        </span>
      </div>

      {/* The two facts, paired rather than ranked. Hovering either one flips it
          round to the spent-so-far view. */}
      <div className="flex items-start">
        <div
          className="relative flex-1 min-w-0 cursor-default"
          onMouseEnter={() => setHovered("amount")}
          onMouseLeave={() => setHovered(null)}
        >
          <AnimatePresence>
            {hovered === "amount" && <Tooltip placement="bottom" text={reading.inverseText} />}
          </AnimatePresence>
          <div
            className="text-[26px] leading-none font-bold tabular-nums"
            style={{ color: tone.color }}
          >
            {reading.value.toFixed(0)}
            <span className="text-base font-semibold">%</span>
          </div>
          <div className="text-[10px] text-muted mt-1.5">{reading.caption}</div>
        </div>

        <div className="w-px self-stretch bg-border mx-3" />

        <div
          className="relative flex-1 min-w-0 text-right cursor-default"
          onMouseEnter={() => setHovered("reset")}
          onMouseLeave={() => setHovered(null)}
        >
          <AnimatePresence>
            {hovered === "reset" && (
              <Tooltip
                placement="bottom"
                text={
                  clock
                    ? `Used ${w.utilization.toFixed(0)}% — back to 0% at ${clock}`
                    : "No reset time reported"
                }
              />
            )}
          </AnimatePresence>
          <div className="text-[26px] leading-none font-bold tabular-nums text-text">
            {countdown}
          </div>
          <div className="text-[10px] text-muted mt-1.5 truncate">
            until reset{clock && ` · ${clock}`}
          </div>
        </div>
      </div>

      {/* Drains as the window is spent, so the bar itself reads as "what's left". */}
      <div className="mt-3 relative h-2 rounded-full bg-bg/40 overflow-hidden border border-border">
        <motion.div
          className="absolute inset-y-0 left-0 rounded-full"
          style={{ backgroundColor: tone.color }}
          initial={{ width: "100%" }}
          animate={{ width: `${Math.min(Math.max(remaining, 0), 100)}%` }}
          transition={{ type: "spring", stiffness: 100, damping: 20 }}
        />
      </div>
    </div>
  );
}

/** Longer windows, compact: how much is left and when it turns over. */
function SecondaryWindow({
  entry,
  now,
  display,
}: {
  entry: LimitWindowEntry;
  now: number;
  display: LimitDisplay;
}) {
  const { key, window: w } = entry;
  const [hovered, setHovered] = useState(false);
  const remaining = Math.max(0, 100 - w.utilization);
  const reading = limitReading(w.utilization, display);
  const tone = limitTone(remaining);

  return (
    <div
      className="relative space-y-1 cursor-default"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <AnimatePresence>
        {hovered && <Tooltip placement="bottom" text={reading.inverseText} />}
      </AnimatePresence>
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10px] font-medium text-text-secondary truncate">
          {limitWindowRoles[key]}
        </span>
        <div className="flex items-baseline gap-1.5 shrink-0">
          <span className="text-[11px] font-bold tabular-nums" style={{ color: tone.color }}>
            {reading.shortText}
          </span>
          {w.resetsAt && (
            <span className="text-[9px] text-muted tabular-nums">
              · {formatCountdown(w.resetsAt, now)}
            </span>
          )}
        </div>
      </div>
      <div className="relative h-1.5 rounded-full bg-bg/40 overflow-hidden border border-border">
        <motion.div
          className="absolute inset-y-0 left-0 rounded-full"
          style={{ backgroundColor: tone.color }}
          initial={{ width: "100%" }}
          animate={{ width: `${Math.min(Math.max(remaining, 0), 100)}%` }}
          transition={{ type: "spring", stiffness: 100, damping: 20, delay: 0.05 }}
        />
      </div>
    </div>
  );
}

export function LimitWindows({ status, providerLabel, loading, sourceType }: Props) {
  const now = useNow(1000);
  const { settings } = useSettings();
  const display: LimitDisplay = settings?.limitDisplay === "used" ? "used" : "remaining";
  const windows = listLimitWindows(status);

  if (windows.length === 0) {
    return loading && status === null ? <HeadlineSkeleton /> : <HeadlineUnavailable sourceType={sourceType} />;
  }

  const [headline, ...rest] = windows;

  return (
    <motion.div
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className="space-y-2"
    >
      <HeadlineWindow entry={headline} now={now} display={display} />

      {rest.length > 0 && (
        <div className="space-y-2 p-2.5 rounded-lg bg-card border border-border">
          {rest.map((entry) => (
            <SecondaryWindow key={entry.key} entry={entry} now={now} display={display} />
          ))}
        </div>
      )}

      {status?.updatedAt && (
        <div className="text-[9px] text-muted text-right">
          {providerLabel} · as of {formatTimeAgo(status.updatedAt)}
        </div>
      )}
    </motion.div>
  );
}
