use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbProfile, NewProfile};
use chrono::Utc;
use sqlx::Row;

pub struct ProfileQueries;

impl ProfileQueries {
    pub async fn create(db: &Database, profile: NewProfile) -> Result<DbProfile> {
        let pool = db.pool()?;

        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO profiles (id, name, age_group, birthday, config, created_at, updated_at, active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.age_group)
        .bind(&profile.birthday)
        .bind(&profile.config)
        .bind(now)
        .bind(now)
        .bind(true)
        .execute(pool)
        .await;

        match result {
            Ok(_) => Self::get_by_id(db, &profile.id).await,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(DbError::Duplicate(format!("Profile '{}' already exists", profile.name)))
            }
            Err(e) => Err(DbError::Sqlx(e)),
        }
    }

    pub async fn get_by_id(db: &Database, id: &str) -> Result<DbProfile> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbProfile>("SELECT * FROM profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Profile {} not found", id)))
    }

    pub async fn get_by_name(db: &Database, name: &str) -> Result<DbProfile> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbProfile>("SELECT * FROM profiles WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Profile '{}' not found", name)))
    }

    pub async fn list_all(db: &Database) -> Result<Vec<DbProfile>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbProfile>("SELECT * FROM profiles WHERE active = ? ORDER BY name")
            .bind(true)
            .fetch_all(pool)
            .await
            .map_err(DbError::Sqlx)
    }

    pub async fn update_config(db: &Database, id: &str, config: &str) -> Result<()> {
        let pool = db.pool()?;

        let result = sqlx::query("UPDATE profiles SET config = ?, updated_at = ? WHERE id = ?")
            .bind(config)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!("Profile {} not found", id)))
        } else {
            Ok(())
        }
    }

    pub async fn deactivate(db: &Database, id: &str) -> Result<()> {
        let pool = db.pool()?;

        let result = sqlx::query("UPDATE profiles SET active = ?, updated_at = ? WHERE id = ?")
            .bind(false)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!("Profile {} not found", id)))
        } else {
            Ok(())
        }
    }

    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        let pool = db.pool()?;

        let result =
            sqlx::query("DELETE FROM profiles WHERE id = ?").bind(id).execute(pool).await?;

        if result.rows_affected() == 0 {
            Err(DbError::NotFound(format!("Profile {} not found", id)))
        } else {
            Ok(())
        }
    }

    pub async fn count(db: &Database) -> Result<i64> {
        let pool = db.pool()?;

        let row = sqlx::query("SELECT COUNT(*) FROM profiles WHERE active = ?")
            .bind(true)
            .fetch_one(pool)
            .await?;

        Ok(row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DatabaseConfig;
    use tempfile::tempdir;

    async fn setup_test_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.run_migrations().await.unwrap();
        (db, dir)
    }

    #[tokio::test]
    async fn test_create_profile() {
        let (db, _dir) = setup_test_db().await;

        let profile =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());

        let created = ProfileQueries::create(&db, profile).await.unwrap();
        assert_eq!(created.name, "TestChild");
        assert_eq!(created.age_group, "8-12");
        assert!(created.active);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let (db, _dir) = setup_test_db().await;

        let profile =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());

        let created = ProfileQueries::create(&db, profile).await.unwrap();
        let fetched = ProfileQueries::get_by_id(&db, &created.id).await.unwrap();

        assert_eq!(created.id, fetched.id);
        assert_eq!(created.name, fetched.name);
    }

    #[tokio::test]
    async fn test_duplicate_profile() {
        let (db, _dir) = setup_test_db().await;

        let profile1 =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());

        ProfileQueries::create(&db, profile1).await.unwrap();

        let profile2 =
            NewProfile::new("TestChild".to_string(), "8-12".to_string(), "{}".to_string());

        let result = ProfileQueries::create(&db, profile2).await;
        assert!(matches!(result, Err(DbError::Duplicate(_))));
    }
}
