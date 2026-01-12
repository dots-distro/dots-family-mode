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
            Event::TimeLimitWarning {
                minutes_remaining, ..
            } => {
                assert_eq!(minutes_remaining, 15);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
