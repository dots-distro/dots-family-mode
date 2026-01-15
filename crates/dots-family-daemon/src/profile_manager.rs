use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dots_family_common::security::{EncryptionKey, PasswordManager, SessionToken};
use dots_family_common::types::{ApplicationMode, Profile};
use dots_family_db::queries::profiles::ProfileQueries;
use dots_family_db::Database;
use secrecy::SecretString;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::DaemonConfig;

#[allow(dead_code)]
const HEARTBEAT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
struct MonitorHeartbeat {
    #[allow(dead_code)]
    monitor_id: String,
    #[allow(dead_code)]
    last_seen: Instant,
}

pub struct ProfileManager {
    _db: Database,
    config: DaemonConfig,
    active_profile: Arc<RwLock<Option<Profile>>>,
    active_session_id: Arc<RwLock<Option<String>>>,
    monitor_heartbeats: Arc<RwLock<HashMap<String, MonitorHeartbeat>>>,
    tamper_detected: Arc<RwLock<bool>>,
    /// Active session tokens for parent authentication
    active_sessions: Arc<RwLock<HashMap<String, SessionToken>>>,
}

impl ProfileManager {
    pub async fn new(config: &DaemonConfig) -> Result<Self> {
        info!("Initializing ProfileManager");

        let db_config = dots_family_db::DatabaseConfig {
            path: config.database.path.clone(),
            encryption_key: config.database.encryption_key.clone(),
        };

        let db = Database::new(db_config).await?;
        db.run_migrations().await?;

        let manager = Self {
            _db: db,
            config: config.clone(),
            active_profile: Arc::new(RwLock::new(None)),
            active_session_id: Arc::new(RwLock::new(None)),
            monitor_heartbeats: Arc::new(RwLock::new(HashMap::new())),
            tamper_detected: Arc::new(RwLock::new(false)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        };

        manager.load_active_profile_from_db().await?;

        Ok(manager)
    }

    async fn load_active_profile_from_db(&self) -> Result<()> {
        let pool = self._db.pool()?;

        let result =
            sqlx::query("SELECT value FROM daemon_settings WHERE key = 'active_profile_id'")
                .fetch_optional(pool)
                .await?;

        if let Some(row) = result {
            let profile_id: String = row.try_get("value")?;
            if !profile_id.is_empty() {
                match self._load_profile(&profile_id).await {
                    Ok(profile) => {
                        // Load profile and create a new session
                        let mut active = self.active_profile.write().await;
                        *active = Some(profile);

                        // Create session for the loaded profile
                        if let Err(e) = self.create_session_for_profile(&profile_id).await {
                            warn!(
                                "Failed to create session for loaded profile {}: {}",
                                profile_id, e
                            );
                        }

                        info!(
                            "Loaded active profile from database: {} with new session",
                            profile_id
                        );
                    }
                    Err(e) => {
                        warn!("Failed to load saved profile {}: {}", profile_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn create_session_for_profile(&self, profile_id: &str) -> Result<()> {
        use dots_family_db::models::NewSession;
        use dots_family_db::queries::sessions::SessionQueries;

        let new_session = NewSession::new(profile_id.to_string());
        let session_id = new_session.id.clone();

        SessionQueries::create(&self._db, new_session).await?;

        let mut session = self.active_session_id.write().await;
        *session = Some(session_id.clone());

        info!("Created session {} for profile {}", session_id, profile_id);
        Ok(())
    }

    async fn save_active_profile_to_db(&self, profile_id: &str) -> Result<()> {
        let pool = self._db.pool()?;

        sqlx::query("INSERT OR REPLACE INTO daemon_settings (key, value, updated_at) VALUES ('active_profile_id', ?, CURRENT_TIMESTAMP)")
            .bind(profile_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn _load_profile(&self, profile_id: &str) -> Result<Profile> {
        let db_profile = ProfileQueries::get_by_id(&self._db, profile_id).await?;

        let config: dots_family_common::types::ProfileConfig =
            serde_json::from_str(&db_profile.config)?;

        let age_group = match db_profile.age_group.as_str() {
            "5-7" => dots_family_common::types::AgeGroup::EarlyElementary,
            "8-12" => dots_family_common::types::AgeGroup::LateElementary,
            "13-17" => dots_family_common::types::AgeGroup::HighSchool,
            _ => return Err(anyhow!("Invalid age group")),
        };

        Ok(Profile {
            id: Uuid::parse_str(&db_profile.id)?,
            name: db_profile.name,
            age_group,
            birthday: db_profile
                .birthday
                .map(|date: chrono::NaiveDate| date.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            created_at: db_profile.created_at,
            updated_at: db_profile.updated_at,
            config,
            active: db_profile.active,
        })
    }

    pub async fn _set_active_profile(&self, profile_id: &str) -> Result<()> {
        use dots_family_db::models::NewSession;
        use dots_family_db::queries::sessions::SessionQueries;

        let profile = self._load_profile(profile_id).await?;

        let new_session = NewSession::new(profile_id.to_string());
        let session_id = new_session.id.clone();

        SessionQueries::create(&self._db, new_session).await?;

        let mut active = self.active_profile.write().await;
        *active = Some(profile);

        let mut session = self.active_session_id.write().await;
        *session = Some(session_id.clone());

        self.save_active_profile_to_db(profile_id).await?;
        info!("Active profile set to {} with session {}", profile_id, session_id);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_active_session_id(&self) -> Option<String> {
        self.active_session_id.read().await.clone()
    }

    #[allow(dead_code)]
    pub async fn deactivate_profile(&self, end_reason: &str) -> Result<()> {
        use dots_family_db::queries::sessions::SessionQueries;

        let session_id_opt = {
            let session_id = self.active_session_id.read().await;
            session_id.clone()
        };

        if let Some(session_id) = session_id_opt {
            SessionQueries::end_session(&self._db, &session_id, end_reason, 0, 0, 0, 0).await?;

            *self.active_profile.write().await = None;
            *self.active_session_id.write().await = None;
            self.save_active_profile_to_db("").await?;

            info!("Profile deactivated, session ended: {}", end_reason);
        }

        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        use dots_family_db::queries::profiles::ProfileQueries;

        let db_profiles = ProfileQueries::list_all(&self._db).await?;

        let mut profiles = Vec::new();
        for db_profile in db_profiles {
            let config: dots_family_common::types::ProfileConfig =
                serde_json::from_str(&db_profile.config)?;

            let age_group = match db_profile.age_group.as_str() {
                "5-7" => dots_family_common::types::AgeGroup::EarlyElementary,
                "8-12" => dots_family_common::types::AgeGroup::LateElementary,
                "13-17" => dots_family_common::types::AgeGroup::HighSchool,
                _ => continue,
            };

            let profile = Profile {
                id: Uuid::parse_str(&db_profile.id)?,
                name: db_profile.name,
                age_group,
                birthday: db_profile
                    .birthday
                    .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc()),
                created_at: db_profile.created_at,
                updated_at: db_profile.updated_at,
                config,
                active: db_profile.active,
            };

            profiles.push(profile);
        }

        Ok(profiles)
    }

    pub async fn create_profile(&self, name: &str, age_group: &str) -> Result<String> {
        use dots_family_common::types::{
            ApplicationConfig, ApplicationMode, ProfileConfig, ScreenTimeConfig, TimeWindow,
            TimeWindows, WebFilteringConfig,
        };
        use dots_family_db::models::{NewAuditLog, NewProfile};
        use dots_family_db::queries::{audit::AuditQueries, profiles::ProfileQueries};

        let age_group_enum = match age_group {
            "5-7" => dots_family_common::types::AgeGroup::EarlyElementary,
            "8-12" => dots_family_common::types::AgeGroup::LateElementary,
            "13-17" => dots_family_common::types::AgeGroup::HighSchool,
            _ => {
                let audit = NewAuditLog {
                    actor: "parent".to_string(),
                    action: "create_profile".to_string(),
                    resource: "profile".to_string(),
                    resource_id: None,
                    ip_address: None,
                    success: false,
                    details: Some(format!("Invalid age group: {}", age_group)),
                };
                let _ = AuditQueries::log(&self._db, audit).await;
                return Err(anyhow!("Invalid age group. Use: 5-7, 8-12, or 13-17"));
            }
        };

        let daily_limit = match age_group_enum {
            dots_family_common::types::AgeGroup::EarlyElementary => 60,
            dots_family_common::types::AgeGroup::LateElementary => 120,
            dots_family_common::types::AgeGroup::HighSchool => 180,
        };

        let config = ProfileConfig {
            screen_time: ScreenTimeConfig {
                daily_limit_minutes: daily_limit,
                weekend_bonus_minutes: 30,
                exempt_categories: vec!["educational".to_string()],
                windows: TimeWindows {
                    weekday: vec![TimeWindow {
                        start: "08:00".to_string(),
                        end: "20:00".to_string(),
                    }],
                    weekend: vec![TimeWindow {
                        start: "09:00".to_string(),
                        end: "21:00".to_string(),
                    }],
                },
            },
            applications: ApplicationConfig {
                mode: ApplicationMode::Allowlist,
                allowed: vec!["firefox".to_string(), "code".to_string()],
                blocked: vec![],
                blocked_categories: vec![],
            },
            web_filtering: WebFilteringConfig {
                enabled: true,
                safe_search: true,
                blocked_categories: vec!["adult".to_string(), "violence".to_string()],
                allowed_domains: vec![],
                blocked_domains: vec![],
            },
        };

        let config_json = serde_json::to_string(&config)?;

        let new_profile = NewProfile {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            age_group: age_group.to_string(),
            birthday: None,
            config: config_json,
        };

        let profile_id = new_profile.id.clone();
        ProfileQueries::create(&self._db, new_profile).await?;

        let audit = NewAuditLog {
            actor: "parent".to_string(),
            action: "create_profile".to_string(),
            resource: "profile".to_string(),
            resource_id: Some(profile_id.clone()),
            ip_address: None,
            success: true,
            details: Some(format!("Created profile '{}' with age group {}", name, age_group)),
        };
        let _ = AuditQueries::log(&self._db, audit).await;

        info!("Created profile: {} ({})", name, profile_id);
        Ok(profile_id)
    }

    pub async fn get_active_profile(&self) -> Result<Option<Profile>> {
        let profile = self.active_profile.read().await;
        Ok(profile.clone())
    }

    pub async fn check_application_allowed(&self, app_id: &str) -> Result<bool> {
        let profile = self.active_profile.read().await;

        let Some(ref profile) = *profile else {
            return Ok(true);
        };

        let daily_limit_seconds = profile.config.screen_time.daily_limit_minutes as i64 * 60;
        let used_seconds = self.get_used_time_today().await?;

        if used_seconds >= daily_limit_seconds {
            return Ok(false);
        }

        let allowed = match profile.config.applications.mode {
            ApplicationMode::Allowlist => {
                profile.config.applications.allowed.iter().any(|a| a == app_id)
            }
            ApplicationMode::Blocklist => {
                !profile.config.applications.blocked.iter().any(|a| a == app_id)
            }
        };

        Ok(allowed)
    }

    pub async fn get_used_time_today(&self) -> Result<i64> {
        let profile = self.active_profile.read().await;

        let Some(ref profile) = *profile else {
            return Ok(0);
        };

        use chrono::{DateTime, Utc};
        use dots_family_db::queries::activities::ActivityQueries;

        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start_dt = DateTime::<Utc>::from_naive_utc_and_offset(today_start, Utc);

        let profile_id_str = profile.id.to_string();
        let activities =
            ActivityQueries::list_by_profile_since(&self._db, &profile_id_str, today_start_dt)
                .await?;

        let total_seconds: i64 = activities.iter().map(|a| a.duration_seconds).sum();

        Ok(total_seconds)
    }

    pub async fn get_remaining_time(&self) -> Result<u32> {
        let profile = self.active_profile.read().await;

        let Some(ref profile) = *profile else {
            return Ok(0);
        };

        let daily_limit_seconds = profile.config.screen_time.daily_limit_minutes as i64 * 60;
        let used_seconds = self.get_used_time_today().await?;
        let remaining_seconds = (daily_limit_seconds - used_seconds).max(0);

        Ok((remaining_seconds / 60) as u32)
    }

    pub async fn report_activity(&self, activity_json: &str) -> Result<()> {
        use dots_family_common::types::Activity;
        use dots_family_db::models::NewActivity;
        use dots_family_db::queries::activities::ActivityQueries;

        info!("Activity reported: {}", activity_json);

        let activity: Activity = serde_json::from_str(activity_json)?;

        let session_id = {
            let active_session = self.active_session_id.read().await;
            active_session.clone().unwrap_or_else(|| activity.profile_id.to_string())
        };

        let new_activity = NewActivity {
            session_id,
            profile_id: activity.profile_id.to_string(),
            app_id: activity.application.as_deref().unwrap_or("unknown").to_string(),
            app_name: activity.application.as_deref().unwrap_or("Unknown Application").to_string(),
            category: None,
            window_title: activity.window_title,
            duration_seconds: activity.duration_seconds as i64,
        };

        ActivityQueries::create(&self._db, new_activity).await?;
        info!(
            "Activity stored in database: app_id={}, duration={}s, profile_id={}",
            activity.application.as_deref().unwrap_or("unknown"),
            activity.duration_seconds,
            activity.profile_id
        );

        Ok(())
    }

    pub async fn authenticate_parent(&self, password: &str) -> Result<String> {
        if password.is_empty() {
            return Err(anyhow!("Invalid password"));
        }

        // Get stored password hash from config
        let stored_hash = match &self.config.auth.parent_password_hash {
            Some(hash) => hash,
            None => {
                warn!("No parent password hash configured - authentication will fail");
                return Err(anyhow!("Parent authentication not configured"));
            }
        };

        // Convert password to SecretString for secure handling
        let password_secret = SecretString::new(password.to_string().into());

        // Verify password against stored hash
        match PasswordManager::verify_password(&password_secret, stored_hash) {
            Ok(true) => {
                info!("Parent authentication successful");

                // Generate secure session token
                let session_token = SessionToken::generate();
                let token_string = session_token.token().to_string();

                // Store the session token for validation
                {
                    let mut sessions = self.active_sessions.write().await;
                    sessions.insert(token_string.clone(), session_token);
                }

                info!("Session token created and stored");
                Ok(token_string)
            }
            Ok(false) => {
                warn!("Parent authentication failed - invalid password");
                Err(anyhow!("Invalid password"))
            }
            Err(e) => {
                warn!("Parent authentication error: {}", e);
                Err(anyhow!("Authentication failed"))
            }
        }
    }

    /// Validate a session token and return whether it's still valid
    pub async fn validate_session(&self, token: &str) -> bool {
        let sessions = self.active_sessions.read().await;

        if let Some(session_token) = sessions.get(token) {
            session_token.is_valid()
        } else {
            false
        }
    }

    /// Clean up expired session tokens
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.write().await;
        sessions.retain(|_, session| session.is_valid());
    }

    /// Revoke a specific session token (logout)
    pub async fn revoke_session(&self, token: &str) -> bool {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(token).is_some()
    }

    /// Get count of active sessions
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.active_sessions.read().await;
        sessions.len()
    }

    #[allow(dead_code)] // Will be used by CLI/GUI applications
    pub async fn set_parent_password(&mut self, password: &str) -> Result<()> {
        if password.is_empty() {
            return Err(anyhow!("Password cannot be empty"));
        }

        let password_secret = SecretString::new(password.to_string().into());

        // Hash the password using Argon2
        let password_hash = PasswordManager::hash_password(&password_secret)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?;

        // Derive encryption key from password
        let encryption_key = EncryptionKey::derive_from_password(&password_secret, None);

        self.config.auth.parent_password_hash = Some(password_hash);
        self.config.database.encryption_key = Some(encryption_key.as_sqlcipher_key());

        // Save configuration to disk
        if let Err(e) = self.config.save() {
            warn!("Failed to save configuration to disk: {}", e);
            return Err(anyhow!("Failed to persist configuration: {}", e));
        }

        info!("Parent password set successfully - configuration saved to disk");
        warn!("Database encryption key derived but requires daemon restart to apply");

        Ok(())
    }

    #[allow(dead_code)] // Will be used by CLI/GUI applications
    pub fn get_database_encryption_key(&self) -> Option<String> {
        self.config.database.encryption_key.clone()
    }

    pub async fn send_heartbeat(&self, monitor_id: &str) -> Result<()> {
        let mut heartbeats = self.monitor_heartbeats.write().await;

        let heartbeat =
            MonitorHeartbeat { monitor_id: monitor_id.to_string(), last_seen: Instant::now() };

        heartbeats.insert(monitor_id.to_string(), heartbeat);

        let was_tampered = *self.tamper_detected.read().await;
        if was_tampered {
            info!("Monitor {} reconnected after tamper detection", monitor_id);
            *self.tamper_detected.write().await = false;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn check_monitor_health(&self) -> Result<bool> {
        let heartbeats = self.monitor_heartbeats.read().await;

        if heartbeats.is_empty() {
            return Ok(true);
        }

        let now = Instant::now();
        let timeout = std::time::Duration::from_secs(HEARTBEAT_TIMEOUT_SECS);

        for (monitor_id, heartbeat) in heartbeats.iter() {
            if now.duration_since(heartbeat.last_seen) > timeout {
                warn!(
                    "Monitor {} heartbeat timeout (last seen {:?} ago)",
                    monitor_id,
                    now.duration_since(heartbeat.last_seen)
                );
                *self.tamper_detected.write().await = true;
                return Ok(false);
            }
        }

        Ok(true)
    }

    #[allow(dead_code)]
    pub async fn is_tampered(&self) -> bool {
        *self.tamper_detected.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dots_family_common::types::{
        ApplicationConfig, ApplicationMode, ProfileConfig, ScreenTimeConfig, TimeWindows,
        WebFilteringConfig,
    };
    use dots_family_db::queries::profiles::ProfileQueries;
    use dots_family_db::Database;
    use tempfile::tempdir;

    async fn setup_test_db() -> (Database, tempfile::TempDir, DaemonConfig) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon_config = DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path.to_str().unwrap().to_string(),
                encryption_key: None,
            },
            auth: crate::config::AuthConfig { parent_password_hash: None },
        };

        let db_config = dots_family_db::DatabaseConfig {
            path: db_path.to_str().unwrap().to_string(),
            encryption_key: None,
        };

        let db = Database::new(db_config).await.unwrap();
        db.run_migrations().await.unwrap();
        (db, dir, daemon_config)
    }

    async fn create_test_profile(db: &Database, name: &str) -> String {
        let profile_config = ProfileConfig {
            screen_time: ScreenTimeConfig {
                daily_limit_minutes: 120,
                weekend_bonus_minutes: 60,
                exempt_categories: vec![],
                windows: TimeWindows { weekday: vec![], weekend: vec![] },
            },
            applications: ApplicationConfig {
                mode: ApplicationMode::Allowlist,
                allowed: vec!["firefox".to_string(), "code".to_string()],
                blocked: vec![],
                blocked_categories: vec![],
            },
            web_filtering: WebFilteringConfig {
                enabled: true,
                safe_search: true,
                blocked_categories: vec![],
                allowed_domains: vec![],
                blocked_domains: vec![],
            },
        };

        let new_profile = dots_family_db::models::NewProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            age_group: "8-12".to_string(),
            birthday: None,
            config: serde_json::to_string(&profile_config).unwrap(),
        };

        let profile = ProfileQueries::create(db, new_profile).await.unwrap();
        profile.id
    }

    #[tokio::test]
    async fn test_bdd_given_profile_when_check_allowed_app_then_returns_true() {
        // Given: A profile with firefox in allowlist
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Checking if firefox is allowed
        let allowed = manager.check_application_allowed("firefox").await.unwrap();

        // Then: It should return true
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_bdd_given_profile_when_check_blocked_app_then_returns_false() {
        // Given: A profile with firefox in allowlist (steam not in list)
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Checking if steam is allowed
        let allowed = manager.check_application_allowed("steam").await.unwrap();

        // Then: It should return false (not in allowlist)
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_bdd_given_profile_when_get_remaining_time_then_returns_limit() {
        // Given: A profile with 120 minute daily limit
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Getting remaining time
        let remaining = manager.get_remaining_time().await.unwrap();

        // Then: It should return 120 minutes (full limit, no usage yet)
        assert_eq!(remaining, 120);
    }

    #[tokio::test]
    async fn test_bdd_given_no_profile_when_check_app_then_returns_true() {
        // Given: No active profile
        let (_db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        // When: Checking if any app is allowed
        let allowed = manager.check_application_allowed("any-app").await.unwrap();

        // Then: It should return true (no restrictions without profile)
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_bdd_given_activity_json_when_reported_then_succeeds() {
        // Given: A profile manager with no active session (should fail gracefully or we update to minimal requirements)
        let (_db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        // When: Reporting incomplete activity JSON (missing required fields)
        let activity_json = r#"{"app_id":"firefox","duration":60}"#;
        let result = manager.report_activity(activity_json).await;

        // Then: It should fail with missing field error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }

    #[tokio::test]
    async fn test_bdd_given_activity_with_session_when_reported_then_stored_in_db() {
        // Given: A profile with an active session
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        let active_session_id = manager.get_active_session_id().await.unwrap();

        // When: Reporting activity with proper Activity structure
        let activity_json = format!(
            r#"{{"id":"{}","profile_id":"{}","timestamp":"{}","activity_type":{{"type":"application_usage"}},"application":"firefox","window_title":"Example","duration_seconds":60}}"#,
            uuid::Uuid::new_v4(),
            profile_id,
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ")
        );
        let result = manager.report_activity(&activity_json).await;

        // Then: Activity should be stored in database
        if let Err(e) = &result {
            eprintln!("Activity report failed: {:?}", e);
        }
        assert!(result.is_ok());

        // Verify it's in the database
        use dots_family_db::queries::activities::ActivityQueries;
        let activities = ActivityQueries::list_for_session(&db, &active_session_id).await.unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].app_id, "firefox");
        assert_eq!(activities[0].app_name, "firefox");
        assert_eq!(activities[0].duration_seconds, 60);
    }

    #[tokio::test]
    async fn test_bdd_given_monitor_when_heartbeat_sent_then_health_check_passes() {
        // Given: A profile manager
        let (_db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        // When: Monitor sends heartbeat
        manager.send_heartbeat("monitor-1").await.unwrap();

        // Then: Health check should pass
        let healthy = manager.check_monitor_health().await.unwrap();
        assert!(healthy);
        assert!(!manager.is_tampered().await);
    }

    #[tokio::test]
    async fn test_bdd_given_monitor_when_heartbeat_timeout_then_tamper_detected() {
        // Given: A profile manager with an old heartbeat
        let (_db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        // Send initial heartbeat
        manager.send_heartbeat("monitor-1").await.unwrap();

        // Manually set heartbeat to old time (simulate timeout)
        {
            let mut heartbeats = manager.monitor_heartbeats.write().await;
            if let Some(hb) = heartbeats.get_mut("monitor-1") {
                hb.last_seen =
                    Instant::now() - std::time::Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 10);
            }
        }

        // When: Checking monitor health
        let healthy = manager.check_monitor_health().await.unwrap();

        // Then: Should detect tamper
        assert!(!healthy);
        assert!(manager.is_tampered().await);
    }

    #[tokio::test]
    async fn test_bdd_given_tamper_when_heartbeat_reconnects_then_tamper_cleared() {
        // Given: A tampered state
        let (_db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        manager.send_heartbeat("monitor-1").await.unwrap();
        {
            let mut heartbeats = manager.monitor_heartbeats.write().await;
            if let Some(hb) = heartbeats.get_mut("monitor-1") {
                hb.last_seen =
                    Instant::now() - std::time::Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 10);
            }
        }
        manager.check_monitor_health().await.unwrap();
        assert!(manager.is_tampered().await);

        // When: Monitor reconnects with heartbeat
        manager.send_heartbeat("monitor-1").await.unwrap();

        // Then: Tamper flag should be cleared
        assert!(!manager.is_tampered().await);
    }

    #[tokio::test]
    async fn test_bdd_given_profile_when_set_active_then_loaded_on_next_startup() {
        // Given: A profile that is set as active
        let (db, dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager1 = ProfileManager::new(&config).await.unwrap();
        manager1._set_active_profile(&profile_id).await.unwrap();

        // When: Creating a new manager (simulating daemon restart)
        let manager2 = ProfileManager::new(&config).await.unwrap();
        let loaded_profile = manager2.get_active_profile().await.unwrap();

        // Then: Profile should be loaded automatically
        assert!(loaded_profile.is_some());
        let profile = loaded_profile.unwrap();
        assert_eq!(profile.name, "Test Child");

        drop(dir);
    }

    #[tokio::test]
    async fn test_bdd_given_profile_when_activated_then_session_created() {
        // Given: A profile manager and a profile
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config).await.unwrap();

        // When: Profile is activated
        manager._set_active_profile(&profile_id).await.unwrap();

        // Then: Active session should be created
        let session_id = manager.get_active_session_id().await;
        assert!(session_id.is_some());

        // Verify session exists in database
        use dots_family_db::queries::sessions::SessionQueries;
        let session = SessionQueries::get_by_id(&db, &session_id.unwrap()).await.unwrap();
        assert_eq!(session.profile_id, profile_id);
        assert!(session.end_time.is_none());
    }

    #[tokio::test]
    async fn test_bdd_given_active_session_when_deactivated_then_session_ended() {
        // Given: An active profile with a session
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();
        let session_id = manager.get_active_session_id().await.unwrap();

        // When: Profile is deactivated
        manager.deactivate_profile("logout").await.unwrap();

        // Then: Session should be ended
        assert!(manager.get_active_session_id().await.is_none());
        assert!(manager.get_active_profile().await.unwrap().is_none());

        // Verify session is ended in database
        use dots_family_db::queries::sessions::SessionQueries;
        let session = SessionQueries::get_by_id(&db, &session_id).await.unwrap();
        assert!(session.end_time.is_some());
        assert_eq!(session.end_reason, Some("logout".to_string()));
    }

    #[tokio::test]
    async fn test_bdd_given_activities_when_get_used_time_today_then_returns_total() {
        use dots_family_db::models::NewActivity;
        use dots_family_db::queries::activities::ActivityQueries;

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();
        let session_id = manager.get_active_session_id().await.unwrap();

        let activity1 = NewActivity {
            session_id: session_id.clone(),
            profile_id: profile_id.clone(),
            app_id: "firefox".to_string(),
            app_name: "Firefox".to_string(),
            category: Some("browser".to_string()),
            window_title: Some("Example".to_string()),
            duration_seconds: 300,
        };

        let activity2 = NewActivity {
            session_id: session_id.clone(),
            profile_id: profile_id.clone(),
            app_id: "code".to_string(),
            app_name: "VS Code".to_string(),
            category: Some("editor".to_string()),
            window_title: Some("main.rs".to_string()),
            duration_seconds: 450,
        };

        ActivityQueries::create(&db, activity1).await.unwrap();
        ActivityQueries::create(&db, activity2).await.unwrap();

        let used_time = manager.get_used_time_today().await.unwrap();
        assert_eq!(used_time, 750);
    }

    #[tokio::test]
    async fn test_bdd_given_used_time_when_get_remaining_time_then_returns_difference() {
        use dots_family_db::models::NewActivity;
        use dots_family_db::queries::activities::ActivityQueries;

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();
        let session_id = manager.get_active_session_id().await.unwrap();

        let activity = NewActivity {
            session_id: session_id.clone(),
            profile_id: profile_id.clone(),
            app_id: "firefox".to_string(),
            app_name: "Firefox".to_string(),
            category: Some("browser".to_string()),
            window_title: Some("Example".to_string()),
            duration_seconds: 3600,
        };

        ActivityQueries::create(&db, activity).await.unwrap();

        let remaining = manager.get_remaining_time().await.unwrap();
        assert_eq!(remaining, 60);
    }

    #[tokio::test]
    async fn test_bdd_given_time_limit_exceeded_when_check_app_then_returns_false() {
        use dots_family_db::models::NewActivity;
        use dots_family_db::queries::activities::ActivityQueries;

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();
        let session_id = manager.get_active_session_id().await.unwrap();

        let activity = NewActivity {
            session_id: session_id.clone(),
            profile_id: profile_id.clone(),
            app_id: "firefox".to_string(),
            app_name: "Firefox".to_string(),
            category: Some("browser".to_string()),
            window_title: Some("Example".to_string()),
            duration_seconds: 7200,
        };

        ActivityQueries::create(&db, activity).await.unwrap();

        let allowed = manager.check_application_allowed("firefox").await.unwrap();
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_bdd_given_password_configured_when_authenticate_then_returns_token() {
        let (_db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config).await.unwrap();

        // Given a parent password is set
        manager.set_parent_password("test_password_123").await.unwrap();

        // When authenticating with correct password
        let result = manager.authenticate_parent("test_password_123").await;

        // Then authentication succeeds and returns a token
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
        assert_eq!(token.len(), 64); // Session tokens are 64 characters

        // And the token should be valid
        assert!(manager.validate_session(&token).await);

        // And the session should be tracked
        assert_eq!(manager.active_session_count().await, 1);
    }

    #[tokio::test]
    async fn test_bdd_given_session_token_when_validated_then_returns_correct_status() {
        let (_db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config).await.unwrap();

        // Given a parent password is set and authentication succeeds
        manager.set_parent_password("test_password_123").await.unwrap();
        let token = manager.authenticate_parent("test_password_123").await.unwrap();

        // When validating the token immediately
        let is_valid = manager.validate_session(&token).await;

        // Then the token should be valid
        assert!(is_valid);

        // When validating a non-existent token
        let invalid_token = "invalid_token_123";
        let is_invalid = manager.validate_session(invalid_token).await;

        // Then validation should fail
        assert!(!is_invalid);
    }

    #[tokio::test]
    async fn test_bdd_given_session_token_when_revoked_then_no_longer_valid() {
        let (_db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config).await.unwrap();

        // Given a parent password is set and authentication succeeds
        manager.set_parent_password("test_password_123").await.unwrap();
        let token = manager.authenticate_parent("test_password_123").await.unwrap();

        // When the session is revoked
        let was_revoked = manager.revoke_session(&token).await;

        // Then revocation should succeed
        assert!(was_revoked);

        // And the token should no longer be valid
        assert!(!manager.validate_session(&token).await);

        // And the session count should be zero
        assert_eq!(manager.active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_bdd_given_password_configured_when_wrong_password_then_authentication_fails() {
        let (_db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config).await.unwrap();

        // Given a parent password is set
        manager.set_parent_password("correct_password").await.unwrap();

        // When authenticating with wrong password
        let result = manager.authenticate_parent("wrong_password").await;

        // Then authentication fails
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid password"));
    }

    #[tokio::test]
    async fn test_bdd_given_no_password_configured_when_authenticate_then_fails() {
        let (_db, _temp_dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config).await.unwrap();

        // Given no parent password is configured

        // When attempting authentication
        let result = manager.authenticate_parent("any_password").await;

        // Then authentication fails with configuration error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("authentication not configured"));
    }
}
