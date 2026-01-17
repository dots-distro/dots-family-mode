use dots_family_proto::daemon::FamilyDaemonProxy;
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn test_ping_method_response() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.ping().await;
        assert!(result.is_ok(), "ping method should succeed");

        let response = result.unwrap();
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        // Should have required fields
        assert!(parsed.get("status").is_some());
        assert!(parsed.get("message").is_some());

        // Status should be valid
        let status = parsed["status"].as_str().unwrap();
        assert!(status == "ok" || status == "degraded" || status == "error");
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

#[tokio::test]
async fn test_report_activity_method() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        // Valid activity JSON
        let activity_json =
            r#"{"session_id":"test","profile_id":"test","app_id":"firefox","duration_seconds":60}"#;
        let result = proxy.report_activity(activity_json).await;
        assert!(result.is_ok(), "report_activity DBus call should succeed");

        let response = result.unwrap();
        // Should return success or error format
        assert!(response == "success" || response.starts_with("error:"));
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

#[tokio::test]
async fn test_report_activity_invalid_json() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let invalid_json = "{ this is not valid json }";
        let result = proxy.report_activity(invalid_json).await;
        assert!(result.is_ok(), "report_activity should not panic on invalid JSON");

        let response = result.unwrap();
        // Should return error format
        assert!(response.starts_with("error:"));
    } else {
        println!("SKIPPED: No daemon available on DBus system bus");
    }
}

async fn get_daemon_proxy() -> Option<FamilyDaemonProxy<'static>> {
    let conn = zbus::Connection::system().await.ok()?;
    match FamilyDaemonProxy::new(&conn).await {
        Ok(proxy) => {
            // Test connection with timeout
            match timeout(Duration::from_secs(2), proxy.ping()).await {
                Ok(Ok(_)) => Some(proxy),
                _ => None,
            }
        }
        Err(_) => None,
    }
}
