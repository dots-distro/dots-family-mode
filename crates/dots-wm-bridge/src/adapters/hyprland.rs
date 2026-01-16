use crate::types::{WMCapabilities, WindowInfo, WindowState};
use crate::WindowManagerAdapter;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;
use tracing::debug;

pub struct HyprlandAdapter;

impl HyprlandAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() && which::which("hyprctl").is_ok()
    }
}

#[async_trait]
impl WindowManagerAdapter for HyprlandAdapter {
    async fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        debug!("Getting focused window from Hyprland");

        let output = Command::new("hyprctl").arg("activewindow").arg("-j").output()?;

        if !output.status.success() {
            debug!("hyprctl activewindow failed");
            return Ok(None);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() {
            debug!("No active window");
            return Ok(None);
        }

        let value: serde_json::Value = serde_json::from_str(&json_str)?;

        let window_info = WindowInfo {
            app_id: value.get("class").and_then(|v| v.as_str()).map(String::from),
            title: value.get("title").and_then(|v| v.as_str()).map(String::from),
            pid: value.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
            workspace: value
                .get("workspace")
                .and_then(|w| w.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            geometry: None,
            state: WindowState::default(),
        };

        debug!("Found focused window: {:?}", window_info);
        Ok(Some(window_info))
    }

    async fn get_all_windows(&self) -> Result<Vec<WindowInfo>> {
        debug!("Getting all windows from Hyprland");

        let output = Command::new("hyprctl").arg("clients").arg("-j").output()?;

        if !output.status.success() {
            debug!("hyprctl clients failed");
            return Ok(vec![]);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let windows_json: serde_json::Value = serde_json::from_str(&json_str)?;

        let mut windows = Vec::new();

        if let Some(windows_array) = windows_json.as_array() {
            for window in windows_array {
                let window_info = WindowInfo {
                    app_id: window.get("class").and_then(|v| v.as_str()).map(String::from),
                    title: window.get("title").and_then(|v| v.as_str()).map(String::from),
                    pid: window.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                    workspace: window
                        .get("workspace")
                        .and_then(|w| w.get("name"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    geometry: None,
                    state: WindowState::default(),
                };
                windows.push(window_info);
            }
        }

        debug!("Found {} windows", windows.len());
        Ok(windows)
    }

    async fn subscribe_to_events(&self) -> Result<()> {
        debug!("Hyprland event subscription not yet implemented");
        Ok(())
    }

    fn get_capabilities(&self) -> WMCapabilities {
        WMCapabilities {
            can_get_focused_window: true,
            can_get_all_windows: true,
            can_subscribe_to_events: false,
            can_control_windows: true,
            supports_workspaces: true,
            supports_window_geometry: false,
        }
    }

    fn get_name(&self) -> &'static str {
        "Hyprland"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyprland_availability() {
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            assert!(HyprlandAdapter::is_available());
        }
    }

    #[tokio::test]
    async fn test_hyprland_adapter() {
        if !HyprlandAdapter::is_available() {
            return;
        }

        let adapter = HyprlandAdapter::new();
        assert_eq!(adapter.get_name(), "Hyprland");

        let caps = adapter.get_capabilities();
        assert!(caps.can_get_focused_window);
        assert!(caps.can_get_all_windows);
        assert!(caps.supports_workspaces);

        let result = adapter.get_focused_window().await;
        assert!(result.is_ok());

        let result = adapter.get_all_windows().await;
        assert!(result.is_ok());
    }
}
