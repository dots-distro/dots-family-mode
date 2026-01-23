use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Age group classifications for children with pre-configured defaults.
/// Each age group has appropriate screen time limits and restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeGroup {
    /// Ages 5-7: Early elementary with strict restrictions
    #[serde(rename = "5-7")]
    EarlyElementary,
    /// Ages 8-12: Late elementary with moderate restrictions
    #[serde(rename = "8-12")]
    LateElementary,
    /// Ages 13-17: High school with relaxed restrictions
    #[serde(rename = "13-17")]
    HighSchool,
}

/// Core user profile containing all child settings and configurations.
///
/// This is the primary type used throughout the system to manage
/// individual child accounts and their associated policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier for the profile
    pub id: Uuid,
    /// Display name for the child
    pub name: String,
    /// Age-based classification determining default restrictions
    pub age_group: AgeGroup,
    /// Optional birthday for more precise age-based filtering
    pub birthday: Option<DateTime<Utc>>,
    /// When this profile was created
    pub created_at: DateTime<Utc>,
    /// Last modification time
    pub updated_at: DateTime<Utc>,
    /// Complete configuration for this profile
    pub config: ProfileConfig,
    /// Whether this profile is currently active
    pub active: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            age_group: AgeGroup::EarlyElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig::default(),
            active: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    pub screen_time: ScreenTimeConfig,
    pub applications: ApplicationConfig,
    pub web_filtering: WebFilteringConfig,
    pub terminal_filtering: TerminalFilteringConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenTimeConfig {
    pub daily_limit_minutes: u32,
    pub weekend_bonus_minutes: u32,
    pub exempt_categories: Vec<String>,
    pub windows: TimeWindows,
}

impl Default for ScreenTimeConfig {
    fn default() -> Self {
        Self {
            daily_limit_minutes: 120,
            weekend_bonus_minutes: 0,
            exempt_categories: Vec::new(),
            windows: TimeWindows { weekday: Vec::new(), weekend: Vec::new() },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindows {
    pub weekday: Vec<TimeWindow>,
    pub weekend: Vec<TimeWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub mode: ApplicationMode,
    pub allowed: Vec<String>,
    pub blocked: Vec<String>,
    pub blocked_categories: Vec<String>,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            mode: ApplicationMode::Allowlist,
            allowed: Vec::new(),
            blocked: Vec::new(),
            blocked_categories: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationMode {
    Allowlist,
    Blocklist,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFilteringConfig {
    pub enabled: bool,
    pub safe_search: bool,
    pub blocked_categories: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
}

impl Default for WebFilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            safe_search: true,
            blocked_categories: Vec::new(),
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFilteringConfig {
    pub enabled: bool,
    pub block_threshold: String,
    pub approval_threshold: String,
    pub blocked_commands: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub educational_messages: bool,
    pub log_all_commands: bool,
}

impl Default for TerminalFilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            block_threshold: "critical".to_string(),
            approval_threshold: "high".to_string(),
            blocked_commands: Vec::new(),
            allowed_commands: Vec::new(),
            educational_messages: true,
            log_all_commands: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub activity_type: ActivityType,
    pub application: Option<String>,
    pub window_title: Option<String>,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityType {
    ApplicationUsage,
    WebBrowsing { url: String },
    TerminalCommand { command: String },
    PolicyViolation { reason: String },
}

// ============================================================================
// Exception Management System
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionType {
    /// Temporarily allows a blocked application
    ApplicationOverride { app_id: String },
    /// Temporarily allows access to a blocked website or domain
    WebsiteOverride { domain: String },
    /// Extends screen time beyond daily limit
    ScreenTimeExtension { extra_minutes: u32 },
    /// Allows access outside normal time windows
    TimeWindowOverride { start: DateTime<Utc>, end: DateTime<Utc> },
    /// Temporarily allows a blocked terminal command
    TerminalCommandOverride { command: String },
    /// Custom override with specific policy changes
    CustomOverride { description: String, policy_changes: HashMap<String, String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionDuration {
    /// Exception expires after specified duration
    Duration(Duration),
    /// Exception expires at specific time
    UntilTime(DateTime<Utc>),
    /// Exception expires after current session ends
    UntilSessionEnd,
    /// Exception expires at end of current day
    UntilEndOfDay,
    /// Manual expiration only (requires parent to revoke)
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionStatus {
    /// Exception is active and being enforced
    Active,
    /// Exception has expired naturally
    Expired,
    /// Exception was manually revoked by parent
    Revoked,
    /// Exception is scheduled for future activation
    Scheduled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub exception_type: ExceptionType,
    pub reason: String,
    pub duration: ExceptionDuration,
    pub status: ExceptionStatus,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_by: String, // "parent" or "auto" or "system"
}

impl Exception {
    pub fn new(
        profile_id: Uuid,
        exception_type: ExceptionType,
        reason: String,
        duration: ExceptionDuration,
        created_by: String,
    ) -> Self {
        let now = Utc::now();
        let expires_at = match &duration {
            ExceptionDuration::Duration(d) => Some(now + *d),
            ExceptionDuration::UntilTime(time) => Some(*time),
            ExceptionDuration::UntilEndOfDay => {
                let end_of_day = now.date_naive().and_hms_opt(23, 59, 59).unwrap();
                Some(DateTime::from_naive_utc_and_offset(end_of_day, Utc))
            }
            ExceptionDuration::UntilSessionEnd | ExceptionDuration::Manual => None,
        };

        Self {
            id: Uuid::new_v4(),
            profile_id,
            exception_type,
            reason,
            duration,
            status: ExceptionStatus::Active,
            created_at: now,
            activated_at: Some(now),
            expires_at,
            revoked_at: None,
            created_by,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == ExceptionStatus::Active && !self.is_expired()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn revoke(&mut self) {
        self.status = ExceptionStatus::Revoked;
        self.revoked_at = Some(Utc::now());
    }
}

// ============================================================================
// Approval Request System
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestType {
    /// Child requests access to blocked application
    ApplicationAccess { app_id: String },
    /// Child requests access to blocked website
    WebsiteAccess { url: String, domain: String },
    /// Child requests extra screen time
    ScreenTimeExtension { requested_minutes: u32 },
    /// Child requests to extend time window
    TimeExtension { requested_end_time: DateTime<Utc> },
    /// Child requests to run blocked command
    TerminalCommand { command: String },
    /// Custom request with free-form description
    Custom { description: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    /// Request is pending parent review
    Pending,
    /// Parent approved the request
    Approved,
    /// Parent denied the request
    Denied,
    /// Request was auto-approved by system rules
    AutoApproved,
    /// Request expired without response
    Expired,
    /// Request was cancelled by child
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub request_type: RequestType,
    pub message: Option<String>, // Optional message from child
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
    pub response_message: Option<String>, // Optional message from parent
    pub expires_at: DateTime<Utc>,        // Requests expire after 1 hour by default
    pub auto_approve_rule: Option<String>, // ID of auto-approval rule if applicable
}

impl ApprovalRequest {
    pub fn new(
        profile_id: Uuid,
        request_type: RequestType,
        message: Option<String>,
        expires_in: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            profile_id,
            request_type,
            message,
            status: RequestStatus::Pending,
            created_at: now,
            responded_at: None,
            response_message: None,
            expires_at: now + expires_in,
            auto_approve_rule: None,
        }
    }

    pub fn approve(&mut self, response_message: Option<String>) {
        self.status = RequestStatus::Approved;
        self.responded_at = Some(Utc::now());
        self.response_message = response_message;
    }

    pub fn deny(&mut self, response_message: Option<String>) {
        self.status = RequestStatus::Denied;
        self.responded_at = Some(Utc::now());
        self.response_message = response_message;
    }

    pub fn auto_approve(&mut self, rule_id: String) {
        self.status = RequestStatus::AutoApproved;
        self.responded_at = Some(Utc::now());
        self.auto_approve_rule = Some(rule_id);
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at && self.status == RequestStatus::Pending
    }

    pub fn cancel(&mut self) {
        self.status = RequestStatus::Cancelled;
        self.responded_at = Some(Utc::now());
    }
}

// ============================================================================
// Notification System
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// New approval request from child
    ApprovalRequest { request_id: Uuid },
    /// Policy violation occurred
    PolicyViolation { violation_type: String, details: String },
    /// Screen time limit approaching
    ScreenTimeLimitWarning { minutes_remaining: u32 },
    /// Time window ending soon
    TimeWindowEnding { minutes_remaining: u32 },
    /// Unusual activity detected
    UnusualActivity { activity_description: String },
    /// System error or issue
    SystemAlert { severity: AlertSeverity, message: String },
    /// Daily/weekly usage report available
    UsageReport { report_type: String, period: String },
    /// New exception created
    ExceptionCreated { exception_id: Uuid },
    /// Exception expired or revoked
    ExceptionEnded { exception_id: Uuid, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub profile_id: Option<Uuid>, // None for system-wide notifications
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub status: NotificationStatus,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub priority: NotificationPriority,
    pub channels: Vec<NotificationChannel>, // Where to send (desktop, email, etc.)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Desktop, // freedesktop.org notifications
    Email,   // SMTP email
    Sms,     // SMS (future)
    Push,    // Mobile push (future)
    InApp,   // GUI application notification
}

impl Notification {
    pub fn new(
        profile_id: Option<Uuid>,
        notification_type: NotificationType,
        title: String,
        message: String,
        priority: NotificationPriority,
        channels: Vec<NotificationChannel>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            profile_id,
            notification_type,
            title,
            message,
            status: NotificationStatus::Pending,
            created_at: Utc::now(),
            sent_at: None,
            read_at: None,
            priority,
            channels,
        }
    }

    pub fn mark_sent(&mut self) {
        self.status = NotificationStatus::Sent;
        self.sent_at = Some(Utc::now());
    }

    pub fn mark_read(&mut self) {
        self.status = NotificationStatus::Read;
        self.read_at = Some(Utc::now());
    }
}

// ============================================================================
// Behavioral Pattern Detection
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityPattern {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub pattern_type: PatternType,
    pub description: String,
    pub threshold_value: f64,
    pub current_value: f64,
    pub detection_window: Duration, // How far back to look
    pub created_at: DateTime<Utc>,
    pub last_detected: Option<DateTime<Utc>>,
    pub alert_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    /// Unusual spike in activity for specific app
    ApplicationSpike { app_id: String },
    /// Accessing blocked content repeatedly
    RepeatedViolations { violation_type: String },
    /// Screen time consistently hitting limits
    ScreenTimeTrend,
    /// Trying to access restricted content at unusual times
    OffHoursActivity,
    /// Pattern of requesting many exceptions
    ExcessiveRequests,
    /// Time spent on specific category changing significantly
    CategoryUsageChange { category: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorAlert {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub pattern_id: Uuid,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub description: String,
    pub recommendation: Option<String>,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub dismissed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    TrendAlert,
    ThresholdExceeded,
    AnomalyDetected,
    ComplianceIssue,
    SystemHealth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_default() {
        let profile = Profile::default();
        assert!(!profile.active);
        assert_eq!(profile.name, "");
        assert_eq!(profile.age_group, AgeGroup::EarlyElementary);
    }

    #[test]
    fn test_age_group_serialization() {
        let age = AgeGroup::EarlyElementary;
        let json = serde_json::to_string(&age).unwrap();
        assert_eq!(json, r#""5-7""#);

        let deserialized: AgeGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AgeGroup::EarlyElementary);
    }

    #[test]
    fn test_application_mode_serialization() {
        let mode = ApplicationMode::Allowlist;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""allowlist""#);
    }

    #[test]
    fn test_profile_creation() {
        let profile = Profile {
            id: Uuid::new_v4(),
            name: "Test Child".to_string(),
            age_group: AgeGroup::LateElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 120,
                    weekend_bonus_minutes: 60,
                    exempt_categories: vec!["education".to_string()],
                    windows: TimeWindows { weekday: vec![], weekend: vec![] },
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Allowlist,
                    allowed: vec!["firefox".to_string()],
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
            },
            active: true,
        };

        let json = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json.contains("Test Child"));
    }

    #[test]
    fn test_profile_serialization_roundtrip() {
        let original = Profile {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            age_group: AgeGroup::HighSchool,
            birthday: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 180,
                    weekend_bonus_minutes: 120,
                    exempt_categories: vec!["education".to_string(), "creativity".to_string()],
                    windows: TimeWindows {
                        weekday: vec![TimeWindow {
                            start: "16:00".to_string(),
                            end: "21:00".to_string(),
                        }],
                        weekend: vec![TimeWindow {
                            start: "09:00".to_string(),
                            end: "22:00".to_string(),
                        }],
                    },
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Blocklist,
                    allowed: vec![],
                    blocked: vec!["discord".to_string(), "tiktok".to_string()],
                    blocked_categories: vec!["social-media".to_string(), "games".to_string()],
                },
                web_filtering: WebFilteringConfig {
                    enabled: true,
                    safe_search: true,
                    blocked_categories: vec!["adult".to_string(), "gambling".to_string()],
                    allowed_domains: vec!["khan-academy.org".to_string()],
                    blocked_domains: vec!["reddit.com".to_string()],
                },
                terminal_filtering: TerminalFilteringConfig::default(),
            },
            active: true,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Profile = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.age_group, deserialized.age_group);
        assert_eq!(original.active, deserialized.active);
        assert_eq!(
            original.config.screen_time.daily_limit_minutes,
            deserialized.config.screen_time.daily_limit_minutes
        );
        assert_eq!(original.config.applications.mode, deserialized.config.applications.mode);
    }

    #[test]
    fn test_activity_type_variants() {
        let app_usage = ActivityType::ApplicationUsage;
        let json = serde_json::to_string(&app_usage).unwrap();
        assert!(json.contains("application_usage"));

        let web_browsing = ActivityType::WebBrowsing { url: "https://example.com".to_string() };
        let json = serde_json::to_string(&web_browsing).unwrap();
        assert!(json.contains("web_browsing"));
        assert!(json.contains("https://example.com"));

        let terminal = ActivityType::TerminalCommand { command: "rm -rf /".to_string() };
        let json = serde_json::to_string(&terminal).unwrap();
        assert!(json.contains("terminal_command"));
        assert!(json.contains("rm -rf /"));

        let violation = ActivityType::PolicyViolation { reason: "Time limit exceeded".to_string() };
        let json = serde_json::to_string(&violation).unwrap();
        assert!(json.contains("policy_violation"));
    }

    #[test]
    fn test_activity_creation() {
        let activity = Activity {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            activity_type: ActivityType::ApplicationUsage,
            application: Some("firefox".to_string()),
            window_title: Some("GitHub - Mozilla Firefox".to_string()),
            duration_seconds: 300,
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("firefox"));
        assert!(json.contains("GitHub"));

        let deserialized: Activity = serde_json::from_str(&json).unwrap();
        assert_eq!(activity.id, deserialized.id);
        assert_eq!(activity.profile_id, deserialized.profile_id);
        assert_eq!(activity.duration_seconds, deserialized.duration_seconds);
    }

    #[test]
    fn test_time_window_validation() {
        let window = TimeWindow { start: "09:00".to_string(), end: "17:00".to_string() };

        let json = serde_json::to_string(&window).unwrap();
        let deserialized: TimeWindow = serde_json::from_str(&json).unwrap();

        assert_eq!(window.start, deserialized.start);
        assert_eq!(window.end, deserialized.end);
    }

    #[test]
    fn test_application_config_allowlist() {
        let config = ApplicationConfig {
            mode: ApplicationMode::Allowlist,
            allowed: vec!["firefox".to_string(), "vscode".to_string()],
            blocked: vec![],
            blocked_categories: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("allowlist"));
        assert!(json.contains("firefox"));
        assert!(json.contains("vscode"));
    }

    #[test]
    fn test_application_config_blocklist() {
        let config = ApplicationConfig {
            mode: ApplicationMode::Blocklist,
            allowed: vec![],
            blocked: vec!["steam".to_string(), "discord".to_string()],
            blocked_categories: vec!["games".to_string(), "social-media".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("blocklist"));
        assert!(json.contains("steam"));
        assert!(json.contains("games"));
    }

    #[test]
    fn test_web_filtering_config() {
        let config = WebFilteringConfig {
            enabled: true,
            safe_search: true,
            blocked_categories: vec!["adult".to_string(), "violence".to_string()],
            allowed_domains: vec!["wikipedia.org".to_string()],
            blocked_domains: vec!["4chan.org".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebFilteringConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.safe_search, deserialized.safe_search);
        assert_eq!(config.blocked_categories.len(), deserialized.blocked_categories.len());
    }

    #[test]
    fn test_screen_time_config() {
        let config = ScreenTimeConfig {
            daily_limit_minutes: 120,
            weekend_bonus_minutes: 60,
            exempt_categories: vec!["education".to_string()],
            windows: TimeWindows {
                weekday: vec![TimeWindow { start: "16:00".to_string(), end: "20:00".to_string() }],
                weekend: vec![],
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ScreenTimeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.daily_limit_minutes, deserialized.daily_limit_minutes);
        assert_eq!(config.weekend_bonus_minutes, deserialized.weekend_bonus_minutes);
        assert_eq!(config.exempt_categories, deserialized.exempt_categories);
    }

    #[test]
    fn test_all_age_groups() {
        let groups =
            vec![AgeGroup::EarlyElementary, AgeGroup::LateElementary, AgeGroup::HighSchool];

        for group in groups {
            let json = serde_json::to_string(&group).unwrap();
            let deserialized: AgeGroup = serde_json::from_str(&json).unwrap();
            assert_eq!(group, deserialized);
        }
    }
}
