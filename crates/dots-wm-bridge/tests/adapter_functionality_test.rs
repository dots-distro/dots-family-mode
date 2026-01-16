use dots_wm_bridge::{
    adapters::{GenericAdapter, HyprlandAdapter, NiriAdapter, SwayAdapter},
    detect_compositor, CompositorType, WindowManagerAdapter, WindowManagerBridge,
};

#[tokio::test]
async fn test_all_wm_adapters_functionality() {
    println!("Testing all WM adapters individually");

    // Test Niri adapter
    let niri_adapter = NiriAdapter::new();
    assert_eq!(niri_adapter.get_name(), "Niri");
    let niri_caps = niri_adapter.get_capabilities();
    assert!(niri_caps.can_get_focused_window);
    assert!(niri_caps.can_get_all_windows);
    println!("✓ Niri adapter created successfully");

    // Test Sway adapter
    let sway_adapter = SwayAdapter::new();
    assert_eq!(sway_adapter.get_name(), "Sway");
    let sway_caps = sway_adapter.get_capabilities();
    assert!(sway_caps.can_get_focused_window);
    assert!(sway_caps.can_get_all_windows);
    assert!(sway_caps.supports_workspaces);
    println!("✓ Sway adapter created successfully");

    // Test Hyprland adapter
    let hyprland_adapter = HyprlandAdapter::new();
    assert_eq!(hyprland_adapter.get_name(), "Hyprland");
    let hyprland_caps = hyprland_adapter.get_capabilities();
    assert!(hyprland_caps.can_get_focused_window);
    assert!(hyprland_caps.can_get_all_windows);
    assert!(hyprland_caps.supports_workspaces);
    println!("✓ Hyprland adapter created successfully");

    // Test Generic adapter
    let generic_adapter = GenericAdapter::new();
    assert_eq!(generic_adapter.get_name(), "Generic");
    let generic_caps = generic_adapter.get_capabilities();
    assert!(!generic_caps.can_get_focused_window);
    assert!(!generic_caps.can_get_all_windows);
    assert!(!generic_caps.supports_workspaces);
    println!("✓ Generic adapter created successfully");

    println!("All WM adapters test completed successfully!");
}

#[tokio::test]
async fn test_compositor_type_display() {
    println!("Testing CompositorType display implementations");

    assert_eq!(format!("{}", CompositorType::Niri), "Niri");
    assert_eq!(format!("{}", CompositorType::Sway), "Sway");
    assert_eq!(format!("{}", CompositorType::Hyprland), "Hyprland");
    assert_eq!(format!("{}", CompositorType::Unknown), "Unknown");

    println!("✓ CompositorType display test completed successfully!");
}

#[test]
fn test_adapter_availability_checks() {
    println!("Testing adapter availability checks");

    // These should work without actual WM running
    let niri_available = NiriAdapter::is_available();
    let sway_available = SwayAdapter::is_available();
    let hyprland_available = HyprlandAdapter::is_available();
    let generic_available = GenericAdapter::is_available();

    println!("Adapter availability:");
    println!("  Niri: {}", niri_available);
    println!("  Sway: {}", sway_available);
    println!("  Hyprland: {}", hyprland_available);
    println!("  Generic: {}", generic_available);

    // At least one should be available in any Wayland environment
    let any_available = niri_available || sway_available || hyprland_available || generic_available;
    assert!(any_available, "At least one adapter should be available");

    println!("✓ Adapter availability test completed successfully!");
}
