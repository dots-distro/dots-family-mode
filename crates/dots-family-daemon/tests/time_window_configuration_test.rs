use std::sync::Arc;

use anyhow::Result;
use dots_family_common::{security::PasswordManager, types::ProfileConfig};
use dots_family_daemon::{
    config::{AuthConfig, DaemonConfig},
    profile_manager::ProfileManager,
};
use dots_family_db::{models::NewProfile, queries::ProfileQueries, Database, DatabaseConfig};
use secrecy::SecretString;
use tempfile::tempdir;
use uuid::Uuid;

/// Helper to initialize a test database with migrations
async fn setup_test_database() -> Result<Arc<Database>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_time_window_config.db");

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config =
        DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

    let database = Database::new(config).await?;
    database.run_migrations().await?;

    // Keep temp_dir alive for the duration of the test
    std::mem::forget(temp_dir);

    Ok(Arc::new(database))
}

/// Helper to create a test profile
async fn create_test_profile(db: &Arc<Database>) -> Result<String> {
    let config = ProfileConfig::default();
    let config_json = serde_json::to_string(&config)?;

    let profile = NewProfile {
        id: Uuid::new_v4().to_string(),
        name: "TestChild".to_string(),
        username: Some("testchild".to_string()),
        age_group: "8-12".to_string(),
        birthday: None,
        config: config_json,
    };

    let db_profile = ProfileQueries::create(db, profile).await?;
    Ok(db_profile.id)
}

/// Helper to setup ProfileManager with authentication
async fn setup_profile_manager(db: &Arc<Database>) -> Result<(ProfileManager, String)> {
    let test_password = "test_parent_password";
    let password_secret = SecretString::new(test_password.to_string().into());
    let password_hash = PasswordManager::hash_password(&password_secret)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let daemon_config = DaemonConfig {
        auth: AuthConfig { parent_password_hash: Some(password_hash) },
        ..Default::default()
    };
    let database = (**db).clone();
    let profile_manager = ProfileManager::new(&daemon_config, database).await?;

    // Authenticate as parent to get a token
    let token = profile_manager.authenticate_parent(test_password).await?;

    Ok((profile_manager, token))
}

#[tokio::test]
async fn test_add_weekday_time_window() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add a weekday time window
    profile_manager.add_time_window(&profile_id, "weekday", "08:00", "12:00", &token).await?;

    // Verify the window was added
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");

    assert_eq!(weekday_windows.len(), 1);
    assert_eq!(weekday_windows[0]["start"], "08:00");
    assert_eq!(weekday_windows[0]["end"], "12:00");

    Ok(())
}

#[tokio::test]
async fn test_add_multiple_time_windows() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add multiple time windows
    profile_manager.add_time_window(&profile_id, "weekday", "06:00", "08:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekday", "15:00", "19:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekend", "08:00", "21:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "holiday", "09:00", "20:00", &token).await?;

    // Verify all windows were added
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");
    assert_eq!(weekday_windows.len(), 2);

    let weekend_windows =
        result.get("weekend").and_then(|w| w.as_array()).expect("Should have weekend array");
    assert_eq!(weekend_windows.len(), 1);

    let holiday_windows =
        result.get("holiday").and_then(|w| w.as_array()).expect("Should have holiday array");
    assert_eq!(holiday_windows.len(), 1);

    // Verify windows are sorted by start time
    assert_eq!(weekday_windows[0]["start"], "06:00");
    assert_eq!(weekday_windows[1]["start"], "15:00");

    Ok(())
}

#[tokio::test]
async fn test_remove_time_window() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add two time windows
    profile_manager.add_time_window(&profile_id, "weekday", "06:00", "08:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekday", "15:00", "19:00", &token).await?;

    // Remove the first window
    profile_manager.remove_time_window(&profile_id, "weekday", "06:00", "08:00", &token).await?;

    // Verify only one window remains
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");

    assert_eq!(weekday_windows.len(), 1);
    assert_eq!(weekday_windows[0]["start"], "15:00");
    assert_eq!(weekday_windows[0]["end"], "19:00");

    Ok(())
}

#[tokio::test]
async fn test_clear_time_windows() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add multiple time windows
    profile_manager.add_time_window(&profile_id, "weekday", "06:00", "08:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekday", "15:00", "19:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekend", "08:00", "21:00", &token).await?;

    // Clear weekday windows
    profile_manager.clear_time_windows(&profile_id, "weekday", &token).await?;

    // Verify weekday windows are cleared but weekend remains
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");
    assert_eq!(weekday_windows.len(), 0);

    let weekend_windows =
        result.get("weekend").and_then(|w| w.as_array()).expect("Should have weekend array");
    assert_eq!(weekend_windows.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_overlapping_windows_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add a time window
    profile_manager.add_time_window(&profile_id, "weekday", "08:00", "12:00", &token).await?;

    // Try to add an overlapping window (should fail)
    let result =
        profile_manager.add_time_window(&profile_id, "weekday", "10:00", "14:00", &token).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("overlap"), "Error should mention overlap: {}", error_msg);

    Ok(())
}

#[tokio::test]
async fn test_invalid_time_format_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Try to add window with invalid time format
    let result =
        profile_manager.add_time_window(&profile_id, "weekday", "25:00", "12:00", &token).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Hours must be 0-23") || error_msg.contains("Invalid"),
        "Error should mention invalid time: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_start_after_end_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Try to add window with start after end
    let result =
        profile_manager.add_time_window(&profile_id, "weekday", "15:00", "12:00", &token).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Start time must be before end time"),
        "Error should mention start/end order: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_window_type_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Try to add window with invalid type
    let result =
        profile_manager.add_time_window(&profile_id, "invalid", "08:00", "12:00", &token).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Invalid window type"),
        "Error should mention invalid window type: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_unauthenticated_access_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, _token) = setup_profile_manager(&db).await?;

    // Try to add window with invalid token
    let result = profile_manager
        .add_time_window(&profile_id, "weekday", "08:00", "12:00", "invalid_token")
        .await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Invalid") || error_msg.contains("expired"),
        "Error should mention invalid token: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_profile_lookup_by_name() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add window using profile name instead of ID
    profile_manager.add_time_window("TestChild", "weekday", "08:00", "12:00", &token).await?;

    // Verify using both profile ID and name
    let result_by_id = profile_manager.list_time_windows(&profile_id, &token).await?;
    let result_by_name = profile_manager.list_time_windows("TestChild", &token).await?;

    assert_eq!(result_by_id, result_by_name);

    Ok(())
}

#[tokio::test]
async fn test_nonexistent_profile_rejected() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Try to add window to non-existent profile
    let result = profile_manager
        .add_time_window("NonExistentProfile", "weekday", "08:00", "12:00", &token)
        .await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not found"),
        "Error should mention profile not found: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_windows_persist_across_sessions() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add a time window
    profile_manager.add_time_window(&profile_id, "weekday", "08:00", "12:00", &token).await?;

    // Create a new ProfileManager instance (simulates daemon restart)
    let (new_profile_manager, new_token) = setup_profile_manager(&db).await?;

    // Verify the window persists
    let result = new_profile_manager.list_time_windows(&profile_id, &new_token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");

    assert_eq!(weekday_windows.len(), 1);
    assert_eq!(weekday_windows[0]["start"], "08:00");
    assert_eq!(weekday_windows[0]["end"], "12:00");

    Ok(())
}

#[tokio::test]
async fn test_edge_case_midnight_boundary() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add window from 00:00 to 23:59
    profile_manager.add_time_window(&profile_id, "weekday", "00:00", "23:59", &token).await?;

    // Verify the window was added
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");

    assert_eq!(weekday_windows.len(), 1);
    assert_eq!(weekday_windows[0]["start"], "00:00");
    assert_eq!(weekday_windows[0]["end"], "23:59");

    Ok(())
}

#[tokio::test]
async fn test_adjacent_windows_allowed() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;
    let (profile_manager, token) = setup_profile_manager(&db).await?;

    // Add two adjacent windows (08:00-12:00 and 12:00-16:00)
    profile_manager.add_time_window(&profile_id, "weekday", "08:00", "12:00", &token).await?;
    profile_manager.add_time_window(&profile_id, "weekday", "12:00", "16:00", &token).await?;

    // Verify both windows were added (adjacent is OK, overlapping is not)
    let result = profile_manager.list_time_windows(&profile_id, &token).await?;

    let weekday_windows =
        result.get("weekday").and_then(|w| w.as_array()).expect("Should have weekday array");

    assert_eq!(weekday_windows.len(), 2);

    Ok(())
}
