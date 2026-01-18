use crate::types::{WMCapabilities, WindowInfo, WindowState};
use crate::WindowManagerAdapter;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;
use tracing::debug;

#[derive(Default)]
pub struct SwayAdapter;

impl SwayAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        std::env::var("SWAYSOCK").is_ok() && which::which("swaymsg").is_ok()
    }
}

#[async_trait]
impl WindowManagerAdapter for SwayAdapter {
    async fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        debug!("Getting focused window from Sway");

        let output = Command::new("swaymsg").arg("-t").arg("get_tree").output()?;

        if !output.status.success() {
            debug!("swaymsg get_tree failed");
            return Ok(None);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let value: serde_json::Value = serde_json::from_str(&json_str)?;

        if let Some(focused) = find_focused_node(&value) {
            let window_info = WindowInfo {
                app_id: focused.get("app_id").and_then(|v| v.as_str()).map(String::from),
                title: focused.get("name").and_then(|v| v.as_str()).map(String::from),
                pid: focused.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                workspace: focused.get("workspace").and_then(|v| v.as_str()).map(String::from),
                geometry: None,
                state: WindowState::default(),
            };

            debug!("Found focused window: {:?}", window_info);
            Ok(Some(window_info))
        } else {
            debug!("No focused window found");
            Ok(None)
        }
    }

    async fn get_all_windows(&self) -> Result<Vec<WindowInfo>> {
        debug!("Getting all windows from Sway");

        let output = Command::new("swaymsg").arg("-t").arg("get_tree").output()?;

        if !output.status.success() {
            debug!("swaymsg get_tree failed");
            return Ok(vec![]);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let value: serde_json::Value = serde_json::from_str(&json_str)?;

        let mut windows = Vec::new();
        collect_all_windows(&value, &mut windows);

        debug!("Found {} windows", windows.len());
        Ok(windows)
    }

    async fn subscribe_to_events(&self) -> Result<()> {
        debug!("Sway event subscription not yet implemented");
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
        "Sway"
    }
}

fn find_focused_node(node: &serde_json::Value) -> Option<&serde_json::Value> {
    if let Some(focused) = node.get("focused") {
        if focused.as_bool() == Some(true) {
            return Some(node);
        }
    }

    if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
        for child in nodes {
            if let Some(found) = find_focused_node(child) {
                return Some(found);
            }
        }
    }

    if let Some(floating) = node.get("floating_nodes").and_then(|n| n.as_array()) {
        for child in floating {
            if let Some(found) = find_focused_node(child) {
                return Some(found);
            }
        }
    }

    None
}

fn collect_all_windows(node: &serde_json::Value, windows: &mut Vec<WindowInfo>) {
    if let Some(app_id) = node.get("app_id").and_then(|v| v.as_str()) {
        let window_info = WindowInfo {
            app_id: Some(app_id.to_string()),
            title: node.get("name").and_then(|v| v.as_str()).map(String::from),
            pid: node.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
            workspace: node.get("workspace").and_then(|v| v.as_str()).map(String::from),
            geometry: None,
            state: WindowState::default(),
        };
        windows.push(window_info);
    }

    if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
        for child in nodes {
            collect_all_windows(child, windows);
        }
    }

    if let Some(floating) = node.get("floating_nodes").and_then(|n| n.as_array()) {
        for child in floating {
            collect_all_windows(child, windows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sway_availability() {
        if std::env::var("SWAYSOCK").is_ok() {
            assert!(SwayAdapter::is_available());
        }
    }

    #[tokio::test]
    async fn test_sway_adapter() {
        if !SwayAdapter::is_available() {
            return;
        }

        let adapter = SwayAdapter::new();
        assert_eq!(adapter.get_name(), "Sway");

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
