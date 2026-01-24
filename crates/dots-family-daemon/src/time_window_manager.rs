use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Local};
use dots_family_common::{
    types::{Profile, TimeWindow},
    AccessResult, TimeWindowConfig, TimeWindowEnforcer,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::notification_manager::NotificationManager;

/// Manages time window enforcement for user sessions
pub struct TimeWindowManager {
    enforcer: Arc<RwLock<Option<TimeWindowEnforcer>>>,
    notification_manager: NotificationManager,
    active_profile: Arc<RwLock<Option<Profile>>>,
    last_warning_sent: Arc<RwLock<Option<DateTime<Local>>>>,
}

impl TimeWindowManager {
    pub fn new(notification_manager: NotificationManager) -> Self {
        info!("Initializing time window manager");
        Self {
            enforcer: Arc::new(RwLock::new(None)),
            notification_manager,
            active_profile: Arc::new(RwLock::new(None)),
            last_warning_sent: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the active profile and configure time window enforcement
    pub async fn set_active_profile(&self, profile: Profile) -> Result<()> {
        info!("Setting active profile for time window enforcement: {}", profile.name);

        // Create TimeWindowConfig from profile
        let config = TimeWindowConfig {
            weekday_windows: profile.config.screen_time.windows.weekday.clone(),
            weekend_windows: profile.config.screen_time.windows.weekend.clone(),
            holiday_windows: Vec::new(), // TODO: Add holiday support to Profile
            grace_period_minutes: 2,     // Default grace period
            warning_minutes: 5,          // Default warning time
        };

        // Create enforcer
        let enforcer = TimeWindowEnforcer::new(config);

        // Update state
        let mut enforcer_lock = self.enforcer.write().await;
        *enforcer_lock = Some(enforcer);

        let mut profile_lock = self.active_profile.write().await;
        *profile_lock = Some(profile);

        Ok(())
    }

    /// Check if current time is within allowed windows
    pub async fn check_access(&self) -> Result<AccessResult> {
        let enforcer_lock = self.enforcer.read().await;
        let enforcer = match enforcer_lock.as_ref() {
            Some(e) => e,
            None => {
                debug!("No time window enforcer configured, allowing access");
                return Ok(AccessResult::Allowed);
            }
        };

        let now = Local::now();
        let result = enforcer.check_access(now);

        debug!("Time window check result: {:?}", result);
        Ok(result)
    }

    /// Check if we should show a warning (session ending soon)
    pub async fn should_warn(&self) -> Result<bool> {
        let enforcer_lock = self.enforcer.read().await;
        let enforcer = match enforcer_lock.as_ref() {
            Some(e) => e,
            None => return Ok(false),
        };

        let now = Local::now();
        let should_warn = enforcer.should_warn(now);

        if should_warn {
            // Check if we've already sent a warning recently
            let last_warning = self.last_warning_sent.read().await;
            if let Some(last) = *last_warning {
                // Don't spam warnings - only send once per minute
                let elapsed = now.signed_duration_since(last);
                if elapsed.num_minutes() < 1 {
                    return Ok(false);
                }
            }
        }

        Ok(should_warn)
    }

    /// Get the warning message to display
    pub async fn get_warning_message(&self) -> Result<Option<String>> {
        let enforcer_lock = self.enforcer.read().await;
        let enforcer = match enforcer_lock.as_ref() {
            Some(e) => e,
            None => return Ok(None),
        };

        let now = Local::now();
        Ok(enforcer.get_warning_message(now))
    }

    /// Send a warning notification
    pub async fn send_warning_notification(&self) -> Result<()> {
        if let Some(message) = self.get_warning_message().await? {
            info!("Sending time window warning: {}", message);

            // TODO: Use NotificationManager to send desktop notification
            // For now, just log
            warn!("TIME WINDOW WARNING: {}", message);

            // Update last warning time
            let mut last_warning = self.last_warning_sent.write().await;
            *last_warning = Some(Local::now());
        }

        Ok(())
    }

    /// Check if session should be locked
    pub async fn should_lock(&self) -> Result<bool> {
        let enforcer_lock = self.enforcer.read().await;
        let enforcer = match enforcer_lock.as_ref() {
            Some(e) => e,
            None => return Ok(false),
        };

        let now = Local::now();
        Ok(enforcer.should_lock(now))
    }

    /// Get the next available time window
    pub async fn get_next_window(&self) -> Result<Option<TimeWindow>> {
        let enforcer_lock = self.enforcer.read().await;
        let enforcer = match enforcer_lock.as_ref() {
            Some(e) => e,
            None => return Ok(None),
        };

        let now = Local::now();
        let result = enforcer.check_access(now);

        // Extract next window from the result
        match result {
            AccessResult::Denied { next_window: Some(next_start), .. } => {
                // We only have the start time, need to find the corresponding window
                // For now, return a placeholder window
                // TODO: Enhance TimeWindowEnforcer to return full TimeWindow object
                Ok(Some(TimeWindow { start: next_start, end: "unknown".to_string() }))
            }
            _ => Ok(None),
        }
    }

    /// Clear the active profile and enforcer
    pub async fn clear_profile(&self) -> Result<()> {
        info!("Clearing active profile from time window manager");

        let mut enforcer_lock = self.enforcer.write().await;
        *enforcer_lock = None;

        let mut profile_lock = self.active_profile.write().await;
        *profile_lock = None;

        let mut last_warning = self.last_warning_sent.write().await;
        *last_warning = None;

        Ok(())
    }

    /// Get the active profile (if any)
    pub async fn get_active_profile(&self) -> Option<Profile> {
        let profile_lock = self.active_profile.read().await;
        profile_lock.clone()
    }
}

#[cfg(test)]
mod tests {
    use dots_family_common::types::{AgeGroup, ProfileConfig, ScreenTimeConfig, TimeWindows};
    use uuid::Uuid;

    use super::*;

    fn create_test_profile() -> Profile {
        Profile {
            id: Uuid::new_v4(),
            name: "Test Child".to_string(),
            age_group: AgeGroup::LateElementary,
            birthday: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 120,
                    weekend_bonus_minutes: 0,
                    exempt_categories: Vec::new(),
                    windows: TimeWindows {
                        weekday: vec![
                            TimeWindow { start: "06:00".to_string(), end: "08:00".to_string() },
                            TimeWindow { start: "15:00".to_string(), end: "19:00".to_string() },
                        ],
                        weekend: vec![TimeWindow {
                            start: "08:00".to_string(),
                            end: "21:00".to_string(),
                        }],
                    },
                },
                applications: Default::default(),
                web_filtering: Default::default(),
                terminal_filtering: Default::default(),
            },
            active: true,
        }
    }

    #[tokio::test]
    async fn test_time_window_manager_creation() {
        let notification_manager = NotificationManager::new();
        let manager = TimeWindowManager::new(notification_manager);

        // Initially no profile
        assert!(manager.get_active_profile().await.is_none());
    }

    #[tokio::test]
    async fn test_set_active_profile() {
        let notification_manager = NotificationManager::new();
        let manager = TimeWindowManager::new(notification_manager);

        let profile = create_test_profile();
        let profile_name = profile.name.clone();

        manager.set_active_profile(profile).await.unwrap();

        let active = manager.get_active_profile().await;
        assert!(active.is_some());
        assert_eq!(active.unwrap().name, profile_name);
    }

    #[tokio::test]
    async fn test_check_access_without_profile() {
        let notification_manager = NotificationManager::new();
        let manager = TimeWindowManager::new(notification_manager);

        // Without a profile, access should be allowed
        let result = manager.check_access().await.unwrap();
        assert!(matches!(result, AccessResult::Allowed));
    }
}
