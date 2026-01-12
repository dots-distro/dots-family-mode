use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbSession, NewSession};
use chrono::Utc;

pub struct SessionQueries;

impl SessionQueries {
    pub async fn create(db: &Database, session: NewSession) -> Result<DbSession> {
        let pool = db.pool()?;

        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, profile_id, start_time)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.profile_id)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get_by_id(db, &session.id).await
    }

    pub async fn get_by_id(db: &Database, id: &str) -> Result<DbSession> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbSession>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", id)))
    }

    pub async fn get_active_session(db: &Database, profile_id: &str) -> Result<Option<DbSession>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbSession>(
            "SELECT * FROM sessions WHERE profile_id = ? AND end_time IS NULL ORDER BY start_time DESC LIMIT 1"
        )
        .bind(profile_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn end_session(
        db: &Database,
        id: &str,
        end_reason: &str,
        duration_seconds: i64,
        screen_time_seconds: i64,
        active_time_seconds: i64,
        idle_time_seconds: i64,
    ) -> Result<()> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            UPDATE sessions 
            SET end_time = ?, end_reason = ?, duration_seconds = ?, 
                screen_time_seconds = ?, active_time_seconds = ?, idle_time_seconds = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now())
        .bind(end_reason)
        .bind(duration_seconds)
        .bind(screen_time_seconds)
        .bind(active_time_seconds)
        .bind(idle_time_seconds)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!("Session {} not found", id)))
        } else {
            Ok(())
        }
    }

    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbSession>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbSession>(
            "SELECT * FROM sessions WHERE profile_id = ? ORDER BY start_time DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}
