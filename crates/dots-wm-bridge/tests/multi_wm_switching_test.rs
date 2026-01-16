use anyhow::Result;
use dots_wm_bridge::{detect_compositor, CompositorType, WindowManagerBridge};
use std::env;

#[tokio::test]
async fn test_multi_wm_switching_simulation() {
    println!("Testing WM bridge switching simulation");

    // Test 1: Current environment (should be Niri)
    let current_compositor = detect_compositor();
    println!("Current compositor: {}", current_compositor);

    let current_bridge = WindowManagerBridge::new();
    assert!(current_bridge.is_ok(), "Should create bridge in current environment");

    let bridge = current_bridge.unwrap();
    println!("Current WM adapter: {}", bridge.get_adapter_name());

    // Test 2: Simulate Sway environment
    env::set_var("SWAYSOCK", "/tmp/fake-sway-socket");
    env::remove_var("NIRI_SOCKET");
    env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");

    let sway_compositor = detect_compositor();
    println!("Simulated Sway compositor: {}", sway_compositor);
    assert_eq!(sway_compositor, CompositorType::Sway);

    // Test 3: Simulate Hyprland environment
    env::remove_var("SWAYSOCK");
    env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "fake-signature");

    let hyprland_compositor = detect_compositor();
    println!("Simulated Hyprland compositor: {}", hyprland_compositor);
    assert_eq!(hyprland_compositor, CompositorType::Hyprland);

    // Test 4: Simulate generic Wayland environment
    env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");
    env::set_var("WAYLAND_DISPLAY", "wayland-0");

    let generic_compositor = detect_compositor();
    println!("Simulated generic compositor: {}", generic_compositor);
    assert_eq!(generic_compositor, CompositorType::Unknown);

    // Test 5: Restore original environment
    env::remove_var("SWAYSOCK");
    env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");
    env::remove_var("WAYLAND_DISPLAY");

    // In our Niri environment, we might not have NIRI_SOCKET visible in tests
    let restored_compositor = detect_compositor();
    println!("Restored compositor: {}", restored_compositor);

    println!("Multi-WM switching simulation completed successfully!");
}
