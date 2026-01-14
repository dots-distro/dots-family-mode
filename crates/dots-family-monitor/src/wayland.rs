use anyhow::Result;
use std::process::Command;
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub _pid: Option<u32>,
}

pub struct WaylandMonitor {
    compositor: CompositorType,
}

#[derive(Debug, Clone, Copy)]
enum CompositorType {
    Niri,
    Sway,
    Hyprland,
    Unknown,
}

impl WaylandMonitor {
    pub fn new() -> Result<Self> {
        let compositor = detect_compositor();
        debug!("Detected compositor: {:?}", compositor);
        Ok(Self { compositor })
    }

    pub async fn get_focused_window(&mut self) -> Result<Option<WindowInfo>> {
        match self.compositor {
            CompositorType::Niri => self.get_niri_focused_window().await,
            CompositorType::Sway => self.get_sway_focused_window().await,
            CompositorType::Hyprland => self.get_hyprland_focused_window().await,
            CompositorType::Unknown => Ok(None),
        }
    }

    async fn get_niri_focused_window(&self) -> Result<Option<WindowInfo>> {
        let output = Command::new("niri").arg("msg").arg("--json").arg("focused-window").output();

        if let Ok(output) = output {
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    return Ok(Some(WindowInfo {
                        app_id: value.get("app_id").and_then(|v| v.as_str()).map(String::from),
                        title: value.get("title").and_then(|v| v.as_str()).map(String::from),
                        _pid: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn get_sway_focused_window(&self) -> Result<Option<WindowInfo>> {
        let output = Command::new("swaymsg").arg("-t").arg("get_tree").output();

        if let Ok(output) = output {
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(focused) = find_focused_node(&value) {
                        return Ok(Some(WindowInfo {
                            app_id: focused
                                .get("app_id")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            title: focused.get("name").and_then(|v| v.as_str()).map(String::from),
                            _pid: focused.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_hyprland_focused_window(&self) -> Result<Option<WindowInfo>> {
        let output = Command::new("hyprctl").arg("activewindow").arg("-j").output();

        if let Ok(output) = output {
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    return Ok(Some(WindowInfo {
                        app_id: value.get("class").and_then(|v| v.as_str()).map(String::from),
                        title: value.get("title").and_then(|v| v.as_str()).map(String::from),
                        _pid: value.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                    }));
                }
            }
        }

        Ok(None)
    }
}

fn detect_compositor() -> CompositorType {
    if std::env::var("NIRI_SOCKET").is_ok() {
        return CompositorType::Niri;
    }

    if std::env::var("SWAYSOCK").is_ok() {
        return CompositorType::Sway;
    }

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return CompositorType::Hyprland;
    }

    CompositorType::Unknown
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
