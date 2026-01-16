use crate::types::CompositorType;
use std::env;

pub fn detect_compositor() -> CompositorType {
    if env::var("NIRI_SOCKET").is_ok() {
        return CompositorType::Niri;
    }

    if env::var("SWAYSOCK").is_ok() {
        return CompositorType::Sway;
    }

    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return CompositorType::Hyprland;
    }

    CompositorType::Unknown
}

pub fn compositor_command_exists(compositor: CompositorType) -> bool {
    match compositor {
        CompositorType::Niri => which::which("niri").is_ok(),
        CompositorType::Sway => which::which("swaymsg").is_ok(),
        CompositorType::Hyprland => which::which("hyprctl").is_ok(),
        CompositorType::Unknown => false,
    }
}

pub fn get_compositor_info() -> (CompositorType, bool) {
    let detected = detect_compositor();
    let has_command = compositor_command_exists(detected);
    (detected, has_command)
}
