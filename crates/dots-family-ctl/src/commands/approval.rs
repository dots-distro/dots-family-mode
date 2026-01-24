use anyhow::{Context, Result};
use clap::Subcommand;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

use crate::auth;

#[derive(Subcommand)]
pub enum ApprovalAction {
    /// List all pending approval requests
    List,

    /// Approve a pending request
    Approve {
        /// Request ID to approve
        request_id: String,

        /// Optional response message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Deny a pending request
    Deny {
        /// Request ID to deny
        request_id: String,

        /// Optional response message
        #[arg(short, long)]
        message: Option<String>,
    },
}

pub async fn list() -> Result<()> {
    auth::require_auth(|token| {
        Box::pin(async move {
            let connection =
                Connection::system().await.context("Failed to connect to system bus")?;

            let proxy = FamilyDaemonProxy::new(&connection)
                .await
                .context("Failed to create daemon proxy")?;

            let response = proxy
                .list_pending_requests(&token)
                .await
                .context("Failed to list pending requests")?;

            let requests: serde_json::Value =
                serde_json::from_str(&response).context("Failed to parse response")?;

            if let Some(requests_array) = requests.as_array() {
                if requests_array.is_empty() {
                    println!("✅ No pending approval requests");
                    return Ok(());
                }

                println!("📋 Pending Approval Requests:\n");

                for request in requests_array {
                    let id = request["id"].as_str().unwrap_or("unknown");
                    let profile_name = request["profile_name"].as_str().unwrap_or("unknown");
                    let request_type = request["request_type"].as_str().unwrap_or("unknown");
                    let details = request["details"].as_str().unwrap_or("");
                    let created_at = request["created_at"].as_str().unwrap_or("unknown");

                    println!("🔔 Request ID: {}", id);
                    println!("   Profile: {}", profile_name);
                    println!("   Type: {}", request_type);
                    println!("   Details: {}", details);
                    println!("   Created: {}", created_at);
                    println!();
                }
            } else {
                println!("⚠️  Unexpected response format");
            }

            Ok(())
        })
    })
    .await
}

pub async fn approve(request_id: String, message: Option<String>) -> Result<()> {
    let response_msg = message.unwrap_or_else(|| "Approved".to_string());

    auth::require_auth(|token| {
        let request_id = request_id.clone();
        let response_msg = response_msg.clone();

        Box::pin(async move {
            let connection =
                Connection::system().await.context("Failed to connect to system bus")?;

            let proxy = FamilyDaemonProxy::new(&connection)
                .await
                .context("Failed to create daemon proxy")?;

            let response = proxy
                .approve_request(&request_id, &response_msg, &token)
                .await
                .context("Failed to approve request")?;

            let result: serde_json::Value =
                serde_json::from_str(&response).context("Failed to parse response")?;

            if result["success"].as_bool().unwrap_or(false) {
                println!("✅ Request approved successfully");
                if let Some(exception_id) = result["exception_id"].as_str() {
                    println!("   Exception created: {}", exception_id);
                }
            } else {
                let error = result["error"].as_str().unwrap_or("Unknown error");
                println!("❌ Failed to approve request: {}", error);
            }

            Ok(())
        })
    })
    .await
}

pub async fn deny(request_id: String, message: Option<String>) -> Result<()> {
    let response_msg = message.unwrap_or_else(|| "Denied".to_string());

    auth::require_auth(|token| {
        let request_id = request_id.clone();
        let response_msg = response_msg.clone();

        Box::pin(async move {
            let connection =
                Connection::system().await.context("Failed to connect to system bus")?;

            let proxy = FamilyDaemonProxy::new(&connection)
                .await
                .context("Failed to create daemon proxy")?;

            let response = proxy
                .deny_request(&request_id, &response_msg, &token)
                .await
                .context("Failed to deny request")?;

            let result: serde_json::Value =
                serde_json::from_str(&response).context("Failed to parse response")?;

            if result["success"].as_bool().unwrap_or(false) {
                println!("✅ Request denied");
            } else {
                let error = result["error"].as_str().unwrap_or("Unknown error");
                println!("❌ Failed to deny request: {}", error);
            }

            Ok(())
        })
    })
    .await
}
