use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, Utc};
use dots_family_common::types::{ActivityPattern, AlertSeverity, AlertType, BehaviorAlert};
use dots_family_db::Database;
use tracing::{info, warn};
use uuid::Uuid;

#[allow(dead_code)]
pub struct BehaviorAnalyzer {
    db: Database,
    #[allow(dead_code)]
    pattern_cache: HashMap<String, ActivityPattern>,
}

#[allow(dead_code)]
impl BehaviorAnalyzer {
    pub fn new(db: Database) -> Self {
        Self { db, pattern_cache: HashMap::new() }
    }

    /// Analyze activity data and detect patterns
    pub async fn analyze_activity(&mut self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        let mut alerts = Vec::new();

        // Check for application usage spikes
        alerts.extend(self.check_application_spikes(profile_id).await?);

        // Check for repeated policy violations
        alerts.extend(self.check_repeated_violations(profile_id).await?);

        // Check for screen time trends
        alerts.extend(self.check_screen_time_trends(profile_id).await?);

        // Check for off-hours activity
        alerts.extend(self.check_off_hours_activity(profile_id).await?);

        // Check for excessive exception requests
        alerts.extend(self.check_excessive_requests(profile_id).await?);

        Ok(alerts)
    }

    async fn check_application_spikes(&self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        use dots_family_db::queries::activities::ActivityQueries;

        let mut alerts = Vec::new();
        let since = Utc::now() - Duration::hours(24);

        // Get activities for the last 24 hours
        let activities = ActivityQueries::list_by_profile_since(&self.db, profile_id, since)
            .await
            .unwrap_or_default();

        let mut app_usage: HashMap<String, i64> = HashMap::new();

        for activity in activities {
            if !activity.app_id.is_empty() {
                *app_usage.entry(activity.app_id).or_insert(0) += activity.duration_seconds;
            }
        }

        // Check for apps with unusually high usage (> 3 hours in 24 hours)
        for (app_id, total_seconds) in app_usage {
            if total_seconds > 3 * 60 * 60 {
                // 3 hours
                let alert = BehaviorAlert {
                    id: Uuid::new_v4(),
                    profile_id: Uuid::parse_str(profile_id).unwrap_or_default(),
                    pattern_id: Uuid::new_v4(), // Would be actual pattern ID in full implementation
                    alert_type: AlertType::TrendAlert,
                    severity: AlertSeverity::Warning,
                    description: format!(
                        "High usage detected for {}: {} hours in 24 hours",
                        app_id,
                        total_seconds / 3600
                    ),
                    recommendation: Some(format!(
                        "Consider setting time limits for {} or discussing usage with child",
                        app_id
                    )),
                    created_at: Utc::now(),
                    acknowledged_at: None,
                    dismissed: false,
                };
                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    async fn check_repeated_violations(&self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        let mut alerts = Vec::new();
        let since = Utc::now() - Duration::hours(6);

        let pool = self.db.pool()?;
        let violation_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) 
            FROM activities 
            WHERE profile_id = ? 
              AND app_name LIKE '%BLOCKED%'
              AND timestamp >= ?
            "#,
        )
        .bind(profile_id)
        .bind(since)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if violation_count > 5 {
            let alert = BehaviorAlert {
                id: Uuid::new_v4(),
                profile_id: Uuid::parse_str(profile_id).unwrap_or_default(),
                pattern_id: Uuid::new_v4(),
                alert_type: AlertType::ComplianceIssue,
                severity: AlertSeverity::Warning,
                description: format!(
                    "Multiple policy violations detected: {} attempts to access blocked content in 6 hours",
                    violation_count
                ),
                recommendation: Some(
                    "Consider reviewing restrictions or discussing appropriate usage with child"
                        .to_string(),
                ),
                created_at: Utc::now(),
                acknowledged_at: None,
                dismissed: false,
            };
            alerts.push(alert);
        }

        Ok(alerts)
    }

    async fn check_screen_time_trends(&self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        let mut alerts = Vec::new();

        // Get daily screen time for last 7 days
        let pool = self.db.pool()?;
        let daily_usage: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT 
                DATE(timestamp) as day,
                SUM(duration_seconds) as total_seconds
            FROM activities 
            WHERE profile_id = ? 
              AND timestamp >= DATE('now', '-7 days')
            GROUP BY DATE(timestamp)
            ORDER BY day DESC
            "#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if daily_usage.len() >= 3 {
            let recent_avg = daily_usage.iter().take(3).map(|(_, s)| *s).sum::<i64>() / 3;
            let earlier_avg = if daily_usage.len() >= 6 {
                daily_usage.iter().skip(3).take(3).map(|(_, s)| *s).sum::<i64>() / 3
            } else {
                recent_avg
            };

            let increase_ratio =
                if earlier_avg > 0 { recent_avg as f64 / earlier_avg as f64 } else { 1.0 };

            if increase_ratio > 1.5 {
                let alert = BehaviorAlert {
                    id: Uuid::new_v4(),
                    profile_id: Uuid::parse_str(profile_id).unwrap_or_default(),
                    pattern_id: Uuid::new_v4(),
                    alert_type: AlertType::TrendAlert,
                    severity: AlertSeverity::Info,
                    description: format!(
                        "Screen time trending upward: {}% increase over past 3 days",
                        ((increase_ratio - 1.0) * 100.0) as i32
                    ),
                    recommendation: Some(
                        "Monitor for continued increases and consider adjusting limits".to_string(),
                    ),
                    created_at: Utc::now(),
                    acknowledged_at: None,
                    dismissed: false,
                };
                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    async fn check_off_hours_activity(&self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        let mut alerts = Vec::new();
        let since = Utc::now() - Duration::hours(24);

        let pool = self.db.pool()?;
        let late_activity_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) 
            FROM activities 
            WHERE profile_id = ? 
              AND timestamp >= ?
              AND (
                  strftime('%H', timestamp) > '22' 
                  OR strftime('%H', timestamp) < '07'
              )
            "#,
        )
        .bind(profile_id)
        .bind(since)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if late_activity_count > 0 {
            let alert = BehaviorAlert {
                id: Uuid::new_v4(),
                profile_id: Uuid::parse_str(profile_id).unwrap_or_default(),
                pattern_id: Uuid::new_v4(),
                alert_type: AlertType::AnomalyDetected,
                severity: AlertSeverity::Warning,
                description: format!(
                    "Late-night/early-morning activity detected: {} activities between 10 PM - 7 AM",
                    late_activity_count
                ),
                recommendation: Some(
                    "Consider enforcing bedtime restrictions or device-free time".to_string(),
                ),
                created_at: Utc::now(),
                acknowledged_at: None,
                dismissed: false,
            };
            alerts.push(alert);
        }

        Ok(alerts)
    }

    async fn check_excessive_requests(&self, profile_id: &str) -> Result<Vec<BehaviorAlert>> {
        let mut alerts = Vec::new();

        // Get all requests for this profile from the past 24 hours
        let pool = self.db.pool()?;
        let request_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) 
            FROM approval_requests 
            WHERE profile_id = ? 
              AND requested_at >= datetime('now', '-24 hours')
            "#,
        )
        .bind(profile_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if request_count > 10 {
            let alert = BehaviorAlert {
                id: Uuid::new_v4(),
                profile_id: Uuid::parse_str(profile_id).unwrap_or_default(),
                pattern_id: Uuid::new_v4(),
                alert_type: AlertType::TrendAlert,
                severity: AlertSeverity::Info,
                description: format!(
                    "High number of approval requests: {} requests in 24 hours",
                    request_count
                ),
                recommendation: Some(
                    "Consider reviewing current restrictions or discussing expectations with child"
                        .to_string(),
                ),
                created_at: Utc::now(),
                acknowledged_at: None,
                dismissed: false,
            };
            alerts.push(alert);
        }

        Ok(alerts)
    }

    /// Store behavior patterns in database for tracking
    pub async fn store_pattern(&self, pattern: &ActivityPattern) -> Result<()> {
        let pool = self.db.pool()?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO activity_patterns 
            (id, profile_id, pattern_type, description, threshold_value, 
             current_value, detection_window_hours, created_at, last_detected, alert_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(pattern.id.to_string())
        .bind(pattern.profile_id.to_string())
        .bind(serde_json::to_string(&pattern.pattern_type)?)
        .bind(&pattern.description)
        .bind(pattern.threshold_value)
        .bind(pattern.current_value)
        .bind(pattern.detection_window.num_hours())
        .bind(pattern.created_at)
        .bind(pattern.last_detected)
        .bind(pattern.alert_count)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Store behavior alert in database
    pub async fn store_alert(&self, alert: &BehaviorAlert) -> Result<()> {
        let pool = self.db.pool()?;

        sqlx::query(
            r#"
            INSERT INTO behavior_alerts 
            (id, profile_id, pattern_id, alert_type, severity, description, 
             recommendation, created_at, acknowledged_at, dismissed)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(alert.id.to_string())
        .bind(alert.profile_id.to_string())
        .bind(alert.pattern_id.to_string())
        .bind(serde_json::to_string(&alert.alert_type)?)
        .bind(serde_json::to_string(&alert.severity)?)
        .bind(&alert.description)
        .bind(&alert.recommendation)
        .bind(alert.created_at)
        .bind(alert.acknowledged_at)
        .bind(alert.dismissed)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Run periodic analysis for all active profiles
    pub async fn run_periodic_analysis(&mut self) -> Result<()> {
        info!("Starting periodic behavior analysis");

        let pool = self.db.pool()?;
        let active_profiles: Vec<String> =
            sqlx::query_scalar("SELECT id FROM profiles WHERE active = true")
                .fetch_all(pool)
                .await?;

        for profile_id in active_profiles {
            match self.analyze_activity(&profile_id).await {
                Ok(alerts) => {
                    info!("Found {} behavior alerts for profile {}", alerts.len(), profile_id);

                    for alert in alerts {
                        if let Err(e) = self.store_alert(&alert).await {
                            warn!("Failed to store behavior alert: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to analyze behavior for profile {}: {}", profile_id, e);
                }
            }
        }

        info!("Completed periodic behavior analysis");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_behavior_analyzer_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db_config = dots_family_db::DatabaseConfig {
            path: db_path.to_str().unwrap().to_string(),
            encryption_key: None,
        };

        let db = Database::new(db_config).await.unwrap();
        db.run_migrations().await.unwrap();

        let mut analyzer = BehaviorAnalyzer::new(db);

        let alerts = analyzer.analyze_activity("test-profile").await;
        assert!(alerts.is_ok());
        assert!(alerts.unwrap().is_empty()); // No activity data should mean no alerts
    }
}
