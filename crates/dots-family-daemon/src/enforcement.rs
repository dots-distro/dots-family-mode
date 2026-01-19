use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, error, info, warn};

pub struct EnforcementEngine {
    dry_run: bool,
}

impl EnforcementEngine {
    pub fn new(dry_run: bool) -> Self {
        info!("Initializing enforcement engine (dry_run: {})", dry_run);
        Self { dry_run }
    }

    pub async fn terminate_process(&self, pid: u32, reason: &str) -> Result<()> {
        info!("Attempting to terminate process {} (reason: {})", pid, reason);

        if self.dry_run {
            warn!("DRY RUN: Would terminate process {}", pid);
            return Ok(());
        }

        let output = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output()
            .context("Failed to execute kill command")?;

        if output.status.success() {
            info!("Successfully sent SIGTERM to process {}", pid);
        } else {
            warn!("SIGTERM failed, attempting SIGKILL for process {}", pid);

            let kill_output = Command::new("kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .output()
                .context("Failed to execute kill -KILL command")?;

            if kill_output.status.success() {
                info!("Successfully sent SIGKILL to process {}", pid);
            } else {
                let error_msg = String::from_utf8_lossy(&kill_output.stderr);
                error!("Failed to kill process {}: {}", pid, error_msg);
                return Err(anyhow::anyhow!("Failed to kill process {}: {}", pid, error_msg));
            }
        }

        Ok(())
    }

    pub async fn close_window(&self, app_id: &str, pid: u32) -> Result<()> {
        info!("Attempting to close window for app {} (PID: {})", app_id, pid);

        if self.dry_run {
            warn!("DRY RUN: Would close window for app {} (PID: {})", app_id, pid);
            return Ok(());
        }

        match self.detect_window_manager().await? {
            WindowManager::Niri => self.close_window_niri(app_id, pid).await,
            WindowManager::Sway => self.close_window_sway(app_id, pid).await,
            WindowManager::Hyprland => self.close_window_hyprland(app_id, pid).await,
            WindowManager::Unknown => {
                warn!("Unknown window manager, falling back to process termination");
                self.terminate_process(pid, "Window manager not supported").await
            }
        }
    }

    async fn detect_window_manager(&self) -> Result<WindowManager> {
        if std::env::var("NIRI_SOCKET").is_ok() {
            Ok(WindowManager::Niri)
        } else if std::env::var("SWAYSOCK").is_ok() {
            Ok(WindowManager::Sway)
        } else if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            Ok(WindowManager::Hyprland)
        } else {
            Ok(WindowManager::Unknown)
        }
    }

    async fn close_window_niri(&self, app_id: &str, pid: u32) -> Result<()> {
        debug!("Closing Niri window for app {} (PID: {})", app_id, pid);

        let output = Command::new("niri")
            .args(&["msg", "action", "close-window"])
            .output()
            .context("Failed to execute niri command")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!("Niri close-window failed: {}, falling back to process termination", error_msg);
            self.terminate_process(pid, "Niri window close failed").await?;
        } else {
            info!("Successfully closed Niri window for app {}", app_id);
        }

        Ok(())
    }

    async fn close_window_sway(&self, app_id: &str, pid: u32) -> Result<()> {
        debug!("Closing Sway window for app {} (PID: {})", app_id, pid);

        let kill_command = format!("[app_id=\"{}\"] kill", app_id);
        let output = Command::new("swaymsg")
            .arg(&kill_command)
            .output()
            .context("Failed to execute swaymsg command")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!("Sway kill command failed: {}, falling back to process termination", error_msg);
            self.terminate_process(pid, "Sway window kill failed").await?;
        } else {
            info!("Successfully closed Sway window for app {}", app_id);
        }

        Ok(())
    }

    async fn close_window_hyprland(&self, app_id: &str, pid: u32) -> Result<()> {
        debug!("Closing Hyprland window for app {} (PID: {})", app_id, pid);

        let kill_command = format!("dispatch closewindow pid:{}", pid);
        let output = Command::new("hyprctl")
            .arg(&kill_command)
            .output()
            .context("Failed to execute hyprctl command")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Hyprland closewindow failed: {}, falling back to process termination",
                error_msg
            );
            self.terminate_process(pid, "Hyprland window close failed").await?;
        } else {
            info!("Successfully closed Hyprland window for app {}", app_id);
        }

        Ok(())
    }

    pub async fn block_network_connection(&self, pid: u32, remote_addr: &str) -> Result<()> {
        info!("Attempting to block network connection from PID {} to {}", pid, remote_addr);

        if self.dry_run {
            warn!("DRY RUN: Would block network connection from PID {} to {}", pid, remote_addr);
            return Ok(());
        }

        let iptables_rule = format!(
            "-I OUTPUT -p tcp --dport 80,443 -m owner --pid-owner {} -d {} -j DROP",
            pid, remote_addr
        );

        let output = Command::new("iptables")
            .args(iptables_rule.split_whitespace())
            .output()
            .context("Failed to execute iptables command")?;

        if output.status.success() {
            info!("Successfully blocked network connection from PID {} to {}", pid, remote_addr);
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!("iptables rule failed: {}, falling back to process termination", error_msg);
            self.terminate_process(pid, "Network blocking failed").await?;
        }

        Ok(())
    }

    pub async fn notify_user(&self, title: &str, message: &str) -> Result<()> {
        info!("Sending notification: {} - {}", title, message);

        if self.dry_run {
            warn!("DRY RUN: Would send notification: {} - {}", title, message);
            return Ok(());
        }

        let output = Command::new("notify-send")
            .arg("--urgency=critical")
            .arg("--app-name=DOTS Family Mode")
            .arg(title)
            .arg(message)
            .output();

        match output {
            Ok(result) if result.status.success() => {
                debug!("Notification sent successfully");
            }
            Ok(result) => {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                warn!("Notification failed: {}", error_msg);
            }
            Err(e) => {
                warn!("Failed to send notification: {}", e);
            }
        }

        Ok(())
    }

    pub async fn enforce_policy_decision(
        &self,
        decision: &crate::policy_engine::PolicyDecision,
        app_id: Option<&str>,
        pid: Option<u32>,
    ) -> Result<()> {
        info!("Enforcing policy decision: {} - {}", decision.action, decision.reason);

        match decision.action.as_str() {
            "block" => {
                // Send notification first
                self.notify_user(
                    "Access Blocked",
                    &format!("Access restricted: {}", decision.reason),
                )
                .await?;

                // Close window if we have app and pid info
                if let (Some(app), Some(process_id)) = (app_id, pid) {
                    self.close_window(app, process_id).await?;
                } else if let Some(process_id) = pid {
                    // If we only have PID, terminate the process
                    self.terminate_process(process_id, &decision.reason).await?;
                }
            }
            "allow" => {
                debug!("Policy allows access: {}", decision.reason);
            }
            "warn" => {
                // Send warning notification but don't block
                self.notify_user("Warning", &format!("Warning: {}", decision.reason)).await?;
            }
            _ => {
                warn!("Unknown policy action: {}", decision.action);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
enum WindowManager {
    Niri,
    Sway,
    Hyprland,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enforcement_engine_creation() {
        let engine = EnforcementEngine::new(true);
        assert!(engine.dry_run);
    }

    #[tokio::test]
    async fn test_dry_run_terminate_process() {
        let engine = EnforcementEngine::new(true);
        let result = engine.terminate_process(1, "test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dry_run_close_window() {
        let engine = EnforcementEngine::new(true);
        let result = engine.close_window("test-app", 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dry_run_block_network() {
        let engine = EnforcementEngine::new(true);
        let result = engine.block_network_connection(1, "192.168.1.1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification() {
        let engine = EnforcementEngine::new(true);
        let result = engine.notify_user("Test Title", "Test Message").await;
        assert!(result.is_ok());
    }
}
