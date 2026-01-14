use dots_family_proto::daemon::FamilyDaemonProxy;
use serde_json;
use std::time::Duration;
use tokio::time::sleep;
use zbus::Connection;

async fn get_daemon_proxy() -> Option<FamilyDaemonProxy<'static>> {
    let conn = Connection::session().await.ok()?;
    match FamilyDaemonProxy::new(&conn).await {
        Ok(proxy) => {
            if proxy.check_application_allowed("test").await.is_ok() {
                Some(proxy)
            } else {
                None
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
    let conn = Connection::session().await.expect("Failed to connect to session bus");
    let result = conn.request_name("org.dots.FamilyTest").await;
    assert!(result.is_ok());
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
        println!("SKIPPED: No daemon available on DBus session bus");
    }
}

#[tokio::test]
async fn test_check_application_allowed_default() {
    sleep(Duration::from_millis(100)).await;

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus session bus");
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
        println!("SKIPPED: No daemon available on DBus session bus");
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

    if !daemon_available().await {
        println!("SKIPPED: No daemon available on DBus session bus");
        return;
    }

    if let Some(proxy) = get_daemon_proxy().await {
        let activity_json = r#"{"app":"firefox","duration":60}"#;
        let result = proxy.report_activity(activity_json).await;
        assert!(result.is_ok(), "report_activity should succeed");
    }
}

#[tokio::test]
async fn test_authenticate_parent_empty() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.authenticate_parent("").await;
        if let Ok(token) = result {
            assert!(token.is_empty());
        }
    } else {
        println!("SKIPPED: No daemon available on DBus session bus");
    }
}

#[tokio::test]
async fn test_authenticate_parent_valid() {
    sleep(Duration::from_millis(100)).await;

    if let Some(proxy) = get_daemon_proxy().await {
        let result = proxy.authenticate_parent("test_password").await;
        if let Ok(token) = result {
            assert!(!token.is_empty() && token != "mock_token");
        }
    } else {
        println!("SKIPPED: No daemon available on DBus session bus");
    }
}
