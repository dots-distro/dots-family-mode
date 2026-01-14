use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbPolicyVersion, NewPolicyVersion};
use chrono::Utc;

pub struct PolicyVersionQueries;

impl PolicyVersionQueries {
    /// Create a new policy version entry for audit trail
    pub async fn create(db: &Database, version: NewPolicyVersion) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO policy_versions 
            (profile_id, version, config, changed_by, reason, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&version.profile_id)
        .bind(version.version)
        .bind(&version.config)
        .bind(&version.changed_by)
        .bind(&version.reason)
        .bind(Utc::now())
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get policy history for a specific profile
    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbPolicyVersion>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbPolicyVersion>(
            "SELECT * FROM policy_versions WHERE profile_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get all policy changes by a specific admin user
    pub async fn list_by_changed_by(
        db: &Database,
        changed_by: &str,
        limit: i64,
    ) -> Result<Vec<DbPolicyVersion>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbPolicyVersion>(
            "SELECT * FROM policy_versions WHERE changed_by = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(changed_by)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get the latest version number for a profile
    pub async fn get_latest_version(db: &Database, profile_id: &str) -> Result<Option<i64>> {
        let pool = db.pool()?;

        let row: Option<(Option<i64>,)> =
            sqlx::query_as("SELECT MAX(version) FROM policy_versions WHERE profile_id = ?")
                .bind(profile_id)
                .fetch_optional(pool)
                .await
                .map_err(DbError::Sqlx)?;

        Ok(row.and_then(|(version,)| version))
    }

    /// Get a specific policy version by profile and version number
    pub async fn get_by_version(
        db: &Database,
        profile_id: &str,
        version: i64,
    ) -> Result<Option<DbPolicyVersion>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbPolicyVersion>(
            "SELECT * FROM policy_versions WHERE profile_id = ? AND version = ?",
        )
        .bind(profile_id)
        .bind(version)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Rollback to a previous policy version by copying it as a new version
    pub async fn rollback_to_version(
        db: &Database,
        profile_id: &str,
        target_version: i64,
        changed_by: &str,
        reason: Option<&str>,
    ) -> Result<i64> {
        let pool = db.pool()?;

        // First, get the target version's config
        let target_policy = sqlx::query_as::<_, DbPolicyVersion>(
            "SELECT * FROM policy_versions WHERE profile_id = ? AND version = ?",
        )
        .bind(profile_id)
        .bind(target_version)
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;

        // Get the latest version number
        let latest_version = Self::get_latest_version(db, profile_id).await?.unwrap_or(0);
        let new_version = latest_version + 1;

        // Create a new version entry with the old config
        let rollback_reason = reason
            .map(|r| format!("Rollback to version {}: {}", target_version, r))
            .unwrap_or_else(|| format!("Rollback to version {}", target_version));

        let new_version_entry = NewPolicyVersion {
            profile_id: profile_id.to_string(),
            version: new_version,
            config: target_policy.config,
            changed_by: changed_by.to_string(),
            reason: Some(rollback_reason),
        };

        Self::create(db, new_version_entry).await
    }

    /// Get policy changes within a date range
    pub async fn list_by_date_range(
        db: &Database,
        profile_id: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<DbPolicyVersion>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbPolicyVersion>(
            "SELECT * FROM policy_versions WHERE profile_id = ? AND created_at BETWEEN ? AND ? ORDER BY created_at DESC",
        )
        .bind(profile_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    /// Get stats about policy changes for a profile
    pub async fn get_change_stats(
        db: &Database,
        profile_id: &str,
    ) -> Result<(i64, Option<chrono::DateTime<Utc>>, Option<String>)> {
        let pool = db.pool()?;

        let row: Option<(i64, Option<chrono::DateTime<Utc>>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_changes,
                MAX(created_at) as last_change_at,
                (SELECT changed_by FROM policy_versions 
                 WHERE profile_id = ? 
                 ORDER BY created_at DESC 
                 LIMIT 1) as last_changed_by
            FROM policy_versions 
            WHERE profile_id = ?
            "#,
        )
        .bind(profile_id)
        .bind(profile_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(row.unwrap_or((0, None, None)))
    }
}
