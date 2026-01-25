// Time Window Enforcement Module
//
// This module implements the logic for enforcing time-based access controls
// based on weekday, weekend, and holiday schedules.

use chrono::{DateTime, Datelike, Local, NaiveTime, Timelike, Weekday};

use crate::types::TimeWindow;

/// Configuration for time window enforcement
#[derive(Debug, Clone)]
pub struct TimeWindowConfig {
    pub weekday_windows: Vec<TimeWindow>,
    pub weekend_windows: Vec<TimeWindow>,
    pub holiday_windows: Vec<TimeWindow>,
    pub grace_period_minutes: u32,
    pub warning_minutes: u32,
}

impl Default for TimeWindowConfig {
    fn default() -> Self {
        Self {
            weekday_windows: Vec::new(),
            weekend_windows: Vec::new(),
            holiday_windows: Vec::new(),
            grace_period_minutes: 2,
            warning_minutes: 5,
        }
    }
}

/// Result of checking if access is allowed
#[derive(Debug, Clone, PartialEq)]
pub enum AccessResult {
    /// Access is allowed
    Allowed,
    /// Access is denied with reason and next available window
    Denied { reason: String, next_window: Option<String> },
}

/// Time window enforcement engine
pub struct TimeWindowEnforcer {
    config: TimeWindowConfig,
    is_holiday: bool,
}

impl TimeWindowEnforcer {
    pub fn new(config: TimeWindowConfig) -> Self {
        Self { config, is_holiday: false }
    }

    pub fn with_holiday(mut self, is_holiday: bool) -> Self {
        self.is_holiday = is_holiday;
        self
    }

    /// Check if access is allowed at the given time
    pub fn check_access(&self, current_time: DateTime<Local>) -> AccessResult {
        let weekday = current_time.weekday();
        let time_str = current_time.format("%H:%M").to_string();

        // Determine which set of windows to use
        let windows = if self.is_holiday {
            &self.config.holiday_windows
        } else if is_weekend(weekday) {
            &self.config.weekend_windows
        } else {
            &self.config.weekday_windows
        };

        // Check if we're in any allowed window
        if self.is_in_window(&time_str, windows) {
            return AccessResult::Allowed;
        }

        // Find next available window
        let next_window = self.find_next_window(&time_str, windows);
        let reason = self.format_denial_reason(windows, next_window.as_deref());

        AccessResult::Denied { reason, next_window }
    }

    /// Check if the given time is within any of the windows
    fn is_in_window(&self, time_str: &str, windows: &[TimeWindow]) -> bool {
        let current_time = match parse_time(time_str) {
            Ok(t) => t,
            Err(_) => return false,
        };

        for window in windows {
            let start = match parse_time(&window.start) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let end = match parse_time(&window.end) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if current_time >= start && current_time < end {
                return true;
            }
        }

        false
    }

    /// Find the next available window after the given time
    fn find_next_window(&self, time_str: &str, windows: &[TimeWindow]) -> Option<String> {
        let current_time = parse_time(time_str).ok()?;

        let mut next: Option<NaiveTime> = None;

        for window in windows {
            if let Ok(start_time) = parse_time(&window.start) {
                if start_time > current_time {
                    match next {
                        None => next = Some(start_time),
                        Some(n) if start_time < n => next = Some(start_time),
                        _ => {}
                    }
                }
            }
        }

        next.map(|t| format!("{:02}:{:02}", t.hour(), t.minute()))
    }

    /// Format a denial reason message
    fn format_denial_reason(&self, windows: &[TimeWindow], next_window: Option<&str>) -> String {
        if windows.is_empty() {
            return "No time windows configured for today".to_string();
        }

        // Check if we're before the first window of the day
        if let Some(next) = next_window {
            // If next window is the first window (no windows have passed yet)
            let is_before_first = windows.iter().all(|w| {
                parse_time(&w.end)
                    .ok()
                    .and_then(|end_time| {
                        parse_time(next).ok().map(|next_time| next_time <= end_time)
                    })
                    .unwrap_or(false)
            });

            if is_before_first && windows.len() == 1 {
                return format!("Computer access starts at {}", next);
            }
        }

        // Otherwise, show all windows as ranges
        let ranges: Vec<String> =
            windows.iter().map(|w| format!("{}-{}", w.start, w.end)).collect();
        format!("Computer access is restricted to: {}", ranges.join(", "))
    }

    /// Check if a warning should be displayed (window closing soon)
    pub fn should_warn(&self, current_time: DateTime<Local>) -> bool {
        let weekday = current_time.weekday();
        let time_str = current_time.format("%H:%M").to_string();

        let windows = if self.is_holiday {
            &self.config.holiday_windows
        } else if is_weekend(weekday) {
            &self.config.weekend_windows
        } else {
            &self.config.weekday_windows
        };

        self.is_warning_time(&time_str, windows)
    }

    /// Check if we're in warning period (N minutes before window end)
    fn is_warning_time(&self, time_str: &str, windows: &[TimeWindow]) -> bool {
        let current_time = match parse_time(time_str) {
            Ok(t) => t,
            Err(_) => return false,
        };

        for window in windows {
            let end = match parse_time(&window.end) {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Check if we're within the warning period before window end
            let warning_duration = chrono::Duration::minutes(self.config.warning_minutes as i64);
            let warning_start = end - warning_duration;

            if current_time >= warning_start && current_time < end {
                return true;
            }
        }

        false
    }

    /// Get warning message with minutes remaining
    pub fn get_warning_message(&self, current_time: DateTime<Local>) -> Option<String> {
        if !self.should_warn(current_time) {
            return None;
        }

        Some(format!("{} minutes remaining in this window", self.config.warning_minutes))
    }

    /// Check if session should be locked (outside window or at window end)
    pub fn should_lock(&self, current_time: DateTime<Local>) -> bool {
        matches!(self.check_access(current_time), AccessResult::Denied { .. })
    }
}

/// Helper function to check if a weekday is a weekend day
fn is_weekend(day: Weekday) -> bool {
    matches!(day, Weekday::Sat | Weekday::Sun)
}

/// Helper function to parse time string in HH:MM format
fn parse_time(time_str: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_str, "%H:%M").map_err(|e| format!("Invalid time format: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_windows(ranges: &[(&str, &str)]) -> Vec<TimeWindow> {
        ranges
            .iter()
            .map(|(start, end)| TimeWindow { start: start.to_string(), end: end.to_string() })
            .collect()
    }

    #[test]
    fn test_parse_time() {
        assert!(parse_time("10:00").is_ok());
        assert!(parse_time("23:59").is_ok());
        assert!(parse_time("00:00").is_ok());
        assert!(parse_time("25:00").is_err());
        assert!(parse_time("invalid").is_err());
    }

    #[test]
    fn test_is_weekend() {
        assert!(!is_weekend(Weekday::Mon));
        assert!(!is_weekend(Weekday::Fri));
        assert!(is_weekend(Weekday::Sat));
        assert!(is_weekend(Weekday::Sun));
    }

    #[test]
    fn test_is_in_window() {
        let windows = make_windows(&[("06:00", "08:00"), ("15:00", "19:00")]);
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig {
            weekday_windows: windows,
            ..Default::default()
        });

        assert!(enforcer.is_in_window("07:00", &enforcer.config.weekday_windows));
        assert!(enforcer.is_in_window("16:00", &enforcer.config.weekday_windows));
        assert!(!enforcer.is_in_window("10:00", &enforcer.config.weekday_windows));
        assert!(!enforcer.is_in_window("08:00", &enforcer.config.weekday_windows));
    }

    #[test]
    fn test_find_next_window() {
        let windows = make_windows(&[("06:00", "08:00"), ("15:00", "19:00")]);
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig {
            weekday_windows: windows,
            ..Default::default()
        });

        assert_eq!(
            enforcer.find_next_window("10:00", &enforcer.config.weekday_windows),
            Some("15:00".to_string())
        );
        assert_eq!(
            enforcer.find_next_window("05:00", &enforcer.config.weekday_windows),
            Some("06:00".to_string())
        );
        assert_eq!(enforcer.find_next_window("20:00", &enforcer.config.weekday_windows), None);
    }

    #[test]
    fn test_access_allowed_in_window() {
        let windows = make_windows(&[("06:00", "08:00")]);
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig {
            weekday_windows: windows,
            ..Default::default()
        });

        // Use a specific Monday to ensure weekday windows are checked
        let time = chrono::NaiveDate::from_ymd_opt(2026, 1, 19) // Monday, Jan 19, 2026
            .unwrap()
            .and_hms_opt(7, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        assert_eq!(enforcer.check_access(time), AccessResult::Allowed);
    }

    #[test]
    fn test_access_denied_outside_window() {
        let windows = make_windows(&[("06:00", "08:00"), ("15:00", "19:00")]);
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig {
            weekday_windows: windows,
            ..Default::default()
        });

        // Use a specific Monday to ensure weekday windows are checked
        let time = chrono::NaiveDate::from_ymd_opt(2026, 1, 19) // Monday, Jan 19, 2026
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        match enforcer.check_access(time) {
            AccessResult::Denied { reason, next_window } => {
                // The reason contains the window ranges, and should include "15:00"
                assert!(
                    reason.contains("15:00"),
                    "Expected reason to contain '15:00', got: {}",
                    reason
                );
                assert_eq!(next_window, Some("15:00".to_string()));
            }
            _ => panic!("Expected access to be denied"),
        }
    }

    #[test]
    fn test_holiday_overrides_weekday() {
        let config = TimeWindowConfig {
            weekday_windows: make_windows(&[("06:00", "08:00")]),
            holiday_windows: make_windows(&[("08:00", "21:00")]),
            ..Default::default()
        };

        let enforcer = TimeWindowEnforcer::new(config).with_holiday(true);

        let time = Local::now()
            .date_naive()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        assert_eq!(enforcer.check_access(time), AccessResult::Allowed);
    }

    #[test]
    fn test_empty_windows_denies_access() {
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig::default());

        let time = Local::now()
            .date_naive()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();

        match enforcer.check_access(time) {
            AccessResult::Denied { reason, .. } => {
                assert!(reason.contains("No time windows configured"));
            }
            _ => panic!("Expected access to be denied with empty windows"),
        }
    }

    #[test]
    fn test_warning_before_window_end() {
        let windows = make_windows(&[("15:00", "19:00")]);
        let enforcer = TimeWindowEnforcer::new(TimeWindowConfig {
            weekday_windows: windows,
            warning_minutes: 5,
            ..Default::default()
        });

        assert!(enforcer.is_warning_time("18:55", &enforcer.config.weekday_windows));
        assert!(enforcer.is_warning_time("18:56", &enforcer.config.weekday_windows));
        assert!(!enforcer.is_warning_time("18:54", &enforcer.config.weekday_windows));
        assert!(!enforcer.is_warning_time("19:00", &enforcer.config.weekday_windows));
    }
}
