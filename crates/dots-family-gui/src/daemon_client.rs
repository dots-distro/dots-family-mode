use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DaemonClient {
    connected: Arc<Mutex<bool>>,
}

impl DaemonClient {
    pub async fn new() -> Self {
        Self { connected: Arc::new(Mutex::new(false)) }
    }

    pub async fn connect(&self) -> Result<()> {
        *self.connected.lock().await = true;
        Ok(())
    }

    pub async fn get_active_profile(&self) -> Result<String> {
        Ok("Alice".to_string())
    }

    pub async fn get_remaining_time(&self) -> Result<u32> {
        Ok(85)
    }

    pub async fn check_application_allowed(&self, _app_id: &str) -> Result<bool> {
        Ok(true)
    }

    pub async fn list_profiles(&self) -> Result<String> {
        Ok(r#"[{"id": "1", "name": "Alice"}]"#.to_string())
    }

    pub async fn set_active_profile(&self, _profile_id: &str) -> Result<()> {
        Ok(())
    }

    pub async fn authenticate_parent(&self, _password: &str) -> Result<String> {
        Ok("token123".to_string())
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }
}
