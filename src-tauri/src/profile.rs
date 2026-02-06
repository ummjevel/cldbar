use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub profiles: Vec<Profile>,
    pub settings: AppSettings,
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
    pub theme: String,
    pub refresh_interval_ms: u64,
    pub launch_on_startup: bool,
    pub notifications_enabled: bool,
    pub token_alert_threshold: u64,
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

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse config file: {}", e))
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
        profiles,
        settings: AppSettings {
            theme: "system".to_string(),
            refresh_interval_ms: 5000,
            launch_on_startup: false,
            notifications_enabled: true,
            token_alert_threshold: 1_000_000,
        },
    }
}
