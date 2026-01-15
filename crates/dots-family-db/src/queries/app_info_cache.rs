use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppInfoCache {
    pub app_id: String,
    pub app_name: String,
    pub category: Option<String>,
    pub desktop_file: Option<String>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAppInfoCache {
    pub app_id: String,
    pub app_name: String,
    pub category: Option<String>,
    pub desktop_file: Option<String>,
}

/// Create a new app info cache entry
pub async fn create_app_cache_entry(pool: &SqlitePool, entry: &NewAppInfoCache) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        r#"
        INSERT INTO app_info_cache (app_id, app_name, category, desktop_file, cached_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        entry.app_id,
        entry.app_name,
        entry.category,
        entry.desktop_file,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get cached app info by app_id
pub async fn get_app_cache_entry(pool: &SqlitePool, app_id: &str) -> Result<Option<AppInfoCache>> {
    let row = sqlx::query!(
        r#"
        SELECT app_id, app_name, category, desktop_file, cached_at
        FROM app_info_cache 
        WHERE app_id = ?1
        "#,
        app_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(AppInfoCache {
            app_id: row.app_id.expect("app_id should never be null as it's PRIMARY KEY"),
            app_name: row.app_name,
            category: row.category,
            desktop_file: row.desktop_file,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
        }))
    } else {
        Ok(None)
    }
}

/// Update an existing app cache entry
pub async fn update_app_cache_entry(
    pool: &SqlitePool,
    app_id: &str,
    app_name: &str,
    category: Option<&str>,
    desktop_file: Option<&str>,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        r#"
        UPDATE app_info_cache 
        SET app_name = ?2, category = ?3, desktop_file = ?4, cached_at = ?5
        WHERE app_id = ?1
        "#,
        app_id,
        app_name,
        category,
        desktop_file,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert or update an app cache entry (upsert)
pub async fn upsert_app_cache_entry(pool: &SqlitePool, entry: &NewAppInfoCache) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        r#"
        INSERT INTO app_info_cache (app_id, app_name, category, desktop_file, cached_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT (app_id) 
        DO UPDATE SET 
            app_name = excluded.app_name,
            category = excluded.category,
            desktop_file = excluded.desktop_file,
            cached_at = excluded.cached_at
        "#,
        entry.app_id,
        entry.app_name,
        entry.category,
        entry.desktop_file,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete an app cache entry
pub async fn delete_app_cache_entry(pool: &SqlitePool, app_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM app_info_cache WHERE app_id = ?1", app_id).execute(pool).await?;

    Ok(())
}

/// Get all app cache entries
pub async fn get_all_app_cache_entries(pool: &SqlitePool) -> Result<Vec<AppInfoCache>> {
    let rows = sqlx::query!(
        r#"
        SELECT app_id, app_name, category, desktop_file, cached_at
        FROM app_info_cache 
        ORDER BY app_name ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| AppInfoCache {
            app_id: row.app_id.expect("app_id should never be null as it's PRIMARY KEY"),
            app_name: row.app_name,
            category: row.category,
            desktop_file: row.desktop_file,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
        })
        .collect();

    Ok(entries)
}

/// Search app cache entries by name
pub async fn search_app_cache_by_name(
    pool: &SqlitePool,
    search_term: &str,
) -> Result<Vec<AppInfoCache>> {
    let pattern = format!("%{}%", search_term);
    let rows = sqlx::query!(
        r#"
        SELECT app_id, app_name, category, desktop_file, cached_at
        FROM app_info_cache 
        WHERE app_name LIKE ?1 OR app_id LIKE ?1
        ORDER BY app_name ASC
        "#,
        pattern
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| AppInfoCache {
            app_id: row.app_id.expect("app_id should never be null as it's PRIMARY KEY"),
            app_name: row.app_name,
            category: row.category,
            desktop_file: row.desktop_file,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
        })
        .collect();

    Ok(entries)
}

/// Get app cache entries by category
pub async fn get_app_cache_by_category(
    pool: &SqlitePool,
    category: &str,
) -> Result<Vec<AppInfoCache>> {
    let rows = sqlx::query!(
        r#"
        SELECT app_id, app_name, category, desktop_file, cached_at
        FROM app_info_cache 
        WHERE category = ?1
        ORDER BY app_name ASC
        "#,
        category
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| AppInfoCache {
            app_id: row.app_id.expect("app_id should never be null as it's PRIMARY KEY"),
            app_name: row.app_name,
            category: row.category,
            desktop_file: row.desktop_file,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
        })
        .collect();

    Ok(entries)
}

/// Get unique categories from app cache
pub async fn get_app_categories(pool: &SqlitePool) -> Result<Vec<String>> {
    let categories: Vec<Option<String>> = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT category 
        FROM app_info_cache 
        WHERE category IS NOT NULL
        ORDER BY category ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(categories.into_iter().flatten().collect())
}

/// Count total app cache entries
pub async fn count_app_cache_entries(pool: &SqlitePool) -> Result<i64> {
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM app_info_cache").fetch_one(pool).await?;

    Ok(count)
}

/// Get apps cached after a specific date
pub async fn get_recently_cached_apps(
    pool: &SqlitePool,
    since: DateTime<Utc>,
) -> Result<Vec<AppInfoCache>> {
    let rows = sqlx::query!(
        r#"
        SELECT app_id, app_name, category, desktop_file, cached_at
        FROM app_info_cache 
        WHERE cached_at > ?1
        ORDER BY cached_at DESC
        "#,
        since
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| AppInfoCache {
            app_id: row.app_id.expect("app_id should never be null as it's PRIMARY KEY"),
            app_name: row.app_name,
            category: row.category,
            desktop_file: row.desktop_file,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
        })
        .collect();

    Ok(entries)
}

/// Clear old app cache entries (older than specified date)
pub async fn clear_old_app_cache_entries(
    pool: &SqlitePool,
    older_than: DateTime<Utc>,
) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM app_info_cache WHERE cached_at < ?1", older_than)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Get app cache statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct AppCacheStats {
    pub total_apps: i64,
    pub apps_with_categories: i64,
    pub unique_categories: i64,
    pub apps_with_desktop_files: i64,
}

pub async fn get_app_cache_stats(pool: &SqlitePool) -> Result<AppCacheStats> {
    let total_apps =
        sqlx::query_scalar!("SELECT COUNT(*) FROM app_info_cache").fetch_one(pool).await?;

    let apps_with_categories =
        sqlx::query_scalar!("SELECT COUNT(*) FROM app_info_cache WHERE category IS NOT NULL")
            .fetch_one(pool)
            .await?;

    let unique_categories = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT category) FROM app_info_cache WHERE category IS NOT NULL"
    )
    .fetch_one(pool)
    .await?;

    let apps_with_desktop_files =
        sqlx::query_scalar!("SELECT COUNT(*) FROM app_info_cache WHERE desktop_file IS NOT NULL")
            .fetch_one(pool)
            .await?;

    Ok(AppCacheStats {
        total_apps,
        apps_with_categories,
        unique_categories,
        apps_with_desktop_files,
    })
}

/// Batch insert multiple app cache entries
pub async fn batch_insert_app_cache_entries(
    pool: &SqlitePool,
    entries: &[NewAppInfoCache],
) -> Result<()> {
    let mut transaction = pool.begin().await?;

    for entry in entries {
        let now = Utc::now();
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO app_info_cache (app_id, app_name, category, desktop_file, cached_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            entry.app_id,
            entry.app_name,
            entry.category,
            entry.desktop_file,
            now
        )
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(())
}
