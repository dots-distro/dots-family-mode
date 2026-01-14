use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbNetworkActivity, NewNetworkActivity};
use chrono::Utc;

pub struct NetworkActivityQueries;

impl NetworkActivityQueries {
    pub async fn create(db: &Database, activity: NewNetworkActivity) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO network_activity 
            (profile_id, timestamp, domain, category, duration_seconds, blocked, action, reason)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&activity.profile_id)
        .bind(Utc::now())
        .bind(&activity.domain)
        .bind(&activity.category)
        .bind(activity.duration_seconds)
        .bind(activity.blocked)
        .bind(&activity.action)
        .bind(&activity.reason)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_for_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbNetworkActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbNetworkActivity>(
            "SELECT * FROM network_activity WHERE profile_id = ? ORDER BY timestamp DESC LIMIT ?",
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
    ) -> Result<Vec<DbNetworkActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbNetworkActivity>(
            "SELECT * FROM network_activity WHERE profile_id = ? AND timestamp >= ? ORDER BY timestamp DESC",
        )
        .bind(profile_id)
        .bind(since)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_blocked_by_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbNetworkActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbNetworkActivity>(
            "SELECT * FROM network_activity WHERE profile_id = ? AND blocked = 1 ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_by_domain(
        db: &Database,
        domain: &str,
        limit: i64,
    ) -> Result<Vec<DbNetworkActivity>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbNetworkActivity>(
            "SELECT * FROM network_activity WHERE domain = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(domain)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn get_daily_stats(
        db: &Database,
        profile_id: &str,
        date: chrono::NaiveDate,
    ) -> Result<(i64, i64)> {
        let pool = db.pool()?;

        let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        let row: (i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total_requests,
                SUM(CASE WHEN blocked = 1 THEN 1 ELSE 0 END) as blocked_requests
            FROM network_activity 
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

    pub async fn get_top_domains(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<(String, i64)>> {
        let pool = db.pool()?;

        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT domain, COUNT(*) as visit_count
            FROM network_activity 
            WHERE profile_id = ? AND blocked = 0
            GROUP BY domain 
            ORDER BY visit_count DESC 
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
}
