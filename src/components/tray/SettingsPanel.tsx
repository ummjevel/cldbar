import { motion } from "framer-motion";
import { ArrowLeft, Plus, Trash2, Sun, Moon, Monitor } from "lucide-react";
import { providerColors, providerLabels } from "../../lib/colors";
import { applyTheme } from "../../lib/theme";
import { useSettings } from "../../hooks/useProviderData";
import type { Profile, ProviderType, SourceType } from "../../lib/types";

interface Props {
  profiles: Profile[];
  onBack: () => void;
  onAddProfile: () => void;
  onRemoveProfile: (id: string) => void;
}

const themes = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
];

export function SettingsPanel({ profiles, onBack, onAddProfile, onRemoveProfile }: Props) {
  const { settings, update } = useSettings();

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border">
        <button
          onClick={onBack}
          className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
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
              const colors = providerColors[profile.providerType as ProviderType];
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
                  <div
                    className="flex items-center justify-center rounded-full font-bold text-[10px] shrink-0"
                    style={{
                      width: 24,
                      height: 24,
                      backgroundColor: `${colors?.main}20`,
                      color: colors?.main,
                    }}
                  >
                    {providerLabels[profile.providerType as ProviderType]?.[0] || "?"}
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

                  {/* Delete button */}
                  <button
                    onClick={() => onRemoveProfile(profile.id)}
                    className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 hover:bg-danger/10 transition-all"
                  >
                    <Trash2 size={12} className="text-danger" />
                  </button>
                </motion.div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
