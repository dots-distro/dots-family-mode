use dots_family_db::{create_database_if_not_exists, get_migration_status, run_migrations};
use sqlx::SqlitePool;
use tempfile::tempdir;

#[tokio::test]
async fn test_migrations_apply_successfully() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());

    // Create database if it doesn't exist
    create_database_if_not_exists(&database_url).await.unwrap();

    let pool = SqlitePool::connect(&database_url).await.unwrap();
    let result = run_migrations(&pool).await;

    assert!(result.is_ok(), "Migrations should apply successfully");

    // Verify key tables exist
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();
    assert!(table_names.contains(&"profiles".to_string()));
    assert!(table_names.contains(&"sessions".to_string()));
    assert!(table_names.contains(&"activities".to_string()));
}

#[tokio::test]
async fn test_migration_rollback_detection() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());

    // Create database if it doesn't exist
    create_database_if_not_exists(&database_url).await.unwrap();

    let pool = SqlitePool::connect(&database_url).await.unwrap();

    // Apply migrations
    run_migrations(&pool).await.unwrap();

    // Check migration status
    let status = get_migration_status(&pool).await.unwrap();
    assert!(status.applied_migrations > 0);
    assert!(status.pending_migrations == 0);
}
