use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbActivity, NewActivity};
use chrono::Utc;

pub struct ActivityQueries;

impl ActivityQueries {
    pub async fn create(db: &Database, activity: NewActivity) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO activities 
            (session_id, profile_id, timestamp, app_id, app_name, category, window_title, duration_seconds)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&activity.session_id)
        .bind(&activity.profile_id)
        .bind(Utc::now())
        .bind(&activity.app_id)
        .bind(&activity.app_name)
        .bind(&activity.category)
        .bind(&activity.window_title)
        .bind(activity.duration_seconds)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_for_session(db: &Database, session_id: &str) -> Result<Vec<DbActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbActivity>(
            "SELECT * FROM activities WHERE session_id = ? ORDER BY timestamp DESC",
        )
        .bind(session_id)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbActivity>(
            "SELECT * FROM activities WHERE profile_id = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_by_profile_since(
        db: &Database,
        profile_id: &str,
        since: chrono::DateTime<Utc>,
    ) -> Result<Vec<DbActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbActivity>(
            "SELECT * FROM activities WHERE profile_id = ? AND timestamp >= ? ORDER BY timestamp DESC",
        )
        .bind(profile_id)
        .bind(since)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}
