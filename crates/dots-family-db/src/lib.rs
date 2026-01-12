pub mod connection;
pub mod error;
pub mod migrations;
pub mod models;
pub mod queries;

pub use connection::{Database, DatabaseConfig};
pub use error::{DbError, Result};
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let config = DatabaseConfig {
            path: db_path.to_str().unwrap().to_string(),
            encryption_key: None,
        };
        
        let db = Database::new(config).await.unwrap();
        assert!(db.pool.is_some());
    }
}
