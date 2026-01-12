use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeGroup {
    #[serde(rename = "5-7")]
    EarlyElementary,
    #[serde(rename = "8-12")]
    LateElementary,
    #[serde(rename = "13-17")]
    HighSchool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub age_group: AgeGroup,
    pub birthday: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: ProfileConfig,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub screen_time: ScreenTimeConfig,
    pub applications: ApplicationConfig,
    pub web_filtering: WebFilteringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenTimeConfig {
    pub daily_limit_minutes: u32,
    pub weekend_bonus_minutes: u32,
    pub exempt_categories: Vec<String>,
    pub windows: TimeWindows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindows {
    pub weekday: Vec<TimeWindow>,
    pub weekend: Vec<TimeWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub mode: ApplicationMode,
    pub allowed: Vec<String>,
    pub blocked: Vec<String>,
    pub blocked_categories: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationMode {
    Allowlist,
    Blocklist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFilteringConfig {
    pub enabled: bool,
    pub safe_search: bool,
    pub blocked_categories: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub activity_type: ActivityType,
    pub application: Option<String>,
    pub window_title: Option<String>,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityType {
    ApplicationUsage,
    WebBrowsing { url: String },
    TerminalCommand { command: String },
    PolicyViolation { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_age_group_serialization() {
        let age = AgeGroup::EarlyElementary;
        let json = serde_json::to_string(&age).unwrap();
        assert_eq!(json, r#""5-7""#);

        let deserialized: AgeGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AgeGroup::EarlyElementary);
    }

    #[test]
    fn test_application_mode_serialization() {
        let mode = ApplicationMode::Allowlist;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""allowlist""#);
    }

    #[test]
    fn test_profile_creation() {
        let profile = Profile {
            id: Uuid::new_v4(),
            name: "Test Child".to_string(),
            age_group: AgeGroup::LateElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 120,
                    weekend_bonus_minutes: 60,
                    exempt_categories: vec!["education".to_string()],
                    windows: TimeWindows {
                        weekday: vec![],
                        weekend: vec![],
                    },
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Allowlist,
                    allowed: vec!["firefox".to_string()],
                    blocked: vec![],
                    blocked_categories: vec![],
                },
                web_filtering: WebFilteringConfig {
                    enabled: true,
                    safe_search: true,
                    blocked_categories: vec![],
                    allowed_domains: vec![],
                    blocked_domains: vec![],
                },
            },
            active: true,
        };

        let json = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json.contains("Test Child"));
    }
}
