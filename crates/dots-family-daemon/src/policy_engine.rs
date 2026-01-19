use anyhow::Result;
use chrono::{Datelike, Local, NaiveTime};
use dots_family_common::types::{ApplicationMode, Profile, TimeWindow};
use dots_family_proto::events::ActivityEvent;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub action: String,
    pub reason: String,
    pub blocked: bool,
}

/// Tracks screen time usage for a profile
#[derive(Debug, Clone)]
pub struct ScreenTimeTracker {
    pub daily_usage_minutes: u32,
    pub session_start: Option<SystemTime>,
    pub last_activity: Option<SystemTime>,
}

impl Default for ScreenTimeTracker {
    fn default() -> Self {
        Self { daily_usage_minutes: 0, session_start: None, last_activity: None }
    }
}

impl ScreenTimeTracker {
    pub fn start_session(&mut self) {
        let now = SystemTime::now();
        self.session_start = Some(now);
        self.last_activity = Some(now);
        debug!("Started screen time tracking session");
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Some(SystemTime::now());
    }

    pub fn end_session(&mut self) {
        if let (Some(start), Some(last)) = (self.session_start, self.last_activity) {
            if let Ok(duration) = last.duration_since(start) {
                let minutes = duration.as_secs() / 60;
                self.daily_usage_minutes += minutes as u32;
                debug!(
                    "Ended session, added {} minutes to daily usage (total: {})",
                    minutes, self.daily_usage_minutes
                );
            }
        }
        self.session_start = None;
        self.last_activity = None;
    }

    pub fn get_current_session_minutes(&self) -> u32 {
        if let Some(start) = self.session_start {
            if let Ok(duration) = SystemTime::now().duration_since(start) {
                return (duration.as_secs() / 60) as u32;
            }
        }
        0
    }

    pub fn get_total_usage_today(&self) -> u32 {
        self.daily_usage_minutes + self.get_current_session_minutes()
    }

    pub fn reset_daily_usage(&mut self) {
        self.daily_usage_minutes = 0;
        debug!("Reset daily usage counter");
    }
}

pub struct PolicyEngine {
    active_profile: Option<Profile>,
    screen_time_tracker: ScreenTimeTracker,
}

impl PolicyEngine {
    pub async fn new() -> Result<Self> {
        info!("Initializing policy engine");
        Ok(Self { active_profile: None, screen_time_tracker: ScreenTimeTracker::default() })
    }

    pub async fn set_active_profile(&mut self, profile: Profile) -> Result<()> {
        info!("Setting active profile: {}", profile.name);
        self.active_profile = Some(profile);
        self.screen_time_tracker.reset_daily_usage();
        Ok(())
    }

    pub async fn process_activity(&self, event: ActivityEvent) -> Result<PolicyDecision> {
        debug!("Processing activity event for policy enforcement");

        let profile = match &self.active_profile {
            Some(p) => p,
            None => {
                debug!("No active profile, allowing by default");
                return Ok(PolicyDecision {
                    action: "allow".to_string(),
                    reason: "No active profile".to_string(),
                    blocked: false,
                });
            }
        };

        // First check time-based restrictions
        if let Some(time_decision) = self.check_time_restrictions(profile).await? {
            return Ok(time_decision);
        }

        // Then check application-specific policies
        match event {
            ActivityEvent::WindowFocused { app_id, .. } => {
                self.check_app_policy(profile, &app_id).await
            }
            ActivityEvent::ProcessStarted { executable, .. } => {
                let app_id = executable.split('/').next_back().unwrap_or(&executable);
                self.check_app_policy(profile, app_id).await
            }
            ActivityEvent::NetworkConnection { .. } => Ok(PolicyDecision {
                action: "allow".to_string(),
                reason: "Network activity allowed by default".to_string(),
                blocked: false,
            }),
        }
    }

    async fn check_time_restrictions(&self, profile: &Profile) -> Result<Option<PolicyDecision>> {
        // Check if we're within allowed time windows
        if !self.is_within_allowed_time_window(profile) {
            return Ok(Some(PolicyDecision {
                action: "block".to_string(),
                reason: "Outside allowed time window".to_string(),
                blocked: true,
            }));
        }

        // Check daily screen time limit
        let total_usage = self.screen_time_tracker.get_total_usage_today();
        let daily_limit = self.get_daily_limit(profile);

        if total_usage >= daily_limit {
            return Ok(Some(PolicyDecision {
                action: "block".to_string(),
                reason: format!(
                    "Daily screen time limit exceeded ({} >= {} minutes)",
                    total_usage, daily_limit
                ),
                blocked: true,
            }));
        }

        debug!("Time restrictions passed: {} minutes used of {} limit", total_usage, daily_limit);
        Ok(None)
    }

    fn is_within_allowed_time_window(&self, profile: &Profile) -> bool {
        let now = Local::now();
        let is_weekend = now.weekday().num_days_from_monday() >= 5;

        let time_windows = if is_weekend {
            &profile.config.screen_time.windows.weekend
        } else {
            &profile.config.screen_time.windows.weekday
        };

        if time_windows.is_empty() {
            return true;
        }

        let current_time = now.time();

        for window in time_windows {
            if self.is_time_in_window(&current_time, window) {
                return true;
            }
        }

        false
    }

    fn is_time_in_window(&self, current_time: &chrono::NaiveTime, window: &TimeWindow) -> bool {
        let start_time = match NaiveTime::parse_from_str(&window.start, "%H:%M") {
            Ok(time) => time,
            Err(_) => {
                warn!("Invalid start time format: {}", window.start);
                return false;
            }
        };

        let end_time = match NaiveTime::parse_from_str(&window.end, "%H:%M") {
            Ok(time) => time,
            Err(_) => {
                warn!("Invalid end time format: {}", window.end);
                return false;
            }
        };

        if start_time <= end_time {
            *current_time >= start_time && *current_time <= end_time
        } else {
            *current_time >= start_time || *current_time <= end_time
        }
    }

    fn get_daily_limit(&self, profile: &Profile) -> u32 {
        let now = Local::now();
        let is_weekend = now.weekday().num_days_from_monday() >= 5;

        let base_limit = profile.config.screen_time.daily_limit_minutes;
        if is_weekend {
            base_limit + profile.config.screen_time.weekend_bonus_minutes
        } else {
            base_limit
        }
    }

    pub fn start_activity_session(&mut self) {
        self.screen_time_tracker.start_session();
    }

    pub fn update_activity(&mut self) {
        self.screen_time_tracker.update_activity();
    }

    pub fn end_activity_session(&mut self) {
        self.screen_time_tracker.end_session();
    }

    pub fn get_remaining_screen_time(&self) -> Option<u32> {
        if let Some(profile) = &self.active_profile {
            let daily_limit = self.get_daily_limit(profile);
            let used = self.screen_time_tracker.get_total_usage_today();
            Some(daily_limit.saturating_sub(used))
        } else {
            None
        }
    }

    async fn check_app_policy(&self, profile: &Profile, app_id: &str) -> Result<PolicyDecision> {
        debug!("Checking app policy for: {}", app_id);

        let app_config = &profile.config.applications;

        match app_config.mode {
            ApplicationMode::Allowlist => {
                // In allowlist mode, only explicitly allowed apps are permitted
                if app_config.allowed.contains(&app_id.to_string()) {
                    debug!("App {} is in allowlist", app_id);
                    Ok(PolicyDecision {
                        action: "allow".to_string(),
                        reason: format!("App {} is explicitly allowed", app_id),
                        blocked: false,
                    })
                } else {
                    warn!(
                        "Blocking app: {} - not in allowlist for profile: {}",
                        app_id, profile.name
                    );
                    Ok(PolicyDecision {
                        action: "block".to_string(),
                        reason: format!("App {} is not in allowlist", app_id),
                        blocked: true,
                    })
                }
            }
            ApplicationMode::Blocklist => {
                // In blocklist mode, explicitly blocked apps are denied, everything else allowed
                if app_config.blocked.contains(&app_id.to_string()) {
                    warn!("Blocking app: {} - in blocklist for profile: {}", app_id, profile.name);
                    Ok(PolicyDecision {
                        action: "block".to_string(),
                        reason: format!("App {} is blocked by policy", app_id),
                        blocked: true,
                    })
                } else {
                    debug!("Allowing app: {} - not in blocklist", app_id);
                    Ok(PolicyDecision {
                        action: "allow".to_string(),
                        reason: format!("App {} is not blocked", app_id),
                        blocked: false,
                    })
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_active_profile(&self) -> Option<&Profile> {
        self.active_profile.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use dots_family_common::types::{
        AgeGroup, ApplicationConfig, ProfileConfig, ScreenTimeConfig, TimeWindows,
    };
    use std::time::SystemTime;
    use uuid::Uuid;

    fn create_test_profile(
        age_group: AgeGroup,
        daily_limit: u32,
        time_windows: TimeWindows,
    ) -> Profile {
        Profile {
            id: Uuid::new_v4(),
            name: "test-child".to_string(),
            age_group,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: daily_limit,
                    weekend_bonus_minutes: 30,
                    exempt_categories: vec!["education".to_string()],
                    windows: time_windows,
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Allowlist,
                    allowed: vec!["firefox".to_string()],
                    blocked: vec![],
                    blocked_categories: vec![],
                },
                ..Default::default()
            },
            active: true,
        }
    }

    #[tokio::test]
    async fn test_policy_engine_initialization() {
        let engine = PolicyEngine::new().await;
        assert!(engine.is_ok(), "Policy engine should initialize successfully");
    }

    #[tokio::test]
    async fn test_screen_time_tracker() {
        let mut tracker = ScreenTimeTracker::default();

        tracker.start_session();
        assert!(tracker.session_start.is_some());
        assert!(tracker.last_activity.is_some());

        tracker.update_activity();
        assert!(tracker.last_activity.is_some());

        tracker.daily_usage_minutes = 60;
        tracker.end_session();
        assert!(tracker.session_start.is_none());
        assert!(tracker.last_activity.is_none());
        assert_eq!(tracker.daily_usage_minutes, 60);

        tracker.reset_daily_usage();
        assert_eq!(tracker.daily_usage_minutes, 0);
    }

    #[tokio::test]
    async fn test_daily_limit_enforcement() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = create_test_profile(
            AgeGroup::EarlyElementary,
            60,                                               // 1 hour daily limit
            TimeWindows { weekday: vec![], weekend: vec![] }, // No time window restrictions
        );

        engine.set_active_profile(profile).await.unwrap();
        engine.screen_time_tracker.daily_usage_minutes = 70; // Over limit

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "firefox".to_string(),
            window_title: "Educational Content".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "block");
        assert!(result.blocked);
        assert!(result.reason.contains("screen time limit exceeded"));
    }

    #[tokio::test]
    async fn test_within_daily_limit() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = create_test_profile(
            AgeGroup::EarlyElementary,
            120, // 2 hour daily limit
            TimeWindows { weekday: vec![], weekend: vec![] },
        );

        engine.set_active_profile(profile).await.unwrap();
        engine.screen_time_tracker.daily_usage_minutes = 30; // Well under limit

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "firefox".to_string(),
            window_title: "Educational Content".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "allow");
        assert!(!result.blocked);
        assert!(result.reason.contains("allowed"));
    }

    #[tokio::test]
    async fn test_weekend_bonus_calculation() {
        let engine = PolicyEngine::new().await.unwrap();

        let profile = create_test_profile(
            AgeGroup::LateElementary,
            60, // Base 1 hour
            TimeWindows { weekday: vec![], weekend: vec![] },
        );

        let weekday_limit = engine.get_daily_limit(&profile);
        assert_eq!(weekday_limit, 60);

        // Weekend bonus is handled by is_weekend logic in get_daily_limit
        // This would need date mocking in a real scenario
    }

    #[tokio::test]
    async fn test_remaining_screen_time() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = create_test_profile(
            AgeGroup::EarlyElementary,
            120,
            TimeWindows { weekday: vec![], weekend: vec![] },
        );

        engine.set_active_profile(profile).await.unwrap();
        engine.screen_time_tracker.daily_usage_minutes = 40;

        let remaining = engine.get_remaining_screen_time().unwrap();
        assert_eq!(remaining, 80); // 120 - 40 = 80 minutes remaining
    }

    #[tokio::test]
    async fn test_allowlist_blocked_app() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = Profile {
            id: Uuid::new_v4(),
            name: "test-child".to_string(),
            age_group: AgeGroup::EarlyElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                applications: ApplicationConfig {
                    mode: ApplicationMode::Allowlist,
                    allowed: vec!["firefox".to_string()],
                    blocked: vec![],
                    blocked_categories: vec![],
                },
                ..Default::default()
            },
            active: true,
        };

        engine.set_active_profile(profile).await.unwrap();

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "blocked-app".to_string(),
            window_title: "Blocked Content".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "block");
        assert!(result.blocked);
        assert!(result.reason.contains("allowlist"));
    }

    #[tokio::test]
    async fn test_allowlist_allowed_app() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = Profile {
            id: Uuid::new_v4(),
            name: "test-child".to_string(),
            age_group: AgeGroup::EarlyElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                applications: ApplicationConfig {
                    mode: ApplicationMode::Allowlist,
                    allowed: vec!["allowed-app".to_string()],
                    blocked: vec![],
                    blocked_categories: vec![],
                },
                ..Default::default()
            },
            active: true,
        };

        engine.set_active_profile(profile).await.unwrap();

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "allowed-app".to_string(),
            window_title: "Educational Content".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "allow");
        assert!(!result.blocked);
        assert!(result.reason.contains("allowed"));
    }

    #[tokio::test]
    async fn test_blocklist_mode() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = Profile {
            id: Uuid::new_v4(),
            name: "test-child".to_string(),
            age_group: AgeGroup::LateElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                applications: ApplicationConfig {
                    mode: ApplicationMode::Blocklist,
                    allowed: vec![],
                    blocked: vec!["steam".to_string()],
                    blocked_categories: vec![],
                },
                ..Default::default()
            },
            active: true,
        };

        engine.set_active_profile(profile).await.unwrap();

        // Test blocked app
        let blocked_event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "steam".to_string(),
            window_title: "Steam".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(blocked_event).await.unwrap();
        assert_eq!(result.action, "block");
        assert!(result.blocked);

        // Test allowed app (not in blocklist)
        let allowed_event = ActivityEvent::WindowFocused {
            pid: 5678,
            app_id: "vscode".to_string(),
            window_title: "VS Code".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(allowed_event).await.unwrap();
        assert_eq!(result.action, "allow");
        assert!(!result.blocked);
    }

    #[tokio::test]
    async fn test_process_started_event() {
        let mut engine = PolicyEngine::new().await.unwrap();

        let profile = Profile {
            id: Uuid::new_v4(),
            name: "test-child".to_string(),
            age_group: AgeGroup::EarlyElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                applications: ApplicationConfig {
                    mode: ApplicationMode::Blocklist,
                    allowed: vec![],
                    blocked: vec!["malicious-app".to_string()],
                    blocked_categories: vec![],
                },
                ..Default::default()
            },
            active: true,
        };

        engine.set_active_profile(profile).await.unwrap();

        let event = ActivityEvent::ProcessStarted {
            pid: 9999,
            executable: "/usr/bin/malicious-app".to_string(),
            args: vec!["malicious-app".to_string()],
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "block");
        assert!(result.blocked);
        assert!(result.reason.contains("malicious-app"));
    }

    #[tokio::test]
    async fn test_no_active_profile() {
        let engine = PolicyEngine::new().await.unwrap();

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "any-app".to_string(),
            window_title: "Any Window".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = engine.process_activity(event).await.unwrap();
        assert_eq!(result.action, "allow");
        assert!(!result.blocked);
        assert!(result.reason.contains("No active profile"));
    }
}
