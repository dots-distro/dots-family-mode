use crate::error::DbError;
use crate::models::{DbScriptAnalysis, DbTerminalCommand, DbTerminalPolicy};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn create_terminal_policy(
    pool: &SqlitePool,
    profile_id: &str,
    command_pattern: &str,
    action: &str,
    risk_level: &str,
    educational_message: Option<String>,
) -> Result<DbTerminalPolicy, DbError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let policy = DbTerminalPolicy {
        id: id.clone(),
        profile_id: profile_id.to_string(),
        command_pattern: command_pattern.to_string(),
        action: action.to_string(),
        risk_level: risk_level.to_string(),
        educational_message,
        created_at: now,
        updated_at: now,
        active: true,
    };

    sqlx::query!(
        "INSERT INTO terminal_policies (
            id, profile_id, command_pattern, action, risk_level, 
            educational_message, created_at, updated_at, active
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        policy.id,
        policy.profile_id,
        policy.command_pattern,
        policy.action,
        policy.risk_level,
        policy.educational_message,
        policy.created_at,
        policy.updated_at,
        policy.active
    )
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(policy)
}

pub async fn get_terminal_policies(
    pool: &SqlitePool,
    profile_id: &str,
) -> Result<Vec<DbTerminalPolicy>, DbError> {
    let policies = sqlx::query_as!(
        DbTerminalPolicy,
        "SELECT * FROM terminal_policies WHERE profile_id = ? AND active = true ORDER BY created_at DESC",
        profile_id
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(policies)
}

pub async fn log_terminal_command(
    pool: &SqlitePool,
    session_id: &str,
    profile_id: &str,
    command: &str,
    shell: &str,
    working_directory: &str,
    risk_level: &str,
    action_taken: &str,
    script_path: Option<&str>,
) -> Result<DbTerminalCommand, DbError> {
    let timestamp = Utc::now();

    let command_log = DbTerminalCommand {
        id: 0, // Will be auto-assigned by database
        session_id: session_id.to_string(),
        profile_id: profile_id.to_string(),
        timestamp,
        command: command.to_string(),
        shell: shell.to_string(),
        working_directory: working_directory.to_string(),
        risk_level: risk_level.to_string(),
        action_taken: action_taken.to_string(),
        exit_code: None,
        duration_ms: None,
        script_path: script_path.map(String::from),
    };

    let result = sqlx::query!(
        "INSERT INTO terminal_commands (
            session_id, profile_id, timestamp, command, shell,
            working_directory, risk_level, action_taken, script_path
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        command_log.session_id,
        command_log.profile_id,
        command_log.timestamp,
        command_log.command,
        command_log.shell,
        command_log.working_directory,
        command_log.risk_level,
        command_log.action_taken,
        command_log.script_path
    )
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    let id = result.last_insert_rowid();

    let mut command_log_with_id = command_log;
    command_log_with_id.id = id;

    Ok(command_log_with_id)
}

pub async fn update_command_result(
    pool: &SqlitePool,
    command_id: i64,
    exit_code: i32,
    duration_ms: i64,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE terminal_commands SET exit_code = ?, duration_ms = ? WHERE id = ?",
        exit_code,
        duration_ms,
        command_id
    )
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(())
}

pub async fn get_terminal_commands(
    pool: &SqlitePool,
    session_id: &str,
    limit: Option<i32>,
) -> Result<Vec<DbTerminalCommand>, DbError> {
    let limit = limit.unwrap_or(100);

    let commands = sqlx::query_as!(
        DbTerminalCommand,
        "SELECT * FROM terminal_commands WHERE session_id = ? ORDER BY timestamp DESC LIMIT ?",
        session_id,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(commands)
}

pub async fn cache_script_analysis(
    pool: &SqlitePool,
    script_path: &str,
    content_hash: &str,
    risk_level: &str,
    dangerous_patterns: &str,
    analysis_result: &str,
    file_size: i64,
    line_count: i32,
) -> Result<DbScriptAnalysis, DbError> {
    let id = Uuid::new_v4().to_string();
    let analyzed_at = Utc::now();

    let analysis = DbScriptAnalysis {
        id: id.clone(),
        script_path: script_path.to_string(),
        content_hash: content_hash.to_string(),
        risk_level: risk_level.to_string(),
        dangerous_patterns: dangerous_patterns.to_string(),
        analysis_result: analysis_result.to_string(),
        analyzed_at,
        file_size,
        line_count,
    };

    sqlx::query!(
        "INSERT OR REPLACE INTO script_analysis (
            id, script_path, content_hash, risk_level, dangerous_patterns,
            analysis_result, analyzed_at, file_size, line_count
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        analysis.id,
        analysis.script_path,
        analysis.content_hash,
        analysis.risk_level,
        analysis.dangerous_patterns,
        analysis.analysis_result,
        analysis.analyzed_at,
        analysis.file_size,
        analysis.line_count
    )
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(analysis)
}

pub async fn get_cached_script_analysis(
    pool: &SqlitePool,
    script_path: &str,
    content_hash: &str,
) -> Result<Option<DbScriptAnalysis>, DbError> {
    let analysis = sqlx::query_as!(
        DbScriptAnalysis,
        "SELECT * FROM script_analysis WHERE script_path = ? AND content_hash = ?",
        script_path,
        content_hash
    )
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(analysis)
}

pub async fn get_dangerous_commands(
    pool: &SqlitePool,
    profile_id: &str,
    since: DateTime<Utc>,
    limit: Option<i32>,
) -> Result<Vec<DbTerminalCommand>, DbError> {
    let limit = limit.unwrap_or(50);

    let commands = sqlx::query_as!(
        DbTerminalCommand,
        "SELECT * FROM terminal_commands 
         WHERE profile_id = ? AND timestamp > ? 
         AND risk_level IN ('high', 'critical')
         ORDER BY timestamp DESC LIMIT ?",
        profile_id,
        since,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(commands)
}
