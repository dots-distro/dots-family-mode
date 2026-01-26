// Phase 3 eBPF Metrics Queries

use crate::error::Result;
use crate::models::{DbDiskIoEvent, DbMemoryEvent, NewDiskIoEvent, NewMemoryEvent};
use sqlx::{Row, SqlitePool};

/// Insert a memory event into the database
pub async fn insert_memory_event(pool: &SqlitePool, event: NewMemoryEvent) -> Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO memory_events (profile_id, pid, comm, event_type, size, page_order, timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(event.profile_id)
    .bind(event.pid)
    .bind(&event.comm)
    .bind(event.event_type)
    .bind(event.size)
    .bind(event.page_order)
    .bind(event.timestamp)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

/// Get memory events for a profile within a time range
pub async fn get_memory_events(
    pool: &SqlitePool,
    profile_id: i64,
    start_timestamp: i64,
    end_timestamp: i64,
    limit: i64,
) -> Result<Vec<DbMemoryEvent>> {
    let rows = sqlx::query(
        r#"
        SELECT id, profile_id, pid, comm, event_type, size, page_order, timestamp
        FROM memory_events
        WHERE profile_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
        ORDER BY timestamp DESC
        LIMIT ?4
        "#,
    )
    .bind(profile_id)
    .bind(start_timestamp)
    .bind(end_timestamp)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let events = rows
        .into_iter()
        .map(|row| DbMemoryEvent {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            pid: row.get("pid"),
            comm: row.get("comm"),
            event_type: row.get("event_type"),
            size: row.get("size"),
            page_order: row.get("page_order"),
            timestamp: row.get("timestamp"),
        })
        .collect();

    Ok(events)
}

/// Get memory statistics for a process
pub async fn get_process_memory_stats(
    pool: &SqlitePool,
    profile_id: i64,
    pid: i32,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<(i64, i64, i64)> {
    let row = sqlx::query(
        r#"
        SELECT 
            COALESCE(SUM(CASE WHEN event_type IN (0, 2) THEN size ELSE 0 END), 0) as allocated,
            COALESCE(SUM(CASE WHEN event_type IN (1, 3) THEN size ELSE 0 END), 0) as freed,
            COUNT(*) as event_count
        FROM memory_events
        WHERE profile_id = ?1 AND pid = ?2 
          AND timestamp >= ?3 AND timestamp <= ?4
        "#,
    )
    .bind(profile_id)
    .bind(pid)
    .bind(start_timestamp)
    .bind(end_timestamp)
    .fetch_one(pool)
    .await?;

    Ok((row.get("allocated"), row.get("freed"), row.get("event_count")))
}

/// Insert a disk I/O event into the database
pub async fn insert_disk_io_event(pool: &SqlitePool, event: NewDiskIoEvent) -> Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO disk_io_events 
        (profile_id, pid, comm, device_major, device_minor, sector, nr_sectors, event_type, latency_ns, timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
    )
    .bind(event.profile_id)
    .bind(event.pid)
    .bind(&event.comm)
    .bind(event.device_major)
    .bind(event.device_minor)
    .bind(event.sector)
    .bind(event.nr_sectors)
    .bind(event.event_type)
    .bind(event.latency_ns)
    .bind(event.timestamp)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

/// Get disk I/O events for a profile within a time range
pub async fn get_disk_io_events(
    pool: &SqlitePool,
    profile_id: i64,
    start_timestamp: i64,
    end_timestamp: i64,
    limit: i64,
) -> Result<Vec<DbDiskIoEvent>> {
    let rows = sqlx::query(
        r#"
        SELECT id, profile_id, pid, comm, device_major, device_minor, 
               sector, nr_sectors, event_type, latency_ns, timestamp
        FROM disk_io_events
        WHERE profile_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
        ORDER BY timestamp DESC
        LIMIT ?4
        "#,
    )
    .bind(profile_id)
    .bind(start_timestamp)
    .bind(end_timestamp)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let events = rows
        .into_iter()
        .map(|row| DbDiskIoEvent {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            pid: row.get("pid"),
            comm: row.get("comm"),
            device_major: row.get("device_major"),
            device_minor: row.get("device_minor"),
            sector: row.get("sector"),
            nr_sectors: row.get("nr_sectors"),
            event_type: row.get("event_type"),
            latency_ns: row.get("latency_ns"),
            timestamp: row.get("timestamp"),
        })
        .collect();

    Ok(events)
}

/// Get disk I/O statistics for a process
pub async fn get_process_disk_io_stats(
    pool: &SqlitePool,
    profile_id: i64,
    pid: i32,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<(i64, i64, f64)> {
    let row = sqlx::query(
        r#"
        SELECT 
            COALESCE(SUM(nr_sectors * 512), 0) as total_bytes,
            COUNT(*) as io_count,
            COALESCE(AVG(CAST(latency_ns AS REAL)), 0.0) as avg_latency_ns
        FROM disk_io_events
        WHERE profile_id = ?1 AND pid = ?2 
          AND timestamp >= ?3 AND timestamp <= ?4
          AND event_type = 1  -- Only completed I/O operations
        "#,
    )
    .bind(profile_id)
    .bind(pid)
    .bind(start_timestamp)
    .bind(end_timestamp)
    .fetch_one(pool)
    .await?;

    Ok((row.get("total_bytes"), row.get("io_count"), row.get("avg_latency_ns")))
}

/// Delete old memory events (for cleanup/retention policies)
pub async fn delete_old_memory_events(pool: &SqlitePool, before_timestamp: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM memory_events WHERE timestamp < ?1")
        .bind(before_timestamp)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Delete old disk I/O events (for cleanup/retention policies)
pub async fn delete_old_disk_io_events(pool: &SqlitePool, before_timestamp: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM disk_io_events WHERE timestamp < ?1")
        .bind(before_timestamp)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{Database, DatabaseConfig};

    async fn create_test_db() -> Database {
        use std::env;
        use uuid::Uuid;

        let test_id = Uuid::new_v4().to_string();
        let db_path = env::temp_dir().join(format!("dots_test_{}.db", test_id));

        let config =
            DatabaseConfig { path: db_path.to_str().unwrap().to_string(), encryption_key: None };

        let db = Database::new(config).await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_insert_memory_event() {
        let db = create_test_db().await;
        let pool = db.pool().unwrap();

        // Create a test profile first
        sqlx::query(
            "INSERT INTO profiles (id, name, username, age_group, config, active) VALUES ('1', 'test', 'test', '8-12', '{}', 1)"
        )
        .execute(pool)
        .await
        .unwrap();

        let event = NewMemoryEvent {
            profile_id: 1,
            pid: 1234,
            comm: "test".to_string(),
            event_type: 0, // kmalloc
            size: 1024,
            page_order: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let id = insert_memory_event(pool, event).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_insert_disk_io_event() {
        let db = create_test_db().await;
        let pool = db.pool().unwrap();

        // Create a test profile first
        sqlx::query(
            "INSERT INTO profiles (id, name, username, age_group, config, active) VALUES ('1', 'test', 'test', '8-12', '{}', 1)"
        )
        .execute(pool)
        .await
        .unwrap();

        let event = NewDiskIoEvent {
            profile_id: 1,
            pid: 1234,
            comm: "test".to_string(),
            device_major: 8,
            device_minor: 0,
            sector: 1000,
            nr_sectors: 8,
            event_type: 1,             // complete
            latency_ns: Some(1000000), // 1ms
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let id = insert_disk_io_event(pool, event).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_get_memory_events() {
        let db = create_test_db().await;
        let pool = db.pool().unwrap();

        // Create a test profile first
        sqlx::query(
            "INSERT INTO profiles (id, name, username, age_group, config, active) VALUES ('1', 'test', 'test', '8-12', '{}', 1)"
        )
        .execute(pool)
        .await
        .unwrap();

        // Insert a test event
        let timestamp = chrono::Utc::now().timestamp_millis();
        let event = NewMemoryEvent {
            profile_id: 1,
            pid: 1234,
            comm: "test".to_string(),
            event_type: 0,
            size: 1024,
            page_order: None,
            timestamp,
        };
        insert_memory_event(pool, event).await.unwrap();

        // Query events
        let events =
            get_memory_events(pool, 1, timestamp - 1000, timestamp + 1000, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].pid, 1234);
    }

    #[tokio::test]
    async fn test_get_process_memory_stats() {
        let db = create_test_db().await;
        let pool = db.pool().unwrap();

        // Create a test profile first
        sqlx::query(
            "INSERT INTO profiles (id, name, username, age_group, config, active) VALUES ('1', 'test', 'test', '8-12', '{}', 1)"
        )
        .execute(pool)
        .await
        .unwrap();

        let timestamp = chrono::Utc::now().timestamp_millis();

        // Insert allocation
        let alloc_event = NewMemoryEvent {
            profile_id: 1,
            pid: 1234,
            comm: "test".to_string(),
            event_type: 0, // kmalloc
            size: 1024,
            page_order: None,
            timestamp,
        };
        insert_memory_event(pool, alloc_event).await.unwrap();

        // Insert free
        let free_event = NewMemoryEvent {
            profile_id: 1,
            pid: 1234,
            comm: "test".to_string(),
            event_type: 1, // kfree
            size: 512,
            page_order: None,
            timestamp: timestamp + 1000,
        };
        insert_memory_event(pool, free_event).await.unwrap();

        let (allocated, freed, _count) =
            get_process_memory_stats(pool, 1, 1234, timestamp - 1000, timestamp + 2000)
                .await
                .unwrap();

        assert_eq!(allocated, 1024);
        assert_eq!(freed, 512);
    }
}
