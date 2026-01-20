-- Initial schema for DOTS Family Mode
-- Core tables for profiles, sessions, activities, and system configuration

-- System configuration table
CREATE TABLE daemon_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Child profiles configuration
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

-- Policy change tracking for audit purposes
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
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
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

-- Prevent updates and deletes on audit log
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

-- Terminal command usage tracking
CREATE TABLE terminal_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    command TEXT NOT NULL,  -- Sanitized command
    risk_level TEXT NOT NULL,  -- 'safe', 'educational', 'risky', 'dangerous'
    action TEXT NOT NULL,  -- 'allowed', 'blocked', 'warned', 'approved'
    blocked BOOLEAN NOT NULL,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_terminal_profile_time ON terminal_activity(profile_id, timestamp DESC);
CREATE INDEX idx_terminal_blocked ON terminal_activity(profile_id, blocked, timestamp DESC);
