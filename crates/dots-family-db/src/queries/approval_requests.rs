use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub profile_id: String,
    pub request_type: String, // 'app', 'website', 'command', 'exception'
    pub requested_at: DateTime<Utc>,
    pub status: String, // 'pending', 'approved', 'denied'
    pub details: serde_json::Value,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub response_reason: Option<String>,
}

pub struct ApprovalRequestQueries;

impl ApprovalRequestQueries {
    /// Create a new approval request
    pub async fn create(
        db: &Database,
        profile_id: &str,
        request_type: &str,
        details: &serde_json::Value,
    ) -> Result<String> {
        let pool = db.pool()?;
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO approval_requests 
               (id, profile_id, request_type, details)
               VALUES (?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(profile_id)
        .bind(request_type)
        .bind(details.to_string())
        .execute(pool)
        .await?;

        Ok(id)
    }

    /// List all pending approval requests for a profile
    pub async fn list_pending(db: &Database, profile_id: &str) -> Result<Vec<ApprovalRequest>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, profile_id, request_type, requested_at, status, details,
                      reviewed_by, reviewed_at, response_reason
               FROM approval_requests
               WHERE profile_id = ? AND status = 'pending'
               ORDER BY requested_at DESC"#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let details_str: String = row.get("details");
            let details = serde_json::from_str(&details_str)?;

            requests.push(ApprovalRequest {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                request_type: row.get("request_type"),
                requested_at: row.get("requested_at"),
                status: row.get("status"),
                details,
                reviewed_by: row.get("reviewed_by"),
                reviewed_at: row.get("reviewed_at"),
                response_reason: row.get("response_reason"),
            });
        }

        Ok(requests)
    }

    /// List all approval requests for a profile (pending, approved, denied)
    pub async fn list_all(db: &Database, profile_id: &str) -> Result<Vec<ApprovalRequest>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, profile_id, request_type, requested_at, status, details,
                      reviewed_by, reviewed_at, response_reason
               FROM approval_requests
               WHERE profile_id = ?
               ORDER BY requested_at DESC"#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let details_str: String = row.get("details");
            let details = serde_json::from_str(&details_str)?;

            requests.push(ApprovalRequest {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                request_type: row.get("request_type"),
                requested_at: row.get("requested_at"),
                status: row.get("status"),
                details,
                reviewed_by: row.get("reviewed_by"),
                reviewed_at: row.get("reviewed_at"),
                response_reason: row.get("response_reason"),
            });
        }

        Ok(requests)
    }

    /// Approve or deny an approval request
    pub async fn review_request(
        db: &Database,
        request_id: &str,
        status: &str, // 'approved' or 'denied'
        reviewed_by: &str,
        response_reason: Option<&str>,
    ) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query(
            r#"UPDATE approval_requests 
               SET status = ?, reviewed_by = ?, reviewed_at = CURRENT_TIMESTAMP, response_reason = ?
               WHERE id = ?"#,
        )
        .bind(status)
        .bind(reviewed_by)
        .bind(response_reason)
        .bind(request_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get a specific approval request by ID
    pub async fn get_by_id(db: &Database, request_id: &str) -> Result<Option<ApprovalRequest>> {
        let pool = db.pool()?;

        let row = sqlx::query(
            r#"SELECT id, profile_id, request_type, requested_at, status, details,
                      reviewed_by, reviewed_at, response_reason
               FROM approval_requests
               WHERE id = ?"#,
        )
        .bind(request_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let details_str: String = row.get("details");
            let details = serde_json::from_str(&details_str)?;

            Ok(Some(ApprovalRequest {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                request_type: row.get("request_type"),
                requested_at: row.get("requested_at"),
                status: row.get("status"),
                details,
                reviewed_by: row.get("reviewed_by"),
                reviewed_at: row.get("reviewed_at"),
                response_reason: row.get("response_reason"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete old reviewed approval requests (for cleanup)
    pub async fn cleanup_old_requests(db: &Database, days_old: i32) -> Result<u64> {
        let pool = db.pool()?;

        let result = sqlx::query(
            r#"DELETE FROM approval_requests 
               WHERE status != 'pending' 
               AND reviewed_at < datetime('now', '-' || ? || ' days')"#,
        )
        .bind(days_old)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
