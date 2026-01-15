use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone)]
pub struct FilterList {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub list_type: String, // 'builtin', 'community', 'custom'
    pub enabled: bool,
    pub last_updated: Option<DateTime<Utc>>,
    pub next_update: Option<DateTime<Utc>>,
    pub version: Option<String>,
    pub rules_count: i32,
}

pub struct FilterListQueries;

impl FilterListQueries {
    /// Create a new filter list
    pub async fn create(
        db: &Database,
        id: &str,
        name: &str,
        description: Option<&str>,
        url: Option<&str>,
        list_type: &str,
    ) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query(
            r#"INSERT INTO filter_lists 
               (id, name, description, url, type, enabled, rules_count)
               VALUES (?, ?, ?, ?, ?, 1, 0)"#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(url)
        .bind(list_type)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// List all filter lists
    pub async fn list_all(db: &Database) -> Result<Vec<FilterList>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, name, description, url, type, enabled, 
                      last_updated, next_update, version, rules_count
               FROM filter_lists
               ORDER BY name"#,
        )
        .fetch_all(pool)
        .await?;

        let mut lists = Vec::new();
        for row in rows {
            lists.push(FilterList {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                url: row.get("url"),
                list_type: row.get("type"),
                enabled: row.get("enabled"),
                last_updated: row.get("last_updated"),
                next_update: row.get("next_update"),
                version: row.get("version"),
                rules_count: row.get("rules_count"),
            });
        }

        Ok(lists)
    }

    /// List only enabled filter lists
    pub async fn list_enabled(db: &Database) -> Result<Vec<FilterList>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, name, description, url, type, enabled, 
                      last_updated, next_update, version, rules_count
               FROM filter_lists
               WHERE enabled = 1
               ORDER BY name"#,
        )
        .fetch_all(pool)
        .await?;

        let mut lists = Vec::new();
        for row in rows {
            lists.push(FilterList {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                url: row.get("url"),
                list_type: row.get("type"),
                enabled: row.get("enabled"),
                last_updated: row.get("last_updated"),
                next_update: row.get("next_update"),
                version: row.get("version"),
                rules_count: row.get("rules_count"),
            });
        }

        Ok(lists)
    }

    /// Get a filter list by ID
    pub async fn get_by_id(db: &Database, list_id: &str) -> Result<Option<FilterList>> {
        let pool = db.pool()?;

        let row = sqlx::query(
            r#"SELECT id, name, description, url, type, enabled, 
                      last_updated, next_update, version, rules_count
               FROM filter_lists
               WHERE id = ?"#,
        )
        .bind(list_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(FilterList {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                url: row.get("url"),
                list_type: row.get("type"),
                enabled: row.get("enabled"),
                last_updated: row.get("last_updated"),
                next_update: row.get("next_update"),
                version: row.get("version"),
                rules_count: row.get("rules_count"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Enable or disable a filter list
    pub async fn set_enabled(db: &Database, list_id: &str, enabled: bool) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("UPDATE filter_lists SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(list_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update filter list metadata after loading rules
    pub async fn update_metadata(
        db: &Database,
        list_id: &str,
        version: Option<&str>,
        rules_count: i32,
        next_update: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query(
            r#"UPDATE filter_lists 
               SET last_updated = CURRENT_TIMESTAMP, 
                   version = ?, 
                   rules_count = ?,
                   next_update = ?
               WHERE id = ?"#,
        )
        .bind(version)
        .bind(rules_count)
        .bind(next_update)
        .bind(list_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a filter list and all its rules
    pub async fn delete(db: &Database, list_id: &str) -> Result<()> {
        let pool = db.pool()?;

        // Rules will be cascade deleted due to foreign key constraint
        sqlx::query("DELETE FROM filter_lists WHERE id = ?").bind(list_id).execute(pool).await?;

        Ok(())
    }

    /// Get filter lists that need updates
    pub async fn list_needing_update(db: &Database) -> Result<Vec<FilterList>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, name, description, url, type, enabled, 
                      last_updated, next_update, version, rules_count
               FROM filter_lists
               WHERE enabled = 1 
               AND url IS NOT NULL
               AND (next_update IS NULL OR next_update < CURRENT_TIMESTAMP)
               ORDER BY last_updated ASC"#,
        )
        .fetch_all(pool)
        .await?;

        let mut lists = Vec::new();
        for row in rows {
            lists.push(FilterList {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                url: row.get("url"),
                list_type: row.get("type"),
                enabled: row.get("enabled"),
                last_updated: row.get("last_updated"),
                next_update: row.get("next_update"),
                version: row.get("version"),
                rules_count: row.get("rules_count"),
            });
        }

        Ok(lists)
    }
}
