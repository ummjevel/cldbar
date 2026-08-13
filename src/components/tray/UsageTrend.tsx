import { AreaChart, Area, ResponsiveContainer } from "recharts";
import { TrendingUp } from "lucide-react";
import { formatTokens, formatShortDate, localDateKey } from "../../lib/format";
import { providerColors } from "../../lib/colors";
import type { DailyUsage, ProviderType } from "../../lib/types";

interface Props {
  data: DailyUsage[];
  providerType: ProviderType;
  loading?: boolean;
}

/**
 * Today's total with the recent daily trend behind it.
 *
 * The headline number is genuinely today's tokens (matched by local date, then
 * UTC, since providers stamp their logs differently). Providers return daily
 * rows newest-first, so the chart re-sorts them to make time flow left→right.
 */
export function UsageTrend({ data, providerType, loading }: Props) {
  const colors = providerColors[providerType];
  const days = [...data].sort((a, b) => (a.date < b.date ? -1 : 1));

  const total = days.reduce((sum, d) => sum + d.inputTokens + d.outputTokens, 0);
  const avg = days.length > 0 ? total / days.length : 0;

  const todayKeys = [localDateKey(), new Date().toISOString().slice(0, 10)];
  const today = days.find((d) => todayKeys.includes(d.date));
  const todayTokens = today ? today.inputTokens + today.outputTokens : 0;

  const chartData = days.map((d) => ({
    date: d.date,
    tokens: d.inputTokens + d.outputTokens,
  }));
  const first = days[0];
  const last = days[days.length - 1];

  return (
    <div>
      <div className="flex items-baseline justify-between mb-1.5">
        <span className="text-xs font-medium text-text-secondary">Today's Usage</span>
        <span className="text-lg font-bold text-text tabular-nums">
          {loading && days.length === 0 ? "—" : formatTokens(todayTokens)}
          <span className="text-xs font-normal text-muted ml-1">tokens</span>
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

      {first && last && (
        <div className="flex items-center justify-between mt-1 text-[10px] text-muted tabular-nums">
          <span>{formatShortDate(first.date)}</span>
          <span>{formatTokens(avg)} avg/day</span>
          <span>{todayKeys.includes(last.date) ? "Today" : formatShortDate(last.date)}</span>
        </div>
      )}
    </div>
  );
}
