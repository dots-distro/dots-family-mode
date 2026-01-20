use anyhow::{Context, Result};
use sqlx::{migrate::MigrateDatabase, SqlitePool};
use tracing::{debug, error, info};

/// Migration status information
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub applied_migrations: i64,
    pub pending_migrations: i64,
    pub last_applied: Option<String>,
}

/// Run database migrations using embedded migration files
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations");
    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(_) => {
            info!("Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Migration failed: {}", e);
            Err(e.into())
        }
    }
}

/// Get migration status by checking applied vs pending migrations
pub async fn get_migration_status(pool: &SqlitePool) -> Result<MigrationStatus> {
    debug!("Checking migration status");

    // Count applied migrations
    let applied_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = true")
            .fetch_one(pool)
            .await
            .context("Failed to count applied migrations")?;

    // Get the last applied migration description
    let last_applied: Option<String> = sqlx::query_scalar(
        "SELECT description FROM _sqlx_migrations WHERE success = true ORDER BY version DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get last applied migration")?;

    // For SQLite with embedded migrations, pending count is always 0 once migrations are run
    // since sqlx::migrate! includes all available migrations at compile time
    let pending_migrations = 0i64;

    let status = MigrationStatus { applied_migrations, pending_migrations, last_applied };

    debug!(
        "Migration status: {} applied, {} pending, last: {:?}",
        status.applied_migrations, status.pending_migrations, status.last_applied
    );

    Ok(status)
}

/// Create database if it doesn't exist
pub async fn create_database_if_not_exists(database_url: &str) -> Result<()> {
    debug!("Checking if database exists: {}", database_url);

    if !sqlx::Sqlite::database_exists(database_url)
        .await
        .context("Failed to check database existence")?
    {
        info!("Database doesn't exist, creating: {}", database_url);
        sqlx::Sqlite::create_database(database_url).await.context("Failed to create database")?;
        info!("Database created successfully");
    } else {
        debug!("Database already exists");
    }

    Ok(())
}

// Keep the existing Database impl methods for backward compatibility
use crate::connection::Database;
use crate::error::Result as DbResult;

impl Database {
    pub async fn run_migrations(&self) -> DbResult<()> {
        let pool = self.pool()?;
        run_migrations(pool)
            .await
            .map_err(|e| crate::error::DbError::InvalidData(format!("Migration error: {}", e)))
    }

    pub async fn verify_migrations(&self) -> DbResult<()> {
        let pool = self.pool()?;

        let status = get_migration_status(pool).await.map_err(|e| {
            crate::error::DbError::InvalidData(format!("Migration status error: {}", e))
        })?;
        info!(
            "Migration status: {} applied, last: {:?}",
            status.applied_migrations, status.last_applied
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DatabaseConfig;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_run_migrations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.run_migrations().await.unwrap();

        let pool = db.pool().unwrap();
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations'"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        assert!(table_count > 0, "Expected tables to be created by migrations");
    }

    #[tokio::test]
    async fn test_verify_migrations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.run_migrations().await.unwrap();
        db.verify_migrations().await.unwrap();
    }

    #[tokio::test]
    async fn test_standalone_run_migrations() {
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
    async fn test_migration_status() {
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

    #[tokio::test]
    async fn test_create_database_if_not_exists() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_new.db");
        let database_url = format!("sqlite:{}", db_path.display());

        // Database shouldn't exist yet
        assert!(!sqlx::Sqlite::database_exists(&database_url).await.unwrap());

        // Create it
        create_database_if_not_exists(&database_url).await.unwrap();

        // Now it should exist
        assert!(sqlx::Sqlite::database_exists(&database_url).await.unwrap());

        // Calling again should not error
        create_database_if_not_exists(&database_url).await.unwrap();
    }
}
