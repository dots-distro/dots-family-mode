use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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
