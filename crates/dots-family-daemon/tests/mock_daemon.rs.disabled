use dots_family_proto::daemon::FamilyDaemon;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use zbus::export::Interface;
use zbus::export::Interface;
use zbus::zvariant::{ObjectPath, Value};
use zbus::{Connection, Message};

/// Mock daemon for integration testing
pub struct MockDaemon {
    active_profiles: Arc<RwLock<HashMap<String, String>>>,
    allowed_apps: Arc<RwLock<Vec<String>>>,
}

impl MockDaemon {
    pub fn new() -> Self {
        let mut active_profiles = HashMap::new();
        active_profiles.insert("testchild".to_string(), "8-12".to_string());

        let allowed_apps =
            vec!["firefox".to_string(), "calculator".to_string(), "tuxmath".to_string()];

        Self {
            active_profiles: Arc::new(RwLock::new(active_profiles)),
            allowed_apps: Arc::new(RwLock::new(allowed_apps)),
        }
    }
}

#[zbus::interface(name = "org.dots.FamilyDaemon")]
impl MockDaemon {
    async fn list_profiles(&self) -> Vec<String> {
        let profiles = self.active_profiles.read().await;
        profiles.values().cloned().collect()
    }

    async fn get_active_profile(&self, username: &str) -> Option<String> {
        let profiles = self.active_profiles.read().await;
        profiles.get(username).cloned()
    }

    async fn check_application_allowed(&self, app_id: &str) -> Result<bool, String> {
        let allowed_apps = self.allowed_apps.read().await;
        let is_allowed = allowed_apps.contains(&app_id.to_string());

        if is_allowed {
            Ok(true)
        } else {
            Err(format!("Application '{}' is not allowed", app_id))
        }
    }

    async fn create_profile(&self, username: &str, age_group: &str) -> Result<String, String> {
        let mut profiles = self.active_profiles.write().await;
        profiles.insert(username.to_string(), age_group.to_string());
        Ok(format!("Profile '{}' created", username))
    }

    async fn get_remaining_time(&self, username: &str) -> i32 {
        // Mock: always return 120 minutes
        120
    }
}

/// Start mock daemon on system bus for testing
pub async fn start_mock_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting mock DOTS Family Daemon on system bus...");

    // Connect to system bus
    let conn = Connection::system().await?;

    // Create mock daemon instance
    let mock_daemon = Arc::new(MockDaemon::new());

    // Create the DBus object
    let daemon = FamilyDaemon::new(
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
        mock_daemon.clone(),
    );

    // Serve on system bus
    conn.object_server().at("/org/dots/FamilyDaemon")?.serve(daemon).await?;

    // Request service name
    conn.request_name("org.dots.FamilyDaemon").await?;

    println!("Mock daemon running on system bus");

    // Keep running
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
