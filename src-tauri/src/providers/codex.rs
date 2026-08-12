//! Codex CLI provider.
//!
//! Codex writes one rollout file per session under `~/.codex/sessions/<Y>/<M>/<D>/`.
//! Every turn appends a `token_count` event carrying the cumulative token usage for
//! the session and, when the account is on a ChatGPT plan, the current rate limit
//! windows. Those files are append-only, so parsed results are cached by
//! (path, mtime, size) and each historical file is read at most once per run.

use super::{DailyUsage, ModelUsage, Provider, RateLimitStatus, RateLimitWindow, Session, UsageStats};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

/// A session file untouched for this long is considered finished.
const ACTIVE_WINDOW: Duration = Duration::from_secs(30 * 60);

/// ChatGPT's private Codex usage endpoint. It reports the same windows Codex
/// itself shows, computed server-side, so it stays current between sessions.
const USAGE_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";

pub struct CodexProvider {
    config_dir: PathBuf,
}

/// Parsed rollout files, shared by every `CodexProvider` instance.
///
/// The tray polls limits every few seconds and the alert engine builds a fresh
/// provider on each pass, so a per-instance cache would re-read every session
/// file each time. Keyed by path, so separate profiles never collide.
fn cache() -> &'static Mutex<HashMap<PathBuf, CachedFile>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedFile>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// --- Rollout JSONL records ---

#[derive(Debug, Deserialize)]
struct RolloutLine {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default, rename = "type")]
    line_type: Option<String>,
    #[serde(default)]
    payload: Option<RolloutPayload>,
}

/// Only the fields cldbar reads; every other key in the payload is ignored.
#[derive(Debug, Deserialize)]
struct RolloutPayload {
    #[serde(default, rename = "type")]
    payload_type: Option<String>,
    // session_meta
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    // turn_context
    #[serde(default)]
    model: Option<String>,
    // token_count
    #[serde(default)]
    info: Option<TokenInfo>,
    #[serde(default)]
    rate_limits: Option<CodexRateLimits>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenInfo {
    #[serde(default)]
    total_token_usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct TokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

impl TokenUsage {
    /// Input tokens that were not served from cache, so input and cache never double count.
    fn fresh_input(&self) -> u64 {
        self.input_tokens.saturating_sub(self.cached_input_tokens)
    }
}

#[derive(Debug, Deserialize, Clone)]
struct CodexRateLimits {
    #[serde(default)]
    primary: Option<CodexRateLimitWindow>,
    #[serde(default)]
    secondary: Option<CodexRateLimitWindow>,
}

#[derive(Debug, Deserialize, Clone)]
struct CodexRateLimitWindow {
    #[serde(default)]
    used_percent: f64,
    #[serde(default)]
    window_minutes: u64,
    /// Unix timestamp, in seconds.
    #[serde(default)]
    resets_at: Option<i64>,
}

// --- auth.json / usage endpoint ---

#[derive(Debug, Deserialize)]
struct CodexAuth {
    #[serde(default)]
    tokens: Option<CodexTokens>,
}

#[derive(Debug, Deserialize)]
struct CodexTokens {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[serde(default)]
    rate_limit: Option<UsageRateLimit>,
}

#[derive(Debug, Deserialize)]
struct UsageRateLimit {
    #[serde(default)]
    primary_window: Option<UsageWindow>,
    #[serde(default)]
    secondary_window: Option<UsageWindow>,
}

#[derive(Debug, Deserialize)]
struct UsageWindow {
    #[serde(default)]
    used_percent: f64,
    #[serde(default)]
    limit_window_seconds: u64,
    /// Unix timestamp, in seconds.
    #[serde(default)]
    reset_at: Option<i64>,
}

// --- Per-file scan result ---

#[derive(Debug, Clone)]
struct FileSummary {
    session_id: String,
    project: String,
    model: String,
    /// Cumulative usage reported by the last `token_count` event.
    totals: TokenUsage,
    message_count: u32,
    /// ISO timestamp of the last record in the file.
    last_activity: String,
    /// Date (YYYY-MM-DD) the session started, for the per-day session count.
    start_date: Option<String>,
    /// date -> (fresh input, cache read, output) attributed to that day.
    daily: HashMap<String, (u64, u64, u64)>,
    /// date -> assistant messages on that day.
    daily_messages: HashMap<String, u32>,
    /// Most recent rate limit snapshot in the file, with the timestamp it was taken.
    rate_limits: Option<(String, CodexRateLimits)>,
}

#[derive(Debug, Clone)]
struct CachedFile {
    modified: SystemTime,
    len: u64,
    summary: FileSummary,
}

fn date_of(timestamp: &str) -> Option<String> {
    if timestamp.len() >= 10 {
        Some(timestamp[..10].to_string())
    } else {
        None
    }
}

/// "5-Hour" / "7-Day" from a window length in minutes.
fn window_label(minutes: u64) -> String {
    if minutes >= 1440 && minutes % 1440 == 0 {
        format!("{}-Day", minutes / 1440)
    } else if minutes >= 60 && minutes % 60 == 0 {
        format!("{}-Hour", minutes / 60)
    } else {
        format!("{}-Min", minutes)
    }
}

fn epoch_to_rfc3339(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339())
}

/// Sort reported windows into the shared five-hour / seven-day slots.
///
/// Codex labels its windows primary/secondary and the meaning shifts between
/// releases, so slot them by length instead: under a day is the session
/// window, longer is the weekly one.
fn slot_windows(
    mut windows: Vec<CodexRateLimitWindow>,
) -> (Option<RateLimitWindow>, Option<RateLimitWindow>) {
    windows.sort_by_key(|w| w.window_minutes);

    let mut five_hour = None;
    let mut seven_day = None;

    for w in &windows {
        let converted = RateLimitWindow {
            label: window_label(w.window_minutes),
            utilization: w.used_percent,
            resets_at: w.resets_at.and_then(epoch_to_rfc3339),
        };
        if w.window_minutes < 1440 && five_hour.is_none() {
            five_hour = Some(converted);
        } else if seven_day.is_none() {
            seven_day = Some(converted);
        }
    }

    (five_hour, seven_day)
}

impl CodexProvider {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn find_session_files(&self) -> Vec<PathBuf> {
        let sessions_dir = self.config_dir.join("sessions");
        if !sessions_dir.exists() {
            return Vec::new();
        }

        let pattern = sessions_dir
            .join("**")
            .join("*.jsonl")
            .to_string_lossy()
            .to_string();

        glob::glob(&pattern)
            .map(|paths| paths.filter_map(|p| p.ok()).collect())
            .unwrap_or_default()
    }

    /// Parse one rollout file. Lines are pre-filtered by substring so the bulk of a
    /// session (assistant output, tool results) is never handed to serde.
    fn parse_session_file(path: &PathBuf) -> Option<FileSummary> {
        let data = fs::read_to_string(path).ok()?;

        let mut session_id = String::new();
        let mut project = String::new();
        let mut model = String::new();
        let mut totals = TokenUsage::default();
        let mut prev = TokenUsage::default();
        let mut message_count: u32 = 0;
        let mut last_activity = String::new();
        let mut start_date: Option<String> = None;
        let mut daily: HashMap<String, (u64, u64, u64)> = HashMap::new();
        let mut daily_messages: HashMap<String, u32> = HashMap::new();
        let mut rate_limits: Option<(String, CodexRateLimits)> = None;

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let is_meta = line.contains("\"session_meta\"");
            let is_turn = line.contains("\"turn_context\"");
            let is_tokens = line.contains("\"token_count\"");
            let is_agent_msg = line.contains("\"agent_message\"");
            if !(is_meta || is_turn || is_tokens || is_agent_msg) {
                continue;
            }

            let Ok(entry) = serde_json::from_str::<RolloutLine>(line) else {
                continue;
            };
            let timestamp = entry.timestamp.clone().unwrap_or_default();
            if !timestamp.is_empty() {
                last_activity = timestamp.clone();
            }

            let Some(payload) = entry.payload else {
                continue;
            };

            match entry.line_type.as_deref() {
                Some("session_meta") => {
                    if let Some(id) = payload.id {
                        session_id = id;
                    }
                    if let Some(cwd) = payload.cwd {
                        project = PathBuf::from(&cwd)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or(cwd);
                    }
                    start_date = date_of(&timestamp);
                }
                Some("turn_context") => {
                    if let Some(m) = payload.model {
                        model = m;
                    }
                }
                Some("event_msg") => match payload.payload_type.as_deref() {
                    Some("agent_message") => {
                        message_count += 1;
                        if let Some(date) = date_of(&timestamp) {
                            *daily_messages.entry(date).or_insert(0) += 1;
                        }
                    }
                    Some("token_count") => {
                        if let Some(limits) = payload.rate_limits {
                            let newer = rate_limits
                                .as_ref()
                                .map(|(ts, _)| timestamp.as_str() > ts.as_str())
                                .unwrap_or(true);
                            if newer {
                                rate_limits = Some((timestamp.clone(), limits));
                            }
                        }

                        let Some(usage) = payload.info.and_then(|i| i.total_token_usage) else {
                            continue;
                        };

                        // token_count repeats within a turn, so attribute the growth of the
                        // cumulative counter rather than the per-request figure.
                        if let Some(date) = date_of(&timestamp) {
                            let e = daily.entry(date).or_insert((0, 0, 0));
                            e.0 += usage.fresh_input().saturating_sub(prev.fresh_input());
                            e.1 += usage
                                .cached_input_tokens
                                .saturating_sub(prev.cached_input_tokens);
                            e.2 += usage.output_tokens.saturating_sub(prev.output_tokens);
                        }

                        prev = usage.clone();
                        totals = usage;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if session_id.is_empty() {
            session_id = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
        }

        Some(FileSummary {
            session_id,
            project: if project.is_empty() {
                "unknown".to_string()
            } else {
                project
            },
            model: if model.is_empty() {
                "unknown".to_string()
            } else {
                model
            },
            totals,
            message_count,
            last_activity,
            start_date,
            daily,
            daily_messages,
            rate_limits,
        })
    }

    /// Summaries for every session file, reusing cached results for untouched files.
    fn summaries(&self) -> Vec<(PathBuf, SystemTime, FileSummary)> {
        let files = self.find_session_files();
        let mut out = Vec::with_capacity(files.len());
        let mut cache = cache().lock().ok();

        for path in files {
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let len = meta.len();

            let cached = cache.as_ref().and_then(|c| c.get(&path)).and_then(|c| {
                if c.modified == modified && c.len == len {
                    Some(c.summary.clone())
                } else {
                    None
                }
            });

            let summary = match cached {
                Some(s) => s,
                None => {
                    let Some(s) = Self::parse_session_file(&path) else {
                        continue;
                    };
                    if let Some(c) = cache.as_mut() {
                        c.insert(
                            path.clone(),
                            CachedFile {
                                modified,
                                len,
                                summary: s.clone(),
                            },
                        );
                    }
                    s
                }
            };

            out.push((path, modified, summary));
        }

        // Forget files that no longer exist, leaving other profiles' entries alone
        // since the cache is shared process-wide.
        if let Some(c) = cache.as_mut() {
            let sessions_dir = self.config_dir.join("sessions");
            let live: std::collections::HashSet<&PathBuf> = out.iter().map(|(p, _, _)| p).collect();
            c.retain(|path, _| !path.starts_with(&sessions_dir) || live.contains(path));
        }

        out
    }

    /// Read the ChatGPT OAuth token Codex stores after signing in.
    fn read_auth(&self) -> Option<(String, String)> {
        let data = fs::read_to_string(self.config_dir.join("auth.json")).ok()?;
        let auth: CodexAuth = serde_json::from_str(&data).ok()?;
        let tokens = auth.tokens?;
        Some((tokens.access_token?, tokens.account_id.unwrap_or_default()))
    }

    /// Current limits straight from the usage endpoint, so the numbers stay
    /// live even when Codex has not run for days.
    ///
    /// Returns `None` when the token is missing or expired, the machine is
    /// offline, or the response carries no windows, letting the caller fall
    /// back to the last snapshot in the rollout logs. Callers reach this
    /// through `alerts::rate_limit_for_profile`, which does the rate limiting.
    fn fetch_live_rate_limits(&self) -> Option<RateLimitStatus> {
        let (token, account_id) = self.read_auth()?;

        let resp = super::http_client()
            .get(USAGE_ENDPOINT)
            .header("Authorization", format!("Bearer {}", token))
            .header("chatgpt-account-id", account_id)
            .send()
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let usage: UsageResponse = resp.json().ok()?;
        let rate_limit = usage.rate_limit?;

        let windows: Vec<CodexRateLimitWindow> =
            [rate_limit.primary_window, rate_limit.secondary_window]
                .into_iter()
                .flatten()
                .map(|w| CodexRateLimitWindow {
                    used_percent: w.used_percent,
                    window_minutes: w.limit_window_seconds / 60,
                    resets_at: w.reset_at,
                })
                .collect();

        let (five_hour, seven_day) = slot_windows(windows);
        if five_hour.is_none() && seven_day.is_none() {
            return None;
        }

        Some(RateLimitStatus {
            available: true,
            five_hour,
            seven_day,
            seven_day_opus: None,
            // Fetched live, so there is no staleness to report.
            updated_at: None,
        })
    }

    /// Live limits when the account can be reached, otherwise the newest
    /// snapshot recorded in the rollout logs, tagged with when it was taken.
    pub fn get_rate_limit_status(&self) -> RateLimitStatus {
        if let Some(live) = self.fetch_live_rate_limits() {
            return live;
        }

        let newest = self
            .summaries()
            .into_iter()
            .filter_map(|(_, _, s)| s.rate_limits)
            .max_by(|(a, _), (b, _)| a.cmp(b));

        let Some((observed_at, limits)) = newest else {
            return RateLimitStatus::unavailable();
        };

        let windows: Vec<CodexRateLimitWindow> =
            [limits.primary, limits.secondary].into_iter().flatten().collect();

        let (five_hour, seven_day) = slot_windows(windows);
        if five_hour.is_none() && seven_day.is_none() {
            return RateLimitStatus::unavailable();
        }

        RateLimitStatus {
            available: true,
            five_hour,
            seven_day,
            seven_day_opus: None,
            updated_at: Some(observed_at),
        }
    }
}

impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "Codex"
    }

    fn provider_type(&self) -> &str {
        "codex"
    }

    fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn get_usage_stats(&self) -> Result<UsageStats, String> {
        let summaries = self.summaries();

        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_messages = 0u32;
        let mut model_breakdown: HashMap<String, ModelUsage> = HashMap::new();

        for (_, _, s) in &summaries {
            let fresh = s.totals.fresh_input();
            total_input += fresh;
            total_cache_read += s.totals.cached_input_tokens;
            total_output += s.totals.output_tokens;
            total_messages += s.message_count;

            let entry = model_breakdown
                .entry(s.model.clone())
                .or_insert_with(|| ModelUsage {
                    model: s.model.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_usd: 0.0,
                });
            entry.input_tokens += fresh;
            entry.output_tokens += s.totals.output_tokens;
            entry.cache_read_tokens += s.totals.cached_input_tokens;
        }

        Ok(UsageStats {
            provider: "Codex".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: total_cache_read,
            // Codex does not report cache writes separately.
            total_cache_write_tokens: 0,
            total_sessions: summaries.len() as u32,
            total_messages,
            // Plan-based usage has no per-token price to report.
            estimated_cost_usd: 0.0,
            model_breakdown,
        })
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        let now = SystemTime::now();

        let sessions = self
            .summaries()
            .into_iter()
            .filter(|(_, modified, _)| {
                now.duration_since(*modified)
                    .unwrap_or(Duration::from_secs(u64::MAX))
                    < ACTIVE_WINDOW
            })
            .map(|(_, _, s)| Session {
                id: s.session_id.clone(),
                project: s.project.clone(),
                model: s.model.clone(),
                tokens_used: s.totals.fresh_input()
                    + s.totals.cached_input_tokens
                    + s.totals.output_tokens,
                last_active: s.last_activity.clone(),
                is_active: true,
                message_count: s.message_count,
            })
            .collect();

        Ok(sessions)
    }

    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        let summaries = self.summaries();

        let mut tokens: HashMap<String, (u64, u64)> = HashMap::new();
        let mut messages: HashMap<String, u32> = HashMap::new();
        let mut sessions: HashMap<String, u32> = HashMap::new();

        for (_, _, s) in &summaries {
            for (date, (fresh, cached, output)) in &s.daily {
                let e = tokens.entry(date.clone()).or_insert((0, 0));
                e.0 += fresh + cached;
                e.1 += output;
            }
            for (date, count) in &s.daily_messages {
                *messages.entry(date.clone()).or_insert(0) += count;
            }
            if let Some(date) = &s.start_date {
                *sessions.entry(date.clone()).or_insert(0) += 1;
            }
        }

        let mut all_dates: Vec<String> = tokens
            .keys()
            .chain(messages.keys())
            .chain(sessions.keys())
            .cloned()
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        all_dates.sort();
        all_dates.reverse();
        all_dates.truncate(days as usize);

        Ok(all_dates
            .into_iter()
            .map(|date| {
                let (input, output) = tokens.get(&date).copied().unwrap_or((0, 0));
                DailyUsage {
                    date: date.clone(),
                    input_tokens: input,
                    output_tokens: output,
                    sessions: sessions.get(&date).copied().unwrap_or(0),
                    messages: messages.get(&date).copied().unwrap_or(0),
                }
            })
            .collect())
    }

    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String> {
        let now = SystemTime::now();
        let mut summaries = self.summaries();
        summaries.sort_by(|a, b| b.1.cmp(&a.1));
        summaries.truncate(limit as usize);

        Ok(summaries
            .into_iter()
            .map(|(_, modified, s)| Session {
                id: s.session_id.clone(),
                project: s.project.clone(),
                model: s.model.clone(),
                tokens_used: s.totals.fresh_input()
                    + s.totals.cached_input_tokens
                    + s.totals.output_tokens,
                last_active: s.last_activity.clone(),
                is_active: now
                    .duration_since(modified)
                    .unwrap_or(Duration::from_secs(u64::MAX))
                    < ACTIVE_WINDOW,
                message_count: s.message_count,
            })
            .collect())
    }
}
