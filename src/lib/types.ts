export type ProviderType = "claude" | "codex" | "gemini" | "zai";
export type SourceType = "account" | "api";

/** Providers that support API source type */
export const apiSupportedProviders: ProviderType[] = ["claude"];  // zai temporarily disabled

/** Providers that support account (local folder) source type */
export const accountSupportedProviders: ProviderType[] = ["claude", "codex", "gemini"];

export interface Profile {
  id: string;
  name: string;
  providerType: ProviderType;
  configDir: string;
  enabled: boolean;
  sourceType: SourceType;
  hasApiKey: boolean;
  /** Whether this profile reports limit windows, i.e. whether alerts apply to it */
  supportsRateLimits: boolean;
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
  /** When the reading was taken; null means it is live */
  updatedAt: string | null;
}

/** Machine keys of the limit windows a provider can report */
export type LimitWindowKey = "fiveHour" | "sevenDay" | "sevenDayOpus";

export interface AlertSettings {
  enabled: boolean;
  /** Usage percentages that raise an alert the first time they are crossed */
  usageThresholds: number[];
  /** Minutes before a window resets that raise a reminder */
  resetReminderMinutes: number[];
  /** Profiles to watch. Empty means every profile that reports limits */
  profileIds: string[];
  windows: LimitWindowKey[];
  checkIntervalSecs: number;
  /** Seconds a toast stays on screen; 0 keeps it until dismissed */
  durationSecs: number;
}

/** Whether limit windows lead with what is left or what has been spent. */
export type LimitDisplay = "remaining" | "used";

export interface AppSettings {
  theme: string;
  refreshIntervalMs: number;
  launchOnStartup: boolean;
  notificationsEnabled: boolean;
  tokenAlertThreshold: number;
  limitDisplay: LimitDisplay;
  alerts: AlertSettings;
}

export type AlertKind = "usage" | "reset" | "test";
export type AlertSeverity = "critical" | "warning" | "info";

export interface Alert {
  id: string;
  kind: AlertKind;
  severity: AlertSeverity;
  profileId: string;
  profileName: string;
  providerType: ProviderType;
  windowKey: LimitWindowKey;
  windowLabel: string;
  title: string;
  message: string;
  utilization: number;
  threshold: number | null;
  reminderMinutes: number | null;
  resetsAt: string | null;
  createdAt: string;
}
