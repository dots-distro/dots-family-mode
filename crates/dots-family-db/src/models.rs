use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbProfile {
    pub id: String,
    pub name: String,
    pub age_group: String,
    pub birthday: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: String,
    pub active: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbSession {
    pub id: String,
    pub profile_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub end_reason: Option<String>,
    pub duration_seconds: Option<i64>,
    pub screen_time_seconds: Option<i64>,
    pub active_time_seconds: Option<i64>,
    pub idle_time_seconds: Option<i64>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbActivity {
    pub id: i64,
    pub session_id: String,
    pub profile_id: String,
    pub timestamp: DateTime<Utc>,
    pub app_id: String,
    pub app_name: String,
    pub category: Option<String>,
    pub window_title: Option<String>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbEvent {
    pub id: i64,
    pub profile_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub severity: String,
    pub details: Option<String>,
    pub metadata: Option<String>,
}

/// Terminal command filtering policy
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTerminalPolicy {
    pub id: String,
    pub profile_id: String,
    pub command_pattern: String,
    pub action: String,     // "block", "warn", "allow"
    pub risk_level: String, // "low", "medium", "high", "critical"
    pub educational_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

/// Terminal command execution log
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTerminalCommand {
    pub id: i64,
    pub session_id: String,
    pub profile_id: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub shell: String, // "bash", "zsh", "fish"
    pub working_directory: String,
    pub risk_level: String,
    pub action_taken: String, // "allowed", "blocked", "warned"
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub script_path: Option<String>, // If command was a script
}

/// Script analysis cache and results
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbScriptAnalysis {
    pub id: String,
    pub script_path: String,
    pub content_hash: String, // MD5 hash of script content
    pub risk_level: String,
    pub dangerous_patterns: String, // JSON array of detected patterns
    pub analysis_result: String,    // JSON serialized analysis result
    pub analyzed_at: DateTime<Utc>,
    pub file_size: i64,
    pub line_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuditLog {
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub success: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAuditLog {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub success: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbException {
    pub id: String,
    pub profile_id: String,
    pub exception_type: String,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub amount_minutes: Option<i64>,
    pub app_id: Option<String>,
    pub website: Option<String>,
    pub scope: Option<String>,
    pub active: bool,
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewException {
    pub id: String,
    pub profile_id: String,
    pub exception_type: String,
    pub granted_by: String,
    pub expires_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub amount_minutes: Option<i64>,
    pub app_id: Option<String>,
    pub website: Option<String>,
    pub scope: Option<String>,
}

impl NewException {
    pub fn new(
        profile_id: String,
        exception_type: String,
        granted_by: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            profile_id,
            exception_type,
            granted_by,
            expires_at,
            reason: None,
            amount_minutes: None,
            app_id: None,
            website: None,
            scope: None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDailySummary {
    pub id: i64,
    pub profile_id: String,
    pub date: NaiveDate,
    pub screen_time_seconds: i64,
    pub active_time_seconds: i64,
    pub idle_time_seconds: i64,
    pub app_launches: i64,
    pub unique_apps: i64,
    pub websites_visited: i64,
    pub blocks_count: i64,
    pub violations_count: i64,
    pub top_apps: String,
    pub top_categories: String,
    pub top_websites: String,
    pub summary_generated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProfile {
    pub id: String,
    pub name: String,
    pub age_group: String,
    pub birthday: Option<NaiveDate>,
    pub config: String,
}

impl NewProfile {
    pub fn new(name: String, age_group: String, config: String) -> Self {
        Self { id: Uuid::new_v4().to_string(), name, age_group, birthday: None, config }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSession {
    pub id: String,
    pub profile_id: String,
}

impl NewSession {
    pub fn new(profile_id: String) -> Self {
        Self { id: Uuid::new_v4().to_string(), profile_id }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewActivity {
    pub session_id: String,
    pub profile_id: String,
    pub app_id: String,
    pub app_name: String,
    pub category: Option<String>,
    pub window_title: Option<String>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvent {
    pub profile_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub details: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbNetworkActivity {
    pub id: i64,
    pub profile_id: String,
    pub timestamp: DateTime<Utc>,
    pub domain: String,
    pub category: Option<String>,
    pub duration_seconds: Option<i64>,
    pub blocked: bool,
    pub action: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNetworkActivity {
    pub profile_id: String,
    pub domain: String,
    pub category: Option<String>,
    pub duration_seconds: Option<i64>,
    pub blocked: bool,
    pub action: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTerminalActivity {
    pub id: i64,
    pub profile_id: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub risk_level: String,
    pub action: String,
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTerminalActivity {
    pub profile_id: String,
    pub command: String,
    pub risk_level: String,
    pub action: String,
    pub blocked: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbPolicyVersion {
    pub id: i64,
    pub profile_id: String,
    pub version: i64,
    pub config: String,
    pub changed_by: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPolicyVersion {
    pub profile_id: String,
    pub version: i64,
    pub config: String,
    pub changed_by: String,
    pub reason: Option<String>,
}
