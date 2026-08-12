import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Profile, UsageStats, Session, DailyUsage, RateLimitStatus, AppSettings } from "../lib/types";

/**
 * Everything already pulled from the backend, keyed by command and profile.
 *
 * The tray only hides its window rather than unmounting, so this outlives both
 * tab switches and reopening the popup: each profile is fetched once and then
 * only when the user presses refresh.
 */
const cache = new Map<string, unknown>();

/** Drop a profile's cached data, e.g. once it has been removed. */
export function forgetProfile(profileId: string) {
  for (const key of [...cache.keys()]) {
    if (key.includes(profileId)) cache.delete(key);
  }
}

// Stable empty values, so passing a fallback never re-triggers the effect.
const NO_SESSIONS: Session[] = [];
const NO_DAILY: DailyUsage[] = [];

/**
 * Read a backend value once and remember it. Switching profiles shows what was
 * already loaded instead of blanking and re-fetching; `refresh` is the only
 * thing that goes back to the backend.
 */
function useCachedInvoke<T>(
  command: string,
  cacheKey: string | null,
  args: Record<string, unknown>,
  fallback: T,
  /**
   * Whether a result is worth remembering. Since nothing re-fetches on its own
   * any more, caching a failed read would freeze it in place until the user
   * happened to press refresh.
   */
  shouldCache: (result: T) => boolean = () => true,
) {
  const [data, setData] = useState<T>(() =>
    cacheKey !== null && cache.has(cacheKey) ? (cache.get(cacheKey) as T) : fallback,
  );
  const [loading, setLoading] = useState(() => cacheKey !== null && !cache.has(cacheKey));

  // Arguments are rebuilt every render; hold them in a ref so `refresh` stays stable.
  const argsRef = useRef(args);
  argsRef.current = args;

  const refresh = useCallback(async () => {
    if (cacheKey === null) {
      setData(fallback);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<T>(command, argsRef.current);
      if (shouldCache(result)) cache.set(cacheKey, result);
      setData(result);
    } catch (e) {
      console.error(`Failed to run ${command}:`, e);
    } finally {
      setLoading(false);
    }
  }, [command, cacheKey, fallback, shouldCache]);

  useEffect(() => {
    if (cacheKey === null) {
      setData(fallback);
      setLoading(false);
      return;
    }
    if (cache.has(cacheKey)) {
      setData(cache.get(cacheKey) as T);
      setLoading(false);
      return;
    }
    refresh();
  }, [cacheKey, refresh, fallback]);

  return { data, loading, refresh };
}

export function useProfiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async (): Promise<Profile[] | undefined> => {
    try {
      const result = await invoke<Profile[]>("get_profiles");
      setProfiles(result);
      return result;
    } catch (e) {
      console.error("Failed to get profiles:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  return { profiles, loading, refresh };
}

export function useUsageStats(profileId: string | null) {
  const { data, loading, refresh } = useCachedInvoke<UsageStats | null>(
    "get_usage_stats",
    profileId && `usage:${profileId}`,
    { profileId },
    null,
  );
  return { stats: data, loading, refresh };
}

export function useActiveSessions(profileId: string | null) {
  const { data, refresh } = useCachedInvoke<Session[]>(
    "get_active_sessions",
    profileId && `sessions:${profileId}`,
    { profileId },
    NO_SESSIONS,
  );
  return { sessions: data, refresh };
}

export function useDailyUsage(profileId: string | null, days: number = 7) {
  const { data, refresh } = useCachedInvoke<DailyUsage[]>(
    "get_daily_usage",
    profileId && `daily:${profileId}:${days}`,
    { profileId, days },
    NO_DAILY,
  );
  return { data, refresh };
}

/** Only a reading that actually carries windows is worth keeping. */
const hasLimits = (s: RateLimitStatus | null) => s?.available === true;

export function useRateLimitStatus(profileId: string | null) {
  const { data, loading, refresh } = useCachedInvoke<RateLimitStatus | null>(
    "get_rate_limit_status",
    profileId && `limits:${profileId}`,
    { profileId },
    null,
    hasLimits,
  );
  return { status: data, loading, refresh };
}

export function useAllUsageStats() {
  const [stats, setStats] = useState<UsageStats[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<UsageStats[]>("get_all_usage_stats");
      setStats(result);
    } catch (e) {
      console.error("Failed to get all usage stats:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  return { stats, loading, refresh };
}

/**
 * Settings are shared rather than per-hook: the panel that edits them and the
 * views that read them are mounted at different times, and a private copy in
 * each caller would let them drift apart.
 */
let settingsStore: AppSettings | null = null;
const settingsListeners = new Set<(s: AppSettings) => void>();

function publishSettings(next: AppSettings) {
  settingsStore = next;
  settingsListeners.forEach((notify) => notify(next));
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(settingsStore);

  useEffect(() => {
    settingsListeners.add(setSettings);
    return () => {
      settingsListeners.delete(setSettings);
    };
  }, []);

  const refresh = useCallback(async () => {
    try {
      publishSettings(await invoke<AppSettings>("get_settings"));
    } catch (e) {
      console.error("Failed to get settings:", e);
    }
  }, []);

  const update = useCallback(async (newSettings: AppSettings) => {
    try {
      await invoke("update_settings", { settings: newSettings });
      publishSettings(newSettings);
    } catch (e) {
      console.error("Failed to update settings:", e);
    }
  }, []);

  useEffect(() => {
    if (settingsStore === null) refresh();
  }, [refresh]);

  return { settings, refresh, update };
}
