use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbTerminalActivity, NewTerminalActivity};
use chrono::Utc;

pub struct TerminalActivityQueries;

impl TerminalActivityQueries {
    /// Record a terminal command activity for security monitoring
    pub async fn create(db: &Database, activity: NewTerminalActivity) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO terminal_activity 
            (profile_id, timestamp, command, risk_level, action, blocked)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&activity.profile_id)
        .bind(Utc::now())
        .bind(&activity.command)
        .bind(&activity.risk_level)
        .bind(&activity.action)
        .bind(activity.blocked)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get terminal activity history for a specific profile
    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get blocked terminal commands for security review
    pub async fn list_blocked_by_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? AND blocked = 1 ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get commands by risk level for analysis
    pub async fn list_by_risk_level(
        db: &Database,
        profile_id: &str,
        risk_level: &str,
        limit: i64,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? AND risk_level = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(risk_level)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get terminal activity within a date range
    pub async fn list_by_date_range(
        db: &Database,
        profile_id: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? AND timestamp BETWEEN ? AND ? ORDER BY timestamp DESC",
        )
        .bind(profile_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Search for commands containing specific text (for security analysis)
    pub async fn search_commands(
        db: &Database,
        profile_id: &str,
        search_term: &str,
        limit: i64,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        let search_pattern = format!("%{}%", search_term);
        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? AND command LIKE ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(search_pattern)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get daily terminal activity statistics
    pub async fn get_daily_stats(
        db: &Database,
        profile_id: &str,
        date: chrono::NaiveDate,
    ) -> Result<(i64, i64, i64, i64)> {
        let pool = db.pool()?;

        let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let row: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_commands,
                SUM(CASE WHEN blocked = 1 THEN 1 ELSE 0 END) as blocked_commands,
                SUM(CASE WHEN risk_level = 'dangerous' THEN 1 ELSE 0 END) as dangerous_commands,
                SUM(CASE WHEN risk_level = 'risky' THEN 1 ELSE 0 END) as risky_commands
            FROM terminal_activity 
            WHERE profile_id = ? AND timestamp BETWEEN ? AND ?
            "#,
        )
        .bind(profile_id)
        .bind(start)
        .bind(end)
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(row)
    }

    /// Get most commonly used commands (for educational purposes)
    pub async fn get_top_commands(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<(String, i64)>> {
        let pool = db.pool()?;

        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT command, COUNT(*) as usage_count
            FROM terminal_activity 
            WHERE profile_id = ? AND blocked = 0
            GROUP BY command 
            ORDER BY usage_count DESC 
            LIMIT ?
            "#,
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(rows)
    }

    /// Get risk level distribution for security analysis
    pub async fn get_risk_level_stats(
        db: &Database,
        profile_id: &str,
    ) -> Result<Vec<(String, i64)>> {
        let pool = db.pool()?;

        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT risk_level, COUNT(*) as count
            FROM terminal_activity 
            WHERE profile_id = ?
            GROUP BY risk_level 
            ORDER BY count DESC
            "#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(rows)
    }

    /// Get recently approved commands (commands that were initially risky but approved)
    pub async fn list_recently_approved(
        db: &Database,
        profile_id: &str,
        days: i64,
    ) -> Result<Vec<DbTerminalActivity>> {
        let pool = db.pool()?;

        let since = Utc::now() - chrono::Duration::days(days);
        sqlx::query_as::<_, DbTerminalActivity>(
            "SELECT * FROM terminal_activity WHERE profile_id = ? AND action = 'approved' AND timestamp >= ? ORDER BY timestamp DESC",
        )
        .bind(profile_id)
        .bind(since)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}
