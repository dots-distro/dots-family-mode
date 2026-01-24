use std::sync::Arc;

use anyhow::Result;
use dots_family_common::AccessResult;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{enforcement::EnforcementEngine, time_window_manager::TimeWindowManager};

/// Handles periodic time window enforcement checks
pub struct TimeWindowEnforcementTask {
    time_window_manager: Arc<TimeWindowManager>,
    enforcement_engine: Arc<RwLock<EnforcementEngine>>,
    last_warning_sent: Arc<RwLock<bool>>,
    session_locked: Arc<RwLock<bool>>,
}

impl TimeWindowEnforcementTask {
    pub fn new(
        time_window_manager: Arc<TimeWindowManager>,
        enforcement_engine: Arc<RwLock<EnforcementEngine>>,
    ) -> Self {
        Self {
            time_window_manager,
            enforcement_engine,
            last_warning_sent: Arc::new(RwLock::new(false)),
            session_locked: Arc::new(RwLock::new(false)),
        }
    }

    /// Run one iteration of time window enforcement check
    pub async fn check_and_enforce(&self) -> Result<()> {
        // Check if we have an active profile
        let profile = self.time_window_manager.get_active_profile().await;
        if profile.is_none() {
            debug!("No active profile for time window enforcement");
            return Ok(());
        }

        let profile = profile.unwrap();
        debug!("Checking time window enforcement for profile: {}", profile.name);

        // Check current access status
        let access_result = self.time_window_manager.check_access().await?;

        match access_result {
            AccessResult::Allowed => {
                debug!("Access allowed - within time window");

                // Reset session locked flag if it was locked
                let mut locked = self.session_locked.write().await;
                if *locked {
                    info!("Session was locked but now in valid time window - unlocked");
                    *locked = false;
                }

                // Check if we should send a warning (window ending soon)
                if self.time_window_manager.should_warn().await? {
                    let warning_sent = self.last_warning_sent.read().await;
                    if !*warning_sent {
                        // Send warning notification using TimeWindowManager
                        self.time_window_manager.send_warning_notification().await?;

                        // Mark that we sent the warning
                        drop(warning_sent);
                        let mut warning_sent_mut = self.last_warning_sent.write().await;
                        *warning_sent_mut = true;
                    }
                } else {
                    // Reset warning flag when we're not in warning period
                    let mut warning_sent = self.last_warning_sent.write().await;
                    *warning_sent = false;
                }
            }
            AccessResult::Denied { reason, next_window } => {
                warn!("Access denied - outside time window: {}", reason);

                // Check if we should lock the session
                if self.time_window_manager.should_lock().await? {
                    let locked = self.session_locked.read().await;
                    if !*locked {
                        // Lock the session
                        info!("Locking session for user: {}", profile.name);

                        // Send notification before locking
                        self.send_lockout_notification(&reason, next_window.as_deref()).await?;

                        // Lock the session
                        let enforcement = self.enforcement_engine.read().await;

                        // Use username if available, otherwise use profile name (fallback)
                        let user_to_lock = profile.username.as_deref().or(Some(&profile.name));
                        enforcement.lock_session(user_to_lock).await?;

                        // Mark session as locked
                        drop(locked);
                        let mut locked_mut = self.session_locked.write().await;
                        *locked_mut = true;

                        info!("Session locked for user: {}", profile.name);
                    } else {
                        debug!("Session already locked for user: {}", profile.name);
                    }
                }
            }
        }

        Ok(())
    }

    /// Send warning notification that window is ending soon
    async fn send_warning_notification(&self) -> Result<()> {
        if let Some(warning_message) = self.time_window_manager.get_warning_message().await? {
            info!("Sending time window warning: {}", warning_message);

            let enforcement = self.enforcement_engine.read().await;
            enforcement.notify_user("Time Window Ending Soon", &warning_message).await?;
        }

        Ok(())
    }

    /// Send notification that session is being locked
    async fn send_lockout_notification(
        &self,
        reason: &str,
        next_window: Option<&str>,
    ) -> Result<()> {
        let message = if let Some(next) = next_window {
            format!("{}\n\nNext available time: {}", reason, next)
        } else {
            reason.to_string()
        };

        info!("Sending lockout notification: {}", message);

        let enforcement = self.enforcement_engine.read().await;
        enforcement.notify_user("Time Window Ended", &message).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{enforcement::EnforcementEngine, notification_manager::NotificationManager};

    #[tokio::test]
    async fn test_enforcement_task_creation() {
        let notification_manager = NotificationManager::new();
        let time_window_manager = Arc::new(TimeWindowManager::new(notification_manager));
        let enforcement_engine = Arc::new(RwLock::new(EnforcementEngine::new(true)));

        let task = TimeWindowEnforcementTask::new(time_window_manager, enforcement_engine);

        // Should not crash when checking with no active profile
        let result = task.check_and_enforce().await;
        assert!(result.is_ok());
    }
}
