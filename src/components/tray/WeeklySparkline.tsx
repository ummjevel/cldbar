import { AreaChart, Area, ResponsiveContainer } from "recharts";
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
    <div className="pt-1">
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">7-Day Trend</span>
        <span className="text-[10px] text-muted tabular-nums">
          {formatTokens(avg)} avg/day
        </span>
      </div>
      <div className="h-12 rounded-lg overflow-hidden">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 0, right: 0, left: 0, bottom: 0 }}>
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
          <div className="h-full flex items-center justify-center text-[10px] text-muted">
            No data yet
          </div>
        )}
      </div>
    </div>
  );
}
