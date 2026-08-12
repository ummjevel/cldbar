import { useCallback } from "react";
import { motion } from "framer-motion";
import { ArrowLeft, Bell, BellOff, Send } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { startManualDrag } from "../../lib/windowState";
import { providerLabels } from "../../lib/colors";
import { limitWindowKeys, limitWindowLabels } from "../../lib/limits";
import { useSettings } from "../../hooks/useProviderData";
import { ProviderIcon } from "./ProviderIcon";
import type { AlertSettings, LimitWindowKey, Profile, ProviderType } from "../../lib/types";

interface Props {
  profiles: Profile[];
  onBack: () => void;
}

const THRESHOLD_CHOICES = [50, 60, 70, 80, 90, 95];
const REMINDER_CHOICES = [5, 10, 15, 30, 60, 120];
const INTERVAL_CHOICES = [
  { value: 30, label: "30s" },
  { value: 60, label: "1m" },
  { value: 300, label: "5m" },
  { value: 900, label: "15m" },
];
const DURATION_CHOICES = [
  { value: 8, label: "8s" },
  { value: 12, label: "12s" },
  { value: 30, label: "30s" },
  { value: 0, label: "Until dismissed" },
];

/** Add or remove a value, keeping numeric lists in ascending order. */
function toggleIn<T extends string | number>(list: T[], value: T): T[] {
  if (list.includes(value)) return list.filter((v) => v !== value);
  const next = [...list, value];
  return typeof value === "number"
    ? next.sort((a, b) => Number(a) - Number(b))
    : next;
}

function Section({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <span className="text-xs font-medium text-text-secondary uppercase tracking-wider block">
        {title}
      </span>
      {hint && <p className="text-[10px] text-muted mt-0.5 mb-2 leading-relaxed">{hint}</p>}
      <div className={hint ? "" : "mt-2"}>{children}</div>
    </div>
  );
}

function Chip({
  label,
  active,
  onClick,
  disabled,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      aria-pressed={active}
      className="px-2.5 py-1.5 rounded-lg text-[11px] font-medium border transition-all disabled:opacity-40"
      style={{
        borderColor: active ? "var(--color-text-secondary)" : "var(--color-border)",
        backgroundColor: active ? "var(--color-card-hover)" : "var(--color-card)",
        color: active ? "var(--color-text)" : "var(--color-muted)",
      }}
    >
      {label}
    </button>
  );
}

export function AlertSettingsPanel({ profiles, onBack }: Props) {
  const { settings, update } = useSettings();
  const alerts = settings?.alerts;

  const patch = useCallback(
    (changes: Partial<AlertSettings>) => {
      if (!settings || !settings.alerts) return;
      update({ ...settings, alerts: { ...settings.alerts, ...changes } });
    },
    [settings, update],
  );

  const eligible = profiles.filter((p) => p.supportsRateLimits);
  const enabled = alerts?.enabled ?? false;

  return (
    <div className="h-full flex flex-col">
      <div
        className="flex items-center gap-2 px-4 py-2.5 border-b border-border cursor-grab active:cursor-grabbing"
        onMouseDown={startManualDrag}
      >
        <button onClick={onBack} className="p-1.5 rounded-md hover:bg-card-hover transition-colors">
          <ArrowLeft size={14} className="text-muted" />
        </button>
        <span className="text-sm font-semibold text-text">Alerts</span>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
        {/* Master switch */}
        <button
          onClick={() => patch({ enabled: !enabled })}
          className="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-card border transition-colors text-left"
          style={{ borderColor: enabled ? "var(--color-success)" : "var(--color-border)" }}
        >
          {enabled ? (
            <Bell size={14} style={{ color: "var(--color-success)" }} />
          ) : (
            <BellOff size={14} className="text-muted" />
          )}
          <div className="flex-1 min-w-0">
            <div className="text-xs font-medium text-text">
              {enabled ? "Alerts on" : "Alerts off"}
            </div>
            <div className="text-[10px] text-muted">
              Toasts appear in the bottom-right corner
            </div>
          </div>
          <div
            className="w-8 rounded-full relative transition-colors shrink-0"
            style={{
              backgroundColor: enabled ? "var(--color-success)" : "var(--color-border-light)",
              height: 18,
            }}
          >
            <motion.div
              className="absolute top-0.5 w-3.5 h-3.5 rounded-full bg-white"
              animate={{ left: enabled ? 16 : 2 }}
              transition={{ type: "spring", stiffness: 400, damping: 30 }}
            />
          </div>
        </button>

        <div className={enabled ? "space-y-4" : "space-y-4 opacity-45 pointer-events-none"}>
          {/* Usage thresholds */}
          <Section
            title="Usage thresholds"
            hint="Alert the first time a window crosses one of these."
          >
            <div className="flex flex-wrap gap-1.5">
              {THRESHOLD_CHOICES.map((t) => (
                <Chip
                  key={t}
                  label={`${t}%`}
                  active={alerts?.usageThresholds.includes(t) ?? false}
                  onClick={() =>
                    patch({ usageThresholds: toggleIn(alerts?.usageThresholds ?? [], t) })
                  }
                />
              ))}
            </div>
          </Section>

          {/* Reset reminders */}
          <Section
            title="Before reset"
            hint="Remind me this long before a window resets, if it has been used."
          >
            <div className="flex flex-wrap gap-1.5">
              {REMINDER_CHOICES.map((m) => (
                <Chip
                  key={m}
                  label={m >= 60 ? `${m / 60}h` : `${m}m`}
                  active={alerts?.resetReminderMinutes.includes(m) ?? false}
                  onClick={() =>
                    patch({ resetReminderMinutes: toggleIn(alerts?.resetReminderMinutes ?? [], m) })
                  }
                />
              ))}
            </div>
          </Section>

          {/* Accounts */}
          <Section
            title="Accounts"
            hint={
              eligible.length === 0
                ? "No profile reports limit windows yet."
                : "Watch all of them, or pick specific ones."
            }
          >
            <div className="space-y-1.5">
              <button
                onClick={() => patch({ profileIds: [] })}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-lg border transition-colors text-left"
                style={{
                  borderColor:
                    (alerts?.profileIds.length ?? 0) === 0
                      ? "var(--color-text-secondary)"
                      : "var(--color-border)",
                  backgroundColor:
                    (alerts?.profileIds.length ?? 0) === 0
                      ? "var(--color-card-hover)"
                      : "var(--color-card)",
                }}
              >
                <span className="text-xs font-medium text-text">All accounts</span>
                <span className="text-[10px] text-muted ml-auto">
                  {eligible.length} eligible
                </span>
              </button>

              {eligible.map((p) => {
                const selected = alerts?.profileIds.includes(p.id) ?? false;
                return (
                  <button
                    key={p.id}
                    onClick={() => patch({ profileIds: toggleIn(alerts?.profileIds ?? [], p.id) })}
                    className="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg border transition-colors text-left"
                    style={{
                      borderColor: selected ? "var(--color-text-secondary)" : "var(--color-border)",
                      backgroundColor: selected ? "var(--color-card-hover)" : "var(--color-card)",
                    }}
                  >
                    <ProviderIcon type={p.providerType as ProviderType} size={16} />
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium text-text truncate">{p.name}</div>
                      <div className="text-[10px] text-muted truncate">
                        {providerLabels[p.providerType as ProviderType]}
                      </div>
                    </div>
                    <div
                      className="w-3.5 h-3.5 rounded border shrink-0"
                      style={{
                        borderColor: selected
                          ? "var(--color-text-secondary)"
                          : "var(--color-border-light)",
                        backgroundColor: selected ? "var(--color-text-secondary)" : "transparent",
                      }}
                    />
                  </button>
                );
              })}
            </div>
          </Section>

          {/* Windows */}
          <Section title="Limit windows">
            <div className="flex flex-wrap gap-1.5">
              {limitWindowKeys.map((key: LimitWindowKey) => (
                <Chip
                  key={key}
                  label={limitWindowLabels[key]}
                  active={alerts?.windows.includes(key) ?? false}
                  onClick={() => patch({ windows: toggleIn(alerts?.windows ?? [], key) })}
                />
              ))}
            </div>
          </Section>

          {/* Cadence */}
          <Section title="Check every">
            <div className="flex flex-wrap gap-1.5">
              {INTERVAL_CHOICES.map((c) => (
                <Chip
                  key={c.value}
                  label={c.label}
                  active={alerts?.checkIntervalSecs === c.value}
                  onClick={() => patch({ checkIntervalSecs: c.value })}
                />
              ))}
            </div>
          </Section>

          {/* Toast lifetime */}
          <Section title="Toast stays for">
            <div className="flex flex-wrap gap-1.5">
              {DURATION_CHOICES.map((c) => (
                <Chip
                  key={c.value}
                  label={c.label}
                  active={alerts?.durationSecs === c.value}
                  onClick={() => patch({ durationSecs: c.value })}
                />
              ))}
            </div>
          </Section>
        </div>

        {/* Preview */}
        <button
          onClick={() =>
            invoke("send_test_alert", { profileId: eligible[0]?.id ?? null }).catch((e) =>
              console.error("Failed to send test alert:", e),
            )
          }
          className="w-full flex items-center justify-center gap-1.5 py-2 rounded-lg text-xs font-medium border border-border bg-card hover:bg-card-hover text-text-secondary transition-colors"
        >
          <Send size={12} />
          Show a test alert
        </button>
      </div>
    </div>
  );
}
