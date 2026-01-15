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
        let connection = Connection::session().await.context("Failed to connect to session bus")?;

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
        if token.starts_with("error:") {
            anyhow::bail!("Authentication failed: {}", &token[6..]);
        }

        Ok(token)
    }

    /// Validate an existing session token
    pub async fn validate_session(&self, token: &str) -> Result<bool> {
        self.proxy.validate_session(token).await.context("Failed to validate session with daemon")
    }

    /// Revoke a session token (logout)
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
        // This test will only pass if the daemon is running
        // In a real environment, this would be part of integration tests
        let result = AuthHelper::new().await;

        // We expect this to fail in test environment without daemon
        assert!(result.is_err());
    }
}
