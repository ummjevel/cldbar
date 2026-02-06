use crate::profile::{self, AppConfig, AppSettings, Profile};
use crate::providers::claude::ClaudeProvider;
use crate::providers::claude_api::ClaudeApiProvider;
use crate::providers::gemini::GeminiProvider;
use crate::providers::zai::ZaiProvider;
use crate::providers::{DailyUsage, Provider, Session, UsageStats};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub providers: Mutex<HashMap<String, Box<dyn Provider>>>,
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
        }
    }
}

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<ProfileInfo>, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;
    Ok(config.profiles.iter().map(ProfileInfo::from).collect())
}

#[tauri::command]
pub fn add_profile(state: State<AppState>, profile: Profile) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;

    // Validate config directory for account-type profiles
    if profile.source_type != "api" {
        let dir = std::path::Path::new(&profile.config_dir);
        if !dir.exists() {
            return Err(format!("Config directory does not exist: {}", profile.config_dir));
        }
    }

    // Create and register the provider
    let provider: Box<dyn Provider> = match (profile.provider_type.as_str(), profile.source_type.as_str()) {
        ("claude", "api") => {
            let key = profile.api_key.as_ref()
                .ok_or_else(|| "API key is required for API source type".to_string())?;
            Box::new(ClaudeApiProvider::new(key.clone()))
        }
        ("claude", _) => Box::new(ClaudeProvider::new(profile.config_dir.clone().into())),
        ("gemini", _) => Box::new(GeminiProvider::new(profile.config_dir.clone().into())),
        ("zai", _) => Box::new(ZaiProvider::new(profile.config_dir.clone().into())),
        (other, _) => return Err(format!("Unknown provider type: {}", other)),
    };

    let mut providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    providers.insert(profile.id.clone(), provider);
    config.profiles.push(profile);
    profile::save_config(&config)?;

    Ok(())
}

#[tauri::command]
pub fn remove_profile(state: State<AppState>, id: String) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;

    let mut providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    config.profiles.retain(|p| p.id != id);
    providers.remove(&id);
    profile::save_config(&config)?;

    Ok(())
}

#[tauri::command]
pub fn get_usage_stats(state: State<AppState>, profile_id: String) -> Result<UsageStats, String> {
    let providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    let provider = providers
        .get(&profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    provider.get_usage_stats()
}

#[tauri::command]
pub fn get_active_sessions(
    state: State<AppState>,
    profile_id: String,
) -> Result<Vec<Session>, String> {
    let providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    let provider = providers
        .get(&profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    provider.get_active_sessions()
}

#[tauri::command]
pub fn get_daily_usage(
    state: State<AppState>,
    profile_id: String,
    days: u32,
) -> Result<Vec<DailyUsage>, String> {
    let providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    let provider = providers
        .get(&profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    provider.get_daily_usage(days)
}

#[tauri::command]
pub fn get_session_history(
    state: State<AppState>,
    profile_id: String,
    limit: u32,
) -> Result<Vec<Session>, String> {
    let providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    let provider = providers
        .get(&profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    provider.get_session_history(limit)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;
    Ok(config.settings.clone())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: AppSettings) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;

    config.settings = settings;
    profile::save_config(&config)?;

    Ok(())
}

#[tauri::command]
pub fn get_all_usage_stats(state: State<AppState>) -> Result<Vec<UsageStats>, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;

    let providers = state
        .providers
        .lock()
        .map_err(|e| format!("Failed to lock providers: {}", e))?;

    let mut all_stats = Vec::new();

    for profile in &config.profiles {
        if !profile.enabled {
            continue;
        }

        if let Some(provider) = providers.get(&profile.id) {
            match provider.get_usage_stats() {
                Ok(stats) => all_stats.push(stats),
                Err(_) => {
                    // Skip providers that fail to load stats
                    continue;
                }
            }
        }
    }

    Ok(all_stats)
}

#[tauri::command]
pub fn validate_api_key(api_key: String) -> Result<bool, String> {
    // Try a lightweight API call to check if the key is valid.
    // We request a minimal usage report (1 day, limit 1).
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

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
