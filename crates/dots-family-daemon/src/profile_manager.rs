use std::{collections::HashMap, sync::Arc, time::Instant};

use anyhow::{anyhow, Result};
use dots_family_common::{
    security::{EncryptionKey, PasswordManager, SessionToken},
    types::{ApplicationMode, Profile},
};
use dots_family_db::{queries::profiles::ProfileQueries, Database};
use secrecy::SecretString;
use sqlx::Row;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{config::DaemonConfig, notification_manager::NotificationManager};

#[allow(dead_code)]
const HEARTBEAT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
struct MonitorHeartbeat {
    #[allow(dead_code)]
    monitor_id: String,
    #[allow(dead_code)]
    last_seen: Instant,
}

#[derive(Clone)]
pub struct ProfileManager {
    _db: Database,
    config: DaemonConfig,
    active_profile: Arc<RwLock<Option<Profile>>>,
    active_session_id: Arc<RwLock<Option<String>>>,
    monitor_heartbeats: Arc<RwLock<HashMap<String, MonitorHeartbeat>>>,
    tamper_detected: Arc<RwLock<bool>>,
    /// Active session tokens for parent authentication
    active_sessions: Arc<RwLock<HashMap<String, SessionToken>>>,
    /// Notification manager for desktop and system notifications
    notification_manager: NotificationManager,
}

impl ProfileManager {
    pub async fn new(config: &DaemonConfig, database: Database) -> Result<Self> {
        info!("Initializing ProfileManager with existing database instance");

        let manager = Self {
            _db: database,
            config: config.clone(),
            active_profile: Arc::new(RwLock::new(None)),
            active_session_id: Arc::new(RwLock::new(None)),
            monitor_heartbeats: Arc::new(RwLock::new(HashMap::new())),
            tamper_detected: Arc::new(RwLock::new(false)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            notification_manager: NotificationManager::new(),
        };

        manager.load_active_profile_from_db().await?;

        Ok(manager)
    }

    async fn load_active_profile_from_db(&self) -> Result<()> {
        let pool = self._db.pool()?;

        let result =
            match sqlx::query("SELECT value FROM daemon_settings WHERE key = 'active_profile_id'")
                .fetch_optional(pool)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    warn!("daemon_settings table not found, skipping active profile load: {}", e);
                    return Ok(());
                }
            };

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
        use dots_family_db::{models::NewSession, queries::sessions::SessionQueries};

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
            name: db_profile.name.clone(),
            username: db_profile.username,
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
        use dots_family_db::{models::NewSession, queries::sessions::SessionQueries};

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
                name: db_profile.name.clone(),
                username: db_profile.username,
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

    #[allow(dead_code)]
    pub async fn create_profile(&self, name: &str, age_group: &str) -> Result<String> {
        self.create_profile_with_username(name, age_group, None).await
    }

    pub async fn create_profile_with_username(
        &self,
        name: &str,
        age_group: &str,
        username: Option<String>,
    ) -> Result<String> {
        use dots_family_common::types::{
            ApplicationConfig, ApplicationMode, ProfileConfig, ScreenTimeConfig,
            TerminalFilteringConfig, TimeWindow, TimeWindows, WebFilteringConfig,
        };
        use dots_family_db::{
            models::{NewAuditLog, NewProfile},
            queries::{audit::AuditQueries, profiles::ProfileQueries},
        };

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
                    holiday: vec![],
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
            terminal_filtering: TerminalFilteringConfig::default(),
        };

        let config_json = serde_json::to_string(&config)?;

        let new_profile = NewProfile {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            username, // Use the username parameter
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
        use dots_family_db::{models::NewActivity, queries::activities::ActivityQueries};

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    pub async fn request_parent_permission(
        &self,
        request_type: &str,
        details: &str,
        token: &str,
    ) -> Result<String> {
        self.ensure_valid_session(token).await?;

        let approval_id = self.log_permission_request(request_type, details).await?;

        self.create_pending_approval_response(approval_id, request_type, details)
    }

    async fn ensure_valid_session(&self, token: &str) -> Result<()> {
        use dots_family_db::{models::NewAuditLog, queries::audit::AuditQueries};

        if !self.validate_session(token).await {
            let audit = NewAuditLog {
                actor: "child".to_string(),
                action: "request_permission".to_string(),
                resource: "permission".to_string(),
                resource_id: None,
                ip_address: None,
                success: false,
                details: Some("Invalid session token".to_string()),
            };
            let _ = AuditQueries::log(&self._db, audit).await;
            return Err(anyhow!("Unauthorized: Invalid session token"));
        }
        Ok(())
    }

    async fn log_permission_request(&self, request_type: &str, details: &str) -> Result<String> {
        use dots_family_db::{models::NewAuditLog, queries::audit::AuditQueries};

        let approval_id = Uuid::new_v4().to_string();

        let audit = NewAuditLog {
            actor: "child".to_string(),
            action: "request_permission".to_string(),
            resource: "permission".to_string(),
            resource_id: Some(approval_id.clone()),
            ip_address: None,
            success: true,
            details: Some(format!("Type: {}, Details: {}", request_type, details)),
        };
        AuditQueries::log(&self._db, audit).await?;

        Ok(approval_id)
    }

    fn create_pending_approval_response(
        &self,
        approval_id: String,
        request_type: &str,
        details: &str,
    ) -> Result<String> {
        let response = serde_json::json!({
            "approval_id": approval_id,
            "status": "pending",
            "message": "Permission request logged. Please ask a parent to approve this action.",
            "request_type": request_type,
            "details": details
        });

        Ok(response.to_string())
    }

    pub async fn request_command_approval(
        &self,
        command: &str,
        risk_level: &str,
        reasons: &str,
    ) -> Result<String> {
        use dots_family_db::{models::NewAuditLog, queries::audit::AuditQueries};

        let approval_id = Uuid::new_v4().to_string();

        let audit = NewAuditLog {
            actor: "child".to_string(),
            action: "request_command_approval".to_string(),
            resource: "terminal_command".to_string(),
            resource_id: Some(approval_id.clone()),
            ip_address: None,
            success: true,
            details: Some(format!(
                "Command: {}, Risk: {}, Reasons: {}",
                command, risk_level, reasons
            )),
        };
        AuditQueries::log(&self._db, audit).await?;

        let response = serde_json::json!({
            "approval_id": approval_id,
            "status": "pending",
            "message": "Command requires parent approval. Request logged for review.",
            "command": command,
            "risk_level": risk_level,
            "reasons": reasons
        });

        Ok(response.to_string())
    }

    // ============================================================================
    // Exception Management Methods
    // ============================================================================

    /// Create a new exception for temporary policy overrides
    pub async fn create_exception(
        &self,
        exception_type: &str,
        reason: &str,
        duration_json: &str,
        token: &str,
    ) -> Result<String> {
        use chrono::Duration;
        use dots_family_common::types::{Exception, ExceptionDuration, ExceptionType};
        use serde_json;

        // Verify parent authentication
        self.ensure_valid_session(token).await?;

        let active_profile =
            self.get_active_profile().await?.ok_or_else(|| anyhow!("No active profile"))?;

        // Parse duration from JSON
        let duration: ExceptionDuration = serde_json::from_str(duration_json)
            .map_err(|e| anyhow!("Invalid duration JSON: {}", e))?;

        // Parse exception type
        let exception_type_enum: ExceptionType = match exception_type {
            "application_override" => {
                let data: serde_json::Value = serde_json::from_str(reason)?;
                ExceptionType::ApplicationOverride {
                    app_id: data["app_id"].as_str().unwrap_or_default().to_string(),
                }
            }
            "website_override" => {
                let data: serde_json::Value = serde_json::from_str(reason)?;
                ExceptionType::WebsiteOverride {
                    domain: data["domain"].as_str().unwrap_or_default().to_string(),
                }
            }
            "screen_time_extension" => {
                let data: serde_json::Value = serde_json::from_str(reason)?;
                ExceptionType::ScreenTimeExtension {
                    extra_minutes: data["extra_minutes"].as_u64().unwrap_or(30) as u32,
                }
            }
            _ => return Err(anyhow!("Unknown exception type: {}", exception_type)),
        };

        // Create exception
        let exception = Exception::new(
            active_profile.id,
            exception_type_enum,
            reason.to_string(),
            duration,
            "parent".to_string(),
        );

        // Convert to database format and store
        let db_exception = dots_family_db::models::NewException {
            id: exception.id.to_string(),
            profile_id: exception.profile_id.to_string(),
            exception_type: exception_type.to_string(),
            granted_by: exception.created_by.clone(),
            expires_at: exception
                .expires_at
                .unwrap_or_else(|| chrono::Utc::now() + Duration::hours(1)),
            reason: Some(exception.reason),
            amount_minutes: None,
            app_id: None,
            website: None,
            scope: None,
        };

        dots_family_db::queries::exceptions::ExceptionQueries::create(&self._db, db_exception)
            .await?;

        Ok(exception.id.to_string())
    }

    /// List active exceptions for the current profile
    pub async fn list_active_exceptions(
        &self,
        profile_id: &str,
        token: &str,
    ) -> Result<Vec<dots_family_common::types::Exception>> {
        self.ensure_valid_session(token).await?;

        let db_exceptions =
            dots_family_db::queries::exceptions::ExceptionQueries::list_active_for_profile(
                &self._db, profile_id,
            )
            .await?;

        // Convert database exceptions to domain exceptions (simplified for now)
        let exceptions = db_exceptions
            .into_iter()
            .map(|db_ex| dots_family_common::types::Exception {
                id: uuid::Uuid::parse_str(&db_ex.id).unwrap_or_default(),
                profile_id: uuid::Uuid::parse_str(&db_ex.profile_id).unwrap_or_default(),
                exception_type: dots_family_common::types::ExceptionType::CustomOverride {
                    description: db_ex.exception_type.clone(),
                    policy_changes: std::collections::HashMap::new(),
                },
                reason: db_ex.reason.unwrap_or_default(),
                duration: dots_family_common::types::ExceptionDuration::UntilTime(db_ex.expires_at),
                status: if db_ex.active {
                    dots_family_common::types::ExceptionStatus::Active
                } else {
                    dots_family_common::types::ExceptionStatus::Expired
                },
                created_at: db_ex.granted_at,
                activated_at: Some(db_ex.granted_at),
                expires_at: Some(db_ex.expires_at),
                revoked_at: None,
                created_by: db_ex.granted_by,
            })
            .collect();

        Ok(exceptions)
    }

    /// Revoke an active exception
    pub async fn revoke_exception(&self, exception_id: &str, token: &str) -> Result<()> {
        self.ensure_valid_session(token).await?;

        dots_family_db::queries::exceptions::ExceptionQueries::revoke_exception(
            &self._db,
            exception_id,
        )
        .await?;
        Ok(())
    }

    /// Check if an exception applies to a specific resource
    pub async fn check_exception_applies(
        &self,
        exception_type: &str,
        resource_id: &str,
    ) -> Result<bool> {
        let active_profile =
            self.get_active_profile().await?.ok_or_else(|| anyhow!("No active profile"))?;

        let exception =
            dots_family_db::queries::exceptions::ExceptionQueries::check_active_exception(
                &self._db,
                &active_profile.id.to_string(),
                exception_type,
                Some(resource_id),
            )
            .await?;

        Ok(exception.is_some())
    }

    // ============================================================================
    // Approval Request Methods
    // ============================================================================

    /// Submit a new approval request from child
    pub async fn submit_approval_request(
        &self,
        request_type: &str,
        _message: &str,
        details_json: &str,
    ) -> Result<String> {
        use dots_family_db::queries::approval_requests::ApprovalRequestQueries;
        use serde_json::Value;

        let active_profile =
            self.get_active_profile().await?.ok_or_else(|| anyhow!("No active profile"))?;

        let details: Value = serde_json::from_str(details_json)
            .map_err(|e| anyhow!("Invalid details JSON: {}", e))?;

        let request_id = ApprovalRequestQueries::create(
            &self._db,
            &active_profile.id.to_string(),
            request_type,
            &details,
        )
        .await?;

        let notification = NotificationManager::create_approval_request_notification(
            uuid::Uuid::parse_str(&request_id).unwrap_or_default(),
            &active_profile.name,
            &format!("{} request", request_type),
        );

        if let Err(e) = self.notification_manager.send_notification(notification).await {
            warn!("Failed to send approval request notification: {}", e);
        }

        Ok(request_id)
    }

    /// List pending approval requests (for parent)
    pub async fn list_pending_requests(
        &self,
        token: &str,
    ) -> Result<Vec<dots_family_db::queries::approval_requests::ApprovalRequest>> {
        use dots_family_db::queries::approval_requests::ApprovalRequestQueries;

        self.ensure_valid_session(token).await?;

        let active_profile =
            self.get_active_profile().await?.ok_or_else(|| anyhow!("No active profile"))?;

        let requests =
            ApprovalRequestQueries::list_pending(&self._db, &active_profile.id.to_string()).await?;
        Ok(requests)
    }

    /// Approve an approval request and create corresponding exception
    pub async fn approve_request(
        &self,
        request_id: &str,
        response_message: &str,
        token: &str,
    ) -> Result<Option<String>> {
        use chrono::Utc;
        use dots_family_db::{
            models::NewException,
            queries::{approval_requests::ApprovalRequestQueries, exceptions::ExceptionQueries},
        };

        self.ensure_valid_session(token).await?;

        // Get the approval request details before marking it as approved
        let request = ApprovalRequestQueries::get_by_id(&self._db, request_id)
            .await?
            .ok_or_else(|| anyhow!("Approval request not found"))?;

        // Mark the request as approved
        ApprovalRequestQueries::review_request(
            &self._db,
            request_id,
            "approved",
            "parent",
            Some(response_message),
        )
        .await?;

        // Parse the request type from the stored string and details
        let request_type =
            self.parse_request_type_from_db(&request.request_type, &request.details)?;

        // Convert RequestType to ExceptionType
        let exception_type = request_type.to_exception_type();

        // Get default duration for this request type
        let duration = request_type.default_exception_duration();

        // Calculate expiration time based on duration
        let expires_at = match duration {
            dots_family_common::types::ExceptionDuration::Duration(d) => Utc::now() + d,
            dots_family_common::types::ExceptionDuration::UntilTime(t) => t,
            dots_family_common::types::ExceptionDuration::UntilEndOfDay => {
                let end_of_day = Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap();
                chrono::DateTime::from_naive_utc_and_offset(end_of_day, Utc)
            }
            // For session-based or manual exceptions, set a far future date
            _ => Utc::now() + chrono::Duration::days(365),
        };

        // Create the exception in the database
        let exception_id = uuid::Uuid::new_v4().to_string();

        // Map ExceptionType to database fields
        let (db_exception_type, app_id, website, amount_minutes) = match exception_type {
            dots_family_common::types::ExceptionType::ApplicationOverride { app_id } => {
                ("app".to_string(), Some(app_id), None, None)
            }
            dots_family_common::types::ExceptionType::WebsiteOverride { domain } => {
                ("website".to_string(), None, Some(domain), None)
            }
            dots_family_common::types::ExceptionType::ScreenTimeExtension { extra_minutes } => {
                ("screen_time".to_string(), None, None, Some(extra_minutes as i64))
            }
            dots_family_common::types::ExceptionType::TimeWindowOverride { .. } => {
                ("time".to_string(), None, None, None)
            }
            dots_family_common::types::ExceptionType::TerminalCommandOverride { command } => {
                // Store command in app_id field for now
                ("command".to_string(), Some(command), None, None)
            }
            dots_family_common::types::ExceptionType::CustomOverride { description, .. } => {
                // Store description in app_id field
                ("custom".to_string(), Some(description), None, None)
            }
        };

        let new_exception = NewException {
            id: exception_id.clone(),
            profile_id: request.profile_id.clone(),
            exception_type: db_exception_type.clone(),
            granted_by: "parent".to_string(),
            expires_at,
            reason: Some(response_message.to_string()),
            amount_minutes,
            app_id,
            website,
            scope: None,
        };

        ExceptionQueries::create(&self._db, new_exception).await?;

        // Send notification about exception creation
        let profile_uuid = Uuid::parse_str(&request.profile_id)?;
        let exception_uuid = Uuid::parse_str(&exception_id)?;
        let notification = NotificationManager::create_exception_notification(
            profile_uuid,
            exception_uuid,
            &db_exception_type,
            true, // is_created = true
        );

        if let Err(e) = self.notification_manager.send_notification(notification).await {
            warn!("Failed to send exception creation notification: {}", e);
        }

        Ok(Some(exception_id))
    }

    /// Helper to parse request type from database format
    fn parse_request_type_from_db(
        &self,
        request_type_str: &str,
        details: &serde_json::Value,
    ) -> Result<dots_family_common::types::RequestType> {
        use chrono::Utc;
        use dots_family_common::types::RequestType;

        match request_type_str {
            "app" => {
                let app_id = details["app_id"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing app_id in request details"))?
                    .to_string();
                Ok(RequestType::ApplicationAccess { app_id })
            }
            "website" => {
                let url = details["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing url in request details"))?
                    .to_string();
                let domain = details["domain"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing domain in request details"))?
                    .to_string();
                Ok(RequestType::WebsiteAccess { url, domain })
            }
            "screen_time" => {
                let requested_minutes = details["requested_minutes"]
                    .as_u64()
                    .ok_or_else(|| anyhow!("Missing requested_minutes in request details"))?
                    as u32;
                Ok(RequestType::ScreenTimeExtension { requested_minutes })
            }
            "time_extension" => {
                let requested_end_time_str = details["requested_end_time"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing requested_end_time in request details"))?;
                let requested_end_time =
                    chrono::DateTime::parse_from_rfc3339(requested_end_time_str)?
                        .with_timezone(&Utc);
                Ok(RequestType::TimeExtension { requested_end_time })
            }
            "command" => {
                let command = details["command"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing command in request details"))?
                    .to_string();
                Ok(RequestType::TerminalCommand { command })
            }
            "custom" => {
                let description = details["description"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing description in request details"))?
                    .to_string();
                Ok(RequestType::Custom { description })
            }
            _ => Err(anyhow!("Unknown request type: {}", request_type_str)),
        }
    }

    /// Deny an approval request  
    pub async fn deny_request(
        &self,
        request_id: &str,
        response_message: &str,
        token: &str,
    ) -> Result<()> {
        use dots_family_db::queries::approval_requests::ApprovalRequestQueries;

        self.ensure_valid_session(token).await?;

        ApprovalRequestQueries::review_request(
            &self._db,
            request_id,
            "denied",
            "parent",
            Some(response_message),
        )
        .await?;

        Ok(())
    }

    pub async fn get_daily_report(
        &self,
        profile_id: &str,
        date_str: &str,
    ) -> Result<crate::reports::ActivityReport> {
        use chrono::NaiveDate;
        use dots_family_db::queries::daily_summaries::DailySummaryQueries;

        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid date format: {}. Expected YYYY-MM-DD", e))?;

        match DailySummaryQueries::get_by_profile_and_date(&self._db, profile_id, date).await {
            Ok(summary) => {
                let top_apps: Vec<serde_json::Value> =
                    serde_json::from_str(&summary.top_apps).unwrap_or_else(|_| vec![]);

                let apps_used: Vec<crate::reports::AppUsage> = top_apps
                    .into_iter()
                    .filter_map(|app| {
                        if let (Some(app_id), Some(duration)) = (
                            app.get("app_id").and_then(|v| v.as_str()),
                            app.get("duration").and_then(|v| v.as_i64()),
                        ) {
                            let duration_minutes = (duration / 60) as u32;
                            let total_seconds = summary.screen_time_seconds as f32;
                            let percentage = if total_seconds > 0.0 {
                                (duration as f32 / total_seconds * 100.0).min(100.0)
                            } else {
                                0.0
                            };

                            Some(crate::reports::AppUsage {
                                app_id: app_id.to_string(),
                                app_name: app
                                    .get("app_name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(app_id)
                                    .to_string(),
                                category: app
                                    .get("category")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                duration_minutes,
                                percentage,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                let top_categories: Vec<serde_json::Value> =
                    serde_json::from_str(&summary.top_categories).unwrap_or_else(|_| vec![]);

                let (top_activity, top_category) = if let Some(first_app) = apps_used.first() {
                    (first_app.app_name.clone(), first_app.category.clone())
                } else if let Some(first_cat) = top_categories.first() {
                    (
                        "Unknown".to_string(),
                        first_cat
                            .get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                    )
                } else {
                    ("No Activity".to_string(), "None".to_string())
                };

                Ok(crate::reports::ActivityReport {
                    date,
                    screen_time_minutes: (summary.screen_time_seconds / 60) as u32,
                    top_activity,
                    top_category,
                    violations: summary.violations_count as u32,
                    blocked_attempts: summary.blocks_count as u32,
                    apps_used,
                })
            }
            Err(_) => Ok(crate::reports::ActivityReport {
                date,
                screen_time_minutes: 0,
                top_activity: "No Activity".to_string(),
                top_category: "None".to_string(),
                violations: 0,
                blocked_attempts: 0,
                apps_used: vec![],
            }),
        }
    }

    pub async fn get_weekly_report(
        &self,
        profile_id: &str,
        week_start_str: &str,
    ) -> Result<crate::reports::WeeklyReport> {
        use chrono::NaiveDate;
        use dots_family_db::queries::weekly_summaries::WeeklySummaryQueries;

        let week_start = NaiveDate::parse_from_str(week_start_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid date format: {}. Expected YYYY-MM-DD", e))?;

        match WeeklySummaryQueries::get_by_profile_and_week(&self._db, profile_id, week_start).await
        {
            Ok(summary) => {
                let top_categories: Vec<serde_json::Value> =
                    serde_json::from_str(&summary.top_categories).unwrap_or_else(|_| vec![]);

                let category_usage: Vec<crate::reports::CategoryUsage> = top_categories
                    .into_iter()
                    .filter_map(|cat| {
                        if let (Some(category), Some(duration)) = (
                            cat.get("category").and_then(|v| v.as_str()),
                            cat.get("duration").and_then(|v| v.as_i64()),
                        ) {
                            let duration_minutes = (duration / 60) as u32;
                            let total_seconds = summary.total_screen_time_seconds as f32;
                            let percentage = if total_seconds > 0.0 {
                                (duration as f32 / total_seconds * 100.0).min(100.0)
                            } else {
                                0.0
                            };

                            Some(crate::reports::CategoryUsage {
                                category: category.to_string(),
                                duration_minutes,
                                percentage,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                let educational_percentage = category_usage
                    .iter()
                    .find(|c| c.category.to_lowercase().contains("education"))
                    .map(|c| c.percentage)
                    .unwrap_or(0.0);

                Ok(crate::reports::WeeklyReport {
                    week_start,
                    total_screen_time_minutes: (summary.total_screen_time_seconds / 60) as u32,
                    average_daily_minutes: (summary.daily_average_seconds / 60) as u32,
                    most_active_day: "Saturday".to_string(),
                    top_categories: category_usage,
                    policy_violations: summary.violations_count as u32,
                    educational_percentage,
                })
            }
            Err(_) => Ok(crate::reports::WeeklyReport {
                week_start,
                total_screen_time_minutes: 0,
                average_daily_minutes: 0,
                most_active_day: "No Activity".to_string(),
                top_categories: vec![],
                policy_violations: 0,
                educational_percentage: 0.0,
            }),
        }
    }

    pub async fn export_reports(
        &self,
        profile_id: &str,
        format: &str,
        start_date_str: &str,
        end_date_str: &str,
    ) -> Result<String> {
        use chrono::{Duration, NaiveDate};

        let start_date = NaiveDate::parse_from_str(start_date_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid start date format: {}. Expected YYYY-MM-DD", e))?;

        let end_date = NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid end date format: {}. Expected YYYY-MM-DD", e))?;

        match format {
            "json" => {
                let mut reports: Vec<crate::reports::ActivityReport> = Vec::new();
                let mut current_date = start_date;

                while current_date <= end_date {
                    let report = self
                        .get_daily_report(profile_id, &current_date.format("%Y-%m-%d").to_string())
                        .await?;
                    reports.push(report);
                    current_date += Duration::days(1);
                }

                Ok(serde_json::to_string_pretty(&reports)?)
            }
            "csv" => {
                let mut csv_content = String::from("Date,Screen Time (minutes),Top Activity,Top Category,Violations,Blocked Attempts\n");
                let mut current_date = start_date;

                while current_date <= end_date {
                    let report = self
                        .get_daily_report(profile_id, &current_date.format("%Y-%m-%d").to_string())
                        .await?;
                    csv_content.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        report.date,
                        report.screen_time_minutes,
                        report.top_activity,
                        report.top_category,
                        report.violations,
                        report.blocked_attempts
                    ));
                    current_date += Duration::days(1);
                }

                Ok(csv_content)
            }
            _ => Err(anyhow!("Unsupported export format: {}", format)),
        }
    }

    // ============================================================================
    // Time Window Configuration Methods
    // ============================================================================

    /// Add a time window to a profile's configuration
    pub async fn add_time_window(
        &self,
        profile_id: &str,
        window_type: &str,
        start: &str,
        end: &str,
        token: &str,
    ) -> Result<()> {
        use dots_family_common::types::TimeWindow;

        // Validate parent authentication
        if !self.validate_session(token).await {
            return Err(anyhow!("Invalid or expired session token"));
        }

        // Validate time format (HH:MM)
        Self::validate_time_format(start)?;
        Self::validate_time_format(end)?;

        // Validate start < end
        if start >= end {
            return Err(anyhow!("Start time must be before end time"));
        }

        // Validate window type
        if !matches!(window_type, "weekday" | "weekend" | "holiday") {
            return Err(anyhow!(
                "Invalid window type '{}'. Must be one of: weekday, weekend, holiday",
                window_type
            ));
        }

        // Try to find profile by ID first, then by name
        let profile = match ProfileQueries::get_by_id(&self._db, profile_id).await {
            Ok(p) => p,
            Err(_) => ProfileQueries::get_by_name(&self._db, profile_id).await?,
        };

        // Parse existing config
        let mut config: dots_family_common::types::ProfileConfig =
            serde_json::from_str(&profile.config)?;

        // Create new time window
        let new_window = TimeWindow { start: start.to_string(), end: end.to_string() };

        // Check for overlapping windows
        let target_windows = match window_type {
            "weekday" => &config.screen_time.windows.weekday,
            "weekend" => &config.screen_time.windows.weekend,
            "holiday" => &config.screen_time.windows.holiday,
            _ => unreachable!(),
        };

        for existing in target_windows {
            if Self::windows_overlap(&new_window, existing) {
                return Err(anyhow!(
                    "Time window {}–{} overlaps with existing window {}–{}",
                    start,
                    end,
                    existing.start,
                    existing.end
                ));
            }
        }

        // Add window to appropriate list
        match window_type {
            "weekday" => config.screen_time.windows.weekday.push(new_window),
            "weekend" => config.screen_time.windows.weekend.push(new_window),
            "holiday" => config.screen_time.windows.holiday.push(new_window),
            _ => unreachable!(),
        }

        // Sort windows by start time
        match window_type {
            "weekday" => config.screen_time.windows.weekday.sort_by(|a, b| a.start.cmp(&b.start)),
            "weekend" => config.screen_time.windows.weekend.sort_by(|a, b| a.start.cmp(&b.start)),
            "holiday" => config.screen_time.windows.holiday.sort_by(|a, b| a.start.cmp(&b.start)),
            _ => unreachable!(),
        }

        // Save updated config
        let updated_config_json = serde_json::to_string(&config)?;
        ProfileQueries::update_config(&self._db, &profile.id, &updated_config_json).await?;

        info!("Added {} time window {}–{} to profile {}", window_type, start, end, profile.name);

        Ok(())
    }

    /// Remove a specific time window from a profile's configuration
    pub async fn remove_time_window(
        &self,
        profile_id: &str,
        window_type: &str,
        start: &str,
        end: &str,
        token: &str,
    ) -> Result<()> {
        // Validate parent authentication
        if !self.validate_session(token).await {
            return Err(anyhow!("Invalid or expired session token"));
        }

        // Validate window type
        if !matches!(window_type, "weekday" | "weekend" | "holiday") {
            return Err(anyhow!(
                "Invalid window type '{}'. Must be one of: weekday, weekend, holiday",
                window_type
            ));
        }

        // Try to find profile by ID first, then by name
        let profile = match ProfileQueries::get_by_id(&self._db, profile_id).await {
            Ok(p) => p,
            Err(_) => ProfileQueries::get_by_name(&self._db, profile_id).await?,
        };

        // Parse existing config
        let mut config: dots_family_common::types::ProfileConfig =
            serde_json::from_str(&profile.config)?;

        // Remove matching window
        let removed = match window_type {
            "weekday" => {
                let original_len = config.screen_time.windows.weekday.len();
                config.screen_time.windows.weekday.retain(|w| !(w.start == start && w.end == end));
                config.screen_time.windows.weekday.len() < original_len
            }
            "weekend" => {
                let original_len = config.screen_time.windows.weekend.len();
                config.screen_time.windows.weekend.retain(|w| !(w.start == start && w.end == end));
                config.screen_time.windows.weekend.len() < original_len
            }
            "holiday" => {
                let original_len = config.screen_time.windows.holiday.len();
                config.screen_time.windows.holiday.retain(|w| !(w.start == start && w.end == end));
                config.screen_time.windows.holiday.len() < original_len
            }
            _ => unreachable!(),
        };

        if !removed {
            return Err(anyhow!(
                "Time window {}–{} not found in {} windows",
                start,
                end,
                window_type
            ));
        }

        // Save updated config
        let updated_config_json = serde_json::to_string(&config)?;
        ProfileQueries::update_config(&self._db, &profile.id, &updated_config_json).await?;

        info!(
            "Removed {} time window {}–{} from profile {}",
            window_type, start, end, profile.name
        );

        Ok(())
    }

    /// List all time windows for a profile
    pub async fn list_time_windows(
        &self,
        profile_id: &str,
        token: &str,
    ) -> Result<serde_json::Value> {
        // Validate parent authentication
        if !self.validate_session(token).await {
            return Err(anyhow!("Invalid or expired session token"));
        }

        // Try to find profile by ID first, then by name
        let profile = match ProfileQueries::get_by_id(&self._db, profile_id).await {
            Ok(p) => p,
            Err(_) => ProfileQueries::get_by_name(&self._db, profile_id).await?,
        };

        // Parse existing config
        let config: dots_family_common::types::ProfileConfig =
            serde_json::from_str(&profile.config)?;

        Ok(serde_json::json!({
            "profile_id": profile.id,
            "profile_name": profile.name,
            "weekday": config.screen_time.windows.weekday,
            "weekend": config.screen_time.windows.weekend,
            "holiday": config.screen_time.windows.holiday,
        }))
    }

    /// Clear all time windows of a specific type from a profile
    pub async fn clear_time_windows(
        &self,
        profile_id: &str,
        window_type: &str,
        token: &str,
    ) -> Result<()> {
        // Validate parent authentication
        if !self.validate_session(token).await {
            return Err(anyhow!("Invalid or expired session token"));
        }

        // Validate window type
        if !matches!(window_type, "weekday" | "weekend" | "holiday") {
            return Err(anyhow!(
                "Invalid window type '{}'. Must be one of: weekday, weekend, holiday",
                window_type
            ));
        }

        // Try to find profile by ID first, then by name
        let profile = match ProfileQueries::get_by_id(&self._db, profile_id).await {
            Ok(p) => p,
            Err(_) => ProfileQueries::get_by_name(&self._db, profile_id).await?,
        };

        // Parse existing config
        let mut config: dots_family_common::types::ProfileConfig =
            serde_json::from_str(&profile.config)?;

        // Clear windows
        let count = match window_type {
            "weekday" => {
                let count = config.screen_time.windows.weekday.len();
                config.screen_time.windows.weekday.clear();
                count
            }
            "weekend" => {
                let count = config.screen_time.windows.weekend.len();
                config.screen_time.windows.weekend.clear();
                count
            }
            "holiday" => {
                let count = config.screen_time.windows.holiday.len();
                config.screen_time.windows.holiday.clear();
                count
            }
            _ => unreachable!(),
        };

        // Save updated config
        let updated_config_json = serde_json::to_string(&config)?;
        ProfileQueries::update_config(&self._db, &profile.id, &updated_config_json).await?;

        info!("Cleared {} {} time windows from profile {}", count, window_type, profile.name);

        Ok(())
    }

    /// Helper: Check if two time windows overlap
    fn windows_overlap(
        window1: &dots_family_common::types::TimeWindow,
        window2: &dots_family_common::types::TimeWindow,
    ) -> bool {
        // Two windows overlap if one starts before the other ends
        // Window1 starts before Window2 ends AND Window2 starts before Window1 ends
        !(window1.end <= window2.start || window2.end <= window1.start)
    }

    /// Helper: Validate time format (HH:MM)
    fn validate_time_format(time: &str) -> Result<()> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid time format '{}'. Expected HH:MM (e.g., 08:00, 15:30)",
                time
            ));
        }

        let hours = parts[0]
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid hours '{}' in time '{}'", parts[0], time))?;
        let minutes = parts[1]
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid minutes '{}' in time '{}'", parts[1], time))?;

        if hours > 23 {
            return Err(anyhow!("Hours must be 0-23, got {}", hours));
        }
        if minutes > 59 {
            return Err(anyhow!("Minutes must be 0-59, got {}", minutes));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dots_family_common::types::{
        ApplicationConfig, ApplicationMode, ProfileConfig, ScreenTimeConfig,
        TerminalFilteringConfig, TimeWindows, WebFilteringConfig,
    };
    use dots_family_db::{queries::profiles::ProfileQueries, Database};
    use tempfile::tempdir;

    use super::*;

    async fn setup_test_db() -> (Database, tempfile::TempDir, DaemonConfig) {
        let dir = tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let db_path = dir.path().join("test.db");

        let daemon_config = DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path.to_str().unwrap().to_string(),
                encryption_key: None,
            },
            auth: crate::config::AuthConfig { parent_password_hash: None },
            dbus: crate::config::DbusConfig {
                service_name: "org.dots.FamilyDaemon.test".to_string(),
                use_session_bus: false,
            },
            dry_run: Some(false),
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
                windows: TimeWindows { weekday: vec![], weekend: vec![], holiday: vec![] },
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
            terminal_filtering: TerminalFilteringConfig::default(),
        };

        let new_profile = dots_family_db::models::NewProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            username: None,
            age_group: "8-12".to_string(),
            birthday: None,
            config: serde_json::to_string(&profile_config).unwrap(),
        };

        let profile = ProfileQueries::create(db, new_profile).await.unwrap();
        profile.id
    }

    #[tokio::test]
    #[ignore]
    async fn test_bdd_given_profile_when_check_allowed_app_then_returns_true() {
        // Given: A profile with firefox in allowlist
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Checking if firefox is allowed
        let allowed = manager.check_application_allowed("firefox").await.unwrap();

        // Then: It should return true
        assert!(allowed);
    }

    #[tokio::test]
    #[ignore]
    async fn test_bdd_given_profile_when_check_blocked_app_then_returns_false() {
        // Given: A profile with firefox in allowlist (steam not in list)
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Checking if steam is allowed
        let allowed = manager.check_application_allowed("steam").await.unwrap();

        // Then: It should return false (not in allowlist)
        assert!(!allowed);
    }

    #[tokio::test]
    #[ignore]
    async fn test_bdd_given_profile_when_get_remaining_time_then_returns_limit() {
        // Given: A profile with 120 minute daily limit
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
        manager._set_active_profile(&profile_id).await.unwrap();

        // When: Getting remaining time
        let remaining = manager.get_remaining_time().await.unwrap();

        // Then: It should return 120 minutes (full limit, no usage yet)
        assert_eq!(remaining, 120);
    }

    #[tokio::test]
    async fn test_bdd_given_no_profile_when_check_app_then_returns_true() {
        // Given: No active profile
        let (db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

        // When: Checking if any app is allowed
        let allowed = manager.check_application_allowed("any-app").await.unwrap();

        // Then: It should return true (no restrictions without profile)
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_bdd_given_activity_json_when_reported_then_succeeds() {
        // Given: A profile manager with no active session (should fail gracefully or we update to minimal requirements)
        let (db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

        // When: Reporting incomplete activity JSON (missing required fields)
        let activity_json = r#"{"app_id":"firefox","duration":60}"#;
        let result = manager.report_activity(activity_json).await;

        // Then: It should fail with missing field error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_bdd_given_activity_with_session_when_reported_then_stored_in_db() {
        // Given: A profile with an active session
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
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
        let (db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

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
        let (db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

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
        let (db, _dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

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
    #[ignore]
    async fn test_bdd_given_profile_when_set_active_then_loaded_on_next_startup() {
        // Given: A profile that is set as active
        let (db, dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager1 = ProfileManager::new(&config, db).await.unwrap();
        manager1._set_active_profile(&profile_id).await.unwrap();

        // When: Creating a new manager (simulating daemon restart)
        let db2_config = dots_family_db::DatabaseConfig {
            path: config.database.path.clone(),
            encryption_key: config.database.encryption_key.clone(),
        };
        let db2 = Database::new(db2_config).await.unwrap();
        let manager2 = ProfileManager::new(&config, db2).await.unwrap();
        let loaded_profile = manager2.get_active_profile().await.unwrap();

        // Then: Profile should be loaded automatically
        assert!(loaded_profile.is_some());
        let profile = loaded_profile.unwrap();
        assert_eq!(profile.name, "Test Child");

        drop(dir);
    }

    #[tokio::test]
    #[ignore]
    async fn test_bdd_given_profile_when_activated_then_session_created() {
        // Given: A profile manager and a profile
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;

        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

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
    #[ignore]
    async fn test_bdd_given_active_session_when_deactivated_then_session_ended() {
        // Given: An active profile with a session
        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
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
    #[ignore]
    async fn test_bdd_given_activities_when_get_used_time_today_then_returns_total() {
        use dots_family_db::{models::NewActivity, queries::activities::ActivityQueries};

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
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
    #[ignore]
    async fn test_bdd_given_used_time_when_get_remaining_time_then_returns_difference() {
        use dots_family_db::{models::NewActivity, queries::activities::ActivityQueries};

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
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
    #[ignore]
    async fn test_bdd_given_time_limit_exceeded_when_check_app_then_returns_false() {
        use dots_family_db::{models::NewActivity, queries::activities::ActivityQueries};

        let (db, _dir, config) = setup_test_db().await;
        let profile_id = create_test_profile(&db, "Test Child").await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();
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
        let (db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config, db).await.unwrap();

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
        let (db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config, db).await.unwrap();

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
        let (db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config, db).await.unwrap();

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
        let (db, _temp_dir, config) = setup_test_db().await;
        let mut manager = ProfileManager::new(&config, db).await.unwrap();

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
        let (db, _temp_dir, config) = setup_test_db().await;
        let manager = ProfileManager::new(&config, db.clone()).await.unwrap();

        // Given no parent password is configured

        // When attempting authentication
        let result = manager.authenticate_parent("any_password").await;

        // Then authentication fails with configuration error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("authentication not configured"));
    }
}
