import { motion } from "framer-motion";
import { formatTokens } from "../../lib/format";
import { providerColors } from "../../lib/colors";
import type { ProviderType } from "../../lib/types";

interface Props {
  used: number;
  providerType: ProviderType;
  loading: boolean;
}

export function UsageMeter({ used, providerType, loading }: Props) {
  const colors = providerColors[providerType];
  // No hard cap - show relative to a reference point for visual effect
  const displayPercent = Math.min((used / 1_000_000) * 100, 100);

  return (
    <div>
      <div className="flex items-baseline justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">Today's Usage</span>
        <span className="text-lg font-bold text-text tabular-nums">
          {loading ? "\u2014" : formatTokens(used)}
          <span className="text-xs font-normal text-muted ml-1">tokens</span>
        </span>
      </div>
      <div className="relative h-2 rounded-full bg-card overflow-hidden border border-border">
        <motion.div
          className="absolute inset-y-0 left-0 rounded-full"
          style={{
            background: `linear-gradient(90deg, ${colors.main}, ${colors.light})`,
          }}
          initial={{ width: 0 }}
          animate={{ width: `${displayPercent}%` }}
          transition={{ type: "spring", stiffness: 100, damping: 20, delay: 0.1 }}
        />
        {/* Shimmer effect */}
        <motion.div
          className="absolute inset-y-0 w-20 rounded-full"
          style={{
            background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.15), transparent)",
          }}
          animate={{ left: ["-20%", "120%"] }}
          transition={{ duration: 2, repeat: Infinity, repeatDelay: 3, ease: "linear" }}
        />
      </div>
    </div>
  );
}
