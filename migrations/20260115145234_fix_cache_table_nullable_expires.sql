-- Fix policy_cache table to make expires_at nullable (some cache entries don't expire)
-- Drop and recreate table with correct schema

DROP TABLE IF EXISTS policy_cache;

CREATE TABLE policy_cache (
    profile_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,  -- Now nullable for permanent cache entries

    PRIMARY KEY (profile_id, key)
);

CREATE INDEX idx_policy_cache_expiry ON policy_cache(expires_at);