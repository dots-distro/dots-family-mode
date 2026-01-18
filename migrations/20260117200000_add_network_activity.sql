-- Add missing network_activity table for web browsing tracking
-- This table is required by the DATA_SCHEMA.md specification

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