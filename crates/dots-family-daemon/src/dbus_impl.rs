use anyhow::Result;
use tracing::warn;
use zbus::interface;

use crate::config::DaemonConfig;
use crate::profile_manager::ProfileManager;

pub struct FamilyDaemonService {
    profile_manager: ProfileManager,
}

impl FamilyDaemonService {
    pub async fn new(config: &DaemonConfig) -> Result<Self> {
        let profile_manager = ProfileManager::new(config).await?;
        Ok(Self { profile_manager })
    }
}

#[interface(name = "org.dots.FamilyDaemon")]
impl FamilyDaemonService {
    async fn get_active_profile(&self) -> String {
        match self.profile_manager.get_active_profile().await {
            Ok(Some(profile)) => {
                serde_json::to_string(&profile).unwrap_or_else(|_| "{}".to_string())
            }
            Ok(None) => r#"{"error":"no_active_profile"}"#.to_string(),
            Err(e) => {
                warn!("Failed to get active profile: {}", e);
                format!(r#"{{"error":"{}"}}"#, e)
            }
        }
    }

    async fn check_application_allowed(&self, app_id: &str) -> bool {
        match self.profile_manager.check_application_allowed(app_id).await {
            Ok(allowed) => allowed,
            Err(e) => {
                warn!("Failed to check application {}: {}", app_id, e);
                false
            }
        }
    }

    async fn get_remaining_time(&self) -> u32 {
        match self.profile_manager.get_remaining_time().await {
            Ok(minutes) => minutes,
            Err(e) => {
                warn!("Failed to get remaining time: {}", e);
                0
            }
        }
    }

    async fn report_activity(&self, activity_json: &str) -> String {
        match self.profile_manager.report_activity(activity_json).await {
            Ok(()) => "success".to_string(),
            Err(e) => {
                warn!("Failed to report activity: {}", e);
                format!("error:{}", e)
            }
        }
    }

    async fn send_heartbeat(&self, monitor_id: &str) -> String {
        match self.profile_manager.send_heartbeat(monitor_id).await {
            Ok(()) => "success".to_string(),
            Err(e) => {
                warn!("Failed to process heartbeat from {}: {}", monitor_id, e);
                format!("error:{}", e)
            }
        }
    }

    async fn list_profiles(&self) -> String {
        match self.profile_manager.list_profiles().await {
            Ok(profiles) => serde_json::to_string(&profiles).unwrap_or_else(|_| "[]".to_string()),
            Err(e) => {
                warn!("Failed to list profiles: {}", e);
                format!("error:{}", e)
            }
        }
    }

    async fn create_profile(&self, name: &str, age_group: &str) -> String {
        match self.profile_manager.create_profile(name, age_group).await {
            Ok(profile_id) => profile_id,
            Err(e) => {
                warn!("Failed to create profile: {}", e);
                format!("error:{}", e)
            }
        }
    }

    async fn authenticate_parent(&self, password: &str) -> String {
        match self.profile_manager.authenticate_parent(password).await {
            Ok(token) => token,
            Err(e) => {
                warn!("Authentication failed: {}", e);
                format!("error:{}", e)
            }
        }
    }

    async fn set_active_profile(&self, profile_id: &str) {
        if let Err(e) = self.profile_manager._set_active_profile(profile_id).await {
            warn!("Failed to set active profile: {}", e);
        }
    }

    #[zbus(signal)]
    async fn policy_updated(
        signal_ctxt: &zbus::SignalContext<'_>,
        profile_id: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn time_limit_warning(
        signal_ctxt: &zbus::SignalContext<'_>,
        minutes_remaining: u32,
    ) -> zbus::Result<()>;
}
