pub mod adapters;
pub mod detection;
pub mod types;

pub use adapters::*;
pub use detection::*;
pub use types::*;

use anyhow::Result;

#[async_trait::async_trait]
pub trait WindowManagerAdapter {
    async fn get_focused_window(&self) -> Result<Option<WindowInfo>>;
    async fn get_all_windows(&self) -> Result<Vec<WindowInfo>>;
    async fn subscribe_to_events(&self) -> Result<()>;
    fn get_capabilities(&self) -> WMCapabilities;
    fn get_name(&self) -> &'static str;
}

pub struct WindowManagerBridge {
    adapter: Box<dyn WindowManagerAdapter + Send + Sync>,
    compositor_type: CompositorType,
}

impl WindowManagerBridge {
    pub fn new() -> Result<Self> {
        let compositor_type = detect_compositor();
        let adapter = create_adapter(compositor_type)?;

        Ok(Self { adapter, compositor_type })
    }

    pub async fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        self.adapter.get_focused_window().await
    }

    pub async fn get_all_windows(&self) -> Result<Vec<WindowInfo>> {
        self.adapter.get_all_windows().await
    }

    pub fn get_compositor_type(&self) -> CompositorType {
        self.compositor_type
    }

    pub fn get_capabilities(&self) -> WMCapabilities {
        self.adapter.get_capabilities()
    }

    pub fn get_adapter_name(&self) -> &'static str {
        self.adapter.get_name()
    }
}

fn create_adapter(
    compositor_type: CompositorType,
) -> Result<Box<dyn WindowManagerAdapter + Send + Sync>> {
    use adapters::*;

    match compositor_type {
        CompositorType::Niri => Ok(Box::new(NiriAdapter::new())),
        CompositorType::Sway => Ok(Box::new(SwayAdapter::new())),
        CompositorType::Hyprland => Ok(Box::new(HyprlandAdapter::new())),
        CompositorType::Unknown => Ok(Box::new(GenericAdapter::new())),
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_wm_bridge_functionality() {
        let compositor = detect_compositor();
        println!("Detected compositor: {}", compositor);

        let bridge_result = WindowManagerBridge::new();
        assert!(bridge_result.is_ok(), "Should create WM bridge successfully");

        let bridge = bridge_result.unwrap();
        println!("Created bridge for: {}", bridge.get_adapter_name());

        let caps = bridge.get_capabilities();
        println!("Testing capabilities for {}", bridge.get_adapter_name());

        if caps.can_get_focused_window {
            let focused_result = bridge.get_focused_window().await;
            assert!(focused_result.is_ok(), "get_focused_window should not error");

            if let Ok(Some(window)) = focused_result {
                println!("Found focused window: {:?} - {:?}", window.app_id, window.title);
            }
        }

        if caps.can_get_all_windows {
            let windows_result = bridge.get_all_windows().await;
            assert!(windows_result.is_ok(), "get_all_windows should not error");

            if let Ok(windows) = windows_result {
                println!("Found {} total windows", windows.len());
            }
        }

        println!("Multi-WM bridge test completed successfully!");
    }
}
