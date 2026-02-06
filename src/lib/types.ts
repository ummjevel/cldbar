export type ProviderType = "claude" | "gemini" | "zai";
export type SourceType = "account" | "api";

/** Providers that support API source type */
export const apiSupportedProviders: ProviderType[] = ["claude"];  // zai temporarily disabled

/** Providers that support account (local folder) source type */
export const accountSupportedProviders: ProviderType[] = ["claude", "gemini"];

export interface Profile {
  id: string;
  name: string;
  providerType: ProviderType;
  configDir: string;
  enabled: boolean;
  sourceType: SourceType;
  hasApiKey: boolean;
}

export interface UsageStats {
  provider: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheReadTokens: number;
  totalCacheWriteTokens: number;
  totalSessions: number;
  totalMessages: number;
  estimatedCostUsd: number;
  modelBreakdown: Record<string, ModelUsage>;
}

export interface ModelUsage {
  model: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  costUsd: number;
}

export interface Session {
  id: string;
  project: string;
  model: string;
  tokensUsed: number;
  lastActive: string;
  isActive: boolean;
  messageCount: number;
}

export interface DailyUsage {
  date: string;
  inputTokens: number;
  outputTokens: number;
  sessions: number;
  messages: number;
}

export interface RateLimitWindow {
  label: string;
  utilization: number;
  resetsAt: string | null;
}

export interface RateLimitStatus {
  available: boolean;
  fiveHour: RateLimitWindow | null;
  sevenDay: RateLimitWindow | null;
  sevenDayOpus: RateLimitWindow | null;
}

export interface AppSettings {
  theme: string;
  refreshIntervalMs: number;
  launchOnStartup: boolean;
  notificationsEnabled: boolean;
  tokenAlertThreshold: number;
}
