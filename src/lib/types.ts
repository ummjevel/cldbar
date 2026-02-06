export type ProviderType = "claude" | "gemini" | "zai";
export type SourceType = "account" | "api";

/** Providers that support API source type */
export const apiSupportedProviders: ProviderType[] = ["claude"];

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

export interface AppSettings {
  theme: string;
  refreshIntervalMs: number;
  launchOnStartup: boolean;
  notificationsEnabled: boolean;
  tokenAlertThreshold: number;
}
