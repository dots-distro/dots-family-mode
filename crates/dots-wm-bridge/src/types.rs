use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub pid: Option<u32>,
    pub workspace: Option<String>,
    pub geometry: Option<WindowGeometry>,
    pub state: WindowState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum WindowState {
    #[default]
    Normal,
    Maximized,
    Minimized,
    Fullscreen,
    Floating,
    Tiled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompositorType {
    Niri,
    Sway,
    Hyprland,
    Unknown,
}

impl std::fmt::Display for CompositorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositorType::Niri => write!(f, "Niri"),
            CompositorType::Sway => write!(f, "Sway"),
            CompositorType::Hyprland => write!(f, "Hyprland"),
            CompositorType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WMCapabilities {
    pub can_get_focused_window: bool,
    pub can_get_all_windows: bool,
    pub can_subscribe_to_events: bool,
    pub can_control_windows: bool,
    pub supports_workspaces: bool,
    pub supports_window_geometry: bool,
}

impl WMCapabilities {
    pub fn full() -> Self {
        Self {
            can_get_focused_window: true,
            can_get_all_windows: true,
            can_subscribe_to_events: true,
            can_control_windows: true,
            supports_workspaces: true,
            supports_window_geometry: true,
        }
    }

    pub fn basic() -> Self {
        Self {
            can_get_focused_window: true,
            can_get_all_windows: false,
            can_subscribe_to_events: false,
            can_control_windows: false,
            supports_workspaces: false,
            supports_window_geometry: false,
        }
    }

    pub fn none() -> Self {
        Self {
            can_get_focused_window: false,
            can_get_all_windows: false,
            can_subscribe_to_events: false,
            can_control_windows: false,
            supports_workspaces: false,
            supports_window_geometry: false,
        }
    }
}

#[derive(Debug)]
pub enum WMEvent {
    WindowOpened(WindowInfo),
    WindowClosed(u32),
    WindowFocused(Option<WindowInfo>),
    WorkspaceChanged(String),
}
