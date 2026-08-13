import { useState, useCallback, type ReactNode } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Settings, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { ProviderTabs } from "./ProviderTabs";
import { StatCards } from "./StatCards";
import { ActiveSessions } from "./ActiveSessions";
import { UsageTrend } from "./UsageTrend";
import { SettingsPanel } from "./SettingsPanel";
import { AlertSettingsPanel } from "./AlertSettingsPanel";
import { AddProfileForm } from "./AddProfileForm";
import { LimitWindows } from "./LimitWindows";
import {
  useProfiles,
  useUsageStats,
  useActiveSessions,
  useDailyUsage,
  useRateLimitStatus,
  forgetProfile,
} from "../../hooks/useProviderData";
import { useTrayWindow } from "../../hooks/useTrayWindow";
import { startManualDrag } from "../../lib/windowState";
import { providerLabels } from "../../lib/colors";
import type { ProviderType, SourceType } from "../../lib/types";

type View = "main" | "settings" | "addProfile" | "alerts";

/** Where Escape (and the back button) leads from each sub-view. */
const BACK_TARGET: Record<Exclude<View, "main">, View> = {
  settings: "main",
  addProfile: "settings",
  alerts: "settings",
};

/** Logical window height per view; the main view grows with its session list. */
function viewHeight(view: View, sessionCount: number): number {
  if (view === "alerts") return 620;
  if (view !== "main") return 490;
  if (sessionCount <= 1) return 490;
  return sessionCount === 2 ? 530 : 600;
}

/** Shared slide-in used by every view of the popup. Give each one a `key` so
    AnimatePresence can run its exit animation. */
function Panel({ id, children, className = "h-full" }: { id: string; children: ReactNode; className?: string }) {
  const dir = id === "main" ? -20 : 20;
  return (
    <motion.div
      initial={{ opacity: 0, x: dir }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: dir }}
      transition={{ duration: 0.15 }}
      className={className}
    >
      {children}
    </motion.div>
  );
}

export function TrayPopup() {
  const { profiles, refresh: refreshProfiles } = useProfiles();
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [view, setView] = useState<View>("main");

  // Auto-select first profile
  if (profiles.length > 0 && !activeProfileId) {
    setActiveProfileId(profiles[0].id);
  }

  const activeProfile = profiles.find((p) => p.id === activeProfileId);
  const { stats, loading: statsLoading, refresh: refreshStats } = useUsageStats(activeProfileId);
  const { sessions, refresh: refreshSessions } = useActiveSessions(activeProfileId);
  const { data: dailyUsage, loading: dailyLoading, refresh: refreshDaily } = useDailyUsage(activeProfileId, 7);
  const {
    status: rateLimitStatus,
    loading: limitsLoading,
    refresh: refreshRateLimits,
    forceRefresh: forceRefreshRateLimits,
  } = useRateLimitStatus(activeProfileId);

  /** Re-read everything; used when the popup is (re)opened. */
  const refreshAll = useCallback(() => {
    refreshStats();
    refreshSessions();
    refreshDaily();
    refreshRateLimits();
  }, [refreshStats, refreshSessions, refreshDaily, refreshRateLimits]);

  /** The refresh button: an explicit ask, so it bypasses cache and backoff. */
  const forceRefreshAll = useCallback(() => {
    refreshStats();
    refreshSessions();
    refreshDaily();
    forceRefreshRateLimits();
  }, [refreshStats, refreshSessions, refreshDaily, forceRefreshRateLimits]);

  useTrayWindow({
    onOpen: refreshAll,
    onEscape: () => {
      if (view === "main") return false;
      setView(BACK_TARGET[view]);
      return true;
    },
    height: viewHeight(view, sessions.length),
  });

  const refreshing = statsLoading || limitsLoading;
  const sourceType: SourceType = (activeProfile?.sourceType as SourceType) || "account";
  const providerType = (activeProfile?.providerType as ProviderType) || "claude";

  const handleRemoveProfile = useCallback(
    async (id: string) => {
      try {
        await invoke("remove_profile", { id });
        forgetProfile(id);
        const updated = await refreshProfiles();
        if (activeProfileId === id) {
          // Select the next available profile, or null if none
          const remaining = (updated ?? profiles).filter((p) => p.id !== id);
          setActiveProfileId(remaining.length > 0 ? remaining[0].id : null);
        }
      } catch (e) {
        console.error("Failed to remove profile:", e);
      }
    },
    [activeProfileId, profiles, refreshProfiles],
  );

  const handleProfileAdded = useCallback(async () => {
    await refreshProfiles();
    setView("settings");
  }, [refreshProfiles]);

  return (
    <div
      className="h-full flex flex-col rounded-xl border border-border bg-bg overflow-hidden"
      style={{
        boxShadow: "var(--theme-shadow)",
        backdropFilter: "blur(24px)",
        WebkitBackdropFilter: "blur(24px)",
      }}
    >
      <AnimatePresence mode="wait">
        {view === "settings" ? (
          <Panel key="settings" id="settings">
            <SettingsPanel
              profiles={profiles}
              onBack={() => setView("main")}
              onAddProfile={() => setView("addProfile")}
              onRemoveProfile={handleRemoveProfile}
              onOpenAlerts={() => setView("alerts")}
            />
          </Panel>
        ) : view === "alerts" ? (
          <Panel key="alerts" id="alerts">
            <AlertSettingsPanel profiles={profiles} onBack={() => setView("settings")} />
          </Panel>
        ) : view === "addProfile" ? (
          <Panel key="addProfile" id="addProfile">
            <AddProfileForm onBack={() => setView("settings")} onAdded={handleProfileAdded} />
          </Panel>
        ) : (
          <Panel key="main" id="main" className="h-full flex flex-col">
            {/* Title bar - draggable */}
            <div
              className="flex items-center justify-between px-4 py-2.5 border-b border-border cursor-grab active:cursor-grabbing"
              onMouseDown={startManualDrag}
            >
              <div className="flex items-center gap-2">
                <svg width="16" height="16" viewBox="0 0 100 100" aria-hidden="true">
                  <circle fill="none" stroke="var(--color-border-light)" strokeWidth="20" cx="50" cy="50" r="40" />
                  <circle
                    fill="none"
                    stroke="#e87b35"
                    strokeWidth="20"
                    cx="50"
                    cy="50"
                    r="40"
                    strokeDasharray="150 251.3"
                    strokeDashoffset="0"
                    transform="rotate(-90 50 50)"
                    opacity="0.95"
                  />
                  <circle
                    fill="none"
                    stroke="#4285f4"
                    strokeWidth="20"
                    cx="50"
                    cy="50"
                    r="40"
                    strokeDasharray="93 251.3"
                    strokeDashoffset="-155"
                    transform="rotate(-90 50 50)"
                    opacity="0.95"
                  />
                </svg>
                <span className="text-sm font-semibold text-text tracking-wide">cldbar</span>
              </div>
              <div className="flex items-center gap-1.5">
                <button
                  className="p-1.5 rounded-md hover:bg-card-hover transition-colors disabled:opacity-60"
                  onClick={forceRefreshAll}
                  disabled={refreshing}
                  aria-label="Refresh"
                  title="Refresh"
                >
                  <RefreshCw size={13} className={`text-muted ${refreshing ? "animate-spin" : ""}`} />
                </button>
                <button
                  className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
                  onClick={() => setView("settings")}
                  aria-label="Settings"
                  title="Settings"
                >
                  <Settings size={13} className="text-muted" />
                </button>
              </div>
            </div>

            {/* Provider tabs (hidden when no profiles) */}
            {profiles.length > 0 && (
              <ProviderTabs profiles={profiles} activeProfileId={activeProfileId} onSelect={setActiveProfileId} />
            )}

            {/* Content area with scroll */}
            <div className="flex-1 overflow-y-auto px-4 py-3 rounded-b-xl">
              {!activeProfile ? (
                <div className="flex flex-col items-center justify-center h-full gap-4">
                  <div className="flex flex-col items-center gap-1">
                    <svg width="32" height="32" viewBox="0 0 100 100" className="opacity-30" aria-hidden="true">
                      <circle fill="none" stroke="currentColor" strokeWidth="16" cx="50" cy="50" r="40" />
                    </svg>
                    <p className="text-xs text-muted mt-2">No profiles configured</p>
                    <p className="text-[10px] text-muted/60">Add a provider to start tracking usage</p>
                  </div>
                  <button
                    onClick={() => setView("addProfile")}
                    className="px-4 py-1.5 rounded-lg text-xs font-medium bg-card border border-border hover:border-border-light transition-colors text-text-secondary"
                  >
                    Add Profile
                  </button>
                </div>
              ) : (
                <motion.div
                  key={activeProfileId}
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.2 }}
                  className="flex flex-col gap-3 min-h-full"
                >
                  {/* What's left and when it resets — the reason the popup gets opened */}
                  <LimitWindows
                    status={rateLimitStatus}
                    providerLabel={providerLabels[providerType]}
                    loading={limitsLoading}
                    sourceType={sourceType}
                  />

                  {/* Today's tokens + recent trend */}
                  <UsageTrend data={dailyUsage} providerType={providerType} loading={dailyLoading} />

                  {/* All-time stat cards */}
                  <StatCards stats={stats} providerType={providerType} sourceType={sourceType} />

                  {/* Active sessions */}
                  <ActiveSessions sessions={sessions} sourceType={sourceType} />
                </motion.div>
              )}
            </div>
          </Panel>
        )}
      </AnimatePresence>
    </div>
  );
}
