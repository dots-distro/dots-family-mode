use crate::connection::Database;
use crate::error::{DbError, Result};
use chrono::{NaiveDate, Utc};
use sqlx::Row;

#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub top_apps: String,       // JSON array
    pub top_categories: String, // JSON array
    pub top_websites: String,   // JSON array
    pub summary_generated: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewDailySummary {
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
    pub top_apps: String,       // JSON array
    pub top_categories: String, // JSON array
    pub top_websites: String,   // JSON array
}

impl NewDailySummary {
    pub fn new(profile_id: String, date: NaiveDate) -> Self {
        Self {
            profile_id,
            date,
            screen_time_seconds: 0,
            active_time_seconds: 0,
            idle_time_seconds: 0,
            app_launches: 0,
            unique_apps: 0,
            websites_visited: 0,
            blocks_count: 0,
            violations_count: 0,
            top_apps: "[]".to_string(),
            top_categories: "[]".to_string(),
            top_websites: "[]".to_string(),
        }
    }
}

pub struct DailySummaryQueries;

impl DailySummaryQueries {
    pub async fn create(db: &Database, summary: NewDailySummary) -> Result<DbDailySummary> {
        let pool = db.pool()?;

        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO daily_summaries (
                profile_id, date, screen_time_seconds, active_time_seconds, idle_time_seconds,
                app_launches, unique_apps, websites_visited, blocks_count, violations_count,
                top_apps, top_categories, top_websites, summary_generated
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&summary.profile_id)
        .bind(summary.date)
        .bind(summary.screen_time_seconds)
        .bind(summary.active_time_seconds)
        .bind(summary.idle_time_seconds)
        .bind(summary.app_launches)
        .bind(summary.unique_apps)
        .bind(summary.websites_visited)
        .bind(summary.blocks_count)
        .bind(summary.violations_count)
        .bind(&summary.top_apps)
        .bind(&summary.top_categories)
        .bind(&summary.top_websites)
        .bind(now)
        .execute(pool)
        .await;

        match result {
            Ok(_) => Self::get_by_profile_and_date(db, &summary.profile_id, summary.date).await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(DbError::Duplicate(format!(
                    "Daily summary for {} on {} already exists",
                    summary.profile_id, summary.date
                )))
            }
            Err(e) => Err(DbError::Sqlx(e)),
        }
    }

    pub async fn get_by_profile_and_date(
        db: &Database,
        profile_id: &str,
        date: NaiveDate,
    ) -> Result<DbDailySummary> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbDailySummary>(
            "SELECT * FROM daily_summaries WHERE profile_id = ? AND date = ?",
        )
        .bind(profile_id)
        .bind(date)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            DbError::NotFound(format!("Daily summary for {} on {} not found", profile_id, date))
        })
    }

    pub async fn list_by_profile_range(
        db: &Database,
        profile_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DbDailySummary>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbDailySummary>(
            "SELECT * FROM daily_summaries WHERE profile_id = ? AND date >= ? AND date <= ? ORDER BY date DESC"
        )
        .bind(profile_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_recent_by_profile(
        db: &Database,
        profile_id: &str,
        limit: i64,
    ) -> Result<Vec<DbDailySummary>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbDailySummary>(
            "SELECT * FROM daily_summaries WHERE profile_id = ? ORDER BY date DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn update_summary(
        db: &Database,
        profile_id: &str,
        date: NaiveDate,
        summary: NewDailySummary,
    ) -> Result<()> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            UPDATE daily_summaries SET
                screen_time_seconds = ?, active_time_seconds = ?, idle_time_seconds = ?,
                app_launches = ?, unique_apps = ?, websites_visited = ?,
                blocks_count = ?, violations_count = ?,
                top_apps = ?, top_categories = ?, top_websites = ?,
                summary_generated = CURRENT_TIMESTAMP
            WHERE profile_id = ? AND date = ?
            "#,
        )
        .bind(summary.screen_time_seconds)
        .bind(summary.active_time_seconds)
        .bind(summary.idle_time_seconds)
        .bind(summary.app_launches)
        .bind(summary.unique_apps)
        .bind(summary.websites_visited)
        .bind(summary.blocks_count)
        .bind(summary.violations_count)
        .bind(&summary.top_apps)
        .bind(&summary.top_categories)
        .bind(&summary.top_websites)
        .bind(profile_id)
        .bind(date)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!(
                "Daily summary for {} on {} not found",
                profile_id, date
            )))
        } else {
            Ok(())
        }
    }

    pub async fn get_total_screen_time_by_profile(
        db: &Database,
        profile_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<i64> {
        let pool = db.pool()?;

        let row = sqlx::query(
            "SELECT COALESCE(SUM(screen_time_seconds), 0) FROM daily_summaries WHERE profile_id = ? AND date >= ? AND date <= ?"
        )
        .bind(profile_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        Ok(row.get(0))
    }

    pub async fn get_daily_average(
        db: &Database,
        profile_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<f64> {
        let pool = db.pool()?;

        let row = sqlx::query(
            "SELECT AVG(CAST(screen_time_seconds AS REAL)) FROM daily_summaries WHERE profile_id = ? AND date >= ? AND date <= ?"
        )
        .bind(profile_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await?;

        Ok(row.get::<Option<f64>, _>(0).unwrap_or(0.0))
    }

    pub async fn delete_old_summaries(db: &Database, cutoff_date: NaiveDate) -> Result<u64> {
        let pool = db.pool()?;

        let result = sqlx::query("DELETE FROM daily_summaries WHERE date < ?")
            .bind(cutoff_date)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DatabaseConfig;
    use chrono::NaiveDate;
    use tempfile::tempdir;

    async fn setup_test_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.run_migrations().await.unwrap();
        (db, dir)
    }

    #[tokio::test]
    async fn test_create_daily_summary() {
        let (db, _dir) = setup_test_db().await;

        let summary = NewDailySummary {
            profile_id: "test-profile".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            screen_time_seconds: 3600,
            active_time_seconds: 3000,
            idle_time_seconds: 600,
            app_launches: 10,
            unique_apps: 5,
            websites_visited: 20,
            blocks_count: 2,
            violations_count: 1,
            top_apps: r#"[{"app_id":"firefox","duration":1800}]"#.to_string(),
            top_categories: r#"[{"category":"browser","duration":1800}]"#.to_string(),
            top_websites: r#"[{"domain":"example.com","visits":5}]"#.to_string(),
        };

        let created = DailySummaryQueries::create(&db, summary).await.unwrap();
        assert_eq!(created.profile_id, "test-profile");
        assert_eq!(created.screen_time_seconds, 3600);
        assert_eq!(created.app_launches, 10);
    }

    #[tokio::test]
    async fn test_get_total_screen_time_by_profile() {
        let (db, _dir) = setup_test_db().await;

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();

        let summary1 = NewDailySummary {
            profile_id: "test-profile".to_string(),
            date: date1,
            screen_time_seconds: 3600,
            ..NewDailySummary::new("test-profile".to_string(), date1)
        };

        let summary2 = NewDailySummary {
            profile_id: "test-profile".to_string(),
            date: date2,
            screen_time_seconds: 2400,
            ..NewDailySummary::new("test-profile".to_string(), date2)
        };

        DailySummaryQueries::create(&db, summary1).await.unwrap();
        DailySummaryQueries::create(&db, summary2).await.unwrap();

        let total = DailySummaryQueries::get_total_screen_time_by_profile(
            &db,
            "test-profile",
            date1,
            date2,
        )
        .await
        .unwrap();

        assert_eq!(total, 6000);
    }

    #[tokio::test]
    async fn test_daily_average() {
        let (db, _dir) = setup_test_db().await;

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();

        let summary1 = NewDailySummary {
            profile_id: "test-profile".to_string(),
            date: date1,
            screen_time_seconds: 3600,
            ..NewDailySummary::new("test-profile".to_string(), date1)
        };

        let summary2 = NewDailySummary {
            profile_id: "test-profile".to_string(),
            date: date2,
            screen_time_seconds: 2400,
            ..NewDailySummary::new("test-profile".to_string(), date2)
        };

        DailySummaryQueries::create(&db, summary1).await.unwrap();
        DailySummaryQueries::create(&db, summary2).await.unwrap();

        let average = DailySummaryQueries::get_daily_average(&db, "test-profile", date1, date2)
            .await
            .unwrap();

        assert_eq!(average, 3000.0);
    }
}
