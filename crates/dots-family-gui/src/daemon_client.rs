use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityReport {
    pub date: NaiveDate,
    pub screen_time_minutes: u32,
    pub top_activity: String,
    pub top_category: String,
    pub violations: u32,
    pub blocked_attempts: u32,
    pub apps_used: Vec<AppUsage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppUsage {
    pub app_id: String,
    pub app_name: String,
    pub category: String,
    pub duration_minutes: u32,
    pub percentage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeeklyReport {
    pub week_start: NaiveDate,
    pub total_screen_time_minutes: u32,
    pub average_daily_minutes: u32,
    pub most_active_day: String,
    pub top_categories: Vec<CategoryUsage>,
    pub policy_violations: u32,
    pub educational_percentage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryUsage {
    pub category: String,
    pub duration_minutes: u32,
    pub percentage: f32,
}

#[derive(Clone)]
pub struct DaemonClient {
    connected: Arc<Mutex<bool>>,
}

impl DaemonClient {
    pub async fn new() -> Self {
        Self { connected: Arc::new(Mutex::new(false)) }
    }

    pub async fn connect(&self) -> Result<()> {
        *self.connected.lock().await = true;
        Ok(())
    }

    pub async fn get_active_profile(&self) -> Result<String> {
        Ok("Alice".to_string())
    }

    pub async fn get_remaining_time(&self) -> Result<u32> {
        Ok(85)
    }

    pub async fn check_application_allowed(&self, _app_id: &str) -> Result<bool> {
        Ok(true)
    }

    pub async fn list_profiles(&self) -> Result<String> {
        Ok(r#"[{"id": "1", "name": "Alice"}]"#.to_string())
    }

    pub async fn set_active_profile(&self, _profile_id: &str) -> Result<()> {
        Ok(())
    }

    pub async fn authenticate_parent(&self, _password: &str) -> Result<String> {
        Ok("token123".to_string())
    }

    pub async fn get_daily_report(
        &self,
        profile_id: &str,
        date: NaiveDate,
    ) -> Result<ActivityReport> {
        // TODO: Connect to actual daemon/database
        // For now, generate realistic mock data based on the date

        let screen_time = match date.weekday() {
            Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri => {
                90 + (date.day() % 3) * 20
            }
            _ => 150 + (date.day() % 4) * 25,
        };

        Ok(ActivityReport {
            date,
            screen_time_minutes: screen_time,
            top_activity: "Educational Apps".to_string(),
            top_category: "Education".to_string(),
            violations: if date.day() % 7 == 0 { 1 } else { 0 },
            blocked_attempts: if date.day() % 5 == 0 { 2 } else { 0 },
            apps_used: vec![
                AppUsage {
                    app_id: "org.gnome.Calculator".to_string(),
                    app_name: "Calculator".to_string(),
                    category: "Education".to_string(),
                    duration_minutes: screen_time * 40 / 100,
                    percentage: 40.0,
                },
                AppUsage {
                    app_id: "firefox".to_string(),
                    app_name: "Firefox".to_string(),
                    category: "Web Browser".to_string(),
                    duration_minutes: screen_time * 35 / 100,
                    percentage: 35.0,
                },
                AppUsage {
                    app_id: "org.gnome.TextEditor".to_string(),
                    app_name: "Text Editor".to_string(),
                    category: "Productivity".to_string(),
                    duration_minutes: screen_time * 25 / 100,
                    percentage: 25.0,
                },
            ],
        })
    }

    pub async fn get_weekly_report(
        &self,
        profile_id: &str,
        week_start: NaiveDate,
    ) -> Result<WeeklyReport> {
        // TODO: Connect to actual daemon/database
        // Generate realistic aggregated weekly data

        let total_minutes = 980; // About 2.3 hours per day average

        Ok(WeeklyReport {
            week_start,
            total_screen_time_minutes: total_minutes,
            average_daily_minutes: total_minutes / 7,
            most_active_day: "Saturday".to_string(),
            top_categories: vec![
                CategoryUsage {
                    category: "Education".to_string(),
                    duration_minutes: 392, // 40%
                    percentage: 40.0,
                },
                CategoryUsage {
                    category: "Web Browser".to_string(),
                    duration_minutes: 294, // 30%
                    percentage: 30.0,
                },
                CategoryUsage {
                    category: "Productivity".to_string(),
                    duration_minutes: 196, // 20%
                    percentage: 20.0,
                },
                CategoryUsage {
                    category: "Games".to_string(),
                    duration_minutes: 98, // 10%
                    percentage: 10.0,
                },
            ],
            policy_violations: 2,
            educational_percentage: 40.0,
        })
    }

    pub async fn export_reports(
        &self,
        profile_id: &str,
        format: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<String> {
        // TODO: Connect to actual daemon/database
        match format {
            "json" => {
                let mut reports = Vec::new();
                let mut current_date = start_date;

                while current_date <= end_date {
                    let report = self.get_daily_report(profile_id, current_date).await?;
                    reports.push(report);
                    current_date = current_date + Duration::days(1);
                }

                Ok(serde_json::to_string_pretty(&reports)?)
            }
            "csv" => {
                let mut csv_content = String::from("Date,Screen Time (minutes),Top Activity,Top Category,Violations,Blocked Attempts\n");
                let mut current_date = start_date;

                while current_date <= end_date {
                    let report = self.get_daily_report(profile_id, current_date).await?;
                    csv_content.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        report.date,
                        report.screen_time_minutes,
                        report.top_activity,
                        report.top_category,
                        report.violations,
                        report.blocked_attempts
                    ));
                    current_date = current_date + Duration::days(1);
                }

                Ok(csv_content)
            }
            _ => Err(anyhow::anyhow!("Unsupported export format: {}", format)),
        }
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }
}
