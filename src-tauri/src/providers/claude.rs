use super::{DailyUsage, ModelUsage, Provider, Session, UsageStats};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct ClaudeProvider {
    config_dir: PathBuf,
}

// --- Deserialization types for stats-cache.json ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatsCache {
    #[serde(default)]
    model_usage: HashMap<String, StatsModelUsage>,
    #[serde(default)]
    total_sessions: u32,
    #[serde(default)]
    total_messages: u32,
    #[serde(default)]
    daily_activity: Vec<DailyActivity>,
    #[serde(default)]
    daily_model_tokens: Vec<DailyModelTokens>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatsModelUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default, rename = "costUSD")]
    _cost_usd: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyActivity {
    #[serde(default)]
    date: String,
    #[serde(default)]
    message_count: u32,
    #[serde(default)]
    session_count: u32,
    #[serde(default, rename = "toolCallCount")]
    _tool_call_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyModelTokens {
    #[serde(default)]
    date: String,
    #[serde(default)]
    tokens_by_model: HashMap<String, u64>,
}

// --- Deserialization types for JSONL session lines ---

#[derive(Debug, Deserialize)]
struct SessionLine {
    #[serde(default, rename = "type")]
    line_type: Option<String>,
    #[serde(default)]
    message: Option<SessionMessage>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionMessage {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<SessionUsage>,
}

#[derive(Debug, Deserialize)]
struct SessionUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

impl ClaudeProvider {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn read_stats_cache(&self) -> Option<StatsCache> {
        let path = self.config_dir.join("stats-cache.json");
        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Estimate cost in USD for a given model name and token counts.
    fn estimate_cost(
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
    ) -> f64 {
        let model_lower = model.to_lowercase();

        let (input_rate, output_rate, cache_read_rate, cache_write_rate) =
            if model_lower.contains("opus") {
                // $15 input, $75 output per million tokens
                // Cache read is 90% discount, cache write is 25% premium
                (15.0, 75.0, 1.50, 18.75)
            } else if model_lower.contains("haiku") {
                (0.25, 1.25, 0.025, 0.3125)
            } else {
                // Sonnet and default
                (3.0, 15.0, 0.30, 3.75)
            };

        let cost = (input_tokens as f64 * input_rate
            + output_tokens as f64 * output_rate
            + cache_read_tokens as f64 * cache_read_rate
            + cache_write_tokens as f64 * cache_write_rate)
            / 1_000_000.0;

        (cost * 100.0).round() / 100.0
    }

    /// Scan the projects directory for JSONL session files.
    fn find_session_files(&self) -> Vec<PathBuf> {
        let projects_dir = self.config_dir.join("projects");
        if !projects_dir.exists() {
            return Vec::new();
        }

        let pattern = projects_dir
            .join("**")
            .join("*.jsonl")
            .to_string_lossy()
            .to_string();

        glob::glob(&pattern)
            .map(|paths| paths.filter_map(|p| p.ok()).collect())
            .unwrap_or_default()
    }

    /// Parse a single JSONL session file and return aggregated session info.
    fn parse_session_file(&self, path: &PathBuf) -> Option<Session> {
        let data = fs::read_to_string(path).ok()?;
        if data.trim().is_empty() {
            return None;
        }

        let mut total_tokens: u64 = 0;
        let mut message_count: u32 = 0;
        let mut last_model = String::new();
        let mut last_timestamp = String::new();
        let mut session_id = String::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<SessionLine>(line) {
                if let Some(ref sid) = entry.session_id {
                    if session_id.is_empty() {
                        session_id = sid.clone();
                    }
                }

                if let Some(ref ts) = entry.timestamp {
                    last_timestamp = ts.clone();
                }

                let is_assistant = entry
                    .line_type
                    .as_deref()
                    .map(|t| t == "assistant")
                    .unwrap_or(false);

                if is_assistant {
                    if let Some(ref msg) = entry.message {
                        if let Some(ref model) = msg.model {
                            last_model = model.clone();
                        }
                        if let Some(ref usage) = msg.usage {
                            total_tokens += usage.input_tokens
                                + usage.output_tokens
                                + usage.cache_read_input_tokens
                                + usage.cache_creation_input_tokens;
                            message_count += 1;
                        }
                    }
                }
            }
        }

        if message_count == 0 {
            return None;
        }

        // Derive project name from the file path.
        // Session files live under projects/<encoded-path>/<uuid>.jsonl
        let project = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if session_id.is_empty() {
            session_id = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
        }

        // Check if session is active (file modified in last 30 minutes)
        let is_active = fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::from_secs(u64::MAX))
                    < Duration::from_secs(30 * 60)
            })
            .unwrap_or(false);

        Some(Session {
            id: session_id,
            project,
            model: if last_model.is_empty() {
                "unknown".to_string()
            } else {
                last_model
            },
            tokens_used: total_tokens,
            last_active: last_timestamp,
            is_active,
            message_count,
        })
    }
}

impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude"
    }

    fn provider_type(&self) -> &str {
        "claude"
    }

    fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn get_usage_stats(&self) -> Result<UsageStats, String> {
        let cache = self.read_stats_cache().unwrap_or(StatsCache {
            model_usage: HashMap::new(),
            total_sessions: 0,
            total_messages: 0,
            daily_activity: Vec::new(),
            daily_model_tokens: Vec::new(),
        });

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_cache_read: u64 = 0;
        let mut total_cache_write: u64 = 0;
        let mut total_cost: f64 = 0.0;
        let mut model_breakdown: HashMap<String, ModelUsage> = HashMap::new();

        for (model_name, usage) in &cache.model_usage {
            let cost = Self::estimate_cost(
                model_name,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_read_input_tokens,
                usage.cache_creation_input_tokens,
            );

            total_input += usage.input_tokens;
            total_output += usage.output_tokens;
            total_cache_read += usage.cache_read_input_tokens;
            total_cache_write += usage.cache_creation_input_tokens;
            total_cost += cost;

            model_breakdown.insert(
                model_name.clone(),
                ModelUsage {
                    model: model_name.clone(),
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_read_tokens: usage.cache_read_input_tokens,
                    cache_write_tokens: usage.cache_creation_input_tokens,
                    cost_usd: cost,
                },
            );
        }

        Ok(UsageStats {
            provider: "Claude".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: total_cache_read,
            total_cache_write_tokens: total_cache_write,
            total_sessions: cache.total_sessions,
            total_messages: cache.total_messages,
            estimated_cost_usd: (total_cost * 100.0).round() / 100.0,
            model_breakdown,
        })
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        let files = self.find_session_files();
        let now = SystemTime::now();
        let threshold = Duration::from_secs(30 * 60);

        let mut active_sessions = Vec::new();

        for file in files {
            // Quick check: only parse files modified recently
            let is_recent = fs::metadata(&file)
                .and_then(|m| m.modified())
                .map(|modified| {
                    now.duration_since(modified)
                        .unwrap_or(Duration::from_secs(u64::MAX))
                        < threshold
                })
                .unwrap_or(false);

            if !is_recent {
                continue;
            }

            if let Some(session) = self.parse_session_file(&file) {
                active_sessions.push(session);
            }
        }

        Ok(active_sessions)
    }

    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        let cache = match self.read_stats_cache() {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        // Build a map of date -> token totals from daily_model_tokens
        let mut token_map: HashMap<String, (u64, u64)> = HashMap::new();
        for entry in &cache.daily_model_tokens {
            let total: u64 = entry.tokens_by_model.values().sum();
            // We only have aggregate tokens per model per day, so approximate
            // a 30/70 input/output split
            let input_est = total * 30 / 100;
            let output_est = total - input_est;
            let e = token_map.entry(entry.date.clone()).or_insert((0, 0));
            e.0 += input_est;
            e.1 += output_est;
        }

        // Build a map of date -> activity
        let mut activity_map: HashMap<String, (u32, u32)> = HashMap::new();
        for entry in &cache.daily_activity {
            activity_map.insert(entry.date.clone(), (entry.session_count, entry.message_count));
        }

        // Merge into DailyUsage, limited to the last N days
        let mut all_dates: Vec<String> = token_map
            .keys()
            .chain(activity_map.keys())
            .cloned()
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        all_dates.sort();
        all_dates.reverse();
        all_dates.truncate(days as usize);

        let daily: Vec<DailyUsage> = all_dates
            .into_iter()
            .map(|date| {
                let (input, output) = token_map.get(&date).copied().unwrap_or((0, 0));
                let (sessions, messages) = activity_map.get(&date).copied().unwrap_or((0, 0));
                DailyUsage {
                    date,
                    input_tokens: input,
                    output_tokens: output,
                    sessions,
                    messages,
                }
            })
            .collect();

        Ok(daily)
    }

    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String> {
        let files = self.find_session_files();

        // Collect (modified_time, path) so we can sort by recency
        let mut timed_files: Vec<(SystemTime, PathBuf)> = files
            .into_iter()
            .filter_map(|p| {
                let modified = fs::metadata(&p).ok()?.modified().ok()?;
                Some((modified, p))
            })
            .collect();

        // Sort by modified time, newest first
        timed_files.sort_by(|a, b| b.0.cmp(&a.0));
        timed_files.truncate(limit as usize);

        let sessions: Vec<Session> = timed_files
            .iter()
            .filter_map(|(_, path)| self.parse_session_file(path))
            .collect();

        Ok(sessions)
    }
}
