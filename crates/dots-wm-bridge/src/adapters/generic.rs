use crate::types::{WMCapabilities, WindowInfo};
use crate::WindowManagerAdapter;
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

pub struct GenericAdapter;

impl GenericAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
    }
}

#[async_trait]
impl WindowManagerAdapter for GenericAdapter {
    async fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        debug!("Generic adapter cannot determine focused window");
        Ok(None)
    }

    async fn get_all_windows(&self) -> Result<Vec<WindowInfo>> {
        debug!("Generic adapter cannot enumerate windows");
        Ok(vec![])
    }

    async fn subscribe_to_events(&self) -> Result<()> {
        debug!("Generic adapter does not support event subscription");
        Ok(())
    }

    fn get_capabilities(&self) -> WMCapabilities {
        WMCapabilities::none()
    }

    fn get_name(&self) -> &'static str {
        "Generic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_availability() {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            assert!(GenericAdapter::is_available());
        }
    }

    #[tokio::test]
    async fn test_generic_adapter() {
        let adapter = GenericAdapter::new();
        assert_eq!(adapter.get_name(), "Generic");

        let caps = adapter.get_capabilities();
        assert!(!caps.can_get_focused_window);
        assert!(!caps.can_get_all_windows);
        assert!(!caps.supports_workspaces);

        let result = adapter.get_focused_window().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result = adapter.get_all_windows().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
