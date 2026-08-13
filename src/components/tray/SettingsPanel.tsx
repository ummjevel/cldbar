import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { ArrowLeft, Plus, Trash2, Sun, Moon, Monitor, Bell, ChevronRight, BatteryMedium, Gauge } from "lucide-react";
import { startManualDrag } from "../../lib/windowState";
import { providerLabels } from "../../lib/colors";
import { ProviderIcon } from "./ProviderIcon";
import { applyTheme } from "../../lib/theme";
import { useSettings } from "../../hooks/useProviderData";
import type { LimitDisplay, Profile, ProviderType, SourceType } from "../../lib/types";

interface Props {
  profiles: Profile[];
  onBack: () => void;
  onAddProfile: () => void;
  onRemoveProfile: (id: string) => void;
  onOpenAlerts: () => void;
}

const themes = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
];

const limitDisplays: { value: LimitDisplay; label: string; icon: typeof BatteryMedium }[] = [
  { value: "remaining", label: "Remaining", icon: BatteryMedium },
  { value: "used", label: "Used", icon: Gauge },
];

export function SettingsPanel({ profiles, onBack, onAddProfile, onRemoveProfile, onOpenAlerts }: Props) {
  const { settings, update } = useSettings();

  // Removal is irreversible, so it takes two clicks: the first arms an inline
  // "Remove?" confirmation that disarms itself after a moment.
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  useEffect(() => {
    if (!confirmingId) return;
    const t = setTimeout(() => setConfirmingId(null), 3000);
    return () => clearTimeout(t);
  }, [confirmingId]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div
        className="flex items-center gap-2 px-4 py-2.5 border-b border-border cursor-grab active:cursor-grabbing"
        onMouseDown={startManualDrag}
      >
        <button
          onClick={onBack}
          className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
          aria-label="Back"
        >
          <ArrowLeft size={14} className="text-muted" />
        </button>
        <span className="text-sm font-semibold text-text">Settings</span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
        {/* Theme section */}
        <div>
          <span className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2 block">
            Theme
          </span>
          <div className="flex gap-1.5">
            {themes.map((t) => {
              const active = (settings?.theme || "system") === t.value;
              return (
                <button
                  key={t.value}
                  onClick={() => {
                    if (!settings) return;
                    update({ ...settings, theme: t.value });
                    applyTheme(t.value);
                  }}
                  className="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-lg text-xs font-medium border transition-all"
                  style={{
                    borderColor: active ? "var(--color-text-secondary)" : "var(--color-border)",
                    backgroundColor: active ? "var(--color-card-hover)" : "var(--color-card)",
                    color: active ? "var(--color-text)" : "var(--color-muted)",
                  }}
                >
                  <t.icon size={12} />
                  {t.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Limit display section */}
        <div>
          <span className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2 block">
            Limit display
          </span>
          <div className="flex gap-1.5">
            {limitDisplays.map((d) => {
              const active = (settings?.limitDisplay || "remaining") === d.value;
              return (
                <button
                  key={d.value}
                  onClick={() => {
                    if (!settings) return;
                    update({ ...settings, limitDisplay: d.value });
                  }}
                  className="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-lg text-xs font-medium border transition-all"
                  style={{
                    borderColor: active ? "var(--color-text-secondary)" : "var(--color-border)",
                    backgroundColor: active ? "var(--color-card-hover)" : "var(--color-card)",
                    color: active ? "var(--color-text)" : "var(--color-muted)",
                  }}
                >
                  <d.icon size={12} />
                  {d.label}
                </button>
              );
            })}
          </div>
          <p className="text-[10px] text-muted mt-1.5 leading-relaxed">
            Hovering a limit always shows the other way round.
          </p>
        </div>

        {/* Alerts section */}
        <div>
          <span className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2 block">
            Notifications
          </span>
          <button
            onClick={onOpenAlerts}
            className="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-card border border-border hover:bg-card-hover transition-colors text-left"
          >
            <Bell size={14} className="text-muted shrink-0" />
            <div className="flex-1 min-w-0">
              <div className="text-xs font-medium text-text">Alerts</div>
              <div className="text-[10px] text-muted">
                {settings?.alerts?.enabled
                  ? "Warn me before I run out of limit"
                  : "Off"}
              </div>
            </div>
            <ChevronRight size={13} className="text-muted shrink-0" />
          </button>
        </div>

        {/* Profiles section */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-medium text-text-secondary uppercase tracking-wider">
              Profiles
            </span>
            <button
              onClick={onAddProfile}
              className="flex items-center gap-1 text-[10px] font-medium text-muted hover:text-text transition-colors"
            >
              <Plus size={12} />
              Add
            </button>
          </div>

          <div className="space-y-1.5">
            {profiles.length === 0 && (
              <p className="text-xs text-muted text-center py-4">No profiles configured</p>
            )}
            {profiles.map((profile, i) => {
              const sourceLabel = (profile.sourceType as SourceType) === "api" ? "API" : "Account";
              return (
                <motion.div
                  key={profile.id}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.03 * i }}
                  className="flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-card border border-border group"
                >
                  {/* Provider icon */}
                  <div className="shrink-0">
                    <ProviderIcon type={profile.providerType as ProviderType} size={22} />
                  </div>

                  {/* Info */}
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-text truncate">
                      {profile.name}
                    </div>
                    <div className="text-[10px] text-muted">
                      {providerLabels[profile.providerType as ProviderType]} · {sourceLabel}
                    </div>
                  </div>

                  {/* Delete button: armed on first click, confirmed on the second */}
                  {confirmingId === profile.id ? (
                    <button
                      onClick={() => {
                        setConfirmingId(null);
                        onRemoveProfile(profile.id);
                      }}
                      className="px-2 py-1 rounded-md text-[10px] font-semibold text-danger bg-danger/10 border border-danger/20 transition-colors"
                      aria-label={`Confirm removing ${profile.name}`}
                    >
                      Remove?
                    </button>
                  ) : (
                    <button
                      onClick={() => setConfirmingId(profile.id)}
                      className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-danger/10 transition-all"
                      aria-label={`Remove ${profile.name}`}
                    >
                      <Trash2 size={12} className="text-danger" />
                    </button>
                  )}
                </motion.div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
