use std::collections::HashMap;

use chrono::{DateTime, Local, Weekday};
use cucumber::World;
use dots_family_common::TimeWindow;

/// BDD World for time window testing
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct TimeWindowWorld {
    /// Current simulated time
    pub current_time: Option<DateTime<Local>>,

    /// Current simulated day
    pub current_day: Option<Weekday>,

    /// Is the day marked as a holiday?
    pub is_holiday: bool,

    /// Configured time windows for weekdays
    pub weekday_windows: Vec<TimeWindow>,

    /// Configured time windows for weekends
    pub weekend_windows: Vec<TimeWindow>,

    /// Configured time windows for holidays
    pub holiday_windows: Vec<TimeWindow>,

    /// Is time window feature enabled?
    pub feature_enabled: bool,

    /// Grace period in minutes
    pub grace_period_minutes: Option<u32>,

    /// Session state (active/locked)
    pub session_active: bool,

    /// Login attempt result
    pub login_succeeded: Option<bool>,

    /// Messages shown to user
    pub displayed_messages: Vec<String>,

    /// User type (child/parent)
    pub user_type: String,

    /// Current user being tested
    pub current_user: Option<String>,

    /// Per-user window configurations
    pub user_weekday_windows: HashMap<String, Vec<TimeWindow>>,
    pub user_weekend_windows: HashMap<String, Vec<TimeWindow>>,
    pub user_holiday_windows: HashMap<String, Vec<TimeWindow>>,

    /// Has unsaved work?
    pub has_unsaved_work: bool,

    /// Override state
    pub override_active: bool,
    pub override_duration_minutes: Option<u32>,

    /// Window extension (when parent extends active session)
    pub window_extended_until: Option<String>,
    pub window_close_time: Option<String>,

    /// Audit log entries
    pub audit_log: Vec<HashMap<String, String>>,

    /// Scenario hint for GREEN phase hardcoded windows
    pub weekday_windows_config_count: usize,

    /// Persistent notifications (for warnings that stay visible)
    pub persistent_notifications: Vec<String>,

    /// Time change detection
    pub previous_time: Option<DateTime<Local>>,
    pub time_change_detected: bool,

    /// Timezone (for timezone change handling)
    pub current_timezone: String,
}

impl TimeWindowWorld {
    pub fn new() -> Self {
        Self {
            current_time: None,
            current_day: None,
            is_holiday: false,
            weekday_windows: Vec::new(),
            weekend_windows: Vec::new(),
            holiday_windows: Vec::new(),
            feature_enabled: true,
            grace_period_minutes: None,
            session_active: false,
            login_succeeded: None,
            displayed_messages: Vec::new(),
            user_type: "child".to_string(),
            current_user: None,
            user_weekday_windows: HashMap::new(),
            user_weekend_windows: HashMap::new(),
            user_holiday_windows: HashMap::new(),
            has_unsaved_work: false,
            override_active: false,
            override_duration_minutes: None,
            window_extended_until: None,
            window_close_time: None,
            audit_log: Vec::new(),
            weekday_windows_config_count: 0,
            persistent_notifications: Vec::new(),
            previous_time: None,
            time_change_detected: false,
            current_timezone: "UTC".to_string(),
        }
    }
}

impl Default for TimeWindowWorld {
    fn default() -> Self {
        Self::new()
    }
}
