use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use dots_family_proto::daemon::FamilyDaemonProxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::Connection;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityReport {
    pub date: NaiveDate,
    pub screen_time_minutes: u32,
    pub top_activity: String,
    pub top_category: String,
    pub violations: u32,
    pub blocked_attempts: u32,
    pub apps_used: Vec<AppUsage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppUsage {
    pub app_id: String,
    pub app_name: String,
    pub category: String,
    pub duration_minutes: u32,
    pub percentage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeeklyReport {
    pub week_start: NaiveDate,
    pub total_screen_time_minutes: u32,
    pub average_daily_minutes: u32,
    pub most_active_day: String,
    pub top_categories: Vec<CategoryUsage>,
    pub policy_violations: u32,
    pub educational_percentage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryUsage {
    pub category: String,
    pub duration_minutes: u32,
    pub percentage: f32,
}

#[derive(Clone, Debug)]
pub struct DaemonClient {
    proxy: Arc<Mutex<Option<FamilyDaemonProxy<'static>>>>,
    connected: Arc<Mutex<bool>>,
}

impl DaemonClient {
    pub async fn new() -> Self {
        Self { proxy: Arc::new(Mutex::new(None)), connected: Arc::new(Mutex::new(false)) }
    }

    pub async fn connect(&self) -> Result<()> {
        let connection = Connection::session()
            .await
            .map_err(|e| anyhow!("Failed to connect to session bus: {}", e))?;

        let proxy = FamilyDaemonProxy::new(&connection)
            .await
            .map_err(|e| anyhow!("Failed to create daemon proxy: {}", e))?;

        match proxy.ping().await {
            Ok(_) => {
                *self.proxy.lock().await = Some(proxy);
                *self.connected.lock().await = true;
                Ok(())
            }
            Err(e) => Err(anyhow!("Failed to ping daemon: {}", e)),
        }
    }

    pub async fn get_active_profile(&self) -> Result<String> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.get_active_profile().await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn get_remaining_time(&self) -> Result<u32> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.get_remaining_time().await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn check_application_allowed(&self, app_id: &str) -> Result<bool> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.check_application_allowed(app_id).await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn list_profiles(&self) -> Result<String> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.list_profiles().await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn set_active_profile(&self, profile_id: &str) -> Result<()> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.set_active_profile(profile_id).await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn authenticate_parent(&self, password: &str) -> Result<String> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        proxy.authenticate_parent(password).await.map_err(|e| anyhow!("D-Bus error: {}", e))
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    pub async fn get_daily_report(
        &self,
        profile_id: &str,
        date: NaiveDate,
    ) -> Result<ActivityReport> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        let date_str = date.format("%Y-%m-%d").to_string();

        match proxy.get_daily_report(profile_id, &date_str).await {
            Ok(response_json) => {
                if response_json.starts_with(r#"{"error""#) {
                    return Err(anyhow!("Daemon error: {}", response_json));
                }

                let report: ActivityReport = serde_json::from_str(&response_json)
                    .map_err(|e| anyhow!("Failed to parse daily report: {}", e))?;

                Ok(report)
            }
            Err(e) => Err(anyhow!("D-Bus error getting daily report: {}", e)),
        }
    }

    pub async fn get_weekly_report(
        &self,
        profile_id: &str,
        week_start: NaiveDate,
    ) -> Result<WeeklyReport> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        let week_start_str = week_start.format("%Y-%m-%d").to_string();

        match proxy.get_weekly_report(profile_id, &week_start_str).await {
            Ok(response_json) => {
                if response_json.starts_with(r#"{"error""#) {
                    return Err(anyhow!("Daemon error: {}", response_json));
                }

                let report: WeeklyReport = serde_json::from_str(&response_json)
                    .map_err(|e| anyhow!("Failed to parse weekly report: {}", e))?;

                Ok(report)
            }
            Err(e) => Err(anyhow!("D-Bus error getting weekly report: {}", e)),
        }
    }

    pub async fn export_reports(
        &self,
        profile_id: &str,
        format: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<String> {
        let proxy_guard = self.proxy.lock().await;
        let proxy = proxy_guard.as_ref().ok_or_else(|| anyhow!("Not connected to daemon"))?;

        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        match proxy.export_reports(profile_id, format, &start_date_str, &end_date_str).await {
            Ok(response) => {
                if response.starts_with(r#"{"error""#) {
                    return Err(anyhow!("Daemon error: {}", response));
                }
                Ok(response)
            }
            Err(e) => Err(anyhow!("D-Bus error exporting reports: {}", e)),
        }
    }
}
