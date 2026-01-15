-- Add filter tables and cache tables
-- For content filtering, custom rules, and performance optimization

-- Filter Lists: Web filter list metadata
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

-- Filter Rules: Individual filter rules (cached from lists)
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

-- Custom Rules: User-defined custom rules
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

-- Policy Cache: Performance-critical lookups (unencrypted cache database)
CREATE TABLE policy_cache (
    profile_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,

    PRIMARY KEY (profile_id, key)
);

CREATE INDEX idx_policy_cache_expiry ON policy_cache(expires_at);

-- App Info Cache: Application metadata cache (unencrypted cache database)
CREATE TABLE app_info_cache (
    app_id TEXT PRIMARY KEY,
    app_name TEXT NOT NULL,
    category TEXT,
    desktop_file TEXT,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);