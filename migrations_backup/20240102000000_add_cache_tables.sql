-- Add missing tables for app cache and policy cache

-- App information cache for performance optimization
CREATE TABLE app_info_cache (
    app_id TEXT PRIMARY KEY,
    app_name TEXT NOT NULL,
    category TEXT,
    desktop_file TEXT,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_app_info_cache_category ON app_info_cache(category);
CREATE INDEX idx_app_info_cache_cached_at ON app_info_cache(cached_at);

-- Policy cache for improved performance
CREATE TABLE policy_cache (
    profile_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,

    PRIMARY KEY (profile_id, key),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_policy_cache_expires ON policy_cache(expires_at) WHERE expires_at IS NOT NULL;

-- Terminal policies (from Phase 4 terminal filtering)
CREATE TABLE terminal_policies (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    command_pattern TEXT NOT NULL,
    action TEXT NOT NULL, -- 'allow', 'block', 'warn', 'require_approval'
    risk_level TEXT NOT NULL, -- 'safe', 'educational', 'risky', 'dangerous'
    educational_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    active BOOLEAN NOT NULL DEFAULT 1,

    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_terminal_policies_profile ON terminal_policies(profile_id);
CREATE INDEX idx_terminal_policies_active ON terminal_policies(active);

-- Terminal commands log
CREATE TABLE terminal_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    profile_id TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    command TEXT NOT NULL,
    shell TEXT NOT NULL,
    working_directory TEXT,
    exit_code INTEGER,
    duration_ms INTEGER,
    risk_level TEXT NOT NULL,
    action_taken TEXT NOT NULL,
    script_path TEXT,

    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_terminal_commands_profile_time ON terminal_commands(profile_id, timestamp DESC);
CREATE INDEX idx_terminal_commands_session ON terminal_commands(session_id);

-- Script analysis cache
CREATE TABLE script_analysis (
    id TEXT PRIMARY KEY,
    script_path TEXT NOT NULL UNIQUE,
    content_hash TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    dangerous_patterns TEXT, -- JSON array
    analysis_result TEXT NOT NULL, -- JSON
    analyzed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    file_size INTEGER,
    line_count INTEGER
);

CREATE INDEX idx_script_analysis_path ON script_analysis(script_path);
CREATE INDEX idx_script_analysis_analyzed_at ON script_analysis(analyzed_at);