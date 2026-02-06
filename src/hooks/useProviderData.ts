import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Profile, UsageStats, Session, DailyUsage, AppSettings } from "../lib/types";

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
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    if (!profileId) { setStats(null); setLoading(false); return; }
    try {
      const result = await invoke<UsageStats>("get_usage_stats", { profileId });
      setStats(result);
    } catch (e) {
      console.error("Failed to get usage stats:", e);
    } finally {
      setLoading(false);
    }
  }, [profileId]);

  useEffect(() => { setStats(null); setLoading(true); refresh(); }, [refresh]);
  return { stats, loading, refresh };
}

export function useActiveSessions(profileId: string | null) {
  const [sessions, setSessions] = useState<Session[]>([]);

  const refresh = useCallback(async () => {
    if (!profileId) { setSessions([]); return; }
    try {
      const result = await invoke<Session[]>("get_active_sessions", { profileId });
      setSessions(result);
    } catch (e) {
      console.error("Failed to get active sessions:", e);
    }
  }, [profileId]);

  useEffect(() => { setSessions([]); refresh(); }, [refresh]);
  return { sessions, refresh };
}

export function useDailyUsage(profileId: string | null, days: number = 7) {
  const [data, setData] = useState<DailyUsage[]>([]);

  const refresh = useCallback(async () => {
    if (!profileId) { setData([]); return; }
    try {
      const result = await invoke<DailyUsage[]>("get_daily_usage", { profileId, days });
      setData(result);
    } catch (e) {
      console.error("Failed to get daily usage:", e);
    }
  }, [profileId, days]);

  useEffect(() => { setData([]); refresh(); }, [refresh]);
  return { data, refresh };
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

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<AppSettings>("get_settings");
      setSettings(result);
    } catch (e) {
      console.error("Failed to get settings:", e);
    }
  }, []);

  const update = useCallback(async (newSettings: AppSettings) => {
    try {
      await invoke("update_settings", { settings: newSettings });
      setSettings(newSettings);
    } catch (e) {
      console.error("Failed to update settings:", e);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  return { settings, refresh, update };
}
