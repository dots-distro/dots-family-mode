use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::{distributions::Alphanumeric, Rng};
use secrecy::{ExposeSecret, SecretString};
use sha2::Digest;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure password hashing and verification using Argon2id
pub struct PasswordManager;

impl PasswordManager {
    /// Hash a password using Argon2id with secure defaults
    pub fn hash_password(password: &SecretString) -> Result<String, argon2::password_hash::Error> {
        let password_bytes = password.expose_secret().as_bytes();
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2.hash_password(password_bytes, &salt)?.to_string();
        Ok(password_hash)
    }

    /// Verify a password against a stored hash
    pub fn verify_password(
        password: &SecretString,
        hash: &str,
    ) -> Result<bool, argon2::password_hash::Error> {
        let password_bytes = password.expose_secret().as_bytes();
        let parsed_hash = PasswordHash::new(hash)?;

        match Argon2::default().verify_password(password_bytes, &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// Secure session token generation and validation
#[derive(Debug, Clone)]
pub struct SessionToken {
    token: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl SessionToken {
    /// Generate a new secure session token with 15-minute expiry
    pub fn generate() -> Self {
        let token = Self::generate_secure_token();
        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::minutes(15);

        Self { token, created_at, expires_at }
    }

    /// Generate a cryptographically secure random token
    fn generate_secure_token() -> String {
        rand::thread_rng().sample_iter(&Alphanumeric).take(64).map(char::from).collect()
    }

    /// Check if the token is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        chrono::Utc::now() < self.expires_at
    }

    /// Get the token string (should be stored securely)
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Get the expiration time
    pub fn expires_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.expires_at
    }

    /// Get the creation time
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }
}

/// Derive SQLCipher encryption key from password
#[derive(ZeroizeOnDrop)]
pub struct EncryptionKey {
    #[zeroize(skip)]
    key_hex: String,
}

impl EncryptionKey {
    /// Derive encryption key from parent password using PBKDF2
    pub fn derive_from_password(password: &SecretString, salt: Option<&[u8]>) -> Self {
        use sha2::Sha256;

        // Generate deterministic salt from password if none provided
        let default_salt;
        let salt = if let Some(provided_salt) = salt {
            provided_salt
        } else {
            let mut hasher = Sha256::new();
            hasher.update(password.expose_secret().as_bytes());
            hasher.update(b"dots-family-salt"); // Application-specific salt
            let hash = hasher.finalize();
            default_salt = hash[..16].to_vec();
            &default_salt
        };

        // Use PBKDF2 with 600,000 iterations (recommended for 2024)
        let mut key = vec![0u8; 32]; // 256-bit key
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(600_000).unwrap(),
            salt,
            password.expose_secret().as_bytes(),
            &mut key,
        );

        let key_hex = hex::encode(&key);

        // Zero out the key bytes
        key.zeroize();

        Self { key_hex }
    }

    /// Get the key in SQLCipher hex format
    pub fn as_sqlcipher_key(&self) -> String {
        format!("x'{}'", self.key_hex)
    }
}

/// Rate limiting for authentication attempts
#[derive(Debug, Clone)]
pub struct AuthAttempt {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub successful: bool,
    pub ip_address: Option<String>,
}

#[derive(Debug)]
pub struct RateLimiter {
    attempts: Vec<AuthAttempt>,
    max_attempts: usize,
    window_minutes: i64,
}

impl RateLimiter {
    /// Create a new rate limiter (5 attempts per 15 minutes by default)
    pub fn new() -> Self {
        Self { attempts: Vec::new(), max_attempts: 5, window_minutes: 15 }
    }

    /// Check if authentication attempt should be allowed
    pub fn check_rate_limit(&mut self, ip_address: Option<String>) -> bool {
        // Clean up old attempts outside the window
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(self.window_minutes);
        self.attempts.retain(|attempt| attempt.timestamp > cutoff);

        // If there's been a recent successful attempt, allow the request
        if self.attempts.iter().any(|attempt| attempt.successful) {
            return true;
        }

        // Count recent failed attempts
        let recent_failures = self.attempts.iter().filter(|attempt| !attempt.successful).count();

        if recent_failures >= self.max_attempts {
            tracing::warn!(
                "Rate limit exceeded: {} failed attempts in {} minutes from {:?}",
                recent_failures,
                self.window_minutes,
                ip_address
            );
            false
        } else {
            true
        }
    }

    /// Record an authentication attempt
    pub fn record_attempt(&mut self, successful: bool, ip_address: Option<String>) {
        self.attempts.push(AuthAttempt { timestamp: chrono::Utc::now(), successful, ip_address });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = SecretString::new("test_password_123".to_string().into());

        // Hash the password
        let hash = PasswordManager::hash_password(&password).expect("Failed to hash password");

        // Verify correct password
        assert!(
            PasswordManager::verify_password(&password, &hash).expect("Failed to verify password")
        );

        // Verify incorrect password
        let wrong_password = SecretString::new("wrong_password".to_string().into());
        assert!(!PasswordManager::verify_password(&wrong_password, &hash)
            .expect("Failed to verify wrong password"));
    }

    #[test]
    fn test_session_token_generation() {
        let token1 = SessionToken::generate();
        let token2 = SessionToken::generate();

        // Tokens should be different
        assert_ne!(token1.token(), token2.token());

        // Token should be 64 characters long
        assert_eq!(token1.token().len(), 64);

        // Token should be valid when just created
        assert!(token1.is_valid());
    }

    #[test]
    fn test_encryption_key_derivation() {
        let password = SecretString::new("parent_password_123".to_string().into());

        let key1 = EncryptionKey::derive_from_password(&password, None);
        let key2 = EncryptionKey::derive_from_password(&password, None);

        // Same password should produce same key
        assert_eq!(key1.as_sqlcipher_key(), key2.as_sqlcipher_key());

        // Key should be in correct SQLCipher format
        assert!(key1.as_sqlcipher_key().starts_with("x'"));
        assert!(key1.as_sqlcipher_key().ends_with("'"));
    }

    #[test]
    fn test_rate_limiting() {
        let mut limiter = RateLimiter::new();

        // Should allow initial attempts
        assert!(limiter.check_rate_limit(None));

        // Record multiple failed attempts
        for _ in 0..5 {
            limiter.record_attempt(false, None);
        }

        // Should block after max failures
        assert!(!limiter.check_rate_limit(None));

        // Should allow after successful attempt
        limiter.record_attempt(true, None);
        assert!(limiter.check_rate_limit(None));
    }
}
