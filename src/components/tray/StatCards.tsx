import { motion } from "framer-motion";
import { ArrowUpRight, ArrowDownLeft, MessageSquare, DollarSign } from "lucide-react";
import { formatTokens, formatCost } from "../../lib/format";
import { providerColors } from "../../lib/colors";
import type { UsageStats, ProviderType, SourceType } from "../../lib/types";

interface Props {
  stats: UsageStats | null;
  providerType: ProviderType;
  sourceType?: SourceType;
}

export function StatCards({ stats, providerType, sourceType }: Props) {
  const colors = providerColors[providerType];

  const isApi = sourceType === "api";

  const cards = [
    {
      label: "Input",
      value: stats ? formatTokens(stats.totalInputTokens) : "\u2014",
      icon: ArrowDownLeft,
      color: colors.main,
    },
    {
      label: "Output",
      value: stats ? formatTokens(stats.totalOutputTokens) : "\u2014",
      icon: ArrowUpRight,
      color: colors.light,
    },
    ...(isApi ? [
      {
        label: "Sessions",
        value: "N/A",
        icon: MessageSquare,
        color: "#8b8b9e",
      },
      {
        label: "Cost",
        value: stats ? formatCost(stats.estimatedCostUsd) : "\u2014",
        icon: DollarSign,
        color: "#22c55e",
      },
    ] : [
      {
        label: "Sessions",
        value: stats ? stats.totalSessions.toString() : "\u2014",
        icon: MessageSquare,
        color: "#8b8b9e",
      },
      {
        label: "Messages",
        value: stats ? stats.totalMessages.toString() : "\u2014",
        icon: MessageSquare,
        color: "#8b8b9e",
      },
    ]),
  ];

  return (
    <div className="grid grid-cols-2 gap-2">
      {cards.map((card, i) => (
        <motion.div
          key={card.label}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.05 * i, duration: 0.3 }}
          className="group relative p-3 rounded-lg bg-card border border-border hover:border-border-light hover:bg-card-hover transition-all duration-200 cursor-default"
        >
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-[10px] font-medium text-muted uppercase tracking-wider">
              {card.label}
            </span>
            <card.icon size={12} style={{ color: card.color }} className="opacity-60" />
          </div>
          <div className="text-base font-bold text-text tabular-nums">{card.value}</div>
        </motion.div>
      ))}
    </div>
  );
}
