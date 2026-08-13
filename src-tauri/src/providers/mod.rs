use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub provider: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_sessions: u32,
    pub total_messages: u32,
    pub estimated_cost_usd: f64,
    pub model_breakdown: HashMap<String, ModelUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub project: String,
    pub model: String,
    pub tokens_used: u64,
    pub last_active: String,
    pub is_active: bool,
    pub message_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub sessions: u32,
    pub messages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitStatus {
    pub available: bool,
    pub five_hour: Option<RateLimitWindow>,
    pub seven_day: Option<RateLimitWindow>,
    pub seven_day_opus: Option<RateLimitWindow>,
    /// When the reading was taken, for providers that report limits from a local
    /// snapshot rather than live. `None` means the numbers are current.
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl RateLimitStatus {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            five_hour: None,
            seven_day: None,
            seven_day_opus: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindow {
    pub label: String,
    pub utilization: f64,
    pub resets_at: Option<String>,
}

/// One process-wide blocking HTTP client.
///
/// The data commands run on the async runtime, and dropping a
/// `reqwest::blocking::Client` inside an async context panics with "Cannot drop
/// a runtime in a context where blocking is not allowed". A single client that
/// outlives every request never reaches that drop, and it reuses connections
/// instead of standing up a runtime per call.
pub fn http_client() -> &'static reqwest::blocking::Client {
    static CLIENT: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

pub trait Provider: Send + Sync {
    fn get_usage_stats(&self) -> Result<UsageStats, String>;
    fn get_active_sessions(&self) -> Result<Vec<Session>, String>;
    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String>;
    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String>;
}

/// Build the provider implementation for a profile.
///
/// Shared by app startup and the add-profile command, so the two can never
/// drift apart on how a profile maps to an implementation. Returned as `Arc`
/// so commands can snapshot a provider and release the registry lock before
/// doing any slow work with it.
pub fn create_provider(profile: &crate::profile::Profile) -> Result<Arc<dyn Provider>, String> {
    let provider: Arc<dyn Provider> = match (profile.provider_type.as_str(), profile.source_type.as_str()) {
        ("claude", "api") => {
            let key = profile.api_key.as_ref()
                .ok_or_else(|| "API key is required for API source type".to_string())?;
            Arc::new(claude_api::ClaudeApiProvider::new(key.clone()))
        }
        ("claude", _) => Arc::new(claude::ClaudeProvider::new(profile.config_dir.clone().into())),
        ("codex", _) => Arc::new(codex::CodexProvider::new(profile.config_dir.clone().into())),
        ("gemini", _) => Arc::new(gemini::GeminiProvider::new(profile.config_dir.clone().into())),
        ("zai", "api") => {
            let key = profile.api_key.as_ref()
                .ok_or_else(|| "API key is required for z.ai API source type".to_string())?;
            Arc::new(zai_api::ZaiApiProvider::new(key.clone()))
        }
        ("zai", _) => Arc::new(zai::ZaiProvider::new(profile.config_dir.clone().into())),
        (other, _) => return Err(format!("Unknown provider type: {}", other)),
    };
    Ok(provider)
}

pub mod claude;
pub mod claude_api;
pub mod codex;
pub mod gemini;
pub mod zai;
pub mod zai_api;
