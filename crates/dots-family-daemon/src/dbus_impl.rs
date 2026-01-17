use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use zbus::interface;

use crate::config::DaemonConfig;
use crate::daemon::Daemon;
use crate::monitoring_service::MonitoringService;
use crate::profile_manager::ProfileManager;
use dots_family_proto::events::ActivityEvent;

pub struct FamilyDaemonService {
    profile_manager: ProfileManager,
    monitoring_service: MonitoringService,
    daemon: Option<Arc<Daemon>>,
}

impl FamilyDaemonService {
    pub async fn new(config: &DaemonConfig, monitoring_service: MonitoringService) -> Result<Self> {
        let profile_manager = ProfileManager::new(config).await?;
        Ok(Self { profile_manager, monitoring_service, daemon: None })
    }

    pub async fn new_with_daemon(
        config: &DaemonConfig,
        monitoring_service: MonitoringService,
        daemon: Arc<Daemon>,
    ) -> Result<Self> {
        let profile_manager = ProfileManager::new(config).await?;
        Ok(Self { profile_manager, monitoring_service, daemon: Some(daemon) })
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

    async fn report_activity_event(&self, event_json: &str) -> String {
        match serde_json::from_str::<ActivityEvent>(event_json) {
            Ok(event) => {
                info!("Received activity event: {:?}", event);

                match event {
                    ActivityEvent::WindowFocused { pid, app_id, window_title, .. } => {
                        info!(
                            "Window focused - PID: {}, App: {}, Title: {}",
                            pid, app_id, window_title
                        );
                    }
                    ActivityEvent::ProcessStarted { pid, executable, args, .. } => {
                        info!(
                            "Process started - PID: {}, Executable: {}, Args: {:?}",
                            pid, executable, args
                        );
                    }
                    ActivityEvent::NetworkConnection { pid, local_addr, remote_addr, .. } => {
                        info!(
                            "Network connection - PID: {}, Local: {}, Remote: {}",
                            pid, local_addr, remote_addr
                        );
                    }
                }

                "success".to_string()
            }
            Err(e) => {
                error!("Failed to parse activity event JSON: {}", e);
                format!("error:Invalid event JSON: {}", e)
            }
        }
    }

    async fn ping(&self) -> bool {
        debug!("Received ping from monitor");
        true
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

    async fn validate_session(&self, token: &str) -> bool {
        self.profile_manager.validate_session(token).await
    }

    async fn revoke_session(&self, token: &str) -> bool {
        self.profile_manager.revoke_session(token).await
    }

    async fn set_active_profile(&self, profile_id: &str) {
        if let Err(e) = self.profile_manager._set_active_profile(profile_id).await {
            warn!("Failed to set active profile: {}", e);
        }
    }

    async fn request_parent_permission(
        &self,
        request_type: &str,
        details: &str,
        token: &str,
    ) -> String {
        match self.profile_manager.request_parent_permission(request_type, details, token).await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to process permission request: {}", e);
                format!(r#"{{"error":"{}","status":"denied"}}"#, e)
            }
        }
    }

    async fn request_command_approval(
        &self,
        command: &str,
        risk_level: &str,
        reasons: &str,
    ) -> String {
        match self.profile_manager.request_command_approval(command, risk_level, reasons).await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to process command approval request: {}", e);
                format!(r#"{{"error":"{}","status":"denied"}}"#, e)
            }
        }
    }

    // ============================================================================
    // Exception Management Methods
    // ============================================================================

    async fn create_exception(
        &self,
        exception_type: &str,
        reason: &str,
        duration_json: &str,
        token: &str,
    ) -> String {
        match self
            .profile_manager
            .create_exception(exception_type, reason, duration_json, token)
            .await
        {
            Ok(exception_id) => {
                format!(r#"{{"status":"success","exception_id":"{}"}}"#, exception_id)
            }
            Err(e) => {
                warn!("Failed to create exception: {}", e);
                format!(r#"{{"error":"{}","status":"failed"}}"#, e)
            }
        }
    }

    async fn list_active_exceptions(&self, profile_id: &str, token: &str) -> String {
        match self.profile_manager.list_active_exceptions(profile_id, token).await {
            Ok(exceptions) => {
                serde_json::to_string(&exceptions).unwrap_or_else(|_| "[]".to_string())
            }
            Err(e) => {
                warn!("Failed to list active exceptions: {}", e);
                format!(r#"{{"error":"{}"}}"#, e)
            }
        }
    }

    async fn revoke_exception(&self, exception_id: &str, token: &str) -> String {
        match self.profile_manager.revoke_exception(exception_id, token).await {
            Ok(()) => r#"{"status":"success"}"#.to_string(),
            Err(e) => {
                warn!("Failed to revoke exception: {}", e);
                format!(r#"{{"error":"{}","status":"failed"}}"#, e)
            }
        }
    }

    async fn check_exception_applies(&self, exception_type: &str, resource_id: &str) -> bool {
        match self.profile_manager.check_exception_applies(exception_type, resource_id).await {
            Ok(applies) => applies,
            Err(e) => {
                warn!("Failed to check exception: {}", e);
                false
            }
        }
    }

    // ============================================================================
    // Approval Request Methods
    // ============================================================================

    async fn submit_approval_request(
        &self,
        request_type: &str,
        message: &str,
        details_json: &str,
    ) -> String {
        match self
            .profile_manager
            .submit_approval_request(request_type, message, details_json)
            .await
        {
            Ok(request_id) => {
                format!(r#"{{"status":"success","request_id":"{}"}}"#, request_id)
            }
            Err(e) => {
                warn!("Failed to submit approval request: {}", e);
                format!(r#"{{"error":"{}","status":"failed"}}"#, e)
            }
        }
    }

    async fn list_pending_requests(&self, token: &str) -> String {
        match self.profile_manager.list_pending_requests(token).await {
            Ok(requests) => serde_json::to_string(&requests).unwrap_or_else(|_| "[]".to_string()),
            Err(e) => {
                warn!("Failed to list pending requests: {}", e);
                format!(r#"{{"error":"{}"}}"#, e)
            }
        }
    }

    async fn approve_request(
        &self,
        request_id: &str,
        response_message: &str,
        token: &str,
    ) -> String {
        match self.profile_manager.approve_request(request_id, response_message, token).await {
            Ok(exception_id) => {
                format!(
                    r#"{{"status":"success","exception_id":"{}"}}"#,
                    exception_id.unwrap_or_default()
                )
            }
            Err(e) => {
                warn!("Failed to approve request: {}", e);
                format!(r#"{{"error":"{}","status":"failed"}}"#, e)
            }
        }
    }

    async fn deny_request(&self, request_id: &str, response_message: &str, token: &str) -> String {
        match self.profile_manager.deny_request(request_id, response_message, token).await {
            Ok(()) => r#"{"status":"success"}"#.to_string(),
            Err(e) => {
                warn!("Failed to deny request: {}", e);
                format!(r#"{{"error":"{}","status":"failed"}}"#, e)
            }
        }
    }

    async fn get_monitoring_snapshot(&self) -> String {
        match self.monitoring_service.get_monitoring_snapshot().await {
            Ok(data) => serde_json::to_string(&data)
                .unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string()),
            Err(e) => {
                warn!("Failed to get monitoring snapshot: {}", e);
                format!(r#"{{"error":"{}"}}"#, e)
            }
        }
    }

    async fn get_ebpf_status(&self) -> (u32, bool, String) {
        if let Some(ref daemon) = self.daemon {
            if let Some(status) = daemon.get_ebpf_health().await {
                (
                    status.programs_loaded as u32,
                    status.all_healthy,
                    format!(
                        "eBPF manager active: {}/{} programs loaded",
                        status.programs_loaded,
                        status.program_status.len()
                    ),
                )
            } else {
                (0, false, "eBPF manager not available".to_string())
            }
        } else {
            (0, false, "eBPF status not yet connected".to_string())
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

    #[zbus(signal)]
    async fn tamper_detected(
        signal_ctxt: &zbus::SignalContext<'_>,
        reason: &str,
    ) -> zbus::Result<()>;
}
