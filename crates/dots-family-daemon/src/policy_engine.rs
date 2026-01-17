use anyhow::Result;
use dots_family_common::types::{ApplicationMode, Profile};
use dots_family_proto::events::ActivityEvent;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub action: String,
    pub reason: String,
    pub blocked: bool,
}

pub struct PolicyEngine {
    active_profile: Option<Profile>,
}

impl PolicyEngine {
    pub async fn new() -> Result<Self> {
        info!("Initializing policy engine");
        Ok(Self { active_profile: None })
    }

    pub async fn set_active_profile(&mut self, profile: Profile) -> Result<()> {
        info!("Setting active profile: {}", profile.name);
        self.active_profile = Some(profile);
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

        match event {
            ActivityEvent::WindowFocused { app_id, .. } => {
                self.check_app_policy(profile, &app_id).await
            }
            ActivityEvent::ProcessStarted { executable, .. } => {
                // For process started, use the executable name as app_id
                let app_id = executable.split('/').last().unwrap_or(&executable);
                self.check_app_policy(profile, app_id).await
            }
            ActivityEvent::NetworkConnection { .. } => {
                // Network connections are allowed by default for now
                Ok(PolicyDecision {
                    action: "allow".to_string(),
                    reason: "Network activity allowed by default".to_string(),
                    blocked: false,
                })
            }
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

    pub async fn get_active_profile(&self) -> Option<&Profile> {
        self.active_profile.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use dots_family_common::types::{AgeGroup, ApplicationConfig, ProfileConfig};
    use std::time::SystemTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_policy_engine_initialization() {
        let engine = PolicyEngine::new().await;
        assert!(engine.is_ok(), "Policy engine should initialize successfully");
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
