use crate::connection::Database;
use crate::error::{DbError, Result};
use chrono::{NaiveDate, Utc};
use sqlx::Row;

#[cfg(test)]
use crate::models::NewProfile;
#[cfg(test)]
use crate::queries::profiles::ProfileQueries;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbWeeklySummary {
    pub id: i64,
    pub profile_id: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub total_screen_time_seconds: i64,
    pub daily_average_seconds: i64,
    pub previous_week_seconds: Option<i64>,
    pub change_percentage: Option<f64>,
    pub category_breakdown: String, // JSON
    pub days_within_limit: i64,
    pub days_exceeded_limit: i64,
    pub violations_count: i64,
    pub top_apps: String,       // JSON array
    pub top_categories: String, // JSON array
    pub summary_generated: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewWeeklySummary {
    pub profile_id: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub total_screen_time_seconds: i64,
    pub daily_average_seconds: i64,
    pub previous_week_seconds: Option<i64>,
    pub change_percentage: Option<f64>,
    pub category_breakdown: String, // JSON
    pub days_within_limit: i64,
    pub days_exceeded_limit: i64,
    pub violations_count: i64,
    pub top_apps: String,       // JSON array
    pub top_categories: String, // JSON array
}

impl NewWeeklySummary {
    pub fn new(profile_id: String, week_start: NaiveDate, week_end: NaiveDate) -> Self {
        Self {
            profile_id,
            week_start,
            week_end,
            total_screen_time_seconds: 0,
            daily_average_seconds: 0,
            previous_week_seconds: None,
            change_percentage: None,
            category_breakdown: "{}".to_string(),
            days_within_limit: 0,
            days_exceeded_limit: 0,
            violations_count: 0,
            top_apps: "[]".to_string(),
            top_categories: "[]".to_string(),
        }
    }

    pub fn calculate_daily_average(&mut self) {
        let days_in_week = (self.week_end - self.week_start).num_days() + 1;
        if days_in_week > 0 {
            self.daily_average_seconds = self.total_screen_time_seconds / days_in_week;
        }
    }

    pub fn calculate_change_percentage(&mut self) {
        if let Some(previous) = self.previous_week_seconds {
            if previous > 0 {
                self.change_percentage = Some(
                    ((self.total_screen_time_seconds - previous) as f64 / previous as f64) * 100.0,
                );
            }
        }
    }
}

pub struct WeeklySummaryQueries;

impl WeeklySummaryQueries {
    pub async fn create(db: &Database, mut summary: NewWeeklySummary) -> Result<DbWeeklySummary> {
        let pool = db.pool()?;

        summary.calculate_daily_average();
        summary.calculate_change_percentage();

        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO weekly_summaries (
                profile_id, week_start, week_end, total_screen_time_seconds, daily_average_seconds,
                previous_week_seconds, change_percentage, category_breakdown,
                days_within_limit, days_exceeded_limit, violations_count,
                top_apps, top_categories, summary_generated
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&summary.profile_id)
        .bind(summary.week_start)
        .bind(summary.week_end)
        .bind(summary.total_screen_time_seconds)
        .bind(summary.daily_average_seconds)
        .bind(summary.previous_week_seconds)
        .bind(summary.change_percentage)
        .bind(&summary.category_breakdown)
        .bind(summary.days_within_limit)
        .bind(summary.days_exceeded_limit)
        .bind(summary.violations_count)
        .bind(&summary.top_apps)
        .bind(&summary.top_categories)
        .bind(now)
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                Self::get_by_profile_and_week(db, &summary.profile_id, summary.week_start).await
            }
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(DbError::Duplicate(format!(
                    "Weekly summary for {} starting {} already exists",
                    summary.profile_id, summary.week_start
                )))
            }
            Err(e) => Err(DbError::Sqlx(e)),
        }
    }

    pub async fn get_by_profile_and_week(
        db: &Database,
        profile_id: &str,
        week_start: NaiveDate,
    ) -> Result<DbWeeklySummary> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbWeeklySummary>(
            "SELECT * FROM weekly_summaries WHERE profile_id = ? AND week_start = ?",
        )
        .bind(profile_id)
        .bind(week_start)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            DbError::NotFound(format!(
                "Weekly summary for {} starting {} not found",
                profile_id, week_start
            ))
        })
    }

    pub async fn list_by_profile(
        db: &Database,
        profile_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<DbWeeklySummary>> {
        let pool = db.pool()?;

        let query = match limit {
            Some(l) => {
                sqlx::query_as::<_, DbWeeklySummary>(
                    "SELECT * FROM weekly_summaries WHERE profile_id = ? ORDER BY week_start DESC LIMIT ?"
                )
                .bind(profile_id)
                .bind(l)
            }
            None => {
                sqlx::query_as::<_, DbWeeklySummary>(
                    "SELECT * FROM weekly_summaries WHERE profile_id = ? ORDER BY week_start DESC"
                )
                .bind(profile_id)
            }
        };

        query.fetch_all(pool).await.map_err(DbError::Sqlx)
    }

    pub async fn get_recent_weeks_comparison(
        db: &Database,
        profile_id: &str,
        weeks: i64,
    ) -> Result<Vec<DbWeeklySummary>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbWeeklySummary>(
            "SELECT * FROM weekly_summaries WHERE profile_id = ? ORDER BY week_start DESC LIMIT ?",
        )
        .bind(profile_id)
        .bind(weeks)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn get_trend_data(
        db: &Database,
        profile_id: &str,
        start_week: NaiveDate,
        end_week: NaiveDate,
    ) -> Result<Vec<DbWeeklySummary>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbWeeklySummary>(
            "SELECT * FROM weekly_summaries WHERE profile_id = ? AND week_start >= ? AND week_start <= ? ORDER BY week_start ASC"
        )
        .bind(profile_id)
        .bind(start_week)
        .bind(end_week)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn update_summary(
        db: &Database,
        profile_id: &str,
        week_start: NaiveDate,
        mut summary: NewWeeklySummary,
    ) -> Result<()> {
        let pool = db.pool()?;

        summary.calculate_daily_average();
        summary.calculate_change_percentage();

        let result = sqlx::query(
            r#"
            UPDATE weekly_summaries SET
                total_screen_time_seconds = ?, daily_average_seconds = ?,
                previous_week_seconds = ?, change_percentage = ?,
                category_breakdown = ?, days_within_limit = ?, days_exceeded_limit = ?,
                violations_count = ?, top_apps = ?, top_categories = ?,
                summary_generated = CURRENT_TIMESTAMP
            WHERE profile_id = ? AND week_start = ?
            "#,
        )
        .bind(summary.total_screen_time_seconds)
        .bind(summary.daily_average_seconds)
        .bind(summary.previous_week_seconds)
        .bind(summary.change_percentage)
        .bind(&summary.category_breakdown)
        .bind(summary.days_within_limit)
        .bind(summary.days_exceeded_limit)
        .bind(summary.violations_count)
        .bind(&summary.top_apps)
        .bind(&summary.top_categories)
        .bind(profile_id)
        .bind(week_start)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!(
                "Weekly summary for {} starting {} not found",
                profile_id, week_start
            )))
        } else {
            Ok(())
        }
    }

    pub async fn get_compliance_stats(
        db: &Database,
        profile_id: &str,
        weeks: i64,
    ) -> Result<(i64, i64, f64)> {
        let pool = db.pool()?;

        let row = sqlx::query(
            r#"
            SELECT 
                SUM(days_within_limit) as total_compliant_days,
                SUM(days_exceeded_limit) as total_exceeded_days,
                AVG(CAST(total_screen_time_seconds AS REAL)) as avg_weekly_time
            FROM weekly_summaries 
            WHERE profile_id = ? 
            ORDER BY week_start DESC 
            LIMIT ?
            "#,
        )
        .bind(profile_id)
        .bind(weeks)
        .fetch_one(pool)
        .await?;

        Ok((
            row.get::<Option<i64>, _>(0).unwrap_or(0),
            row.get::<Option<i64>, _>(1).unwrap_or(0),
            row.get::<Option<f64>, _>(2).unwrap_or(0.0),
        ))
    }

    pub async fn delete_old_summaries(db: &Database, cutoff_week: NaiveDate) -> Result<u64> {
        let pool = db.pool()?;

        let result = sqlx::query("DELETE FROM weekly_summaries WHERE week_start < ?")
            .bind(cutoff_week)
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
    async fn test_create_weekly_summary() {
        let (db, _dir) = setup_test_db().await;

        let profile =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());
        let created_profile = ProfileQueries::create(&db, profile).await.unwrap();

        let week_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday
        let week_end = NaiveDate::from_ymd_opt(2024, 1, 21).unwrap(); // Sunday

        let summary = NewWeeklySummary {
            profile_id: created_profile.id.clone(),
            week_start,
            week_end,
            total_screen_time_seconds: 25200, // 7 hours
            days_within_limit: 5,
            days_exceeded_limit: 2,
            violations_count: 3,
            top_apps: r#"[{"app_id":"firefox","duration":12600}]"#.to_string(),
            top_categories: r#"[{"category":"browser","duration":12600}]"#.to_string(),
            ..NewWeeklySummary::new(created_profile.id.clone(), week_start, week_end)
        };

        let created = WeeklySummaryQueries::create(&db, summary).await.unwrap();
        assert_eq!(created.profile_id, created_profile.id);
        assert_eq!(created.total_screen_time_seconds, 25200);
        assert_eq!(created.daily_average_seconds, 3600); // 25200 / 7 days
        assert_eq!(created.days_within_limit, 5);
    }

    #[tokio::test]
    async fn test_change_percentage_calculation() {
        let (db, _dir) = setup_test_db().await;

        let profile =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());
        let created_profile = ProfileQueries::create(&db, profile).await.unwrap();

        let week_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let week_end = NaiveDate::from_ymd_opt(2024, 1, 21).unwrap();

        let summary = NewWeeklySummary {
            profile_id: created_profile.id.clone(),
            week_start,
            week_end,
            total_screen_time_seconds: 10800, // Current week: 3 hours
            previous_week_seconds: Some(7200), // Previous week: 2 hours
            ..NewWeeklySummary::new(created_profile.id.clone(), week_start, week_end)
        };

        let created = WeeklySummaryQueries::create(&db, summary).await.unwrap();

        // Should calculate 50% increase: (10800 - 7200) / 7200 * 100 = 50%
        assert_eq!(created.change_percentage, Some(50.0));
    }

    #[tokio::test]
    async fn test_compliance_stats() {
        let (db, _dir) = setup_test_db().await;

        let profile =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());
        let created_profile = ProfileQueries::create(&db, profile).await.unwrap();

        // Create two weekly summaries
        let week1_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let week1_end = NaiveDate::from_ymd_opt(2024, 1, 21).unwrap();
        let week2_start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let week2_end = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();

        let summary1 = NewWeeklySummary {
            profile_id: created_profile.id.clone(),
            week_start: week1_start,
            week_end: week1_end,
            total_screen_time_seconds: 14400,
            days_within_limit: 5,
            days_exceeded_limit: 2,
            ..NewWeeklySummary::new(created_profile.id.clone(), week1_start, week1_end)
        };

        let summary2 = NewWeeklySummary {
            profile_id: created_profile.id.clone(),
            week_start: week2_start,
            week_end: week2_end,
            total_screen_time_seconds: 10800,
            days_within_limit: 6,
            days_exceeded_limit: 1,
            ..NewWeeklySummary::new(created_profile.id.clone(), week2_start, week2_end)
        };

        WeeklySummaryQueries::create(&db, summary1).await.unwrap();
        WeeklySummaryQueries::create(&db, summary2).await.unwrap();

        let (compliant, exceeded, avg_time) =
            WeeklySummaryQueries::get_compliance_stats(&db, &created_profile.id, 2).await.unwrap();

        assert_eq!(compliant, 11); // 5 + 6
        assert_eq!(exceeded, 3); // 2 + 1
        assert_eq!(avg_time, 12600.0); // (14400 + 10800) / 2
    }
}
