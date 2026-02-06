use super::{DailyUsage, ModelUsage, Provider, RateLimitStatus, RateLimitWindow, Session, UsageStats};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ZaiApiProvider {
    api_key: String,
    base_url: String,
    config_dir: PathBuf,
}

// --- Deserialization types for z.ai monitoring API ---

/// Response from /api/monitor/usage/quota/limit
/// Actual format: { "limits": [{ "type": "...", "percentage": 0.5, "nextResetTime": 1234567890000 }] }
#[derive(Debug, Deserialize)]
struct QuotaLimitResponse {
    #[serde(default)]
    limits: Vec<QuotaLimitItem>,
}

#[derive(Debug, Deserialize)]
struct QuotaLimitItem {
    #[serde(default, rename = "type")]
    limit_type: String,
    #[serde(default)]
    percentage: f64,
    #[serde(default, rename = "nextResetTime")]
    next_reset_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ModelUsageResponse {
    #[serde(default)]
    data: Option<Vec<ModelUsageEntry>>,
}

#[derive(Debug, Deserialize)]
struct ModelUsageEntry {
    #[serde(default, rename = "modelName")]
    model_name: String,
    #[serde(default, rename = "inputTokens")]
    input_tokens: u64,
    #[serde(default, rename = "outputTokens")]
    output_tokens: u64,
    #[serde(default, rename = "callCount")]
    call_count: u32,
}

impl ZaiApiProvider {
    pub fn new(api_key: String) -> Self {
        // Detect platform from API key or default to global
        let base_url = "https://api.z.ai".to_string();
        Self {
            api_key,
            base_url,
            config_dir: PathBuf::new(),
        }
    }

    fn client(&self) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))
    }

    /// Fetch quota/rate limit status from z.ai monitoring API.
    pub fn get_rate_limit_status(&self) -> RateLimitStatus {
        let unavailable = RateLimitStatus {
            available: false,
            five_hour: None,
            seven_day: None,
            seven_day_opus: None,
        };

        let client = match self.client() {
            Ok(c) => c,
            Err(_) => return unavailable,
        };

        let resp = client
            .get(format!("{}/api/monitor/usage/quota/limit", self.base_url))
            .header("Authorization", &self.api_key)
            .header("Accept-Language", "en-US,en")
            .header("Content-Type", "application/json")
            .send();

        match resp {
            Ok(r) if r.status().is_success() => {
                let body = match r.text() {
                    Ok(b) => b,
                    Err(_) => return unavailable,
                };

                let quota: QuotaLimitResponse = match serde_json::from_str(&body) {
                    Ok(q) => q,
                    Err(_) => return unavailable,
                };

                if quota.limits.is_empty() {
                    return unavailable;
                }

                let mut token_window: Option<RateLimitWindow> = None;
                let mut time_window: Option<RateLimitWindow> = None;

                for item in &quota.limits {
                    let reset_str = item.next_reset_time.map(|ms| {
                        let secs = ms / 1000;
                        let nanos = ((ms % 1000) * 1_000_000) as u32;
                        chrono::DateTime::from_timestamp(secs, nanos)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    });

                    // percentage from API is 0-1 range, convert to 0-100
                    let pct = item.percentage * 100.0;

                    if item.limit_type.contains("TOKEN") {
                        token_window = Some(RateLimitWindow {
                            label: "Token Limit".to_string(),
                            utilization: pct,
                            resets_at: reset_str,
                        });
                    } else if item.limit_type.contains("TIME") {
                        time_window = Some(RateLimitWindow {
                            label: "Time Limit".to_string(),
                            utilization: pct,
                            resets_at: reset_str,
                        });
                    }
                }

                RateLimitStatus {
                    available: token_window.is_some() || time_window.is_some(),
                    five_hour: token_window,
                    seven_day: time_window,
                    seven_day_opus: None,
                }
            }
            _ => unavailable,
        }
    }

    /// Fetch model usage from z.ai monitoring API (24h rolling window).
    fn fetch_model_usage(&self) -> Option<Vec<ModelUsageEntry>> {
        let client = self.client().ok()?;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(24);
        let start_time = start.format("%Y-%m-%d %H:00:00").to_string();
        let end_time = now.format("%Y-%m-%d %H:59:59").to_string();

        let resp = client
            .get(format!("{}/api/monitor/usage/model-usage", self.base_url))
            .header("Authorization", &self.api_key)
            .header("Accept-Language", "en-US,en")
            .header("Content-Type", "application/json")
            .query(&[("startTime", &start_time), ("endTime", &end_time)])
            .send()
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let body: ModelUsageResponse = resp.json().ok()?;
        body.data
    }
}

impl Provider for ZaiApiProvider {
    fn name(&self) -> &str {
        "z.ai"
    }

    fn provider_type(&self) -> &str {
        "zai"
    }

    fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn get_usage_stats(&self) -> Result<UsageStats, String> {
        let entries = self.fetch_model_usage().unwrap_or_default();

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_messages: u32 = 0;
        let mut total_cost: f64 = 0.0;
        let mut model_breakdown: HashMap<String, ModelUsage> = HashMap::new();

        for entry in &entries {
            let input_rate = 1.0_f64;
            let output_rate = 4.0_f64;
            let cost = (entry.input_tokens as f64 * input_rate
                + entry.output_tokens as f64 * output_rate)
                / 1_000_000.0;

            total_input += entry.input_tokens;
            total_output += entry.output_tokens;
            total_messages += entry.call_count;
            total_cost += cost;

            model_breakdown.insert(
                entry.model_name.clone(),
                ModelUsage {
                    model: entry.model_name.clone(),
                    input_tokens: entry.input_tokens,
                    output_tokens: entry.output_tokens,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_usd: (cost * 100.0).round() / 100.0,
                },
            );
        }

        Ok(UsageStats {
            provider: "z.ai".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: 0,
            total_cache_write_tokens: 0,
            total_sessions: 0,
            total_messages,
            estimated_cost_usd: (total_cost * 100.0).round() / 100.0,
            model_breakdown,
        })
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        // z.ai API doesn't provide session tracking
        Ok(Vec::new())
    }

    fn get_daily_usage(&self, _days: u32) -> Result<Vec<DailyUsage>, String> {
        // z.ai monitoring API only provides 24h rolling window, not daily breakdown
        Ok(Vec::new())
    }

    fn get_session_history(&self, _limit: u32) -> Result<Vec<Session>, String> {
        Ok(Vec::new())
    }
}
