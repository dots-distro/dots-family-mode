use crate::error::{DbError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use tracing::{info, warn};

/// Format a key for `PRAGMA key`.
///
/// SQLCipher accepts either a passphrase or a raw key (`x'<hex>'`). Both are
/// passed as a quoted SQL string: SQLCipher parses the `x'<hex>'` form itself,
/// and an unquoted blob literal is a syntax error in `PRAGMA`.
fn key_pragma_value(key: &str) -> String {
    format!("\"{}\"", key.replace('"', "\"\""))
}

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

#[derive(Clone)]
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
            options = options
                .pragma("key", key_pragma_value(key))
                .pragma("cipher_page_size", "4096")
                .pragma("kdf_iter", "256000");
        } else {
            warn!("Database encryption is not enabled");
        }

        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;

        if config.encryption_key.is_some() {
            // `PRAGMA key` is silently ignored by plain libsqlite3, which would leave the
            // database in plaintext while we believe it is encrypted. Look for SQLCipher's
            // version pragma and fail closed instead of reporting a false success.
            let cipher_version: Option<String> =
                sqlx::query_scalar("PRAGMA cipher_version").fetch_optional(&pool).await?;

            match cipher_version.as_deref().map(str::trim) {
                Some(version) if !version.is_empty() => {
                    info!("SQLCipher encryption enabled (SQLCipher {})", version);
                }
                _ => {
                    pool.close().await;
                    return Err(DbError::Encryption(
                        "An encryption key was supplied but SQLCipher is not compiled in; \
                         refusing to open the database in plaintext"
                            .to_string(),
                    ));
                }
            }
        }

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
    async fn an_encrypted_database_cannot_be_opened_without_the_key() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("encrypted.db");
        let path = db_path.to_str().unwrap().to_string();
        // 32-byte raw key in SQLCipher hex-literal format. This is a fixture, not a secret.
        let key = format!("x'{}'", "00112233445566778899aabbccddeeff".repeat(2));

        {
            let db =
                Database::new(DatabaseConfig { path: path.clone(), encryption_key: Some(key) })
                    .await
                    .expect("creating an encrypted database must work when SQLCipher is linked");

            let pool = db.pool().unwrap();
            sqlx::query("CREATE TABLE secrets (value TEXT NOT NULL)").execute(pool).await.unwrap();
            sqlx::query("INSERT INTO secrets (value) VALUES ('classified')")
                .execute(pool)
                .await
                .unwrap();

            db.close().await;
        }

        // Opening without the key must not be accepted. Either the open or the
        // subsequent read has to fail; a build without SQLCipher ignores the key
        // and succeeds at both, which is exactly what this test guards against.
        let read_result: std::result::Result<Option<String>, String> =
            match Database::new(DatabaseConfig { path, encryption_key: None }).await {
                Ok(db) => {
                    let pool = db.pool().unwrap();
                    sqlx::query_scalar::<_, String>("SELECT value FROM secrets LIMIT 1")
                        .fetch_one(pool)
                        .await
                        .map(Some)
                        .map_err(|error| error.to_string())
                }
                Err(error) => Err(error.to_string()),
            };

        assert!(
            read_result.is_err(),
            "encrypted database was read without its key: {read_result:?}"
        );
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
