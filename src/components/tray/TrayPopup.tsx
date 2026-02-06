import { useState, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Settings, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { ProviderTabs } from "./ProviderTabs";
import { UsageMeter } from "./UsageMeter";
import { StatCards } from "./StatCards";
import { ActiveSessions } from "./ActiveSessions";
import { WeeklySparkline } from "./WeeklySparkline";
import { SettingsPanel } from "./SettingsPanel";
import { AddProfileForm } from "./AddProfileForm";
import { RateLimits } from "./RateLimits";
import { useProfiles, useUsageStats, useActiveSessions, useDailyUsage, useRateLimitStatus } from "../../hooks/useProviderData";
import { isDialogOpen, isDragging, startManualDrag } from "../../lib/windowState";
import type { ProviderType, SourceType } from "../../lib/types";

type View = "main" | "settings" | "addProfile";

export function TrayPopup() {
  const { profiles, refresh: refreshProfiles } = useProfiles();
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [view, setView] = useState<View>("main");

  // Auto-select first profile
  useEffect(() => {
    if (profiles.length > 0 && !activeProfileId) {
      setActiveProfileId(profiles[0].id);
    }
  }, [profiles, activeProfileId]);

  const activeProfile = profiles.find(p => p.id === activeProfileId);
  const { stats, loading, refresh: refreshStats } = useUsageStats(activeProfileId);
  const { sessions, refresh: refreshSessions } = useActiveSessions(activeProfileId);
  const { data: dailyUsage, refresh: refreshDaily } = useDailyUsage(activeProfileId, 7);
  const { status: rateLimitStatus, refresh: refreshRateLimits } = useRateLimitStatus(activeProfileId);

  // Hide window on blur (debounced to allow drag/dialog interactions)
  useEffect(() => {
    const win = getCurrentWindow();
    let blurTimeout: ReturnType<typeof setTimeout> | null = null;

    const unlistenBlur = win.listen("tauri://blur", () => {
      if (!isDialogOpen() && !isDragging()) {
        blurTimeout = setTimeout(() => win.hide(), 150);
      }
    });
    const unlistenFocus = win.listen("tauri://focus", () => {
      if (blurTimeout) { clearTimeout(blurTimeout); blurTimeout = null; }
    });

    return () => {
      unlistenBlur.then(fn => fn());
      unlistenFocus.then(fn => fn());
      if (blurTimeout) clearTimeout(blurTimeout);
    };
  }, []);

  // Dynamic window height based on session count (top-left stays fixed, only height changes)
  useEffect(() => {
    if (view !== "main") return;
    const win = getCurrentWindow();
    const count = sessions.length;
    const targetH = count <= 1 ? 490 : count === 2 ? 530 : 600;
    const scale = window.devicePixelRatio || 1;
    const W = Math.round(380 * scale);
    const H = Math.round(targetH * scale);

    (async () => {
      try {
        await win.setSize(new PhysicalSize(W, H));
      } catch (e) {
        console.error("Window resize failed:", e);
      }
    })();
  }, [sessions.length, view]);

  // Auto-refresh every 5 seconds (only on main view)
  useEffect(() => {
    if (view !== "main") return;
    const interval = setInterval(() => {
      refreshStats();
      refreshSessions();
      refreshDaily();
      refreshRateLimits();
    }, 5000);
    return () => clearInterval(interval);
  }, [view, refreshStats, refreshSessions, refreshDaily, refreshRateLimits]);

  const sourceType: SourceType = (activeProfile?.sourceType as SourceType) || "account";
  const totalTokens = stats ? stats.totalInputTokens + stats.totalOutputTokens : 0;

  const handleRemoveProfile = useCallback(async (id: string) => {
    try {
      await invoke("remove_profile", { id });
      const updated = await refreshProfiles();
      if (activeProfileId === id) {
        // Select the next available profile, or null if none
        const remaining = (updated ?? profiles).filter(p => p.id !== id);
        setActiveProfileId(remaining.length > 0 ? remaining[0].id : null);
      }
    } catch (e) {
      console.error("Failed to remove profile:", e);
    }
  }, [activeProfileId, profiles, refreshProfiles]);

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
          <motion.div
            key="settings"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 20 }}
            transition={{ duration: 0.15 }}
            className="h-full"
          >
            <SettingsPanel
              profiles={profiles}
              onBack={() => setView("main")}
              onAddProfile={() => setView("addProfile")}
              onRemoveProfile={handleRemoveProfile}
            />
          </motion.div>
        ) : view === "addProfile" ? (
          <motion.div
            key="addProfile"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 20 }}
            transition={{ duration: 0.15 }}
            className="h-full"
          >
            <AddProfileForm
              onBack={() => setView("settings")}
              onAdded={handleProfileAdded}
            />
          </motion.div>
        ) : (
          <motion.div
            key="main"
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            transition={{ duration: 0.15 }}
            className="h-full flex flex-col"
          >
            {/* Title bar - draggable */}
            <div
              className="flex items-center justify-between px-4 py-2.5 border-b border-border cursor-grab active:cursor-grabbing"
              onMouseDown={startManualDrag}
            >
              <div className="flex items-center gap-2">
                <svg width="16" height="16" viewBox="0 0 100 100">
                  <circle fill="none" stroke="#1a1a1e" strokeWidth="20" cx="50" cy="50" r="40"/>
                  <circle fill="none" stroke="#e87b35" strokeWidth="20" cx="50" cy="50" r="40"
                    strokeDasharray="150 251.3" strokeDashoffset="0"
                    transform="rotate(-90 50 50)" opacity="0.95"/>
                  <circle fill="none" stroke="#4285f4" strokeWidth="20" cx="50" cy="50" r="40"
                    strokeDasharray="93 251.3" strokeDashoffset="-155"
                    transform="rotate(-90 50 50)" opacity="0.95"/>
                </svg>
                <span className="text-sm font-semibold text-text tracking-wide">cldbar</span>
              </div>
              <div className="flex items-center gap-1.5">
                <button
                  className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
                  onClick={() => {
                    refreshStats();
                    refreshSessions();
                    refreshDaily();
                    refreshRateLimits();
                  }}
                >
                  <RefreshCw size={13} className="text-muted" />
                </button>
                <button
                  className="p-1.5 rounded-md hover:bg-card-hover transition-colors"
                  onClick={() => setView("settings")}
                >
                  <Settings size={13} className="text-muted" />
                </button>
              </div>
            </div>

            {/* Provider tabs (hidden when no profiles) */}
            {profiles.length > 0 && (
              <ProviderTabs
                profiles={profiles}
                activeProfileId={activeProfileId}
                onSelect={setActiveProfileId}
              />
            )}

            {/* Content area with scroll */}
            <div className="flex-1 overflow-y-auto px-4 py-3 rounded-b-xl">
              {!activeProfile ? (
                <div className="flex flex-col items-center justify-center h-full gap-4">
                  <div className="flex flex-col items-center gap-1">
                    <svg width="32" height="32" viewBox="0 0 100 100" className="opacity-30">
                      <circle fill="none" stroke="currentColor" strokeWidth="16" cx="50" cy="50" r="40"/>
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
                <AnimatePresence mode="wait">
                  <motion.div
                    key={activeProfileId}
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -8 }}
                    transition={{ duration: 0.2 }}
                    className="flex flex-col gap-3 min-h-full"
                  >
                    {/* Usage meter */}
                    <UsageMeter
                      used={totalTokens}
                      providerType={(activeProfile.providerType as ProviderType) || "claude"}
                      loading={loading}
                    />

                    {/* Rate limits (Claude account only) */}
                    <RateLimits status={rateLimitStatus} />

                    {/* Stat cards */}
                    <StatCards
                      stats={stats}
                      providerType={(activeProfile.providerType as ProviderType) || "claude"}
                      sourceType={sourceType}
                    />

                    {/* Active sessions */}
                    <ActiveSessions sessions={sessions} sourceType={sourceType} />

                    {/* Weekly sparkline */}
                    <WeeklySparkline
                        data={dailyUsage}
                        providerType={(activeProfile.providerType as ProviderType) || "claude"}
                      />

                  </motion.div>
                </AnimatePresence>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
