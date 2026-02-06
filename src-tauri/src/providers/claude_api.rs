use super::{DailyUsage, ModelUsage, Provider, Session, UsageStats};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cache entry with TTL
struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
}

/// Claude API provider that fetches usage data from the Anthropic Admin API.
/// Requires an Admin API key (sk-ant-admin...).
pub struct ClaudeApiProvider {
    api_key: String,
    client: reqwest::blocking::Client,
    usage_cache: Mutex<Option<CacheEntry<UsageStats>>>,
    daily_cache: Mutex<Option<CacheEntry<Vec<DailyUsage>>>>,
}

const CACHE_TTL: Duration = Duration::from_secs(60);
const API_BASE: &str = "https://api.anthropic.com";

// --- API response types ---

#[derive(Debug, Deserialize)]
struct UsageReport {
    data: Vec<UsageBucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageBucket {
    starting_at: String,
    #[allow(dead_code)]
    ending_at: String,
    results: Vec<UsageResult>,
}

#[derive(Debug, Deserialize)]
struct UsageResult {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    uncached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation: Option<CacheCreation>,
}

#[derive(Debug, Deserialize)]
struct CacheCreation {
    #[serde(default)]
    ephemeral_5m_input_tokens: u64,
    #[serde(default)]
    ephemeral_1h_input_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct CostReport {
    data: Vec<CostBucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CostBucket {
    #[allow(dead_code)]
    starting_at: String,
    #[allow(dead_code)]
    ending_at: String,
    results: Vec<CostResult>,
}

#[derive(Debug, Deserialize)]
struct CostResult {
    amount: String,
    #[serde(default)]
    currency: Option<String>,
}

impl ClaudeApiProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            api_key,
            client,
            usage_cache: Mutex::new(None),
            daily_cache: Mutex::new(None),
        }
    }

    /// Fetch usage report from Anthropic Admin API with pagination support.
    fn fetch_usage_report(&self, starting_at: &str, ending_at: &str, group_by_model: bool) -> Result<Vec<UsageBucket>, String> {
        let mut all_buckets = Vec::new();
        let mut page: Option<String> = None;

        loop {
            let mut req = self.client
                .get(format!("{}/v1/organizations/usage_report/messages", API_BASE))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .query(&[
                    ("starting_at", starting_at),
                    ("ending_at", ending_at),
                    ("bucket_width", "1d"),
                    ("limit", "31"),
                ]);

            if group_by_model {
                req = req.query(&[("group_by[]", "model")]);
            }

            if let Some(ref p) = page {
                req = req.query(&[("page", p.as_str())]);
            }

            let resp = req
                .send()
                .map_err(|e| format!("API request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                return Err(format!("API error {}: {}", status, body));
            }

            let report: UsageReport = resp
                .json()
                .map_err(|e| format!("Failed to parse usage report: {}", e))?;

            all_buckets.extend(report.data);

            if report.has_more {
                page = report.next_page;
            } else {
                break;
            }
        }

        Ok(all_buckets)
    }

    /// Fetch cost report from Anthropic Admin API with pagination support.
    fn fetch_cost_report(&self, starting_at: &str, ending_at: &str) -> Result<f64, String> {
        let mut total_cents: f64 = 0.0;
        let mut page: Option<String> = None;

        loop {
            let mut req = self.client
                .get(format!("{}/v1/organizations/cost_report", API_BASE))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .query(&[
                    ("starting_at", starting_at),
                    ("ending_at", ending_at),
                    ("bucket_width", "1d"),
                    ("limit", "31"),
                ]);

            if let Some(ref p) = page {
                req = req.query(&[("page", p.as_str())]);
            }

            let resp = req
                .send()
                .map_err(|e| format!("Cost API request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                return Err(format!("Cost API error {}: {}", status, body));
            }

            let report: CostReport = resp
                .json()
                .map_err(|e| format!("Failed to parse cost report: {}", e))?;

            for bucket in &report.data {
                for result in &bucket.results {
                    if let Ok(amount) = result.amount.parse::<f64>() {
                        // amount is in cents for USD
                        let _currency = result.currency.as_deref().unwrap_or("USD");
                        total_cents += amount;
                    }
                }
            }

            if report.has_more {
                page = report.next_page;
            } else {
                break;
            }
        }

        // Convert cents to dollars
        Ok((total_cents / 100.0 * 100.0).round() / 100.0)
    }

    /// Build UsageStats from API data, using cache if available.
    fn build_usage_stats(&self) -> Result<UsageStats, String> {
        // Check cache
        if let Ok(cache) = self.usage_cache.lock() {
            if let Some(ref entry) = *cache {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.data.clone());
                }
            }
        }

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(30);
        let starting_at = start.format("%Y-%m-%dT00:00:00Z").to_string();
        let ending_at = now.format("%Y-%m-%dT23:59:59Z").to_string();

        // Fetch usage grouped by model
        let buckets = self.fetch_usage_report(&starting_at, &ending_at, true)?;

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_cache_read: u64 = 0;
        let mut total_cache_write: u64 = 0;
        let mut model_map: HashMap<String, (u64, u64, u64, u64)> = HashMap::new();
        let mut total_messages: u32 = 0;

        for bucket in &buckets {
            for result in &bucket.results {
                let model = result.model.as_deref().unwrap_or("unknown").to_string();
                let cache_write = result.cache_creation.as_ref()
                    .map(|c| c.ephemeral_5m_input_tokens + c.ephemeral_1h_input_tokens)
                    .unwrap_or(0);

                total_input += result.uncached_input_tokens;
                total_output += result.output_tokens;
                total_cache_read += result.cache_read_input_tokens;
                total_cache_write += cache_write;

                let entry = model_map.entry(model).or_insert((0, 0, 0, 0));
                entry.0 += result.uncached_input_tokens;
                entry.1 += result.output_tokens;
                entry.2 += result.cache_read_input_tokens;
                entry.3 += cache_write;

                // Each result row with tokens likely represents at least one request
                if result.output_tokens > 0 || result.uncached_input_tokens > 0 {
                    total_messages += 1;
                }
            }
        }

        // Fetch actual cost
        let total_cost = self.fetch_cost_report(&starting_at, &ending_at).unwrap_or(0.0);

        // Build model breakdown
        let model_breakdown: HashMap<String, ModelUsage> = model_map
            .into_iter()
            .map(|(model, (input, output, cache_read, cache_write))| {
                let mu = ModelUsage {
                    model: model.clone(),
                    input_tokens: input,
                    output_tokens: output,
                    cache_read_tokens: cache_read,
                    cache_write_tokens: cache_write,
                    cost_usd: 0.0, // Individual model cost not available from cost report
                };
                (model, mu)
            })
            .collect();

        let stats = UsageStats {
            provider: "Claude (API)".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: total_cache_read,
            total_cache_write_tokens: total_cache_write,
            total_sessions: 0, // No session concept in API
            total_messages,
            estimated_cost_usd: total_cost,
            model_breakdown,
        };

        // Update cache
        if let Ok(mut cache) = self.usage_cache.lock() {
            *cache = Some(CacheEntry {
                data: stats.clone(),
                fetched_at: Instant::now(),
            });
        }

        Ok(stats)
    }

    /// Build daily usage from API data, using cache if available.
    fn build_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        // Check cache
        if let Ok(cache) = self.daily_cache.lock() {
            if let Some(ref entry) = *cache {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    let mut data = entry.data.clone();
                    data.truncate(days as usize);
                    return Ok(data);
                }
            }
        }

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(days as i64);
        let starting_at = start.format("%Y-%m-%dT00:00:00Z").to_string();
        let ending_at = now.format("%Y-%m-%dT23:59:59Z").to_string();

        let buckets = self.fetch_usage_report(&starting_at, &ending_at, false)?;

        let mut daily: Vec<DailyUsage> = buckets
            .into_iter()
            .map(|bucket| {
                let mut input: u64 = 0;
                let mut output: u64 = 0;
                let mut messages: u32 = 0;

                for result in &bucket.results {
                    input += result.uncached_input_tokens + result.cache_read_input_tokens;
                    let cache_write = result.cache_creation.as_ref()
                        .map(|c| c.ephemeral_5m_input_tokens + c.ephemeral_1h_input_tokens)
                        .unwrap_or(0);
                    input += cache_write;
                    output += result.output_tokens;
                    if result.output_tokens > 0 || result.uncached_input_tokens > 0 {
                        messages += 1;
                    }
                }

                // Extract date from starting_at (RFC 3339)
                let date = bucket.starting_at.split('T').next().unwrap_or("").to_string();

                DailyUsage {
                    date,
                    input_tokens: input,
                    output_tokens: output,
                    sessions: 0,
                    messages,
                }
            })
            .collect();

        // Sort by date descending
        daily.sort_by(|a, b| b.date.cmp(&a.date));

        // Update cache
        if let Ok(mut cache) = self.daily_cache.lock() {
            *cache = Some(CacheEntry {
                data: daily.clone(),
                fetched_at: Instant::now(),
            });
        }

        daily.truncate(days as usize);
        Ok(daily)
    }
}

impl Provider for ClaudeApiProvider {
    fn name(&self) -> &str {
        "Claude (API)"
    }

    fn provider_type(&self) -> &str {
        "claude"
    }

    fn config_dir(&self) -> &PathBuf {
        // API provider doesn't use a config directory; return a dummy path
        // This is safe because no caller reads files from this path for API providers
        static DUMMY: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| PathBuf::from(""));
        &DUMMY
    }

    fn get_usage_stats(&self) -> Result<UsageStats, String> {
        self.build_usage_stats()
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        // API does not have a session concept
        Ok(Vec::new())
    }

    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        self.build_daily_usage(days)
    }

    fn get_session_history(&self, _limit: u32) -> Result<Vec<Session>, String> {
        // API does not have session history
        Ok(Vec::new())
    }
}
