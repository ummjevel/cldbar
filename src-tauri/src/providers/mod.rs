use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn provider_type(&self) -> &str;
    fn config_dir(&self) -> &PathBuf;
    fn get_usage_stats(&self) -> Result<UsageStats, String>;
    fn get_active_sessions(&self) -> Result<Vec<Session>, String>;
    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String>;
    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String>;
}

pub mod claude;
pub mod claude_api;
pub mod gemini;
pub mod zai;
