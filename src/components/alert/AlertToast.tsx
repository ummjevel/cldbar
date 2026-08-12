import { motion } from "framer-motion";
import { AlertTriangle, Clock, X, TrendingUp } from "lucide-react";
import { useNow } from "../../hooks/useNow";
import { formatCountdown, formatResetClock, limitTone } from "../../lib/limits";
import { providerLabels } from "../../lib/colors";
import { ProviderIcon } from "../tray/ProviderIcon";
import type { Alert, AlertSeverity } from "../../lib/types";

const severityColors: Record<AlertSeverity, string> = {
  critical: "var(--color-danger)",
  warning: "var(--color-warning)",
  info: "var(--color-success)",
};

interface Props {
  alert: Alert;
  onDismiss: () => void;
  onOpen: () => void;
}

export function AlertToast({ alert, onDismiss, onOpen }: Props) {
  const now = useNow(1000);
  const accent = severityColors[alert.severity] ?? severityColors.info;
  const tone = limitTone(Math.max(0, 100 - alert.utilization));
  const countdown = formatCountdown(alert.resetsAt, now);
  const clock = formatResetClock(alert.resetsAt, now);

  const headline =
    alert.kind === "reset"
      ? `${alert.windowLabel} limit resets soon`
      : `${alert.windowLabel} limit at ${alert.utilization.toFixed(0)}%`;

  return (
    <motion.div
      layout
      initial={{ opacity: 0, x: 32, scale: 0.97 }}
      animate={{ opacity: 1, x: 0, scale: 1 }}
      exit={{ opacity: 0, x: 32, scale: 0.97, transition: { duration: 0.15 } }}
      transition={{ type: "spring", stiffness: 260, damping: 26 }}
      className="relative overflow-hidden rounded-xl border border-border bg-bg"
      style={{
        boxShadow: "var(--theme-shadow)",
        backdropFilter: "blur(24px)",
        WebkitBackdropFilter: "blur(24px)",
      }}
    >
      {/* Severity spine */}
      <div className="absolute inset-y-0 left-0 w-[3px]" style={{ backgroundColor: accent }} />

      <div className="pl-3.5 pr-2.5 py-2.5">
        {/* Header: who and what */}
        <div className="flex items-start gap-2">
          <div className="shrink-0 mt-0.5">
            <ProviderIcon type={alert.providerType} size={16} />
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              <span className="text-[11px] font-semibold text-text truncate">
                {alert.profileName}
              </span>
              <span className="text-[9px] text-muted truncate">
                {providerLabels[alert.providerType] ?? alert.providerType}
              </span>
            </div>
            <div className="flex items-center gap-1 mt-0.5">
              {alert.kind === "reset" ? (
                <Clock size={10} style={{ color: accent }} className="shrink-0" />
              ) : alert.severity === "info" ? (
                <TrendingUp size={10} style={{ color: accent }} className="shrink-0" />
              ) : (
                <AlertTriangle size={10} style={{ color: accent }} className="shrink-0" />
              )}
              <span className="text-[12px] font-semibold text-text leading-tight truncate">
                {headline}
              </span>
            </div>
          </div>

          <button
            onClick={onDismiss}
            aria-label="Dismiss"
            className="shrink-0 p-1 rounded-md hover:bg-card-hover transition-colors"
          >
            <X size={12} className="text-muted" />
          </button>
        </div>

        {/* Usage bar */}
        <div className="mt-2 relative h-1.5 rounded-full bg-card overflow-hidden border border-border">
          <motion.div
            className="absolute inset-y-0 left-0 rounded-full"
            style={{ backgroundColor: tone.color }}
            initial={{ width: 0 }}
            animate={{ width: `${Math.min(alert.utilization, 100)}%` }}
            transition={{ type: "spring", stiffness: 120, damping: 20 }}
          />
        </div>

        {/* Facts row: used / left / reset */}
        <div className="mt-2 grid grid-cols-3 gap-1.5">
          <Fact label="Used" value={`${alert.utilization.toFixed(0)}%`} color={tone.color} />
          <Fact label="Left" value={`${Math.max(0, 100 - alert.utilization).toFixed(0)}%`} />
          <Fact label="Resets in" value={countdown} sub={clock} />
        </div>

        <div className="mt-2 flex items-center justify-between gap-2">
          <span className="text-[9px] text-muted truncate">
            {alert.kind === "reset" && alert.reminderMinutes != null
              ? `Reminder · ${alert.reminderMinutes}m before reset`
              : alert.threshold != null
                ? `Crossed your ${alert.threshold}% threshold`
                : "Usage update"}
          </span>
          <button
            onClick={onOpen}
            className="shrink-0 px-2 py-1 rounded-md text-[10px] font-medium border border-border bg-card hover:bg-card-hover text-text-secondary transition-colors"
          >
            View details
          </button>
        </div>
      </div>
    </motion.div>
  );
}

function Fact({
  label,
  value,
  sub,
  color,
}: {
  label: string;
  value: string;
  sub?: string;
  color?: string;
}) {
  return (
    <div className="px-1.5 py-1 rounded-md bg-card border border-border min-w-0">
      <div className="text-[8px] font-medium text-muted uppercase tracking-wider truncate">
        {label}
      </div>
      <div
        className="text-[11px] font-bold tabular-nums truncate"
        style={{ color: color ?? "var(--color-text)" }}
      >
        {value}
      </div>
      {sub && <div className="text-[8px] text-muted tabular-nums truncate">{sub}</div>}
    </div>
  );
}
