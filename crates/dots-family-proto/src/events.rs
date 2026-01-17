use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Activity events from eBPF monitoring and window tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityEvent {
    /// Window gained focus (from window manager monitoring)
    WindowFocused { pid: u32, app_id: String, window_title: String, timestamp: SystemTime },
    /// Process started (from eBPF process monitoring)
    ProcessStarted { pid: u32, executable: String, args: Vec<String>, timestamp: SystemTime },
    /// Network connection established (from eBPF network monitoring)
    NetworkConnection { pid: u32, local_addr: String, remote_addr: String, timestamp: SystemTime },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    PolicyUpdated {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    TimeLimitWarning {
        profile_id: Uuid,
        minutes_remaining: u32,
        timestamp: DateTime<Utc>,
    },
    TimeLimitReached {
        profile_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    ApplicationBlocked {
        profile_id: Uuid,
        application: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    WebsiteBlocked {
        profile_id: Uuid,
        url: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event::TimeLimitWarning {
            profile_id: Uuid::new_v4(),
            minutes_remaining: 15,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("time_limit_warning"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TimeLimitWarning { minutes_remaining, .. } => {
                assert_eq!(minutes_remaining, 15);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_policy_updated_event() {
        let profile_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = Event::PolicyUpdated { profile_id, timestamp };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("policy_updated"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::PolicyUpdated { profile_id: pid, .. } => {
                assert_eq!(pid, profile_id);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_time_limit_reached_event() {
        let event = Event::TimeLimitReached { profile_id: Uuid::new_v4(), timestamp: Utc::now() };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("time_limit_reached"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TimeLimitReached { .. } => {}
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_application_blocked_event() {
        let event = Event::ApplicationBlocked {
            profile_id: Uuid::new_v4(),
            application: "discord".to_string(),
            reason: "Not in allowlist".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("application_blocked"));
        assert!(json.contains("discord"));
        assert!(json.contains("Not in allowlist"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ApplicationBlocked { application, reason, .. } => {
                assert_eq!(application, "discord");
                assert_eq!(reason, "Not in allowlist");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_website_blocked_event() {
        let event = Event::WebsiteBlocked {
            profile_id: Uuid::new_v4(),
            url: "https://reddit.com".to_string(),
            reason: "Social media category blocked".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("website_blocked"));
        assert!(json.contains("reddit.com"));
        assert!(json.contains("Social media"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::WebsiteBlocked { url, reason, .. } => {
                assert_eq!(url, "https://reddit.com");
                assert!(reason.contains("Social media"));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_roundtrip_all_variants() {
        let profile_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let events = vec![
            Event::PolicyUpdated { profile_id, timestamp },
            Event::TimeLimitWarning { profile_id, minutes_remaining: 10, timestamp },
            Event::TimeLimitReached { profile_id, timestamp },
            Event::ApplicationBlocked {
                profile_id,
                application: "steam".to_string(),
                reason: "Games blocked during homework time".to_string(),
                timestamp,
            },
            Event::WebsiteBlocked {
                profile_id,
                url: "https://twitter.com".to_string(),
                reason: "Social media blocked".to_string(),
                timestamp,
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: Event = serde_json::from_str(&json).unwrap();

            let original_json = serde_json::to_string(&event).unwrap();
            let deserialized_json = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(original_json, deserialized_json);
        }
    }

    #[test]
    fn test_activity_event_serialization() {
        use std::time::SystemTime;

        let event = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "firefox".to_string(),
            window_title: "Test Page".to_string(),
            timestamp: SystemTime::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("window_focused"));
        assert!(json.contains("firefox"));

        let deserialized: ActivityEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            ActivityEvent::WindowFocused { pid, app_id, .. } => {
                assert_eq!(pid, 1234);
                assert_eq!(app_id, "firefox");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_process_started_event() {
        use std::time::SystemTime;

        let event = ActivityEvent::ProcessStarted {
            pid: 5678,
            executable: "/usr/bin/discord".to_string(),
            args: vec!["discord".to_string(), "--no-sandbox".to_string()],
            timestamp: SystemTime::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("process_started"));
        assert!(json.contains("discord"));

        let deserialized: ActivityEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            ActivityEvent::ProcessStarted { pid, executable, args, .. } => {
                assert_eq!(pid, 5678);
                assert!(executable.contains("discord"));
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_network_connection_event() {
        use std::time::SystemTime;

        let event = ActivityEvent::NetworkConnection {
            pid: 9012,
            local_addr: "127.0.0.1:45678".to_string(),
            remote_addr: "93.184.216.34:443".to_string(),
            timestamp: SystemTime::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("network_connection"));
        assert!(json.contains("93.184.216.34"));

        let deserialized: ActivityEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            ActivityEvent::NetworkConnection { pid, remote_addr, .. } => {
                assert_eq!(pid, 9012);
                assert!(remote_addr.contains("93.184.216.34"));
            }
            _ => panic!("Wrong event type"),
        }
    }
}
