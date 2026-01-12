use crate::connection::Database;
use crate::error::Result;
use sqlx::migrate::Migrator;
use tracing::info;

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

impl Database {
    pub async fn run_migrations(&self) -> Result<()> {
        let pool = self.pool()?;

        info!("Running database migrations");
        MIGRATOR.run(pool).await?;
        info!("Database migrations completed successfully");

        Ok(())
    }

    pub async fn verify_migrations(&self) -> Result<()> {
        let pool = self.pool()?;

        let version: i64 =
            sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations").fetch_one(pool).await?;

        info!("Current migration version: {}", version);

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
}
