use super::{DailyUsage, ModelUsage, Provider, Session, UsageStats};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ZaiProvider {
    config_dir: PathBuf,
}

impl ZaiProvider {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Determine the database path.
    /// Uses ZAI_CONFIG_PATH env var if set, otherwise config_dir/sessions.db.
    fn db_path(&self) -> PathBuf {
        std::env::var("ZAI_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.config_dir.join("sessions.db"))
    }

    /// Open a read-only connection to the SQLite database.
    /// Returns None if the database file does not exist.
    fn open_db(&self) -> Option<rusqlite::Connection> {
        let path = self.db_path();
        if !path.exists() {
            return None;
        }
        rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .ok()
    }

    /// Estimate cost for z.ai / GLM models (per million tokens).
    fn estimate_cost(_model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_rate = 1.0;
        let output_rate = 4.0;

        let cost =
            (input_tokens as f64 * input_rate + output_tokens as f64 * output_rate) / 1_000_000.0;

        (cost * 100.0).round() / 100.0
    }
}

impl Provider for ZaiProvider {
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
        let conn = match self.open_db() {
            Some(c) => c,
            None => {
                return Ok(UsageStats {
                    provider: "z.ai".to_string(),
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_cache_read_tokens: 0,
                    total_cache_write_tokens: 0,
                    total_sessions: 0,
                    total_messages: 0,
                    estimated_cost_usd: 0.0,
                    model_breakdown: HashMap::new(),
                });
            }
        };

        // Count total sessions
        let total_sessions: u32 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap_or(0);

        // Aggregate token usage from messages table
        // Expected columns: input_tokens, output_tokens, model
        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(model, 'unknown'), \
                 COALESCE(SUM(input_tokens), 0), \
                 COALESCE(SUM(output_tokens), 0), \
                 COUNT(*) \
                 FROM messages \
                 GROUP BY COALESCE(model, 'unknown')",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_messages: u32 = 0;
        let mut total_cost: f64 = 0.0;
        let mut model_breakdown: HashMap<String, ModelUsage> = HashMap::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, u32>(3)?,
                ))
            })
            .map_err(|e| format!("Failed to query messages: {}", e))?;

        for row in rows {
            if let Ok((model, input, output, count)) = row {
                let cost = Self::estimate_cost(&model, input, output);

                total_input += input;
                total_output += output;
                total_messages += count;
                total_cost += cost;

                model_breakdown.insert(
                    model.clone(),
                    ModelUsage {
                        model,
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                        cost_usd: cost,
                    },
                );
            }
        }

        Ok(UsageStats {
            provider: "z.ai".to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_read_tokens: 0,
            total_cache_write_tokens: 0,
            total_sessions,
            total_messages,
            estimated_cost_usd: (total_cost * 100.0).round() / 100.0,
            model_breakdown,
        })
    }

    fn get_active_sessions(&self) -> Result<Vec<Session>, String> {
        let conn = match self.open_db() {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        // Active sessions: updated in the last 30 minutes
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.name, s.working_directory, \
                 COALESCE(s.updated_at, s.created_at, '') as last_active, \
                 COALESCE(m.model, 'unknown') as model, \
                 COALESCE(m.total_tokens, 0) as tokens_used, \
                 COALESCE(m.msg_count, 0) as msg_count \
                 FROM sessions s \
                 LEFT JOIN ( \
                     SELECT session_id, \
                            MAX(COALESCE(model, 'unknown')) as model, \
                            SUM(COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)) as total_tokens, \
                            COUNT(*) as msg_count \
                     FROM messages GROUP BY session_id \
                 ) m ON s.id = m.session_id \
                 WHERE s.updated_at >= datetime('now', '-30 minutes') \
                 ORDER BY last_active DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get::<_, String>(0)?,
                    project: row.get::<_, String>(2).unwrap_or_default(),
                    model: row.get::<_, String>(4).unwrap_or_else(|_| "unknown".to_string()),
                    tokens_used: row.get::<_, u64>(5).unwrap_or(0),
                    last_active: row.get::<_, String>(3).unwrap_or_default(),
                    is_active: true,
                    message_count: row.get::<_, u32>(6).unwrap_or(0),
                })
            })
            .map_err(|e| format!("Failed to query sessions: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        let conn = match self.open_db() {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        let mut stmt = conn
            .prepare(
                "SELECT DATE(m.created_at) as date, \
                 COALESCE(SUM(m.input_tokens), 0), \
                 COALESCE(SUM(m.output_tokens), 0), \
                 COUNT(DISTINCT m.session_id), \
                 COUNT(*) \
                 FROM messages m \
                 WHERE m.created_at >= datetime('now', ?1) \
                 GROUP BY DATE(m.created_at) \
                 ORDER BY date DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let offset = format!("-{} days", days);

        let daily = stmt
            .query_map([&offset], |row| {
                Ok(DailyUsage {
                    date: row.get::<_, String>(0)?,
                    input_tokens: row.get::<_, u64>(1)?,
                    output_tokens: row.get::<_, u64>(2)?,
                    sessions: row.get::<_, u32>(3)?,
                    messages: row.get::<_, u32>(4)?,
                })
            })
            .map_err(|e| format!("Failed to query daily usage: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(daily)
    }

    fn get_session_history(&self, limit: u32) -> Result<Vec<Session>, String> {
        let conn = match self.open_db() {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.name, s.working_directory, \
                 COALESCE(s.updated_at, s.created_at, '') as last_active, \
                 COALESCE(m.model, 'unknown') as model, \
                 COALESCE(m.total_tokens, 0) as tokens_used, \
                 COALESCE(m.msg_count, 0) as msg_count \
                 FROM sessions s \
                 LEFT JOIN ( \
                     SELECT session_id, \
                            MAX(COALESCE(model, 'unknown')) as model, \
                            SUM(COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)) as total_tokens, \
                            COUNT(*) as msg_count \
                     FROM messages GROUP BY session_id \
                 ) m ON s.id = m.session_id \
                 ORDER BY last_active DESC \
                 LIMIT ?1",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let now_str = chrono::Utc::now().to_rfc3339();
        let thirty_min_ago = (chrono::Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();

        let sessions = stmt
            .query_map([limit], |row| {
                let last_active: String = row.get::<_, String>(3).unwrap_or_default();
                let is_active = last_active.as_str() >= thirty_min_ago.as_str()
                    && last_active.as_str() <= now_str.as_str();

                Ok(Session {
                    id: row.get::<_, String>(0)?,
                    project: row.get::<_, String>(2).unwrap_or_default(),
                    model: row.get::<_, String>(4).unwrap_or_else(|_| "unknown".to_string()),
                    tokens_used: row.get::<_, u64>(5).unwrap_or(0),
                    last_active,
                    is_active,
                    message_count: row.get::<_, u32>(6).unwrap_or(0),
                })
            })
            .map_err(|e| format!("Failed to query session history: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }
}
