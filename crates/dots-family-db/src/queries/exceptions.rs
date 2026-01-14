use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbException, NewException};
use chrono::Utc;

pub struct ExceptionQueries;

impl ExceptionQueries {
    /// Create a new exception for temporary policy overrides
    pub async fn create(db: &Database, exception: NewException) -> Result<String> {
        let pool = db.pool()?;

        sqlx::query(
            r#"
            INSERT INTO exceptions 
            (id, profile_id, exception_type, granted_by, granted_at, expires_at, 
             reason, amount_minutes, app_id, website, scope, active, used)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0)
            "#,
        )
        .bind(&exception.id)
        .bind(&exception.profile_id)
        .bind(&exception.exception_type)
        .bind(&exception.granted_by)
        .bind(Utc::now())
        .bind(exception.expires_at)
        .bind(&exception.reason)
        .bind(exception.amount_minutes)
        .bind(&exception.app_id)
        .bind(&exception.website)
        .bind(&exception.scope)
        .execute(pool)
        .await?;

        Ok(exception.id)
    }

    /// Get active exceptions for a profile
    pub async fn list_active_for_profile(
        db: &Database,
        profile_id: &str,
    ) -> Result<Vec<DbException>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbException>(
            "SELECT * FROM exceptions WHERE profile_id = ? AND active = 1 AND expires_at > ? ORDER BY granted_at DESC",
        )
        .bind(profile_id)
        .bind(Utc::now())
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get all exceptions for a profile (including expired and inactive)
    pub async fn list_all_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbException>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbException>(
            "SELECT * FROM exceptions WHERE profile_id = ? ORDER BY granted_at DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Check if there's an active exception for a specific resource
    pub async fn check_active_exception(
        db: &Database,
        profile_id: &str,
        exception_type: &str,
        resource_id: Option<&str>,
    ) -> Result<Option<DbException>> {
        let pool = db.pool()?;

        let result = match (exception_type, resource_id) {
            ("app", Some(app_id)) => {
                sqlx::query_as::<_, DbException>(
                    r#"
                    SELECT * FROM exceptions 
                    WHERE profile_id = ? 
                      AND exception_type = 'app' 
                      AND app_id = ? 
                      AND active = 1 
                      AND expires_at > ?
                    ORDER BY granted_at DESC 
                    LIMIT 1
                    "#,
                )
                .bind(profile_id)
                .bind(app_id)
                .bind(Utc::now())
                .fetch_optional(pool)
                .await
            }
            ("website", Some(website)) => {
                sqlx::query_as::<_, DbException>(
                    r#"
                    SELECT * FROM exceptions 
                    WHERE profile_id = ? 
                      AND exception_type = 'website' 
                      AND website = ? 
                      AND active = 1 
                      AND expires_at > ?
                    ORDER BY granted_at DESC 
                    LIMIT 1
                    "#,
                )
                .bind(profile_id)
                .bind(website)
                .bind(Utc::now())
                .fetch_optional(pool)
                .await
            }
            ("time", None) => {
                sqlx::query_as::<_, DbException>(
                    r#"
                    SELECT * FROM exceptions 
                    WHERE profile_id = ? 
                      AND exception_type = 'time' 
                      AND active = 1 
                      AND expires_at > ?
                    ORDER BY granted_at DESC 
                    LIMIT 1
                    "#,
                )
                .bind(profile_id)
                .bind(Utc::now())
                .fetch_optional(pool)
                .await
            }
            _ => {
                // Generic exception check
                sqlx::query_as::<_, DbException>(
                    r#"
                    SELECT * FROM exceptions 
                    WHERE profile_id = ? 
                      AND exception_type = ? 
                      AND active = 1 
                      AND expires_at > ?
                    ORDER BY granted_at DESC 
                    LIMIT 1
                    "#,
                )
                .bind(profile_id)
                .bind(exception_type)
                .bind(Utc::now())
                .fetch_optional(pool)
                .await
            }
        };

        result.map_err(DbError::Sqlx)
    }

    /// Mark an exception as used (for one-time exceptions)
    pub async fn mark_as_used(db: &Database, exception_id: &str) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("UPDATE exceptions SET used = 1 WHERE id = ?")
            .bind(exception_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Revoke an exception (mark as inactive)
    pub async fn revoke_exception(db: &Database, exception_id: &str) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("UPDATE exceptions SET active = 0 WHERE id = ?")
            .bind(exception_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get exceptions granted by a specific parent
    pub async fn list_by_granted_by(
        db: &Database,
        granted_by: &str,
        limit: i64,
    ) -> Result<Vec<DbException>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbException>(
            "SELECT * FROM exceptions WHERE granted_by = ? ORDER BY granted_at DESC LIMIT ?",
        )
        .bind(granted_by)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Clean up expired exceptions
    pub async fn cleanup_expired(db: &Database) -> Result<u64> {
        let pool = db.pool()?;

        let result =
            sqlx::query("UPDATE exceptions SET active = 0 WHERE expires_at <= ? AND active = 1")
                .bind(Utc::now())
                .execute(pool)
                .await?;

        Ok(result.rows_affected())
    }

    /// Get exception usage statistics for a profile
    pub async fn get_usage_stats(
        db: &Database,
        profile_id: &str,
        days: i64,
    ) -> Result<(i64, i64, i64)> {
        let pool = db.pool()?;

        let since = Utc::now() - chrono::Duration::days(days);
        let row: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_exceptions,
                SUM(CASE WHEN used = 1 THEN 1 ELSE 0 END) as used_exceptions,
                SUM(CASE WHEN active = 1 AND expires_at > ? THEN 1 ELSE 0 END) as active_exceptions
            FROM exceptions 
            WHERE profile_id = ? AND granted_at >= ?
            "#,
        )
        .bind(Utc::now())
        .bind(profile_id)
        .bind(since)
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(row)
    }

    /// Get exceptions by type for analysis
    pub async fn list_by_type(
        db: &Database,
        profile_id: &str,
        exception_type: &str,
        limit: i64,
    ) -> Result<Vec<DbException>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbException>(
            "SELECT * FROM exceptions WHERE profile_id = ? AND exception_type = ? ORDER BY granted_at DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(exception_type)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get recently expired exceptions for review
    pub async fn list_recently_expired(
        db: &Database,
        profile_id: &str,
        hours: i64,
    ) -> Result<Vec<DbException>> {
        let pool = db.pool()?;

        let since = Utc::now() - chrono::Duration::hours(hours);
        sqlx::query_as::<_, DbException>(
            "SELECT * FROM exceptions WHERE profile_id = ? AND expires_at BETWEEN ? AND ? ORDER BY expires_at DESC",
        )
        .bind(profile_id)
        .bind(since)
        .bind(Utc::now())
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}
