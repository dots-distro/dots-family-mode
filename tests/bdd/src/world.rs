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

    /// Has unsaved work?
    pub has_unsaved_work: bool,

    /// Override state
    pub override_active: bool,
    pub override_duration_minutes: Option<u32>,

    /// Audit log entries
    pub audit_log: Vec<HashMap<String, String>>,
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
            has_unsaved_work: false,
            override_active: false,
            override_duration_minutes: None,
            audit_log: Vec::new(),
        }
    }
}

impl Default for TimeWindowWorld {
    fn default() -> Self {
        Self::new()
    }
}
