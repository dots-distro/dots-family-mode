-- Initial schema for DOTS Family Mode
-- Main database (encrypted with SQLCipher)

-- ============================================================================
-- Profile Management
-- ============================================================================

-- Child profile configurations
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,  -- UUID
    name TEXT NOT NULL UNIQUE,
    age_group TEXT NOT NULL,  -- '5-7', '8-12', '13-17'
    birthday DATE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    config TEXT NOT NULL,  -- JSON configuration
    active BOOLEAN NOT NULL DEFAULT 1
);

CREATE INDEX idx_profiles_name ON profiles(name);
CREATE INDEX idx_profiles_active ON profiles(active);

-- Track policy changes for audit purposes
CREATE TABLE policy_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    config TEXT NOT NULL,  -- JSON snapshot
    changed_by TEXT NOT NULL,  -- 'parent', 'system', 'migration'
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_policy_versions_profile ON policy_versions(profile_id, version DESC);

-- ============================================================================
-- Activity Tracking
-- ============================================================================

-- Login sessions tracking
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,  -- UUID
    profile_id TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    end_reason TEXT,  -- 'logout', 'time_limit', 'bedtime', 'crash', 'shutdown'
    duration_seconds INTEGER,
    screen_time_seconds INTEGER,
    active_time_seconds INTEGER,
    idle_time_seconds INTEGER,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_profile_start ON sessions(profile_id, start_time DESC);
CREATE INDEX idx_sessions_active ON sessions(profile_id, end_time) WHERE end_time IS NULL;

-- Application usage within sessions
CREATE TABLE activities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    app_id TEXT NOT NULL,
    app_name TEXT NOT NULL,
    category TEXT,
    window_title TEXT,  -- Optional, privacy-sensitive
    duration_seconds INTEGER NOT NULL,

    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_activities_session ON activities(session_id);
CREATE INDEX idx_activities_profile_time ON activities(profile_id, timestamp DESC);
CREATE INDEX idx_activities_app ON activities(app_id, timestamp DESC);
CREATE INDEX idx_activities_category ON activities(category, timestamp DESC);

-- Web browsing activity tracking
CREATE TABLE network_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    domain TEXT NOT NULL,  -- Not full URL for privacy
    category TEXT,
    duration_seconds INTEGER,
    blocked BOOLEAN NOT NULL,
    action TEXT NOT NULL,  -- 'allowed', 'blocked', 'warned'
    reason TEXT,  -- Block reason if blocked

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_network_profile_time ON network_activity(profile_id, timestamp DESC);
CREATE INDEX idx_network_domain ON network_activity(domain, timestamp DESC);
CREATE INDEX idx_network_blocked ON network_activity(profile_id, blocked, timestamp DESC);

-- Terminal command usage tracking
CREATE TABLE terminal_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    command TEXT NOT NULL,  -- Sanitized command
    risk_level TEXT NOT NULL,  -- 'safe', 'educational', 'risky', 'dangerous'
    action TEXT NOT NULL,  -- 'allowed', 'blocked', 'warned', 'approved'
    blocked BOOLEAN NOT NULL,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_terminal_profile_time ON terminal_activity(profile_id, timestamp DESC);
CREATE INDEX idx_terminal_blocked ON terminal_activity(profile_id, blocked, timestamp DESC);

-- ============================================================================
-- Events and Logging
-- ============================================================================

-- General event logging for audit trail
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT,  -- NULL for system events
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,  -- 'info', 'warning', 'error'
    details TEXT,  -- JSON
    metadata TEXT,  -- JSON

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_events_profile_time ON events(profile_id, timestamp DESC);
CREATE INDEX idx_events_type ON events(event_type, timestamp DESC);
CREATE INDEX idx_events_severity ON events(severity, timestamp DESC) WHERE severity != 'info';

-- Security-focused audit trail (immutable)
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    actor TEXT NOT NULL,  -- 'parent', 'child', 'system'
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    resource_id TEXT,
    ip_address TEXT,
    success BOOLEAN NOT NULL,
    details TEXT,  -- JSON

    -- Prevent modifications
    CHECK (1)  -- Always true, but with trigger enforcement
);

-- Prevent updates and deletes to audit log
CREATE TRIGGER audit_log_immutable_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Audit log is immutable');
END;

CREATE TRIGGER audit_log_immutable_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Audit log is immutable');
END;

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_actor ON audit_log(actor, timestamp DESC);
CREATE INDEX idx_audit_action ON audit_log(action, timestamp DESC);

-- ============================================================================
-- Aggregated Data
-- ============================================================================

-- Pre-aggregated daily statistics for performance
CREATE TABLE daily_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    date DATE NOT NULL,
    screen_time_seconds INTEGER NOT NULL,
    active_time_seconds INTEGER NOT NULL,
    idle_time_seconds INTEGER NOT NULL,
    app_launches INTEGER NOT NULL,
    unique_apps INTEGER NOT NULL,
    websites_visited INTEGER NOT NULL,
    blocks_count INTEGER NOT NULL,
    violations_count INTEGER NOT NULL,

    -- JSON arrays
    top_apps TEXT NOT NULL,  -- [{"app_id": "...", "duration": 3600}, ...]
    top_categories TEXT NOT NULL,
    top_websites TEXT NOT NULL,

    summary_generated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, date)
);

CREATE INDEX idx_daily_summaries_profile_date ON daily_summaries(profile_id, date DESC);

-- Pre-aggregated weekly statistics
CREATE TABLE weekly_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    week_start DATE NOT NULL,  -- Monday of the week
    week_end DATE NOT NULL,

    total_screen_time_seconds INTEGER NOT NULL,
    daily_average_seconds INTEGER NOT NULL,

    -- Comparison with previous week
    previous_week_seconds INTEGER,
    change_percentage REAL,

    -- Category breakdown
    category_breakdown TEXT NOT NULL,  -- JSON

    -- Compliance metrics
    days_within_limit INTEGER NOT NULL,
    days_exceeded_limit INTEGER NOT NULL,
    violations_count INTEGER NOT NULL,

    -- Top items
    top_apps TEXT NOT NULL,
    top_categories TEXT NOT NULL,

    summary_generated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, week_start)
);

CREATE INDEX idx_weekly_summaries_profile ON weekly_summaries(profile_id, week_start DESC);

-- ============================================================================
-- Exception and Override Management
-- ============================================================================

-- Temporary policy exceptions
CREATE TABLE exceptions (
    id TEXT PRIMARY KEY,  -- UUID
    profile_id TEXT NOT NULL,
    exception_type TEXT NOT NULL,  -- 'extra_time', 'allow_app', 'allow_website', 'suspend_monitoring'
    granted_by TEXT NOT NULL,  -- 'parent', 'system'
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    reason TEXT,

    -- Type-specific data
    amount_minutes INTEGER,  -- For extra_time
    app_id TEXT,  -- For allow_app
    website TEXT,  -- For allow_website
    scope TEXT,  -- For suspend_monitoring

    active BOOLEAN NOT NULL DEFAULT 1,
    used BOOLEAN NOT NULL DEFAULT 0,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_exceptions_profile_active ON exceptions(profile_id, active, expires_at);

-- Pending approval requests from children
CREATE TABLE approval_requests (
    id TEXT PRIMARY KEY,  -- UUID
    profile_id TEXT NOT NULL,
    request_type TEXT NOT NULL,  -- 'app', 'website', 'command', 'exception'
    requested_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'approved', 'denied'

    -- Request details
    details TEXT NOT NULL,  -- JSON

    -- Parent response
    reviewed_by TEXT,
    reviewed_at TIMESTAMP,
    response_reason TEXT,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_approval_requests_profile_pending ON approval_requests(profile_id, status, requested_at DESC);

-- ============================================================================
-- Filter Lists and Rules
-- ============================================================================

-- Web filter list metadata
CREATE TABLE filter_lists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    url TEXT,
    type TEXT NOT NULL,  -- 'builtin', 'community', 'custom'
    enabled BOOLEAN NOT NULL DEFAULT 1,

    last_updated TIMESTAMP,
    next_update TIMESTAMP,
    version TEXT,

    rules_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_filter_lists_enabled ON filter_lists(enabled);

-- Individual filter rules (cached from lists)
CREATE TABLE filter_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    list_id TEXT NOT NULL,
    rule_type TEXT NOT NULL,  -- 'domain', 'url', 'pattern', 'category'
    pattern TEXT NOT NULL,
    action TEXT NOT NULL,  -- 'block', 'allow'
    category TEXT,

    FOREIGN KEY (list_id) REFERENCES filter_lists(id) ON DELETE CASCADE
);

CREATE INDEX idx_filter_rules_pattern ON filter_rules(pattern);
CREATE INDEX idx_filter_rules_category ON filter_rules(category);

-- User-defined custom rules
CREATE TABLE custom_rules (
    id TEXT PRIMARY KEY,  -- UUID
    profile_id TEXT,  -- NULL for global
    rule_type TEXT NOT NULL,
    pattern TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,  -- 'parent', 'import'

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_custom_rules_profile ON custom_rules(profile_id);
