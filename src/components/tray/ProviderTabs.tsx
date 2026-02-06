import { useRef } from "react";
import { motion } from "framer-motion";
import { providerColors } from "../../lib/colors";
import { ProviderIcon } from "./ProviderIcon";
import type { Profile, ProviderType } from "../../lib/types";

interface Props {
  profiles: Profile[];
  activeProfileId: string | null;
  onSelect: (id: string) => void;
}

export function ProviderTabs({ profiles, activeProfileId, onSelect }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);

  if (profiles.length === 0) {
    return (
      <div className="px-4 py-3 text-center text-muted text-xs">
        No providers configured
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      className="flex items-center gap-1 px-4 py-2 border-b border-border overflow-x-auto shrink-0"
      onWheel={(e) => {
        if (scrollRef.current) {
          const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
          if (delta !== 0) {
            e.preventDefault();
            scrollRef.current.scrollLeft += delta * 3;
          }
        }
      }}
    >
      <span className="text-[9px] text-muted tabular-nums shrink-0 mr-1">{profiles.length}</span>
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
            {/* <SourceBadge sourceType={(profile.sourceType || "account") as SourceType} /> */}
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
