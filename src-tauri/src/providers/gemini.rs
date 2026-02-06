use super::{DailyUsage, ModelUsage, Provider, Session, UsageStats};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct GeminiProvider {
    config_dir: PathBuf,
}

// --- Deserialization types for Gemini session JSONL ---

#[derive(Debug, Deserialize)]
struct GeminiSessionLine {
    #[serde(default, rename = "type")]
    line_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    tokens: Option<GeminiTokens>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiTokens {
    #[serde(default)]
    input: u64,
    #[serde(default)]
    output: u64,
}

// --- Deserialization types for legacy session JSON ---

#[derive(Debug, Deserialize)]
struct GeminiLegacySession {
    #[serde(default)]
    messages: Vec<GeminiLegacyMessage>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default, rename = "createdAt")]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiLegacyMessage {
    #[serde(default)]
    tokens: Option<GeminiTokens>,
}

impl GeminiProvider {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Determine the Gemini config directory.
    /// Uses GEMINI_CLI_HOME env var if set, otherwise the provided config_dir.
    fn effective_dir(&self) -> PathBuf {
        std::env::var("GEMINI_CLI_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.config_dir.clone())
    }

    /// Find all session JSONL files under tmp/<hash>/chats/
    fn find_session_jsonl_files(&self) -> Vec<PathBuf> {
        let base = self.effective_dir().join("tmp");
        if !base.exists() {
            return Vec::new();
        }

        let pattern = base
            .join("*")
            .join("chats")
            .join("session-*.jsonl")
            .to_string_lossy()
            .to_string();

        glob::glob(&pattern)
            .map(|paths| paths.filter_map(|p| p.ok()).collect())
            .unwrap_or_default()
    }

    /// Find legacy session JSON files under tmp/<hash>/chats/
    fn find_legacy_session_files(&self) -> Vec<PathBuf> {
        let base = self.effective_dir().join("tmp");
        if !base.exists() {
            return Vec::new();
        }

        let pattern = base
            .join("*")
            .join("chats")
            .join("session-*.json")
            .to_string_lossy()
            .to_string();

        glob::glob(&pattern)
            .map(|paths| {
                paths
                    .filter_map(|p| p.ok())
                    // Exclude .jsonl files matched by accident
                    .filter(|p| {
                        p.extension()
                            .map(|e| e == "json")
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse a JSONL session file.
    fn parse_jsonl_session(&self, path: &PathBuf) -> Option<Session> {
        let data = fs::read_to_string(path).ok()?;
        if data.trim().is_empty() {
            return None;
        }

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut message_count: u32 = 0;
        let mut last_model = String::new();
        let mut last_timestamp = String::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<GeminiSessionLine>(line) {
                if let Some(ref ts) = entry.timestamp {
                    last_timestamp = ts.clone();
                }

                if let Some(ref model) = entry.model {
                    last_model = model.clone();
                }

                if let Some(ref tokens) = entry.tokens {
                    total_input += tokens.input;
                    total_output += tokens.output;
                    message_count += 1;
                }
            }
        }

        if message_count == 0 {
            return None;
        }

        let session_id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Derive project from parent directory hash
        let project = path
            .parent()
            .and_then(|chats| chats.parent())
            .and_then(|hash_dir| hash_dir.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

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
                "gemini-unknown".to_string()
            } else {
                last_model
            },
            tokens_used: total_input + total_output,
            last_active: last_timestamp,
            is_active,
            message_count,
        })
    }

    /// Parse a legacy JSON session file.
    fn parse_legacy_session(&self, path: &PathBuf) -> Option<Session> {
        let data = fs::read_to_string(path).ok()?;
        let session: GeminiLegacySession = serde_json::from_str(&data).ok()?;

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let message_count = session.messages.len() as u32;

        for msg in &session.messages {
            if let Some(ref tokens) = msg.tokens {
                total_input += tokens.input;
                total_output += tokens.output;
            }
        }

        if message_count == 0 {
            return None;
        }

        let session_id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let project = path
            .parent()
            .and_then(|chats| chats.parent())
            .and_then(|hash_dir| hash_dir.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let is_active = fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::from_secs(u64::MAX))
                    < Duration::from_secs(30 * 60)
            })
            .unwrap_or(false);

        let last_active = session
            .created_at
            .clone()
            .unwrap_or_default();

        Some(Session {
            id: session_id,
            project,
            model: session.model.unwrap_or_else(|| "gemini-unknown".to_string()),
            tokens_used: total_input + total_output,
            last_active,
            is_active,
            message_count,
        })
    }

    /// Collect all sessions from both JSONL and legacy JSON formats.
    fn all_sessions(&self) -> Vec<Session> {
        let mut sessions = Vec::new();

        for path in self.find_session_jsonl_files() {
            if let Some(s) = self.parse_jsonl_session(&path) {
                sessions.push(s);
            }
        }

        for path in self.find_legacy_session_files() {
            if let Some(s) = self.parse_legacy_session(&path) {
                sessions.push(s);
            }
        }

        sessions
    }

    /// Estimate cost for Gemini models (per million tokens).
    fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        let model_lower = model.to_lowercase();

        let (input_rate, output_rate) = if model_lower.contains("flash") {
            (0.15, 0.60)
        } else {
            // gemini-2.5-pro and default
            (1.25, 10.0)
        };

        let cost =
            (input_tokens as f64 * input_rate + output_tokens as f64 * output_rate) / 1_000_000.0;

        (cost * 100.0).round() / 100.0
    }
}

impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "Gemini"
    }

    fn provider_type(&self) -> &str {
        "gemini"
    }

    fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn get_usage_stats(&self) -> Result<UsageStats, String> {
        let sessions = self.all_sessions();

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_messages: u32 = 0;
        let mut model_map: HashMap<String, (u64, u64, u32)> = HashMap::new();

        for session in &sessions {
            // Approximate input/output split: 40/60 of total tokens_used
            let input_est = session.tokens_used * 40 / 100;
            let output_est = session.tokens_used - input_est;

            total_input += input_est;
            total_output += output_est;
            total_messages += session.message_count;

            let entry = model_map
                .entry(session.model.clone())
                .or_insert((0, 0, 0));
            entry.0 += input_est;
            entry.1 += output_est;
            entry.2 += session.message_count;
        }

        let mut model_breakdown: HashMap<String, ModelUsage> = HashMap::new();
        let mut total_cost: f64 = 0.0;

        for (model_name, (input, output, _count)) in &model_map {
            let cost = Self::estimate_cost(model_name, *input, *output);
            total_cost += cost;
            model_breakdown.insert(
                model_name.clone(),
                ModelUsage {
                    model: model_name.clone(),
                    input_tokens: *input,
                    output_tokens: *output,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_usd: cost,
                },
            );
        }

        Ok(UsageStats {
            provider: "Gemini".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: 0,
            total_cache_write_tokens: 0,
            total_sessions: sessions.len() as u32,
            total_messages,
            estimated_cost_usd: (total_cost * 100.0).round() / 100.0,
            model_breakdown,
        })
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        let sessions = self.all_sessions();
        Ok(sessions.into_iter().filter(|s| s.is_active).collect())
    }

    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        let sessions = self.all_sessions();

        // Group sessions by date (from last_active timestamp)
        let mut date_map: HashMap<String, (u64, u64, u32, u32)> = HashMap::new();

        for session in &sessions {
            // Extract date portion from ISO 8601 timestamp
            let date = if session.last_active.len() >= 10 {
                session.last_active[..10].to_string()
            } else {
                continue;
            };

            let input_est = session.tokens_used * 40 / 100;
            let output_est = session.tokens_used - input_est;

            let entry = date_map.entry(date).or_insert((0, 0, 0, 0));
            entry.0 += input_est;
            entry.1 += output_est;
            entry.2 += 1;
            entry.3 += session.message_count;
        }

        let mut daily: Vec<DailyUsage> = date_map
            .into_iter()
            .map(|(date, (input, output, sessions, messages))| DailyUsage {
                date,
                input_tokens: input,
                output_tokens: output,
                sessions,
                messages,
            })
            .collect();

        daily.sort_by(|a, b| b.date.cmp(&a.date));
        daily.truncate(days as usize);

        Ok(daily)
    }

    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String> {
        let mut sessions = self.all_sessions();

        // Sort by last_active descending
        sessions.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        sessions.truncate(limit as usize);

        Ok(sessions)
    }
}
