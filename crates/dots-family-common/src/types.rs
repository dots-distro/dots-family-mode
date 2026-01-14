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
                    windows: TimeWindows { weekday: vec![], weekend: vec![] },
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

    #[test]
    fn test_profile_serialization_roundtrip() {
        let original = Profile {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            age_group: AgeGroup::HighSchool,
            birthday: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 180,
                    weekend_bonus_minutes: 120,
                    exempt_categories: vec!["education".to_string(), "creativity".to_string()],
                    windows: TimeWindows {
                        weekday: vec![TimeWindow {
                            start: "16:00".to_string(),
                            end: "21:00".to_string(),
                        }],
                        weekend: vec![TimeWindow {
                            start: "09:00".to_string(),
                            end: "22:00".to_string(),
                        }],
                    },
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Blocklist,
                    allowed: vec![],
                    blocked: vec!["discord".to_string(), "tiktok".to_string()],
                    blocked_categories: vec!["social-media".to_string(), "games".to_string()],
                },
                web_filtering: WebFilteringConfig {
                    enabled: true,
                    safe_search: true,
                    blocked_categories: vec!["adult".to_string(), "gambling".to_string()],
                    allowed_domains: vec!["khan-academy.org".to_string()],
                    blocked_domains: vec!["reddit.com".to_string()],
                },
            },
            active: true,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Profile = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.age_group, deserialized.age_group);
        assert_eq!(original.active, deserialized.active);
        assert_eq!(
            original.config.screen_time.daily_limit_minutes,
            deserialized.config.screen_time.daily_limit_minutes
        );
        assert_eq!(original.config.applications.mode, deserialized.config.applications.mode);
    }

    #[test]
    fn test_activity_type_variants() {
        let app_usage = ActivityType::ApplicationUsage;
        let json = serde_json::to_string(&app_usage).unwrap();
        assert!(json.contains("application_usage"));

        let web_browsing = ActivityType::WebBrowsing { url: "https://example.com".to_string() };
        let json = serde_json::to_string(&web_browsing).unwrap();
        assert!(json.contains("web_browsing"));
        assert!(json.contains("https://example.com"));

        let terminal = ActivityType::TerminalCommand { command: "rm -rf /".to_string() };
        let json = serde_json::to_string(&terminal).unwrap();
        assert!(json.contains("terminal_command"));
        assert!(json.contains("rm -rf /"));

        let violation = ActivityType::PolicyViolation { reason: "Time limit exceeded".to_string() };
        let json = serde_json::to_string(&violation).unwrap();
        assert!(json.contains("policy_violation"));
    }

    #[test]
    fn test_activity_creation() {
        let activity = Activity {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            activity_type: ActivityType::ApplicationUsage,
            application: Some("firefox".to_string()),
            window_title: Some("GitHub - Mozilla Firefox".to_string()),
            duration_seconds: 300,
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("firefox"));
        assert!(json.contains("GitHub"));

        let deserialized: Activity = serde_json::from_str(&json).unwrap();
        assert_eq!(activity.id, deserialized.id);
        assert_eq!(activity.profile_id, deserialized.profile_id);
        assert_eq!(activity.duration_seconds, deserialized.duration_seconds);
    }

    #[test]
    fn test_time_window_validation() {
        let window = TimeWindow { start: "09:00".to_string(), end: "17:00".to_string() };

        let json = serde_json::to_string(&window).unwrap();
        let deserialized: TimeWindow = serde_json::from_str(&json).unwrap();

        assert_eq!(window.start, deserialized.start);
        assert_eq!(window.end, deserialized.end);
    }

    #[test]
    fn test_application_config_allowlist() {
        let config = ApplicationConfig {
            mode: ApplicationMode::Allowlist,
            allowed: vec!["firefox".to_string(), "vscode".to_string()],
            blocked: vec![],
            blocked_categories: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("allowlist"));
        assert!(json.contains("firefox"));
        assert!(json.contains("vscode"));
    }

    #[test]
    fn test_application_config_blocklist() {
        let config = ApplicationConfig {
            mode: ApplicationMode::Blocklist,
            allowed: vec![],
            blocked: vec!["steam".to_string(), "discord".to_string()],
            blocked_categories: vec!["games".to_string(), "social-media".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("blocklist"));
        assert!(json.contains("steam"));
        assert!(json.contains("games"));
    }

    #[test]
    fn test_web_filtering_config() {
        let config = WebFilteringConfig {
            enabled: true,
            safe_search: true,
            blocked_categories: vec!["adult".to_string(), "violence".to_string()],
            allowed_domains: vec!["wikipedia.org".to_string()],
            blocked_domains: vec!["4chan.org".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebFilteringConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.safe_search, deserialized.safe_search);
        assert_eq!(config.blocked_categories.len(), deserialized.blocked_categories.len());
    }

    #[test]
    fn test_screen_time_config() {
        let config = ScreenTimeConfig {
            daily_limit_minutes: 120,
            weekend_bonus_minutes: 60,
            exempt_categories: vec!["education".to_string()],
            windows: TimeWindows {
                weekday: vec![TimeWindow { start: "16:00".to_string(), end: "20:00".to_string() }],
                weekend: vec![],
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ScreenTimeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.daily_limit_minutes, deserialized.daily_limit_minutes);
        assert_eq!(config.weekend_bonus_minutes, deserialized.weekend_bonus_minutes);
        assert_eq!(config.exempt_categories, deserialized.exempt_categories);
    }

    #[test]
    fn test_all_age_groups() {
        let groups =
            vec![AgeGroup::EarlyElementary, AgeGroup::LateElementary, AgeGroup::HighSchool];

        for group in groups {
            let json = serde_json::to_string(&group).unwrap();
            let deserialized: AgeGroup = serde_json::from_str(&json).unwrap();
            assert_eq!(group, deserialized);
        }
    }
}
