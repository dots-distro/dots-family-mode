CREATE TABLE IF NOT EXISTS daemon_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR REPLACE INTO daemon_settings (key, value) VALUES ('active_profile_id', '');
