use anyhow::Result;
use dots_family_common::types::Activity;
use dots_family_proto::daemon::FamilyDaemonProxy;
use serde_json;
use tracing::{debug, warn};
use uuid::Uuid;
use zbus::Connection;

pub struct DaemonClient {
    proxy: Option<FamilyDaemonProxy<'static>>,
}

impl DaemonClient {
    pub async fn new() -> Self {
        match Self::connect().await {
            Ok(proxy) => {
                debug!("Successfully connected to daemon via DBus");
                Self { proxy: Some(proxy) }
            }
            Err(e) => {
                warn!("Failed to connect to daemon via DBus: {}. Activity will be logged only.", e);
                Self { proxy: None }
            }
        }
    }

    async fn connect() -> Result<FamilyDaemonProxy<'static>> {
        let conn = Connection::system().await?;
        let proxy = FamilyDaemonProxy::new(&conn).await?;

        proxy.send_heartbeat("monitor").await?;
        Ok(proxy)
    }

    pub async fn get_active_profile_id(&self) -> Result<Uuid> {
        if let Some(proxy) = &self.proxy {
            let profile_json = proxy.get_active_profile().await?;

            let profile: serde_json::Value = serde_json::from_str(&profile_json)?;

            if let Some(error) = profile.get("error") {
                return Err(anyhow::anyhow!("Daemon returned error: {}", error));
            }

            if let Some(id_str) = profile.get("id").and_then(|v| v.as_str()) {
                Ok(Uuid::parse_str(id_str)?)
            } else {
                Err(anyhow::anyhow!("No profile ID found in daemon response"))
            }
        } else {
            Err(anyhow::anyhow!("No daemon connection available"))
        }
    }

    pub async fn report_activity(&self, activity: &Activity) -> Result<()> {
        if let Some(proxy) = &self.proxy {
            let activity_json = serde_json::to_string(activity)?;

            match proxy.report_activity(&activity_json).await {
                Ok(_) => {
                    debug!(
                        "Successfully reported activity: app={:?}, duration={}s",
                        activity.application, activity.duration_seconds
                    );
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to report activity to daemon: {}", e);
                    Err(e.into())
                }
            }
        } else {
            debug!("No daemon connection available, activity logged locally only: app={:?}, duration={}s", 
                  activity.application, activity.duration_seconds);
            Ok(())
        }
    }

    pub async fn send_heartbeat(&self) -> Result<()> {
        if let Some(proxy) = &self.proxy {
            proxy.send_heartbeat("monitor").await?;
            debug!("Sent heartbeat to daemon");
        }
        Ok(())
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        match Self::connect().await {
            Ok(proxy) => {
                debug!("Reconnected to daemon via DBus");
                self.proxy = Some(proxy);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to reconnect to daemon: {}", e);
                self.proxy = None;
                Err(e)
            }
        }
    }
}
