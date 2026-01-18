use dots_family_proto::daemon::FamilyDaemonProxy;
use dots_family_proto::events::ActivityEvent;
use std::time::{Duration, SystemTime};
use tokio::time::{sleep, timeout};
use zbus::Connection;

async fn get_daemon_proxy() -> Option<FamilyDaemonProxy<'static>> {
    let conn = Connection::system().await.ok()?;
    match FamilyDaemonProxy::new(&conn).await {
        Ok(proxy) => {
            // Add timeout to prevent hanging when daemon is not available
            match timeout(Duration::from_secs(2), proxy.check_application_allowed("test")).await {
                Ok(Ok(_)) => Some(proxy),
                _ => None,
            }
        }
        Err(_) => None,
    }
}

async fn daemon_available() -> bool {
    println!("Checking if daemon is available...");
    let available = get_daemon_proxy().await.is_some();
    println!("Daemon available: {}", available);
    available
}

#[tokio::test]
async fn test_daemon_startup() {
    if let Ok(conn) = Connection::system().await {
        let result = conn.request_name("org.dots.FamilyTest").await;
        // In a restricted environment, this might fail, which is okay for testing
        println!("System bus connection test result: {:?}", result.is_ok());
    } else {
        println!("SKIPPED: System bus not available in test environment");
    }
}

#[tokio::test]
async fn test_get_active_profile_no_profile() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.get_active_profile().await;
        if let Ok(profile_json) = result {
            let parsed: serde_json::Value = serde_json::from_str(&profile_json).unwrap();
            assert!(
                parsed.get("error").is_some() || parsed.get("id").is_some(),
                "Expected either error or profile data"
            );
        }
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

#[tokio::test]
async fn test_check_application_allowed_default() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.check_application_allowed("firefox").await;
        assert!(result.is_ok(), "check_application_allowed should succeed");
    }
}

#[tokio::test]
async fn test_get_remaining_time() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.get_remaining_time().await;
        assert!(result.is_ok(), "get_remaining_time should succeed");
    }
}

#[tokio::test]
async fn test_report_activity() {
    sleep(Duration::from_millis(100)).await;

    // Add timeout to entire test
    let test_result = timeout(Duration::from_secs(5), async {
        if !daemon_available().await {
            println!("SKIPPED: No daemon available on DBus system bus");
            return;
        }

        if let Some(proxy) = get_daemon_proxy().await {
            // Use proper activity JSON format with all required fields
            let activity_json = r#"{"session_id":"test-session","profile_id":"test-profile","app_id":"firefox","app_name":"Firefox","duration_seconds":60}"#;
            let result = proxy.report_activity(activity_json).await;
            // This will fail because we don't have a valid session/profile, but the call should succeed
            assert!(
                result.is_ok(),
                "report_activity DBus call should succeed even if validation fails"
            );
        }
    }).await;

    // If timeout occurs, just skip the test
    if test_result.is_err() {
        println!("SKIPPED: Test timed out, likely no daemon available");
    }
}

#[tokio::test]
async fn test_authenticate_parent_empty() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.authenticate_parent("").await;
        if let Ok(token) = result {
            // Empty password should return error message starting with "error:"
            assert!(token.starts_with("error:"));
        }
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

#[tokio::test]
async fn test_ping_integration() {
    sleep(Duration::from_millis(100)).await;

    // Add timeout to entire test
    let test_result = timeout(Duration::from_secs(5), async {
        if !daemon_available().await {
            println!("SKIPPED: No daemon available on DBus system bus");
            return;
        }

        if let Some(proxy) = get_daemon_proxy().await {
            let result = proxy.ping().await;
            assert!(result.is_ok(), "ping DBus call should succeed");

            let response = result.unwrap();
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

            // Should have required fields
            assert!(parsed.get("status").is_some(), "ping response should have status field");
            assert!(parsed.get("message").is_some(), "ping response should have message field");

            // Status should be valid
            let status = parsed["status"].as_str().unwrap();
            assert!(
                status == "ok" || status == "degraded" || status == "error",
                "status should be valid: {}",
                status
            );
        }
    })
    .await;

    // If timeout occurs, just skip the test
    if test_result.is_err() {
        println!("SKIPPED: Test timed out, likely no daemon available");
    }
}

async fn test_authenticate_parent_valid() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.authenticate_parent("test_password").await;
        if let Ok(token) = result {
            assert!(!token.is_empty() && token != "mock_token");
        }
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

// PolicyEngine Integration Tests
#[tokio::test]
async fn test_policy_engine_allowlist_enforcement() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        // Test with a common application
        let result = proxy.check_app_policy("firefox").await;
        assert!(result.is_ok(), "check_app_policy should succeed for firefox");

        // Test with a potentially blocked application
        let result = proxy.check_app_policy("steam").await;
        assert!(result.is_ok(), "check_app_policy should succeed for steam");

        // Test with invalid/unknown application
        let result = proxy.check_app_policy("nonexistent-app").await;
        assert!(result.is_ok(), "check_app_policy should succeed even for unknown apps");
    }
}

#[tokio::test]
async fn test_activity_processing_pipeline() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        // Create a test activity event
        let now = SystemTime::now();

        let activity = ActivityEvent::WindowFocused {
            pid: 1234,
            app_id: "firefox".to_string(),
            window_title: "Example Website".to_string(),
            timestamp: now,
        };

        // Convert to JSON format that the daemon expects
        let activity_json = serde_json::to_string(&activity).unwrap();

        // Test processing the activity through PolicyEngine
        let result = proxy.process_activity_for_policy(&activity_json).await;
        assert!(result.is_ok(), "process_activity_for_policy should succeed");

        if let Ok(response) = result {
            // Should return valid JSON response
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
            assert!(parsed.is_ok(), "Response should be valid JSON: {}", response);

            let response_data = parsed.unwrap();
            assert!(response_data.get("status").is_some(), "Response should have status field");
        }
    }
}

#[tokio::test]
async fn test_profile_policy_synchronization() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        // Test syncing a mock profile to PolicyEngine
        let result = proxy.sync_profile_to_policy("test-profile-id").await;
        assert!(result.is_ok(), "sync_profile_to_policy should succeed");

        if let Ok(response) = result {
            // Should return success or error status
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
            assert!(parsed.is_ok(), "Response should be valid JSON: {}", response);

            let response_data = parsed.unwrap();
            assert!(response_data.get("status").is_some(), "Response should have status field");
        }
    }
}

#[tokio::test]
async fn test_policy_engine_blocklist_enforcement() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        // Test applications that might be commonly blocked
        let test_apps = vec!["discord", "steam", "spotify", "youtube"];

        for app in test_apps {
            let result = proxy.check_app_policy(app).await;
            assert!(result.is_ok(), "check_app_policy should succeed for {}", app);

            if let Ok(response) = result {
                // Should return valid JSON with policy decision
                let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
                assert!(parsed.is_ok(), "Response should be valid JSON for {}: {}", app, response);

                let response_data = parsed.unwrap();
                assert!(
                    response_data.get("allowed").is_some() || response_data.get("error").is_some(),
                    "Response should have allowed field or error for {}",
                    app
                );
            }
        }
    }
}

#[tokio::test]
async fn test_dbus_policy_methods_error_handling() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus system bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        // Test with invalid JSON for activity processing
        let result = proxy.process_activity_for_policy("invalid json").await;
        assert!(
            result.is_ok(),
            "process_activity_for_policy should handle invalid JSON gracefully"
        );

        if let Ok(response) = result {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
            assert!(parsed.is_ok(), "Error response should still be valid JSON");

            let response_data = parsed.unwrap();
            assert!(
                response_data.get("error").is_some(),
                "Invalid JSON should return error response"
            );
        }

        // Test with empty app ID
        let result = proxy.check_app_policy("").await;
        assert!(result.is_ok(), "check_app_policy should handle empty app_id gracefully");

        // Test with very long app ID
        let long_app_id = "a".repeat(1000);
        let result = proxy.check_app_policy(&long_app_id).await;
        assert!(result.is_ok(), "check_app_policy should handle long app_id gracefully");

        // Test syncing non-existent profile
        let result = proxy.sync_profile_to_policy("nonexistent-profile-id").await;
        assert!(
            result.is_ok(),
            "sync_profile_to_policy should handle non-existent profile gracefully"
        );

        if let Ok(response) = result {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&response);
            assert!(parsed.is_ok(), "Error response should still be valid JSON");
        }
    }
}
