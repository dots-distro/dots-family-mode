use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PolicyCacheEntry {
    pub profile_id: Uuid,
    pub key: String,
    pub value: String,
    pub cached_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPolicyCacheEntry {
    pub profile_id: Uuid,
    pub key: String,
    pub value: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Create a new policy cache entry
pub async fn create_cache_entry(pool: &SqlitePool, entry: &NewPolicyCacheEntry) -> Result<()> {
    let now = Utc::now();
    let profile_id_str = entry.profile_id.to_string();
    sqlx::query(
        r#"
        INSERT INTO policy_cache (profile_id, key, value, cached_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )
    .bind(profile_id_str)
    .bind(&entry.key)
    .bind(&entry.value)
    .bind(now)
    .bind(entry.expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a cached policy value
pub async fn get_cache_entry(
    pool: &SqlitePool,
    profile_id: Uuid,
    cache_key: &str,
) -> Result<Option<PolicyCacheEntry>> {
    let profile_id_str = profile_id.to_string();
    let now = Utc::now();

    let row = sqlx::query(
        r#"
        SELECT profile_id, key, value, cached_at, expires_at
        FROM policy_cache 
        WHERE profile_id = ?1 AND key = ?2 
        AND (expires_at IS NULL OR expires_at > ?3)
        "#,
    )
    .bind(profile_id_str)
    .bind(cache_key)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let profile_id_str: String = row.get("profile_id");
        let profile_id = Uuid::parse_str(&profile_id_str)?;

        Ok(Some(PolicyCacheEntry {
            profile_id,
            key: row.get("key"),
            value: row.get("value"),
            cached_at: row.get("cached_at"),
            expires_at: row.get("expires_at"),
        }))
    } else {
        Ok(None)
    }
}

/// Update a cached policy value
pub async fn update_cache_entry(pool: &SqlitePool, entry: &PolicyCacheEntry) -> Result<()> {
    let profile_id_str = entry.profile_id.to_string();
    let cached_at = Utc::now();

    sqlx::query(
        r#"
        UPDATE policy_cache 
        SET value = ?3, cached_at = ?4, expires_at = ?5
        WHERE profile_id = ?1 AND key = ?2
        "#,
    )
    .bind(profile_id_str)
    .bind(&entry.key)
    .bind(&entry.value)
    .bind(cached_at)
    .bind(entry.expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Create or update a cache entry (upsert)
pub async fn upsert_cache_entry(pool: &SqlitePool, entry: &NewPolicyCacheEntry) -> Result<()> {
    let profile_id_str = entry.profile_id.to_string();
    let cached_at = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO policy_cache (profile_id, key, value, cached_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(profile_id, key) DO UPDATE SET
        value = excluded.value,
        cached_at = excluded.cached_at,
        expires_at = excluded.expires_at
        "#,
    )
    .bind(profile_id_str)
    .bind(&entry.key)
    .bind(&entry.value)
    .bind(cached_at)
    .bind(entry.expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a cache entry
pub async fn delete_cache_entry(
    pool: &SqlitePool,
    profile_id: Uuid,
    cache_key: &str,
) -> Result<()> {
    let profile_id_str = profile_id.to_string();

    sqlx::query("DELETE FROM policy_cache WHERE profile_id = ?1 AND key = ?2")
        .bind(profile_id_str)
        .bind(cache_key)
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete all cache entries for a profile
pub async fn delete_profile_cache(pool: &SqlitePool, profile_id: Uuid) -> Result<()> {
    let profile_id_str = profile_id.to_string();

    sqlx::query("DELETE FROM policy_cache WHERE profile_id = ?1")
        .bind(profile_id_str)
        .execute(pool)
        .await?;

    Ok(())
}

/// Clean expired cache entries
pub async fn clean_expired_entries(pool: &SqlitePool) -> Result<u64> {
    let now = Utc::now();

    let result =
        sqlx::query("DELETE FROM policy_cache WHERE expires_at IS NOT NULL AND expires_at <= ?1")
            .bind(now)
            .execute(pool)
            .await?;

    Ok(result.rows_affected())
}

/// Get all cache entries for a profile
pub async fn get_profile_cache_entries(
    pool: &SqlitePool,
    profile_id: Uuid,
) -> Result<Vec<PolicyCacheEntry>> {
    let profile_id_str = profile_id.to_string();
    let now = Utc::now();

    let rows = sqlx::query(
        r#"
        SELECT profile_id, key, value, cached_at, expires_at
        FROM policy_cache 
        WHERE profile_id = ?1 
        AND (expires_at IS NULL OR expires_at > ?2)
        ORDER BY key ASC
        "#,
    )
    .bind(profile_id_str)
    .bind(now)
    .fetch_all(pool)
    .await?;

    let mut entries = Vec::new();
    for row in rows {
        let profile_id_str: String = row.get("profile_id");
        let profile_id = Uuid::parse_str(&profile_id_str)?;

        entries.push(PolicyCacheEntry {
            profile_id,
            key: row.get("key"),
            value: row.get("value"),
            cached_at: row.get("cached_at"),
            expires_at: row.get("expires_at"),
        });
    }
    Ok(entries)
}

/// Get cache statistics
pub async fn get_cache_stats(pool: &SqlitePool) -> Result<PolicyCacheStats> {
    let now = Utc::now();

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM policy_cache 
        WHERE expires_at IS NULL OR expires_at > ?1
        "#,
    )
    .bind(now)
    .fetch_one(pool)
    .await?;

    let total_entries: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM policy_cache").fetch_one(pool).await?;

    let expired_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM policy_cache WHERE expires_at IS NOT NULL AND expires_at <= ?1",
    )
    .bind(now)
    .fetch_one(pool)
    .await?;

    let profiles_with_cache: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT profile_id) 
        FROM policy_cache 
        WHERE expires_at IS NULL OR expires_at > ?1
        "#,
    )
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(PolicyCacheStats {
        active_entries: count,
        total_entries,
        expired_entries,
        profiles_with_cache,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyCacheStats {
    pub active_entries: i64,
    pub total_entries: i64,
    pub expired_entries: i64,
    pub profiles_with_cache: i64,
}
