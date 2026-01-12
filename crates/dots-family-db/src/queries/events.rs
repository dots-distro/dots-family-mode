use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbEvent, NewEvent};

pub struct EventQueries;

impl EventQueries {
    pub async fn create(db: &Database, event: NewEvent) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO events (profile_id, event_type, severity, details, metadata)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.profile_id)
        .bind(&event.event_type)
        .bind(&event.severity)
        .bind(&event.details)
        .bind(&event.metadata)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbEvent>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbEvent>(
            "SELECT * FROM events WHERE profile_id = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_by_type(db: &Database, event_type: &str, limit: i64) -> Result<Vec<DbEvent>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbEvent>(
            "SELECT * FROM events WHERE event_type = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(event_type)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_by_severity(
        db: &Database,
        severity: &str,
        limit: i64,
    ) -> Result<Vec<DbEvent>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbEvent>(
            "SELECT * FROM events WHERE severity = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(severity)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}
