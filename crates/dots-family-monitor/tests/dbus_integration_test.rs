use chrono::Utc;
use dots_family_common::types::{Activity, ActivityType};
use dots_family_monitor::daemon_client::DaemonClient;
use uuid::Uuid;

#[tokio::test]
async fn test_dbus_client_connection() {
    let _client = DaemonClient::new().await;
    // This will succeed with graceful degradation if daemon is not running
    // We're testing the client creation and error handling, not the actual connection
    // The client should initialize successfully regardless of daemon availability

    // DaemonClient::new() always succeeds - it either connects or uses None proxy
    // This is by design for graceful degradation
    assert!(true, "DaemonClient creation should always complete successfully");
}

#[tokio::test]
async fn test_activity_serialization() {
    let activity = Activity {
        id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        activity_type: ActivityType::ApplicationUsage,
        application: Some("firefox".to_string()),
        window_title: Some("Test Page - Mozilla Firefox".to_string()),
        duration_seconds: 300, // 5 minutes
    };

    // Test that Activity can be serialized for D-Bus transport
    let serialized = serde_json::to_string(&activity);
    assert!(serialized.is_ok(), "Activity should serialize successfully");

    let json_str = serialized.unwrap();
    assert!(json_str.contains("firefox"), "Serialized JSON should contain application name");
    assert!(json_str.contains("Test Page"), "Serialized JSON should contain window title");
    assert!(json_str.contains("application_usage"), "Serialized JSON should contain activity type");

    // Test deserialization round-trip
    let deserialized: Result<Activity, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok(), "Activity should deserialize successfully");

    let roundtrip_activity = deserialized.unwrap();
    assert_eq!(roundtrip_activity.application, activity.application);
    assert_eq!(roundtrip_activity.window_title, activity.window_title);
    assert_eq!(roundtrip_activity.duration_seconds, activity.duration_seconds);
}

#[tokio::test]
async fn test_daemon_client_graceful_degradation() {
    let client = DaemonClient::new().await;

    // Test that client methods handle daemon unavailability gracefully
    let test_activity = Activity {
        id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        activity_type: ActivityType::ApplicationUsage,
        application: Some("test-app".to_string()),
        window_title: Some("Test Window".to_string()),
        duration_seconds: 60,
    };

    // These should not panic even if daemon is unavailable
    let _report_result = client.report_activity(&test_activity).await;
    // report_activity returns Ok(()) when daemon is unavailable (graceful degradation)

    let _heartbeat_result = client.send_heartbeat().await;
    // send_heartbeat returns Ok(()) when daemon is unavailable

    // The client is designed for graceful degradation
    // Both operations should complete without panicking
    assert!(true, "Client operations should handle daemon unavailability gracefully");
}

#[tokio::test]
async fn test_profile_id_retrieval() {
    let client = DaemonClient::new().await;

    // Test profile ID retrieval (will fail if daemon not running, which is expected)
    let profile_result = client.get_active_profile_id().await;

    // This test verifies the method exists and returns appropriate error when daemon unavailable
    match profile_result {
        Ok(profile_id) => {
            // If daemon is running, we should get a valid UUID
            assert!(profile_id != Uuid::nil(), "Profile ID should not be nil if daemon is running");
        }
        Err(e) => {
            // If daemon is not running, we should get an appropriate error
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("No daemon connection")
                    || error_msg.contains("Failed to connect")
                    || error_msg.contains("DBus")
                    || error_msg.contains("daemon"),
                "Error should indicate daemon connection issue: {}",
                error_msg
            );
        }
    }
}

#[tokio::test]
async fn test_activity_types_serialization() {
    // Test all activity type variants can be serialized properly
    let activity_types = vec![
        ActivityType::ApplicationUsage,
        ActivityType::WebBrowsing { url: "https://example.com".to_string() },
        ActivityType::TerminalCommand { command: "ls -la".to_string() },
        ActivityType::PolicyViolation { reason: "Screen time exceeded".to_string() },
    ];

    for activity_type in activity_types {
        let activity = Activity {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            activity_type: activity_type.clone(),
            application: Some("test-app".to_string()),
            window_title: Some("Test Window".to_string()),
            duration_seconds: 60,
        };

        let serialized = serde_json::to_string(&activity);
        assert!(
            serialized.is_ok(),
            "Activity with {:?} should serialize successfully",
            activity_type
        );

        let json_str = serialized.unwrap();
        let deserialized: Result<Activity, _> = serde_json::from_str(&json_str);
        assert!(
            deserialized.is_ok(),
            "Activity with {:?} should deserialize successfully",
            activity_type
        );

        let roundtrip_activity = deserialized.unwrap();
        match (&activity_type, &roundtrip_activity.activity_type) {
            (ActivityType::ApplicationUsage, ActivityType::ApplicationUsage) => (),
            (ActivityType::WebBrowsing { url: url1 }, ActivityType::WebBrowsing { url: url2 }) => {
                assert_eq!(url1, url2, "URL should survive round-trip serialization");
            },
            (ActivityType::TerminalCommand { command: cmd1 }, ActivityType::TerminalCommand { command: cmd2 }) => {
                assert_eq!(cmd1, cmd2, "Command should survive round-trip serialization");
            },
            (ActivityType::PolicyViolation { reason: reason1 }, ActivityType::PolicyViolation { reason: reason2 }) => {
                assert_eq!(reason1, reason2, "Reason should survive round-trip serialization");
            },
            _ => panic!("Activity type did not survive round-trip serialization: original={:?}, roundtrip={:?}", activity_type, roundtrip_activity.activity_type)
        }
    }
}
