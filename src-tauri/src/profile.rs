use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub profiles: Vec<Profile>,
    pub settings: AppSettings,
    /// Providers auto-detection has already offered, so a profile the user removed
    /// is not silently added back on the next launch.
    #[serde(default)]
    pub detected_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config_dir: String,
    pub enabled: bool,
    #[serde(default = "default_source_type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

fn default_source_type() -> String {
    "account".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_refresh_interval_ms")]
    pub refresh_interval_ms: u64,
    #[serde(default)]
    pub launch_on_startup: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default = "default_token_alert_threshold")]
    pub token_alert_threshold: u64,
    /// Whether limit windows lead with what is left or what has been spent:
    /// "remaining" | "used".
    #[serde(default = "default_limit_display")]
    pub limit_display: String,
    #[serde(default = "default_alert_settings")]
    pub alerts: AlertSettings,
}

/// User-configurable rules for the usage/limit alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertSettings {
    /// Master switch for the whole alert engine.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Usage percentages (0-100). An alert fires the first time a window crosses one.
    #[serde(default = "default_usage_thresholds")]
    pub usage_thresholds: Vec<u32>,
    /// Minutes before a window resets that a reminder fires.
    #[serde(default = "default_reset_reminder_minutes")]
    pub reset_reminder_minutes: Vec<u32>,
    /// Profile ids to watch. Empty means every profile that reports limits.
    #[serde(default)]
    pub profile_ids: Vec<String>,
    /// Limit windows to watch: "fiveHour" | "sevenDay" | "sevenDayOpus".
    #[serde(default = "default_alert_windows")]
    pub windows: Vec<String>,
    /// How often the engine polls each profile, in seconds.
    #[serde(default = "default_check_interval_secs")]
    pub check_interval_secs: u64,
    /// How long a toast stays on screen, in seconds. 0 keeps it until dismissed.
    #[serde(default = "default_toast_duration_secs")]
    pub duration_secs: u64,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_refresh_interval_ms() -> u64 {
    5000
}

fn default_true() -> bool {
    true
}

fn default_token_alert_threshold() -> u64 {
    1_000_000
}

fn default_limit_display() -> String {
    "remaining".to_string()
}

fn default_usage_thresholds() -> Vec<u32> {
    vec![50, 80, 95]
}

fn default_reset_reminder_minutes() -> Vec<u32> {
    vec![30]
}

fn default_alert_windows() -> Vec<String> {
    vec![
        "fiveHour".to_string(),
        "sevenDay".to_string(),
        "sevenDayOpus".to_string(),
    ]
}

fn default_check_interval_secs() -> u64 {
    60
}

fn default_toast_duration_secs() -> u64 {
    12
}

pub fn default_alert_settings() -> AlertSettings {
    AlertSettings {
        enabled: true,
        usage_thresholds: default_usage_thresholds(),
        reset_reminder_minutes: default_reset_reminder_minutes(),
        profile_ids: Vec::new(),
        windows: default_alert_windows(),
        check_interval_secs: default_check_interval_secs(),
        duration_secs: default_toast_duration_secs(),
    }
}

/// Get the path to the cldbar config file: %APPDATA%/cldbar/config.json
fn config_file_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    Ok(config_dir.join("cldbar").join("config.json"))
}

/// Load the app configuration from disk.
/// Creates a default config if the file does not exist.
pub fn load_config() -> Result<AppConfig, String> {
    let path = config_file_path()?;

    if !path.exists() {
        let config = default_config();
        save_config(&config)?;
        return Ok(config);
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let mut config: AppConfig = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    if detect_new_providers(&mut config) {
        save_config(&config)?;
    }

    Ok(config)
}

/// Offer providers added after this config was written, once each.
/// Returns true when the config changed and should be persisted.
fn detect_new_providers(config: &mut AppConfig) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };

    let mut changed = false;

    // Configs written before Codex support existed have neither the profile nor the
    // detection marker, so seed the marker from the profiles that are already there.
    if config.detected_providers.is_empty() {
        config.detected_providers = config
            .profiles
            .iter()
            .map(|p| p.provider_type.clone())
            .collect();
        changed = true;
    }

    let codex_dir = home.join(".codex");
    if !config.detected_providers.iter().any(|p| p == "codex") {
        config.detected_providers.push("codex".to_string());
        changed = true;

        if codex_dir.exists() {
            config.profiles.push(Profile {
                id: "codex-default".to_string(),
                name: "Codex".to_string(),
                provider_type: "codex".to_string(),
                config_dir: codex_dir.to_string_lossy().to_string(),
                enabled: true,
                source_type: "account".to_string(),
                api_key: None,
            });
        }
    }

    changed
}

/// Save the app configuration to disk.
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_file_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let data = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&path, data)
        .map_err(|e| format!("Failed to write config file: {}", e))
}

/// Create a default configuration that auto-detects installed providers.
pub fn default_config() -> AppConfig {
    let mut profiles = Vec::new();

    // Auto-detect Claude: check if ~/.claude/ exists
    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude");
        if claude_dir.exists() {
            profiles.push(Profile {
                id: "claude-default".to_string(),
                name: "Claude".to_string(),
                provider_type: "claude".to_string(),
                config_dir: claude_dir.to_string_lossy().to_string(),
                enabled: true,
                source_type: "account".to_string(),
                api_key: None,
            });
        }

        // Auto-detect Codex: check if ~/.codex/ exists
        let codex_dir = home.join(".codex");
        if codex_dir.exists() {
            profiles.push(Profile {
                id: "codex-default".to_string(),
                name: "Codex".to_string(),
                provider_type: "codex".to_string(),
                config_dir: codex_dir.to_string_lossy().to_string(),
                enabled: true,
                source_type: "account".to_string(),
                api_key: None,
            });
        }

        // Auto-detect Gemini: check if ~/.gemini/ exists
        let gemini_dir = home.join(".gemini");
        if gemini_dir.exists() {
            profiles.push(Profile {
                id: "gemini-default".to_string(),
                name: "Gemini".to_string(),
                provider_type: "gemini".to_string(),
                config_dir: gemini_dir.to_string_lossy().to_string(),
                enabled: true,
                source_type: "account".to_string(),
                api_key: None,
            });
        }
    }

    // Auto-detect z.ai: check if %APPDATA%/zai/ exists
    if let Some(config_dir) = dirs::config_dir() {
        let zai_dir = config_dir.join("zai");
        if zai_dir.exists() {
            profiles.push(Profile {
                id: "zai-default".to_string(),
                name: "z.ai".to_string(),
                provider_type: "zai".to_string(),
                config_dir: zai_dir.to_string_lossy().to_string(),
                enabled: true,
                source_type: "account".to_string(),
                api_key: None,
            });
        }
    }

    AppConfig {
        detected_providers: vec![
            "claude".to_string(),
            "codex".to_string(),
            "gemini".to_string(),
            "zai".to_string(),
        ],
        profiles,
        settings: AppSettings {
            theme: default_theme(),
            refresh_interval_ms: default_refresh_interval_ms(),
            launch_on_startup: false,
            notifications_enabled: true,
            token_alert_threshold: default_token_alert_threshold(),
            limit_display: default_limit_display(),
            alerts: default_alert_settings(),
        },
    }
}
