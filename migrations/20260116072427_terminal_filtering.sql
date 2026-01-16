-- Migration: Add terminal filtering tables
-- Created: 2026-01-16 07:24:27

-- Terminal command filtering policies
CREATE TABLE terminal_policies (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    command_pattern TEXT NOT NULL,
    action TEXT NOT NULL, -- "block", "warn", "allow"
    risk_level TEXT NOT NULL, -- "low", "medium", "high", "critical"
    educational_message TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    FOREIGN KEY (profile_id) REFERENCES profiles (id) ON DELETE CASCADE
);

-- Index for fast policy lookups
CREATE INDEX idx_terminal_policies_profile_active ON terminal_policies (profile_id, active);
CREATE INDEX idx_terminal_policies_pattern ON terminal_policies (command_pattern);

-- Terminal command execution log
CREATE TABLE terminal_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    command TEXT NOT NULL,
    shell TEXT NOT NULL, -- "bash", "zsh", "fish"
    working_directory TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    action_taken TEXT NOT NULL, -- "allowed", "blocked", "warned"
    exit_code INTEGER,
    duration_ms INTEGER,
    script_path TEXT, -- If command was a script
    FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (profile_id) REFERENCES profiles (id) ON DELETE CASCADE
);

-- Indexes for fast command log queries
CREATE INDEX idx_terminal_commands_session ON terminal_commands (session_id);
CREATE INDEX idx_terminal_commands_profile_time ON terminal_commands (profile_id, timestamp);
CREATE INDEX idx_terminal_commands_risk ON terminal_commands (risk_level, timestamp);

-- Script analysis cache and results
CREATE TABLE script_analysis (
    id TEXT PRIMARY KEY NOT NULL,
    script_path TEXT NOT NULL,
    content_hash TEXT NOT NULL, -- MD5 hash of script content
    risk_level TEXT NOT NULL,
    dangerous_patterns TEXT NOT NULL, -- JSON array of detected patterns
    analysis_result TEXT NOT NULL, -- JSON serialized analysis result
    analyzed_at DATETIME NOT NULL,
    file_size INTEGER NOT NULL,
    line_count INTEGER NOT NULL,
    UNIQUE (script_path, content_hash)
);

-- Index for fast script analysis cache lookups
CREATE INDEX idx_script_analysis_path_hash ON script_analysis (script_path, content_hash);
CREATE INDEX idx_script_analysis_path ON script_analysis (script_path);