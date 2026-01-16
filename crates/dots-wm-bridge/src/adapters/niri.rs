use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;
use tracing::debug;

use crate::types::{WMCapabilities, WindowInfo, WindowState};
use crate::WindowManagerAdapter;

pub struct NiriAdapter;

impl NiriAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        std::env::var("NIRI_SOCKET").is_ok() && which::which("niri").is_ok()
    }
}

#[async_trait]
impl WindowManagerAdapter for NiriAdapter {
    async fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        let output =
            Command::new("niri").arg("msg").arg("--json").arg("focused-window").output()?;

        if !output.status.success() {
            debug!("Niri command failed: {:?}", output.stderr);
            return Ok(None);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
            Ok(Some(WindowInfo {
                app_id: value.get("app_id").and_then(|v| v.as_str()).map(String::from),
                title: value.get("title").and_then(|v| v.as_str()).map(String::from),
                pid: None,
                workspace: None,
                geometry: None,
                state: WindowState::default(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_all_windows(&self) -> Result<Vec<WindowInfo>> {
        let output = Command::new("niri").arg("msg").arg("--json").arg("windows").output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(windows) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(windows_array) = windows.as_array() {
                let mut result = Vec::new();
                for window in windows_array {
                    result.push(WindowInfo {
                        app_id: window.get("app_id").and_then(|v| v.as_str()).map(String::from),
                        title: window.get("title").and_then(|v| v.as_str()).map(String::from),
                        pid: None,
                        workspace: None,
                        geometry: None,
                        state: WindowState::default(),
                    });
                }
                return Ok(result);
            }
        }

        Ok(Vec::new())
    }

    async fn subscribe_to_events(&self) -> Result<()> {
        Ok(())
    }

    fn get_capabilities(&self) -> WMCapabilities {
        WMCapabilities {
            can_get_focused_window: true,
            can_get_all_windows: true,
            can_subscribe_to_events: false,
            can_control_windows: false,
            supports_workspaces: false,
            supports_window_geometry: false,
        }
    }

    fn get_name(&self) -> &'static str {
        "Niri"
    }
}
