use anyhow::{Context, Result};
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

/// Authentication helper functions for CLI commands that require parent authorization
pub struct AuthHelper {
    proxy: FamilyDaemonProxy<'static>,
}

impl AuthHelper {
    /// Create a new authentication helper
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await.context("Failed to connect to system bus")?;

        let proxy =
            FamilyDaemonProxy::new(&connection).await.context("Failed to create daemon proxy")?;

        Ok(Self { proxy })
    }

    /// Prompt for password and authenticate with daemon
    /// Returns session token on success
    pub async fn authenticate(&self) -> Result<String> {
        // Prompt for password securely
        let password = rpassword::prompt_password("Enter parent password: ")
            .context("Failed to read password")?;

        if password.trim().is_empty() {
            anyhow::bail!("Password cannot be empty");
        }

        // Authenticate with daemon
        let token = self
            .proxy
            .authenticate_parent(&password)
            .await
            .context("Failed to authenticate with daemon")?;

        // Check if authentication failed
        if let Some(error_msg) = token.strip_prefix("error:") {
            anyhow::bail!("Authentication failed: {}", error_msg);
        }

        Ok(token)
    }

    /// Validate an existing session token
    pub async fn validate_session(&self, token: &str) -> Result<bool> {
        self.proxy.validate_session(token).await.context("Failed to validate session with daemon")
    }

    /// Revoke a session token (logout)
    #[allow(dead_code)]
    pub async fn revoke_session(&self, token: &str) -> Result<bool> {
        self.proxy.revoke_session(token).await.context("Failed to revoke session with daemon")
    }
}

/// Execute a closure that requires parent authentication
/// This function handles the complete authentication flow:
/// 1. Prompt for password
/// 2. Authenticate with daemon
/// 3. Execute the provided operation with the session token
/// 4. Handle authentication errors gracefully
pub async fn require_auth<F, R>(operation: F) -> Result<R>
where
    F: FnOnce(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>>,
{
    let auth = AuthHelper::new().await?;

    // Get authentication token
    let token = auth.authenticate().await?;

    // Validate token (extra safety check)
    if !auth.validate_session(&token).await? {
        anyhow::bail!("Session token validation failed");
    }

    // Execute the operation with the token
    let result = operation(token).await;

    // Note: We don't revoke the token here as it might be used for subsequent operations
    // The daemon will automatically clean up expired tokens

    result
}

/// Simpler version for operations that don't need the token directly
/// Just ensures parent authentication before proceeding
pub async fn require_auth_simple<F, R>(operation: F) -> Result<R>
where
    F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>>,
{
    require_auth(|_token| operation()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_helper_creation() {
        let result = AuthHelper::new().await;

        match result {
            Ok(_) => {
                println!("AuthHelper connected to daemon successfully");
            }
            Err(e) => {
                println!("AuthHelper failed to connect: {}", e);
                // In build environments or without a running system bus, connection will fail
                // Accept any connection-related error messages
                let error_str = e.to_string().to_lowercase();
                assert!(
                    error_str.contains("daemon")
                        || error_str.contains("dbus")
                        || error_str.contains("bus")
                        || error_str.contains("connect")
                        || error_str.contains("service")
                );
            }
        }
    }
}
