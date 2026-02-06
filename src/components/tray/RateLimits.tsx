import { motion } from "framer-motion";
import { Clock, AlertTriangle } from "lucide-react";
import type { RateLimitStatus, RateLimitWindow } from "../../lib/types";

function formatResetTime(isoDate: string | null): string {
  if (!isoDate) return "";
  const diff = new Date(isoDate).getTime() - Date.now();
  if (diff <= 0) return "resetting...";
  const hours = Math.floor(diff / 3600000);
  const mins = Math.floor((diff % 3600000) / 60000);
  if (hours > 0) return `${hours}h ${mins}m`;
  if (mins > 0) return `${mins}m`;
  return "<1m";
}

function barColor(pct: number): string {
  if (pct >= 80) return "#ef4444";
  if (pct >= 60) return "#f59e0b";
  return "var(--provider-claude)";
}

function WindowBar({ window: w }: { window: RateLimitWindow }) {
  const color = barColor(w.utilization);
  const isHigh = w.utilization >= 80;

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1">
          {isHigh && <AlertTriangle size={9} style={{ color }} />}
          <span className="text-[10px] font-medium text-text-secondary">{w.label}</span>
        </div>
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] font-bold tabular-nums" style={{ color }}>
            {w.utilization.toFixed(1)}%
          </span>
          {w.resetsAt && (
            <span className="text-[9px] text-muted flex items-center gap-0.5">
              <Clock size={8} />
              {formatResetTime(w.resetsAt)}
            </span>
          )}
        </div>
      </div>
      <div className="relative h-1.5 rounded-full bg-card overflow-hidden border border-border">
        <motion.div
          className="absolute inset-y-0 left-0 rounded-full"
          style={{ backgroundColor: color }}
          initial={{ width: 0 }}
          animate={{ width: `${Math.min(w.utilization, 100)}%` }}
          transition={{ type: "spring", stiffness: 100, damping: 20, delay: 0.1 }}
        />
      </div>
    </div>
  );
}

interface Props {
  status: RateLimitStatus | null;
}

export function RateLimits({ status }: Props) {
  if (!status || !status.available) return null;

  const windows: RateLimitWindow[] = [
    status.fiveHour,
    status.sevenDay,
    status.sevenDayOpus,
  ].filter((w): w is RateLimitWindow => w != null);

  if (windows.length === 0) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, delay: 0.05 }}
    >
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">Rate Limits</span>
        <span className="text-[9px] text-muted">Claude Code</span>
      </div>
      <div className="space-y-2 p-2.5 rounded-lg bg-card border border-border">
        {windows.map((w) => (
          <WindowBar key={w.label} window={w} />
        ))}
      </div>
    </motion.div>
  );
}
