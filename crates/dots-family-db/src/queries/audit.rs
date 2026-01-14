use crate::connection::Database;
use crate::error::{DbError, Result};
use crate::models::{DbAuditLog, NewAuditLog};
use chrono::Utc;

pub struct AuditQueries;

impl AuditQueries {
    pub async fn log(db: &Database, audit: NewAuditLog) -> Result<i64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"
            INSERT INTO audit_log 
            (timestamp, actor, action, resource, resource_id, ip_address, success, details)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Utc::now())
        .bind(&audit.actor)
        .bind(&audit.action)
        .bind(&audit.resource)
        .bind(&audit.resource_id)
        .bind(&audit.ip_address)
        .bind(audit.success)
        .bind(&audit.details)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_recent(db: &Database, limit: i64) -> Result<Vec<DbAuditLog>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbAuditLog>("SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(DbError::Sqlx)
    }

    pub async fn list_by_actor(db: &Database, actor: &str, limit: i64) -> Result<Vec<DbAuditLog>> {
        let pool = db.pool()?;

        sqlx::query_as::<_, DbAuditLog>(
            "SELECT * FROM audit_log WHERE actor = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(actor)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }

    pub async fn list_by_resource(
        db: &Database,
        resource: &str,
        resource_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<DbAuditLog>> {
        let pool = db.pool()?;

        if let Some(rid) = resource_id {
            sqlx::query_as::<_, DbAuditLog>(
                "SELECT * FROM audit_log WHERE resource = ? AND resource_id = ? ORDER BY timestamp DESC LIMIT ?",
            )
            .bind(resource)
            .bind(rid)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(DbError::Sqlx)
        } else {
            sqlx::query_as::<_, DbAuditLog>(
                "SELECT * FROM audit_log WHERE resource = ? ORDER BY timestamp DESC LIMIT ?",
            )
            .bind(resource)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(DbError::Sqlx)
        }
    }
}
