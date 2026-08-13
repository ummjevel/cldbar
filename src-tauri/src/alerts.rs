//! Usage / limit alert engine.
//!
//! A background thread polls the rate limit windows of every watched profile and
//! raises a toast when a usage threshold is crossed or a window is about to reset.
//! Toasts are rendered by a dedicated always-on-top webview window pinned to the
//! bottom-right corner of the work area (see `AlertOverlay` on the frontend).

use crate::commands::AppState;
use crate::profile::{AlertSettings, Profile};
use crate::providers::claude::ClaudeProvider;
use crate::providers::codex::CodexProvider;
use crate::providers::zai_api::ZaiApiProvider;
use crate::providers::{RateLimitStatus, RateLimitWindow};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

pub const ALERT_WINDOW: &str = "notification";

/// Event broadcast whenever new alerts are queued. Carries no payload — the
/// overlay pulls the queue with `drain_alerts` so there is a single source of truth.
const ALERT_EVENT: &str = "alert://push";

/// The three limit windows a profile can report, in display order.
const WINDOW_KEYS: [&str; 3] = ["fiveHour", "sevenDay", "sevenDayOpus"];

/// Narrowest reset-reminder window, in minutes.
const MIN_REMINDER_TOLERANCE_MINUTES: u64 = 15;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    /// "usage" | "reset" | "test"
    pub kind: String,
    /// "critical" | "warning" | "info"
    pub severity: String,
    pub profile_id: String,
    pub profile_name: String,
    pub provider_type: String,
    /// Machine key of the limit window, e.g. "fiveHour".
    pub window_key: String,
    /// Human label of the limit window, e.g. "5-Hour".
    pub window_label: String,
    pub title: String,
    pub message: String,
    pub utilization: f64,
    /// Threshold that triggered a usage alert.
    pub threshold: Option<u32>,
    /// Reminder bucket that triggered a reset alert, in minutes.
    pub reminder_minutes: Option<u32>,
    pub resets_at: Option<String>,
    pub created_at: String,
}

/// Engine state: which alerts already fired, and which ones the overlay has not picked up yet.
#[derive(Default)]
pub struct AlertState {
    /// Dedup keys of alerts already raised in the current limit period.
    fired: Mutex<HashSet<String>>,
    /// Alerts queued for the overlay window.
    pending: Mutex<Vec<Alert>>,
}

/// How long a reading is reused before the provider is asked again.
///
/// Claude and Codex both answer over the network, while the tray refreshes
/// every few seconds and the engine polls on its own schedule. The windows
/// themselves move far slower than that, so one shared reading serves both.
const LIMIT_TTL: Duration = Duration::from_secs(120);

/// Longest gap between retries once a provider keeps refusing.
const MAX_BACKOFF: Duration = Duration::from_secs(30 * 60);

struct CachedLimit {
    fetched_at: Instant,
    status: RateLimitStatus,
    /// Consecutive failed reads, used to widen the gap before trying again.
    failures: u32,
}

/// How long to sit on a reading before asking again.
///
/// A provider that answers gets the normal short TTL. One that refuses gets
/// progressively more room: retrying a rate-limited endpoint on a fixed short
/// cycle keeps the limit alive instead of letting it lapse.
fn retry_delay(failures: u32) -> Duration {
    if failures == 0 {
        return LIMIT_TTL;
    }
    let minutes = 1u64.checked_shl(failures - 1).unwrap_or(u64::MAX);
    Duration::from_secs(minutes.saturating_mul(60)).min(MAX_BACKOFF)
}

/// Last reading per profile id, shared by the tray commands and the engine.
fn limit_cache() -> &'static Mutex<HashMap<String, CachedLimit>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedLimit>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Rate limit windows for a profile, served from a short-lived cache.
///
/// Failed reads are cached as well, and back off, so an endpoint that is
/// rate limiting us is not asked again every minute for hours on end.
pub fn rate_limit_for_profile(profile: &Profile) -> RateLimitStatus {
    let mut failures = 0;

    if let Ok(cache) = limit_cache().lock() {
        if let Some(entry) = cache.get(&profile.id) {
            if entry.fetched_at.elapsed() < retry_delay(entry.failures) {
                return entry.status.clone();
            }
            failures = entry.failures;
        }
    }

    let status = fetch_rate_limit(profile);
    let failures = if status.available { 0 } else { failures.saturating_add(1) };

    if let Ok(mut cache) = limit_cache().lock() {
        cache.insert(
            profile.id.clone(),
            CachedLimit {
                fetched_at: Instant::now(),
                status: status.clone(),
                failures,
            },
        );
    }

    status
}

/// Drop a profile's backoff so an explicit refresh is honoured immediately.
pub fn forget_cached_limit(profile_id: &str) {
    if let Ok(mut cache) = limit_cache().lock() {
        cache.remove(profile_id);
    }
}

/// Ask the provider directly. Returns an unavailable status for providers that
/// do not expose limits.
fn fetch_rate_limit(profile: &Profile) -> RateLimitStatus {
    match (profile.provider_type.as_str(), profile.source_type.as_str()) {
        ("claude", "account") | ("claude", "") => {
            ClaudeProvider::new(profile.config_dir.clone().into()).get_rate_limit_status()
        }
        ("codex", "account") | ("codex", "") => {
            CodexProvider::new(profile.config_dir.clone().into()).get_rate_limit_status()
        }
        ("zai", "api") => match &profile.api_key {
            Some(key) => ZaiApiProvider::new(key.clone()).get_rate_limit_status(),
            None => RateLimitStatus::unavailable(),
        },
        _ => RateLimitStatus::unavailable(),
    }
}

/// True when a profile can report limit windows at all, so the settings UI can
/// mark the profiles that alerts actually apply to.
pub fn supports_rate_limits(profile: &Profile) -> bool {
    matches!(
        (profile.provider_type.as_str(), profile.source_type.as_str()),
        ("claude", "account") | ("claude", "") | ("codex", "account") | ("codex", "") | ("zai", "api")
    )
}

fn window_of(status: &RateLimitStatus, key: &str) -> Option<RateLimitWindow> {
    match key {
        "fiveHour" => status.five_hour.clone(),
        "sevenDay" => status.seven_day.clone(),
        "sevenDayOpus" => status.seven_day_opus.clone(),
        _ => None,
    }
}

fn severity_for(utilization: f64) -> &'static str {
    if utilization >= 90.0 {
        "critical"
    } else if utilization >= 70.0 {
        "warning"
    } else {
        "info"
    }
}

/// Minutes remaining until an RFC3339 timestamp. None if unparseable or already past.
fn minutes_until(resets_at: &str) -> Option<i64> {
    let target = chrono::DateTime::parse_from_rfc3339(resets_at).ok()?;
    let diff = target.timestamp() - chrono::Utc::now().timestamp();
    if diff <= 0 {
        None
    } else {
        Some(diff / 60)
    }
}

fn humanize_minutes(minutes: i64) -> String {
    if minutes >= 60 {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{}h", h)
        } else {
            format!("{}h {}m", h, m)
        }
    } else if minutes <= 1 {
        "less than a minute".to_string()
    } else {
        format!("{}m", minutes)
    }
}

// --- Queueing & window handling ---

fn push_alerts(app: &AppHandle, alerts: Vec<Alert>) {
    if alerts.is_empty() {
        return;
    }

    if let Some(state) = app.try_state::<AlertState>() {
        if let Ok(mut pending) = state.pending.lock() {
            pending.extend(alerts);
        }
    }

    if let Some(win) = app.get_webview_window(ALERT_WINDOW) {
        position_alert_window(&win);
        let _ = win.set_always_on_top(true);
        let _ = win.show();
    }

    // Broadcast: the overlay reacts by draining the queue.
    let _ = app.emit(ALERT_EVENT, ());
}

/// Pin the overlay to the bottom-right of the work area so it never covers the taskbar.
fn position_alert_window(win: &WebviewWindow) {
    let monitor = match win.current_monitor() {
        Ok(Some(m)) => Some(m),
        _ => win.primary_monitor().ok().flatten(),
    };
    let monitor = match monitor {
        Some(m) => m,
        None => return,
    };

    let area = monitor.work_area();
    let size = win.outer_size().unwrap_or(PhysicalSize {
        width: 380,
        height: 190,
    });
    let margin = (14.0 * monitor.scale_factor()).round() as i32;

    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;

    let _ = win.set_position(PhysicalPosition { x, y });
}

// --- Detection ---

/// Inspect one profile and return the alerts that should fire right now.
/// `fired` is updated in place so each alert is raised at most once per limit period.
fn collect_alerts(
    profile: &Profile,
    status: &RateLimitStatus,
    settings: &AlertSettings,
    fired: &mut HashSet<String>,
    live_periods: &mut HashSet<String>,
) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    // How near a reminder's mark the reset has to be for it to count as due.
    // It follows the check cadence so two consecutive checks cannot step over
    // a window entirely, with a floor for very frequent checking.
    let tolerance = (settings.check_interval_secs / 60).max(MIN_REMINDER_TOLERANCE_MINUTES) as i64;

    for key in WINDOW_KEYS {
        if !settings.windows.iter().any(|w| w.as_str() == key) {
            continue;
        }
        let Some(w) = window_of(status, key) else {
            continue;
        };

        // A limit period is identified by its reset timestamp: when the window rolls
        // over, the old dedup keys stop being live and get pruned.
        let period = w.resets_at.clone().unwrap_or_else(|| "none".to_string());
        let prefix = format!("{}|{}|{}", profile.id, key, period);
        live_periods.insert(prefix.clone());

        // --- Usage thresholds: fire only the highest one crossed, swallow the rest. ---
        let mut crossed: Vec<u32> = settings
            .usage_thresholds
            .iter()
            .copied()
            .filter(|t| w.utilization >= *t as f64)
            .collect();
        crossed.sort_unstable();

        if let Some(&highest) = crossed.last() {
            let key_for = |t: u32| format!("{}|usage:{}", prefix, t);
            if !fired.contains(&key_for(highest)) {
                let remaining = (100.0 - w.utilization).max(0.0);
                let reset_hint = w
                    .resets_at
                    .as_deref()
                    .and_then(minutes_until)
                    .map(|m| format!(" · resets in {}", humanize_minutes(m)))
                    .unwrap_or_default();

                alerts.push(Alert {
                    id: format!("{}-{}", key_for(highest), now),
                    kind: "usage".to_string(),
                    severity: severity_for(w.utilization).to_string(),
                    profile_id: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    provider_type: profile.provider_type.clone(),
                    window_key: key.to_string(),
                    window_label: w.label.clone(),
                    title: format!("{} · {} limit at {:.0}%", profile.name, w.label, w.utilization),
                    message: format!("{:.0}% remaining{}", remaining, reset_hint),
                    utilization: w.utilization,
                    threshold: Some(highest),
                    reminder_minutes: None,
                    resets_at: w.resets_at.clone(),
                    created_at: now.clone(),
                });
            }
            // Mark every crossed threshold so a lower one never fires late.
            for t in crossed {
                fired.insert(key_for(t));
            }
        }

        // --- Reset reminders: fire the most urgent bucket not yet raised. ---
        // Skipped for untouched windows, where "resets soon" carries no information.
        if w.utilization <= 0.0 {
            continue;
        }
        let Some(minutes_left) = w.resets_at.as_deref().and_then(minutes_until) else {
            continue;
        };

        let key_for = |m: u32| format!("{}|reset:{}", prefix, m);

        // Retire reminders whose window has gone by unnoticed, so an
        // "hour before" warning never turns up with ten minutes left.
        for m in settings.reset_reminder_minutes.iter().copied() {
            if minutes_left < m as i64 - tolerance {
                fired.insert(key_for(m));
            }
        }

        // Due when the reset falls inside the reminder's window. Checks run on
        // a schedule, so an exact match would almost never come up; the window
        // is what makes "an hour before" mean an hour rather than whenever the
        // next check happened to land.
        let mut due: Vec<u32> = settings
            .reset_reminder_minutes
            .iter()
            .copied()
            .filter(|m| {
                let mark = *m as i64;
                minutes_left <= mark && minutes_left >= mark - tolerance
            })
            .collect();
        due.sort_unstable();

        if let Some(&most_urgent) = due.first() {
            if !fired.contains(&key_for(most_urgent)) {
                alerts.push(Alert {
                    id: format!("{}-{}", key_for(most_urgent), now),
                    kind: "reset".to_string(),
                    severity: (if w.utilization >= 90.0 { "warning" } else { "info" }).to_string(),
                    profile_id: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    provider_type: profile.provider_type.clone(),
                    window_key: key.to_string(),
                    window_label: w.label.clone(),
                    title: format!(
                        "{} · {} resets in {}",
                        profile.name,
                        w.label,
                        humanize_minutes(minutes_left)
                    ),
                    message: format!("{:.0}% used so far this window", w.utilization),
                    utilization: w.utilization,
                    threshold: None,
                    reminder_minutes: Some(most_urgent),
                    resets_at: w.resets_at.clone(),
                    created_at: now.clone(),
                });
            }
            // Larger buckets are already past; retire them together.
            for m in due {
                fired.insert(key_for(m));
            }
        }
    }

    alerts
}

/// One polling pass. Returns the number of seconds to wait before the next one,
/// so a settings change takes effect on the following tick.
fn run_check(app: &AppHandle) -> u64 {
    let Some(app_state) = app.try_state::<AppState>() else {
        return 60;
    };

    let (settings, profiles) = match app_state.config.lock() {
        Ok(config) => (config.settings.alerts.clone(), config.profiles.clone()),
        Err(_) => return 60,
    };

    let interval = settings.check_interval_secs.clamp(60, 3600);
    if !settings.enabled {
        return interval;
    }

    let watched: Vec<Profile> = profiles
        .into_iter()
        .filter(|p| p.enabled && supports_rate_limits(p))
        .filter(|p| settings.profile_ids.is_empty() || settings.profile_ids.contains(&p.id))
        .collect();

    if watched.is_empty() {
        return interval;
    }

    // Network calls happen outside every lock.
    let statuses: Vec<(Profile, RateLimitStatus)> = watched
        .into_iter()
        .map(|p| {
            let status = rate_limit_for_profile(&p);
            (p, status)
        })
        .filter(|(_, s)| s.available)
        .collect();

    let Some(alert_state) = app.try_state::<AlertState>() else {
        return interval;
    };
    let Ok(mut fired) = alert_state.fired.lock() else {
        return interval;
    };

    let mut live_periods: HashSet<String> = HashSet::new();
    let mut alerts = Vec::new();
    for (profile, status) in &statuses {
        alerts.extend(collect_alerts(
            profile,
            status,
            &settings,
            &mut fired,
            &mut live_periods,
        ));
    }

    // Drop dedup keys whose limit period rolled over, so the next period alerts
    // again. Only keys of profiles that actually reported this pass are judged:
    // a profile that is offline or rate limited has no live periods, and pruning
    // its keys would re-fire every alert once it comes back.
    let reported: HashSet<&str> = statuses.iter().map(|(p, _)| p.id.as_str()).collect();
    fired.retain(|key| {
        let profile_id = key.split('|').next().unwrap_or("");
        !reported.contains(profile_id)
            || live_periods
                .iter()
                .any(|prefix| key.starts_with(prefix.as_str()))
    });
    drop(fired);

    push_alerts(app, alerts);
    interval
}

/// Start the background polling thread.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        // Let the UI settle before the first network call.
        std::thread::sleep(Duration::from_secs(5));
        loop {
            let wait = run_check(&app);
            std::thread::sleep(Duration::from_secs(wait));
        }
    });
}

// --- Commands ---

/// Hand the queued alerts to the overlay and clear the queue.
#[tauri::command]
pub fn drain_alerts(state: State<AlertState>) -> Result<Vec<Alert>, String> {
    let mut pending = state
        .pending
        .lock()
        .map_err(|e| format!("Failed to lock alert queue: {}", e))?;
    Ok(std::mem::take(&mut *pending))
}

/// Resize the overlay to fit its content and keep it pinned bottom-right.
#[tauri::command]
pub fn resize_alert_window(app: AppHandle, width: u32, height: u32) -> Result<(), String> {
    let win = app
        .get_webview_window(ALERT_WINDOW)
        .ok_or_else(|| "Alert window not found".to_string())?;

    win.set_size(PhysicalSize {
        width: width.clamp(280, 720),
        height: height.clamp(80, 900),
    })
    .map_err(|e| format!("Failed to resize alert window: {}", e))?;

    position_alert_window(&win);
    Ok(())
}

/// Hide the overlay once the last toast is dismissed.
#[tauri::command]
pub fn hide_alert_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(ALERT_WINDOW) {
        win.hide().map_err(|e| format!("Failed to hide alert window: {}", e))?;
    }
    Ok(())
}

/// Bring the tray popup up, e.g. from the toast's "View details" action.
#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

/// Raise a sample toast so the user can preview placement and styling.
/// `(async)` keeps the live limit lookup off the main thread.
#[tauri::command(async)]
pub fn send_test_alert(app: AppHandle, profile_id: Option<String>) -> Result<(), String> {
    let now = chrono::Utc::now();

    // Prefer a real reading from the selected profile; fall back to a synthetic one.
    let real = profile_id.as_ref().and_then(|id| {
        let app_state = app.try_state::<AppState>()?;
        let config = app_state.config.lock().ok()?;
        let profile = config.profiles.iter().find(|p| &p.id == id)?.clone();
        drop(config);

        let status = rate_limit_for_profile(&profile);
        if !status.available {
            return None;
        }
        let w = WINDOW_KEYS
            .into_iter()
            .find_map(|key| window_of(&status, key))?;
        Some((profile, w))
    });

    let alert = match real {
        Some((profile, w)) => {
            let reset_hint = w
                .resets_at
                .as_deref()
                .and_then(minutes_until)
                .map(|m| format!(" · resets in {}", humanize_minutes(m)))
                .unwrap_or_default();
            Alert {
                id: format!("test-{}", now.timestamp_millis()),
                kind: "test".to_string(),
                severity: severity_for(w.utilization).to_string(),
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                provider_type: profile.provider_type.clone(),
                window_key: "fiveHour".to_string(),
                window_label: w.label.clone(),
                title: format!("{} · {} limit at {:.0}%", profile.name, w.label, w.utilization),
                message: format!("{:.0}% remaining{}", (100.0 - w.utilization).max(0.0), reset_hint),
                utilization: w.utilization,
                threshold: None,
                reminder_minutes: None,
                resets_at: w.resets_at.clone(),
                created_at: now.to_rfc3339(),
            }
        }
        None => Alert {
            id: format!("test-{}", now.timestamp_millis()),
            kind: "test".to_string(),
            severity: "warning".to_string(),
            profile_id: profile_id.unwrap_or_default(),
            profile_name: "Claude".to_string(),
            provider_type: "claude".to_string(),
            window_key: "fiveHour".to_string(),
            window_label: "5-Hour".to_string(),
            title: "Claude · 5-Hour limit at 80%".to_string(),
            message: "20% remaining · resets in 1h 20m".to_string(),
            utilization: 80.0,
            threshold: Some(80),
            reminder_minutes: None,
            resets_at: Some((now + chrono::Duration::minutes(80)).to_rfc3339()),
            created_at: now.to_rfc3339(),
        },
    };

    push_alerts(&app, vec![alert]);
    Ok(())
}
