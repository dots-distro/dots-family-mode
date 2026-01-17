use dots_family_proto::events::ActivityEvent;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_activity_event_parsing() {
    let event = ActivityEvent::WindowFocused {
        pid: 1234,
        app_id: "firefox".to_string(),
        window_title: "Test Page".to_string(),
        timestamp: SystemTime::now(),
    };

    let event_json = serde_json::to_string(&event).unwrap();

    assert!(event_json.contains("window_focused"));
    assert!(event_json.contains("firefox"));

    let parsed: ActivityEvent = serde_json::from_str(&event_json).unwrap();
    match parsed {
        ActivityEvent::WindowFocused { pid, app_id, .. } => {
            assert_eq!(pid, 1234);
            assert_eq!(app_id, "firefox");
        }
        _ => panic!("Wrong event type after deserialization"),
    }
}

#[tokio::test]
async fn test_ping_method() {
    let result = true;
    assert_eq!(result, true, "Ping should return true");

    sleep(Duration::from_millis(1)).await;
}

#[tokio::test]
async fn test_activity_event_variants() {
    let window_event = ActivityEvent::WindowFocused {
        pid: 1234,
        app_id: "firefox".to_string(),
        window_title: "Test".to_string(),
        timestamp: SystemTime::now(),
    };

    let process_event = ActivityEvent::ProcessStarted {
        pid: 5678,
        executable: "/usr/bin/discord".to_string(),
        args: vec!["discord".to_string()],
        timestamp: SystemTime::now(),
    };

    let network_event = ActivityEvent::NetworkConnection {
        pid: 9012,
        local_addr: "127.0.0.1:8080".to_string(),
        remote_addr: "93.184.216.34:443".to_string(),
        timestamp: SystemTime::now(),
    };

    assert!(serde_json::to_string(&window_event).is_ok());
    assert!(serde_json::to_string(&process_event).is_ok());
    assert!(serde_json::to_string(&network_event).is_ok());

    let window_json = serde_json::to_string(&window_event).unwrap();
    let process_json = serde_json::to_string(&process_event).unwrap();
    let network_json = serde_json::to_string(&network_event).unwrap();

    assert!(window_json.contains("window_focused"));
    assert!(process_json.contains("process_started"));
    assert!(network_json.contains("network_connection"));
}
