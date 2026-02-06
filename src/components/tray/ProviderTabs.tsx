import { motion } from "framer-motion";
import { providerColors, providerLabels } from "../../lib/colors";
import type { Profile, ProviderType, SourceType } from "../../lib/types";

function SourceBadge({ sourceType }: { sourceType: SourceType }) {
  const isApi = sourceType === "api";
  return (
    <span
      className="text-[8px] font-semibold uppercase leading-none px-1 py-0.5 rounded"
      style={{
        backgroundColor: isApi ? "rgba(59,130,246,0.15)" : "rgba(139,139,158,0.15)",
        color: isApi ? "#3b82f6" : "#8b8b9e",
      }}
    >
      {isApi ? "API" : "Account"}
    </span>
  );
}

function ProviderIcon({ type, size = 18 }: { type: ProviderType; size?: number }) {
  const color = providerColors[type]?.main || "#888";
  return (
    <div
      className="flex items-center justify-center rounded-full font-bold text-[10px]"
      style={{ width: size, height: size, backgroundColor: `${color}20`, color }}
    >
      {providerLabels[type]?.[0] || "?"}
    </div>
  );
}

interface Props {
  profiles: Profile[];
  activeProfileId: string | null;
  onSelect: (id: string) => void;
}

export function ProviderTabs({ profiles, activeProfileId, onSelect }: Props) {
  if (profiles.length === 0) {
    return (
      <div className="px-4 py-3 text-center text-muted text-xs">
        No providers configured
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1 px-4 py-2 border-b border-border overflow-x-auto">
      {profiles.map((profile) => {
        const isActive = profile.id === activeProfileId;
        const colors = providerColors[profile.providerType as ProviderType];
        return (
          <button
            key={profile.id}
            onClick={() => onSelect(profile.id)}
            className="relative flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors whitespace-nowrap"
            style={{
              color: isActive ? colors?.main : "var(--color-muted)",
              backgroundColor: isActive ? colors?.bg : "transparent",
            }}
          >
            <ProviderIcon type={profile.providerType as ProviderType} size={16} />
            <span>{profile.name}</span>
            <SourceBadge sourceType={(profile.sourceType || "account") as SourceType} />
            {isActive && (
              <motion.div
                layoutId="activeTab"
                className="absolute bottom-0 left-2 right-2 h-[2px] rounded-full"
                style={{ backgroundColor: colors?.main }}
                transition={{ type: "spring", stiffness: 500, damping: 30 }}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}
