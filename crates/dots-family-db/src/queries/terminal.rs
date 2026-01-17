use crate::error::DbError;
use crate::models::{DbScriptAnalysis, DbTerminalCommand, DbTerminalPolicy};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
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
        educational_message: educational_message.clone(),
        created_at: now,
        updated_at: now,
        active: true,
    };

    sqlx::query(
        "INSERT INTO terminal_policies (
            id, profile_id, command_pattern, action, risk_level, 
            educational_message, created_at, updated_at, active
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&policy.id)
    .bind(&policy.profile_id)
    .bind(&policy.command_pattern)
    .bind(&policy.action)
    .bind(&policy.risk_level)
    .bind(&policy.educational_message)
    .bind(policy.created_at)
    .bind(policy.updated_at)
    .bind(policy.active)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(policy)
}

pub async fn get_terminal_policy(
    pool: &SqlitePool,
    policy_id: &str,
) -> Result<Option<DbTerminalPolicy>, DbError> {
    let row = sqlx::query(
        "SELECT id, profile_id, command_pattern, action, risk_level, 
         educational_message, created_at, updated_at, active 
         FROM terminal_policies WHERE id = ?",
    )
    .bind(policy_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;

    if let Some(row) = row {
        Ok(Some(DbTerminalPolicy {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            command_pattern: row.get("command_pattern"),
            action: row.get("action"),
            risk_level: row.get("risk_level"),
            educational_message: row.get("educational_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            active: row.get("active"),
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_terminal_policies_for_profile(
    pool: &SqlitePool,
    profile_id: &str,
) -> Result<Vec<DbTerminalPolicy>, DbError> {
    let rows = sqlx::query(
        "SELECT id, profile_id, command_pattern, action, risk_level, 
         educational_message, created_at, updated_at, active 
         FROM terminal_policies 
         WHERE profile_id = ? AND active = 1 
         ORDER BY created_at ASC",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;

    let mut policies = Vec::new();
    for row in rows {
        policies.push(DbTerminalPolicy {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            command_pattern: row.get("command_pattern"),
            action: row.get("action"),
            risk_level: row.get("risk_level"),
            educational_message: row.get("educational_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            active: row.get("active"),
        });
    }

    Ok(policies)
}

pub async fn update_terminal_policy(
    pool: &SqlitePool,
    policy_id: &str,
    command_pattern: Option<&str>,
    action: Option<&str>,
    risk_level: Option<&str>,
    educational_message: Option<Option<String>>,
    active: Option<bool>,
) -> Result<(), DbError> {
    let now = Utc::now();

    // Get current policy to preserve unchanged fields
    let current = get_terminal_policy(pool, policy_id).await?;
    let current = current.ok_or(DbError::NotFound)?;

    let new_command_pattern = command_pattern.unwrap_or(&current.command_pattern);
    let new_action = action.unwrap_or(&current.action);
    let new_risk_level = risk_level.unwrap_or(&current.risk_level);
    let new_educational_message = educational_message.unwrap_or(current.educational_message);
    let new_active = active.unwrap_or(current.active);

    sqlx::query(
        "UPDATE terminal_policies SET 
         command_pattern = ?, action = ?, risk_level = ?, 
         educational_message = ?, active = ?, updated_at = ? 
         WHERE id = ?",
    )
    .bind(new_command_pattern)
    .bind(new_action)
    .bind(new_risk_level)
    .bind(new_educational_message)
    .bind(new_active)
    .bind(now)
    .bind(policy_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(())
}

pub async fn delete_terminal_policy(pool: &SqlitePool, policy_id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM terminal_policies WHERE id = ?")
        .bind(policy_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;

    Ok(())
}

pub async fn create_terminal_command(
    pool: &SqlitePool,
    session_id: &str,
    profile_id: &str,
    command: &str,
    working_directory: &str,
    environment_vars: Option<String>,
    policy_id: Option<String>,
    action_taken: &str,
    risk_assessment: &str,
) -> Result<DbTerminalCommand, DbError> {
    let id = Uuid::new_v4().to_string();
    let executed_at = Utc::now();

    let cmd = DbTerminalCommand {
        id: id.clone(),
        session_id: session_id.to_string(),
        profile_id: profile_id.to_string(),
        command: command.to_string(),
        working_directory: working_directory.to_string(),
        environment_vars,
        executed_at,
        policy_id,
        action_taken: action_taken.to_string(),
        risk_assessment: risk_assessment.to_string(),
    };

    sqlx::query(
        "INSERT INTO terminal_commands (
            id, session_id, profile_id, command, working_directory, 
            environment_vars, executed_at, policy_id, action_taken, risk_assessment
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&cmd.id)
    .bind(&cmd.session_id)
    .bind(&cmd.profile_id)
    .bind(&cmd.command)
    .bind(&cmd.working_directory)
    .bind(&cmd.environment_vars)
    .bind(cmd.executed_at)
    .bind(&cmd.policy_id)
    .bind(&cmd.action_taken)
    .bind(&cmd.risk_assessment)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(cmd)
}

pub async fn get_terminal_commands_for_session(
    pool: &SqlitePool,
    session_id: &str,
    limit: Option<i32>,
) -> Result<Vec<DbTerminalCommand>, DbError> {
    let limit = limit.unwrap_or(100);

    let rows = sqlx::query(
        "SELECT id, session_id, profile_id, command, working_directory, 
         environment_vars, executed_at, policy_id, action_taken, risk_assessment 
         FROM terminal_commands 
         WHERE session_id = ? 
         ORDER BY executed_at DESC 
         LIMIT ?",
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;

    let mut commands = Vec::new();
    for row in rows {
        commands.push(DbTerminalCommand {
            id: row.get("id"),
            session_id: row.get("session_id"),
            profile_id: row.get("profile_id"),
            command: row.get("command"),
            working_directory: row.get("working_directory"),
            environment_vars: row.get("environment_vars"),
            executed_at: row.get("executed_at"),
            policy_id: row.get("policy_id"),
            action_taken: row.get("action_taken"),
            risk_assessment: row.get("risk_assessment"),
        });
    }

    Ok(commands)
}

pub async fn create_script_analysis(
    pool: &SqlitePool,
    file_path: &str,
    profile_id: &str,
    analysis_type: &str,
    risk_score: i32,
    identified_risks: &str,
    recommendations: &str,
) -> Result<DbScriptAnalysis, DbError> {
    let id = Uuid::new_v4().to_string();
    let analyzed_at = Utc::now();

    let analysis = DbScriptAnalysis {
        id: id.clone(),
        file_path: file_path.to_string(),
        profile_id: profile_id.to_string(),
        analysis_type: analysis_type.to_string(),
        risk_score,
        identified_risks: identified_risks.to_string(),
        recommendations: recommendations.to_string(),
        analyzed_at,
    };

    sqlx::query(
        "INSERT INTO script_analysis (
            id, file_path, profile_id, analysis_type, risk_score, 
            identified_risks, recommendations, analyzed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&analysis.id)
    .bind(&analysis.file_path)
    .bind(&analysis.profile_id)
    .bind(&analysis.analysis_type)
    .bind(analysis.risk_score)
    .bind(&analysis.identified_risks)
    .bind(&analysis.recommendations)
    .bind(analysis.analyzed_at)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;

    Ok(analysis)
}

pub async fn get_script_analysis(
    pool: &SqlitePool,
    analysis_id: &str,
) -> Result<Option<DbScriptAnalysis>, DbError> {
    let row = sqlx::query(
        "SELECT id, file_path, profile_id, analysis_type, risk_score, 
         identified_risks, recommendations, analyzed_at 
         FROM script_analysis WHERE id = ?",
    )
    .bind(analysis_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;

    if let Some(row) = row {
        Ok(Some(DbScriptAnalysis {
            id: row.get("id"),
            file_path: row.get("file_path"),
            profile_id: row.get("profile_id"),
            analysis_type: row.get("analysis_type"),
            risk_score: row.get("risk_score"),
            identified_risks: row.get("identified_risks"),
            recommendations: row.get("recommendations"),
            analyzed_at: row.get("analyzed_at"),
        }))
    } else {
        Ok(None)
    }
}
