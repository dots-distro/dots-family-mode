use anyhow::Result;
use dots_family_daemon::daemon;
use dots_family_db::{migrations, Database, DatabaseConfig};
use tempfile::tempdir;

#[tokio::test]
#[ignore]
async fn test_database_creation_and_migration() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());

    migrations::create_database_if_not_exists(&database_url).await.unwrap();
    assert!(db_path.exists(), "Database file should be created");

    let pool = sqlx::SqlitePool::connect(&database_url).await.unwrap();
    let result = migrations::run_migrations(&pool).await;
    assert!(result.is_ok(), "Database migrations should apply successfully");

    let status = migrations::get_migration_status(&pool).await.unwrap();
    assert!(status.applied_migrations > 0, "Should have applied migrations");
    assert_eq!(status.pending_migrations, 0, "Should have no pending migrations");

    pool.close().await;
}

#[tokio::test]
#[ignore]
async fn test_database_initialization_with_encryption() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("encrypted_test.db");

    let config = DatabaseConfig {
        path: db_path.to_str().unwrap().to_string(),
        encryption_key: Some("test_encryption_key_32_chars_long!!".to_string()),
    };

    let database = Database::new(config).await.unwrap();
    assert!(db_path.exists(), "Encrypted database file should be created");

    // Database::new() does NOT automatically run migrations
    database.run_migrations().await.unwrap();

    let pool = database.pool().unwrap();
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();
    assert!(
        table_names.contains(&"profiles".to_string()),
        "profiles table should exist in encrypted db"
    );
    assert!(
        table_names.contains(&"_sqlx_migrations".to_string()),
        "migration table should exist in encrypted db"
    );

    database.close().await;
}

#[tokio::test]
#[ignore]
async fn test_daemon_style_database_initialization() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("daemon_test.db");

    let result = initialize_database_for_daemon(&db_path.to_str().unwrap().to_string()).await;
    assert!(result.is_ok(), "Daemon database initialization should succeed");
    assert!(db_path.exists(), "Database file should be created");

    let database_url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::SqlitePool::connect(&database_url).await.unwrap();

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();

    assert!(table_names.contains(&"profiles".to_string()), "profiles table should exist");
    assert!(table_names.contains(&"sessions".to_string()), "sessions table should exist");
    assert!(table_names.contains(&"activities".to_string()), "activities table should exist");
    assert!(table_names.contains(&"events".to_string()), "events table should exist");
    assert!(
        table_names.contains(&"daemon_settings".to_string()),
        "daemon_settings table should exist"
    );

    pool.close().await;
}

async fn initialize_database_for_daemon(db_path: &str) -> Result<Database> {
    let db_config = DatabaseConfig { path: db_path.to_string(), encryption_key: None };

    let database = Database::new(db_config).await?;
    database.run_migrations().await?;

    Ok(database)
}

#[tokio::test]
#[ignore]
async fn test_database_url_environment_pattern() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("env_test.db");
    let database_url = format!("sqlite:{}", db_path.display());

    std::env::set_var("TEST_DATABASE_URL", &database_url);

    let url_from_env = std::env::var("TEST_DATABASE_URL").unwrap();
    assert_eq!(url_from_env, database_url);

    migrations::create_database_if_not_exists(&url_from_env).await.unwrap();
    assert!(db_path.exists(), "Database file should be created from environment URL");

    std::env::remove_var("TEST_DATABASE_URL");
}

#[tokio::test]
#[ignore]
async fn test_database_struct_initialization() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let config =
        DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

    let database = Database::new(config).await.unwrap();
    assert!(db_path.exists(), "Database file should be created");

    // Database::new() does NOT automatically run migrations
    // We need to manually call run_migrations()
    database.run_migrations().await.unwrap();

    let pool = database.pool().unwrap();
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();
    assert!(table_names.contains(&"profiles".to_string()), "profiles table should exist");
    assert!(table_names.contains(&"_sqlx_migrations".to_string()), "migration table should exist");

    database.close().await;
}

#[tokio::test]
#[ignore]
async fn test_daemon_initializes_database() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    std::env::set_var("DATABASE_URL", format!("sqlite:{}", db_path.display()));

    let result = daemon::initialize_database().await;
    assert!(result.is_ok(), "Database initialization should succeed");

    assert!(db_path.exists(), "Database file should be created");
}
