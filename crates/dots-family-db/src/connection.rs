use crate::error::{DbError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub path: String,
    pub encryption_key: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: "family.db".to_string(), encryption_key: None }
    }
}

pub struct Database {
    pub pool: Option<Pool<Sqlite>>,
}

impl Database {
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let pool = Self::create_pool(&config).await?;

        Ok(Self { pool: Some(pool) })
    }

    async fn create_pool(config: &DatabaseConfig) -> Result<Pool<Sqlite>> {
        let path = Path::new(&config.path);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
                info!("Created database directory: {}", parent.display());
            }
        }

        let mut options = SqliteConnectOptions::from_str(&format!("sqlite://{}", config.path))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        if let Some(key) = &config.encryption_key {
            info!("Configuring SQLCipher encryption");
            options = options
                .pragma("key", format!("x'{}'", key))
                .pragma("cipher_page_size", "4096")
                .pragma("kdf_iter", "256000");
        } else {
            warn!("Database encryption is not enabled");
        }

        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;

        info!("Database connection pool created: {}", config.path);

        Ok(pool)
    }

    pub fn pool(&self) -> Result<&Pool<Sqlite>> {
        self.pool
            .as_ref()
            .ok_or_else(|| DbError::InvalidData("Database pool not initialized".to_string()))
    }

    pub async fn close(mut self) {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
            info!("Database connection pool closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_creation_unencrypted() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        assert!(db.pool.is_some());

        let pool = db.pool().unwrap();
        let result: i32 = sqlx::query_scalar("SELECT 1").fetch_one(pool).await.unwrap();

        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_database_with_subdirectory() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("subdir").join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        assert!(db.pool.is_some());
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_database_close() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.close().await;
    }
}
