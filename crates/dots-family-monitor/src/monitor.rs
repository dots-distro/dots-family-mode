use anyhow::Result;
use chrono::Utc;
use dots_family_common::types::{Activity, ActivityType};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::MonitorConfig;
use crate::daemon_client::DaemonClient;
use crate::wayland::{WaylandMonitor, WindowInfo};

#[derive(Debug)]
struct FocusedWindow {
    info: WindowInfo,
    start_time: Instant,
}

#[derive(Debug, Default)]
pub struct ActivityTracker {
    current_focus: Option<FocusedWindow>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_focus(&mut self, new_window: Option<WindowInfo>) -> Option<Activity> {
        match (&self.current_focus, new_window) {
            (Some(current), Some(new)) => {
                let same_window =
                    current.info.app_id == new.app_id && current.info.title == new.title;

                if same_window {
                    None
                } else {
                    let duration = current.start_time.elapsed();
                    let report = Activity {
                        id: Uuid::new_v4(),
                        profile_id: Uuid::nil(),
                        timestamp: Utc::now(),
                        activity_type: ActivityType::ApplicationUsage,
                        application: current.info.app_id.clone(),
                        window_title: current.info.title.clone(),
                        duration_seconds: duration.as_secs() as u32,
                    };

                    self.current_focus =
                        Some(FocusedWindow { info: new, start_time: Instant::now() });

                    Some(report)
                }
            }
            (Some(current), None) => {
                let duration = current.start_time.elapsed();
                let report = Activity {
                    id: Uuid::new_v4(),
                    profile_id: Uuid::nil(),
                    timestamp: Utc::now(),
                    activity_type: ActivityType::ApplicationUsage,
                    application: current.info.app_id.clone(),
                    window_title: current.info.title.clone(),
                    duration_seconds: duration.as_secs() as u32,
                };

                self.current_focus = None;
                Some(report)
            }
            (None, Some(new)) => {
                self.current_focus = Some(FocusedWindow { info: new, start_time: Instant::now() });
                None
            }
            (None, None) => None,
        }
    }
}

pub async fn run() -> Result<()> {
    info!("Initializing monitor");

    let config = MonitorConfig::load()?;
    let mut wayland_monitor = WaylandMonitor::new()?;
    let mut tracker = ActivityTracker::new();

    let mut daemon_client = DaemonClient::new().await;
    let mut heartbeat_counter = 0;
    let heartbeat_interval = 100;

    info!("Monitor running, polling every {}ms", config.polling_interval_ms);

    loop {
        let window = wayland_monitor.get_focused_window().await?;

        if let Some(mut activity) = tracker.update_focus(window) {
            match daemon_client.get_active_profile_id().await {
                Ok(profile_id) => {
                    activity.profile_id = profile_id;
                    debug!("Updated activity with real profile ID: {}", profile_id);
                }
                Err(e) => {
                    warn!("Failed to get active profile ID: {}. Using nil UUID.", e);
                    activity.profile_id = Uuid::nil();
                }
            }

            info!(
                "Activity completed: app={:?}, duration={}s, profile_id={}",
                activity.application, activity.duration_seconds, activity.profile_id
            );

            if let Err(e) = daemon_client.report_activity(&activity).await {
                warn!("Failed to report activity to daemon: {}", e);

                if daemon_client.reconnect().await.is_err() {
                    warn!("Failed to reconnect to daemon. Will continue monitoring locally.");
                }
            }
        }

        heartbeat_counter += 1;
        if heartbeat_counter >= heartbeat_interval {
            if daemon_client.send_heartbeat().await.is_err() {
                warn!("Heartbeat failed, attempting to reconnect to daemon");
                let _ = daemon_client.reconnect().await;
            }
            heartbeat_counter = 0;
        }

        sleep(Duration::from_millis(config.polling_interval_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bdd_given_new_tracker_when_window_focused_then_no_activity_reported() {
        let mut tracker = ActivityTracker::new();

        let window = WindowInfo {
            app_id: Some("firefox".to_string()),
            title: Some("GitHub".to_string()),
            _pid: None,
        };

        let activity = tracker.update_focus(Some(window));

        assert!(activity.is_none());
    }

    #[test]
    fn test_bdd_given_focused_window_when_window_changes_then_activity_reported() {
        let mut tracker = ActivityTracker::new();

        let window1 = WindowInfo {
            app_id: Some("firefox".to_string()),
            title: Some("GitHub".to_string()),
            _pid: None,
        };
        tracker.update_focus(Some(window1));

        std::thread::sleep(std::time::Duration::from_millis(100));

        let window2 = WindowInfo {
            app_id: Some("code".to_string()),
            title: Some("main.rs".to_string()),
            _pid: None,
        };

        let activity = tracker.update_focus(Some(window2));

        assert!(activity.is_some());
        let report = activity.unwrap();
        assert_eq!(report.application, Some("firefox".to_string()));
        assert_eq!(report.window_title, Some("GitHub".to_string()));
        assert!(report.duration_seconds > 0);
    }

    #[test]
    fn test_bdd_given_focused_window_when_same_window_then_no_activity_reported() {
        let mut tracker = ActivityTracker::new();

        let window1 = WindowInfo {
            app_id: Some("firefox".to_string()),
            title: Some("GitHub".to_string()),
            _pid: None,
        };
        tracker.update_focus(Some(window1.clone()));

        let window2 = WindowInfo {
            app_id: Some("firefox".to_string()),
            title: Some("GitHub".to_string()),
            _pid: None,
        };

        let activity = tracker.update_focus(Some(window2));

        assert!(activity.is_none());
    }

    #[test]
    fn test_bdd_given_focused_window_when_loses_focus_then_activity_reported() {
        let mut tracker = ActivityTracker::new();

        let window = WindowInfo {
            app_id: Some("firefox".to_string()),
            title: Some("GitHub".to_string()),
            _pid: None,
        };
        tracker.update_focus(Some(window));

        std::thread::sleep(std::time::Duration::from_millis(100));

        let activity = tracker.update_focus(None);

        assert!(activity.is_some());
        let report = activity.unwrap();
        assert_eq!(report.application, Some("firefox".to_string()));
        assert!(report.duration_seconds > 0);
    }
}
