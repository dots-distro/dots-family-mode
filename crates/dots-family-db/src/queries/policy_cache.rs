use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
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
    sqlx::query!(
        r#"
        INSERT INTO policy_cache (profile_id, key, value, cached_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        profile_id_str,
        entry.key,
        entry.value,
        now,
        entry.expires_at
    )
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
    let now = Utc::now();
    let profile_id_str = profile_id.to_string();

    let row = sqlx::query!(
        r#"
        SELECT profile_id, key, value, cached_at, expires_at
        FROM policy_cache 
        WHERE profile_id = ?1 AND key = ?2
        AND (expires_at IS NULL OR expires_at > ?3)
        "#,
        profile_id_str,
        cache_key,
        now
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(PolicyCacheEntry {
            profile_id: Uuid::parse_str(&row.profile_id)?,
            key: row.key,
            value: row.value,
            cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
            expires_at: row.expires_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }))
    } else {
        Ok(None)
    }
}

/// Update a cache entry
pub async fn update_cache_entry(
    pool: &SqlitePool,
    profile_id: Uuid,
    cache_key: &str,
    cache_value: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<()> {
    let now = Utc::now();
    let profile_id_str = profile_id.to_string();
    sqlx::query!(
        r#"
        UPDATE policy_cache 
        SET value = ?3, cached_at = ?4, expires_at = ?5
        WHERE profile_id = ?1 AND key = ?2
        "#,
        profile_id_str,
        cache_key,
        cache_value,
        now,
        expires_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert or update a cache entry (upsert)
pub async fn upsert_cache_entry(pool: &SqlitePool, entry: &NewPolicyCacheEntry) -> Result<()> {
    let now = Utc::now();
    let profile_id_str = entry.profile_id.to_string();
    sqlx::query!(
        r#"
        INSERT INTO policy_cache (profile_id, key, value, cached_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT (profile_id, key) 
        DO UPDATE SET 
            value = excluded.value,
            cached_at = excluded.cached_at,
            expires_at = excluded.expires_at
        "#,
        profile_id_str,
        entry.key,
        entry.value,
        now,
        entry.expires_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a specific cache entry
pub async fn delete_cache_entry(
    pool: &SqlitePool,
    profile_id: Uuid,
    cache_key: &str,
) -> Result<()> {
    let profile_id_str = profile_id.to_string();
    sqlx::query!(
        "DELETE FROM policy_cache WHERE profile_id = ?1 AND key = ?2",
        profile_id_str,
        cache_key
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete all cache entries for a profile
pub async fn delete_profile_cache(pool: &SqlitePool, profile_id: Uuid) -> Result<()> {
    let profile_id_str = profile_id.to_string();
    sqlx::query!("DELETE FROM policy_cache WHERE profile_id = ?1", profile_id_str)
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete expired cache entries
pub async fn delete_expired_cache_entries(pool: &SqlitePool) -> Result<u64> {
    let now = Utc::now();
    let result = sqlx::query!(
        "DELETE FROM policy_cache WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        now
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Get all cache entries for a profile
pub async fn get_profile_cache_entries(
    pool: &SqlitePool,
    profile_id: Uuid,
) -> Result<Vec<PolicyCacheEntry>> {
    let now = Utc::now();
    let profile_id_str = profile_id.to_string();
    let rows = sqlx::query!(
        r#"
        SELECT profile_id, key, value, cached_at, expires_at
        FROM policy_cache 
        WHERE profile_id = ?1
        AND (expires_at IS NULL OR expires_at > ?2)
        ORDER BY cached_at DESC
        "#,
        profile_id_str,
        now
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| -> Result<PolicyCacheEntry> {
            Ok(PolicyCacheEntry {
                profile_id: Uuid::parse_str(&row.profile_id)?,
                key: row.key,
                value: row.value,
                cached_at: DateTime::from_naive_utc_and_offset(row.cached_at, Utc),
                expires_at: row.expires_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(entries)
}

/// Count cache entries for a profile
pub async fn count_profile_cache_entries(pool: &SqlitePool, profile_id: Uuid) -> Result<i64> {
    let now = Utc::now();
    let profile_id_str = profile_id.to_string();
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) 
        FROM policy_cache 
        WHERE profile_id = ?1
        AND (expires_at IS NULL OR expires_at > ?2)
        "#,
        profile_id_str,
        now
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Get cache statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: i64,
    pub expired_entries: i64,
    pub profiles_with_cache: i64,
}

pub async fn get_cache_stats(pool: &SqlitePool) -> Result<CacheStats> {
    let now = Utc::now();

    let total_entries =
        sqlx::query_scalar!("SELECT COUNT(*) FROM policy_cache").fetch_one(pool).await?;

    let expired_entries = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM policy_cache WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        now
    )
    .fetch_one(pool)
    .await?;

    let profiles_with_cache = sqlx::query_scalar!(
        r#"
        SELECT COUNT(DISTINCT profile_id) 
        FROM policy_cache 
        WHERE expires_at IS NULL OR expires_at > ?1
        "#,
        now
    )
    .fetch_one(pool)
    .await?;

    Ok(CacheStats { total_entries, expired_entries, profiles_with_cache })
}
