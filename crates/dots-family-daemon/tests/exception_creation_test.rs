use std::sync::Arc;

use anyhow::Result;
use dots_family_common::{security::PasswordManager, types::ProfileConfig};
use dots_family_daemon::{
    config::{AuthConfig, DaemonConfig},
    profile_manager::ProfileManager,
};
use dots_family_db::{
    models::NewProfile,
    queries::{ApprovalRequestQueries, ExceptionQueries, ProfileQueries},
    Database, DatabaseConfig,
};
use secrecy::SecretString;
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

/// Helper to initialize a test database with migrations
async fn setup_test_database() -> Result<Arc<Database>> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_exception_creation.db");

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

/// Helper to create an approval request
async fn create_approval_request(
    db: &Arc<Database>,
    profile_id: &str,
    request_type: &str,
    details: serde_json::Value,
) -> Result<String> {
    let request_id = ApprovalRequestQueries::create(db, profile_id, request_type, &details).await?;
    Ok(request_id)
}

#[tokio::test]
#[ignore]
async fn test_application_access_request_creates_exception() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;

    // Create an ApplicationAccess approval request
    let app_id = "firefox";
    let details = json!({
        "app_id": app_id,
        "reason": "Need it for homework"
    });

    let request_id = create_approval_request(&db, &profile_id, "app", details).await?;

    // Initialize ProfileManager with authentication configured
    let test_password = "test_parent_password";
    let password_secret = SecretString::new(test_password.to_string().into());
    let password_hash = PasswordManager::hash_password(&password_secret)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let daemon_config = DaemonConfig {
        auth: AuthConfig { parent_password_hash: Some(password_hash) },
        ..Default::default()
    };
    let database = (*db).clone();
    let profile_manager = ProfileManager::new(&daemon_config, database).await?;

    // Authenticate as parent to get a token
    let token = profile_manager.authenticate_parent(test_password).await?;

    // Approve the request - this should create an exception
    let exception_id =
        profile_manager.approve_request(&request_id, "Approved for homework", &token).await?;

    // ASSERTION: Exception should be created (currently returns None)
    assert!(
        exception_id.is_some(),
        "Exception should be created when ApplicationAccess request is approved"
    );

    let exception_id = exception_id.unwrap();

    // Verify exception exists in database
    let exceptions = ExceptionQueries::list_active_for_profile(&db, &profile_id).await?;
    assert_eq!(exceptions.len(), 1, "Should have one active exception");

    let exception = &exceptions[0];
    assert_eq!(exception.id, exception_id);
    assert_eq!(exception.exception_type, "app");
    assert_eq!(exception.app_id, Some(app_id.to_string()));
    assert_eq!(exception.profile_id, profile_id);
    assert!(exception.active);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_website_access_request_creates_exception() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;

    // Create a WebsiteAccess approval request
    let domain = "example.com";
    let details = json!({
        "url": "https://example.com/homework",
        "domain": domain,
        "reason": "Need it for research"
    });

    let request_id = create_approval_request(&db, &profile_id, "website", details).await?;

    // Initialize ProfileManager with authentication configured
    let test_password = "test_parent_password";
    let password_secret = SecretString::new(test_password.to_string().into());
    let password_hash = PasswordManager::hash_password(&password_secret)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let daemon_config = DaemonConfig {
        auth: AuthConfig { parent_password_hash: Some(password_hash) },
        ..Default::default()
    };
    let database = (*db).clone();
    let profile_manager = ProfileManager::new(&daemon_config, database).await?;

    // Authenticate as parent to get a token
    let token = profile_manager.authenticate_parent(test_password).await?;

    // Approve the request - this should create an exception
    let exception_id =
        profile_manager.approve_request(&request_id, "Approved for research", &token).await?;

    // ASSERTION: Exception should be created
    assert!(
        exception_id.is_some(),
        "Exception should be created when WebsiteAccess request is approved"
    );

    let exception_id = exception_id.unwrap();

    // Verify exception exists in database
    let exceptions = ExceptionQueries::list_active_for_profile(&db, &profile_id).await?;
    assert_eq!(exceptions.len(), 1, "Should have one active exception");

    let exception = &exceptions[0];
    assert_eq!(exception.id, exception_id);
    assert_eq!(exception.exception_type, "website");
    assert_eq!(exception.website, Some(domain.to_string()));
    assert_eq!(exception.profile_id, profile_id);
    assert!(exception.active);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_screen_time_extension_request_creates_exception() -> Result<()> {
    // Setup
    let db = setup_test_database().await?;
    let profile_id = create_test_profile(&db).await?;

    // Create a ScreenTimeExtension approval request
    let requested_minutes = 30;
    let details = json!({
        "requested_minutes": requested_minutes,
        "reason": "Want to finish my game"
    });

    let request_id = create_approval_request(&db, &profile_id, "screen_time", details).await?;

    // Initialize ProfileManager with authentication configured
    let test_password = "test_parent_password";
    let password_secret = SecretString::new(test_password.to_string().into());
    let password_hash = PasswordManager::hash_password(&password_secret)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let daemon_config = DaemonConfig {
        auth: AuthConfig { parent_password_hash: Some(password_hash) },
        ..Default::default()
    };
    let database = (*db).clone();
    let profile_manager = ProfileManager::new(&daemon_config, database).await?;

    // Authenticate as parent to get a token
    let token = profile_manager.authenticate_parent(test_password).await?;

    // Approve the request - this should create an exception
    let exception_id =
        profile_manager.approve_request(&request_id, "Approved for today", &token).await?;

    // ASSERTION: Exception should be created
    assert!(
        exception_id.is_some(),
        "Exception should be created when ScreenTimeExtension request is approved"
    );

    let exception_id = exception_id.unwrap();

    // Verify exception exists in database
    let exceptions = ExceptionQueries::list_active_for_profile(&db, &profile_id).await?;
    assert_eq!(exceptions.len(), 1, "Should have one active exception");

    let exception = &exceptions[0];
    assert_eq!(exception.id, exception_id);
    assert_eq!(exception.exception_type, "screen_time");
    assert_eq!(exception.amount_minutes, Some(requested_minutes as i64));
    assert_eq!(exception.profile_id, profile_id);
    assert!(exception.active);

    Ok(())
}
