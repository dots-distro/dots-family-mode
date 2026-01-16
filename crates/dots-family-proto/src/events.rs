use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
