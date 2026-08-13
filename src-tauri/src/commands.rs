use crate::alerts;
use crate::profile::{self, AppConfig, AppSettings, Profile};
use crate::providers::{self, DailyUsage, Provider, RateLimitStatus, Session, UsageStats};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    /// Provider implementation per profile id. Values are `Arc` so a command
    /// can clone its provider out and release this lock before doing any slow
    /// filesystem or network work — holding the lock across a fetch would
    /// stall every other command for its duration.
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
}

/// DTO that excludes the API key from frontend exposure.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config_dir: String,
    pub enabled: bool,
    pub source_type: String,
    pub has_api_key: bool,
    /// Whether this profile reports rate limit windows, i.e. whether alerts apply to it.
    pub supports_rate_limits: bool,
}

impl From<&Profile> for ProfileInfo {
    fn from(p: &Profile) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            provider_type: p.provider_type.clone(),
            config_dir: p.config_dir.clone(),
            enabled: p.enabled,
            source_type: p.source_type.clone(),
            has_api_key: p.api_key.is_some(),
            supports_rate_limits: alerts::supports_rate_limits(p),
        }
    }
}

fn lock_err<T>(what: &str) -> impl Fn(std::sync::PoisonError<T>) -> String + '_ {
    move |e| format!("Failed to lock {}: {}", what, e)
}

/// Snapshot the provider for a profile, releasing the registry lock immediately.
fn provider_for(state: &State<AppState>, profile_id: &str) -> Result<Arc<dyn Provider>, String> {
    let providers = state.providers.lock().map_err(lock_err("providers"))?;
    providers
        .get(profile_id)
        .cloned()
        .ok_or_else(|| format!("Profile not found: {}", profile_id))
}

// --- Profile management ---

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<ProfileInfo>, String> {
    let config = state.config.lock().map_err(lock_err("config"))?;
    Ok(config.profiles.iter().map(ProfileInfo::from).collect())
}

#[tauri::command]
pub fn add_profile(state: State<AppState>, profile: Profile) -> Result<(), String> {
    // Validate config directory for account-type profiles
    if profile.source_type != "api" {
        let dir = std::path::Path::new(&profile.config_dir);
        if !dir.exists() {
            return Err(format!("Config directory does not exist: {}", profile.config_dir));
        }
    }

    let provider = providers::create_provider(&profile)?;

    let mut config = state.config.lock().map_err(lock_err("config"))?;
    let mut providers = state.providers.lock().map_err(lock_err("providers"))?;

    providers.insert(profile.id.clone(), provider);
    config.profiles.push(profile);
    profile::save_config(&config)?;

    Ok(())
}

#[tauri::command]
pub fn remove_profile(state: State<AppState>, id: String) -> Result<(), String> {
    let mut config = state.config.lock().map_err(lock_err("config"))?;
    let mut providers = state.providers.lock().map_err(lock_err("providers"))?;

    config.profiles.retain(|p| p.id != id);
    providers.remove(&id);
    profile::save_config(&config)?;

    Ok(())
}

// --- Usage data ---
//
// These commands touch the filesystem and the network. `(async)` runs them on
// the async runtime instead of the main thread, which would otherwise stall
// the UI and the tray for the duration of every poll.

#[tauri::command(async)]
pub fn get_usage_stats(state: State<AppState>, profile_id: String) -> Result<UsageStats, String> {
    provider_for(&state, &profile_id)?.get_usage_stats()
}

#[tauri::command(async)]
pub fn get_active_sessions(
    state: State<AppState>,
    profile_id: String,
) -> Result<Vec<Session>, String> {
    provider_for(&state, &profile_id)?.get_active_sessions()
}

#[tauri::command(async)]
pub fn get_daily_usage(
    state: State<AppState>,
    profile_id: String,
    days: u32,
) -> Result<Vec<DailyUsage>, String> {
    provider_for(&state, &profile_id)?.get_daily_usage(days)
}

#[tauri::command(async)]
pub fn get_session_history(
    state: State<AppState>,
    profile_id: String,
    limit: u32,
) -> Result<Vec<Session>, String> {
    provider_for(&state, &profile_id)?.get_session_history(limit)
}

#[tauri::command(async)]
pub fn get_all_usage_stats(state: State<AppState>) -> Result<Vec<UsageStats>, String> {
    // Snapshot the enabled providers first so neither lock is held while the
    // providers do their (potentially slow) reads.
    let snapshot: Vec<Arc<dyn Provider>> = {
        let config = state.config.lock().map_err(lock_err("config"))?;
        let providers = state.providers.lock().map_err(lock_err("providers"))?;
        config
            .profiles
            .iter()
            .filter(|p| p.enabled)
            .filter_map(|p| providers.get(&p.id).cloned())
            .collect()
    };

    Ok(snapshot
        .iter()
        .filter_map(|provider| provider.get_usage_stats().ok())
        .collect())
}

/// `force` skips the cache and any failure backoff, for an explicit refresh.
#[tauri::command(async)]
pub fn get_rate_limit_status(
    state: State<AppState>,
    profile_id: String,
    force: Option<bool>,
) -> Result<RateLimitStatus, String> {
    if force.unwrap_or(false) {
        alerts::forget_cached_limit(&profile_id);
    }

    let profile = {
        let config = state.config.lock().map_err(lock_err("config"))?;
        config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .ok_or_else(|| format!("Profile not found: {}", profile_id))?
    };

    Ok(alerts::rate_limit_for_profile(&profile))
}

// --- Settings ---

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let config = state.config.lock().map_err(lock_err("config"))?;
    Ok(config.settings.clone())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: AppSettings) -> Result<(), String> {
    let mut config = state.config.lock().map_err(lock_err("config"))?;
    config.settings = settings;
    profile::save_config(&config)?;
    Ok(())
}

// --- Validation ---

#[tauri::command(async)]
pub fn validate_api_key(api_key: String, provider_type: Option<String>) -> Result<bool, String> {
    let client = crate::providers::http_client();

    let provider = provider_type.unwrap_or_else(|| "claude".to_string());

    match provider.as_str() {
        "zai" => {
            // Validate z.ai key by calling the quota endpoint
            let resp = client
                .get("https://api.z.ai/api/monitor/usage/quota/limit")
                .header("Authorization", &api_key)
                .header("Accept-Language", "en-US,en")
                .header("Content-Type", "application/json")
                .send()
                .map_err(|e| format!("API validation request failed: {}", e))?;

            Ok(resp.status().is_success())
        }
        _ => {
            // Claude Admin API key validation
            let now = chrono::Utc::now();
            let start = now - chrono::Duration::days(1);
            let starting_at = start.format("%Y-%m-%dT00:00:00Z").to_string();
            let ending_at = now.format("%Y-%m-%dT23:59:59Z").to_string();

            let resp = client
                .get("https://api.anthropic.com/v1/organizations/usage_report/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .query(&[
                    ("starting_at", starting_at.as_str()),
                    ("ending_at", ending_at.as_str()),
                    ("limit", "1"),
                ])
                .send()
                .map_err(|e| format!("API validation request failed: {}", e))?;

            Ok(resp.status().is_success())
        }
    }
}
