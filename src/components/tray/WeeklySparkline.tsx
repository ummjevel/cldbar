import { AreaChart, Area, ResponsiveContainer } from "recharts";
import { TrendingUp } from "lucide-react";
import { formatTokens } from "../../lib/format";
import { providerColors } from "../../lib/colors";
import type { DailyUsage, ProviderType } from "../../lib/types";

interface Props {
  data: DailyUsage[];
  providerType: ProviderType;
}

export function WeeklySparkline({ data, providerType }: Props) {
  const colors = providerColors[providerType];
  const total = data.reduce((sum, d) => sum + d.inputTokens + d.outputTokens, 0);
  const avg = data.length > 0 ? total / data.length : 0;

  const chartData = data.map(d => ({
    date: d.date,
    tokens: d.inputTokens + d.outputTokens,
  }));

  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">7-Day Trend</span>
        <span className="text-[10px] text-muted tabular-nums">
          {formatTokens(avg)} avg/day
        </span>
      </div>
      <div className="h-12 rounded-lg">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 4, right: 6, left: 6, bottom: 6 }}>
              <defs>
                <linearGradient id={`gradient-${providerType}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={colors.main} stopOpacity={0.3} />
                  <stop offset="100%" stopColor={colors.main} stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <Area
                type="monotone"
                dataKey="tokens"
                stroke={colors.main}
                strokeWidth={1.5}
                fill={`url(#gradient-${providerType})`}
                isAnimationActive={true}
                animationDuration={800}
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="h-full flex-1 flex items-center justify-center gap-2 px-3 rounded-lg bg-card border border-border border-dashed">
            <TrendingUp size={12} className="text-muted opacity-40 shrink-0" />
            <p className="text-[10px] text-muted">No usage data yet</p>
          </div>
        )}
      </div>
    </div>
  );
}
