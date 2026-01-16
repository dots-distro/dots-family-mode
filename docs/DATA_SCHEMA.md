# Data Schema and Storage

## Overview

Family Mode uses SQLite for local data storage with encrypted databases for sensitive information. The schema supports activity tracking, policy management, reporting, and audit logging while maintaining performance through appropriate indexing and data retention policies.

## Database Architecture

### Database Files

```
~/.local/share/dots-family/
├── family.db              # Main database (encrypted)
├── family.db-wal          # Write-Ahead Log
├── family.db-shm          # Shared memory
├── cache.db               # Performance cache (not encrypted)
└── archives/
    ├── 2024-01.db        # Monthly archives
    └── 2024-02.db
```

### Encryption

**SQLCipher Configuration**:
```sql
-- Enable encryption with 256-bit key
PRAGMA key = 'x''<hex-encoded-key>'';
PRAGMA cipher_page_size = 4096;
PRAGMA kdf_iter = 256000;  -- PBKDF2 iterations
PRAGMA cipher_hmac_algorithm = HMAC_SHA512;
PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;
```

**Key Derivation**:
```rust
use argon2::{Argon2, password_hash};

pub fn derive_db_key(parent_password: &str, salt: &[u8]) -> Result<Vec<u8>> {
    let argon2 = Argon2::default();

    let mut key = [0u8; 32];
    argon2.hash_password_into(
        parent_password.as_bytes(),
        salt,
        &mut key,
    )?;

    Ok(key.to_vec())
}
```

## Core Schema

### Configuration Tables

#### profiles

Stores child profile configurations.

```sql
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
```

**config JSON structure**:
```json
{
  "screen_time": {
    "daily_limit_minutes": 120,
    "weekend_bonus_minutes": 60,
    "exempt_categories": ["education"],
    "windows": {
      "weekday": [
        {"start": "06:00", "end": "08:00"},
        {"start": "15:00", "end": "19:00"}
      ],
      "weekend": [
        {"start": "08:00", "end": "21:00"}
      ]
    }
  },
  "applications": {
    "mode": "allowlist",
    "allowed": ["firefox", "inkscape"],
    "blocked": [],
    "blocked_categories": ["games", "social-media"]
  },
  "web_filtering": {
    "enabled": true,
    "mode": "strict",
    "safe_search_enforced": true,
    "blocked_categories": ["adult", "violence"]
  },
  "terminal": {
    "enabled": false,
    "mode": "filter",
    "blocked_commands": ["rm -rf /"]
  }
}
```

#### policy_versions

Track policy changes for audit purposes.

```sql
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
```

### Activity Tables

#### sessions

Tracks login sessions.

```sql
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
```

#### activities

Tracks application usage within sessions.

```sql
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
```

#### network_activity

Tracks web browsing activity.

```sql
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
```

#### terminal_activity

Tracks terminal command usage.

```sql
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
```

### Events and Logging

#### events

General event logging for audit trail.

```sql
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
```

**Event Types**:
- `app_launched`
- `app_blocked`
- `app_closed`
- `time_limit_warning`
- `time_limit_reached`
- `time_limit_extended`
- `website_blocked`
- `command_blocked`
- `policy_violation`
- `parent_override`
- `monitoring_started`
- `monitoring_stopped`
- `profile_switched`

#### audit_log

Security-focused audit trail (immutable).

```sql
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

-- Prevent updates and deletes
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
```

### Aggregated Data

#### daily_summaries

Pre-aggregated daily statistics for performance.

```sql
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
```

#### weekly_summaries

Pre-aggregated weekly statistics.

```sql
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
```

### Exception and Override Management

#### exceptions

Temporary policy exceptions.

```sql
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
```

#### approval_requests

Pending approval requests from children.

```sql
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
```

### Filter Lists and Rules

#### filter_lists

Web filter list metadata.

```sql
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
```

#### filter_rules

Individual filter rules (cached from lists).

```sql
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
```

#### custom_rules

User-defined custom rules.

```sql
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
```

## Cache Database (Unencrypted)

Used for performance-critical lookups that don't contain sensitive data.

### policy_cache

```sql
CREATE TABLE policy_cache (
    profile_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,

    PRIMARY KEY (profile_id, key)
);

CREATE INDEX idx_policy_cache_expiry ON policy_cache(expires_at);
```

### app_info_cache

```sql
CREATE TABLE app_info_cache (
    app_id TEXT PRIMARY KEY,
    app_name TEXT NOT NULL,
    category TEXT,
    desktop_file TEXT,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## Database Migrations

### Migration System

```rust
use sqlx::migrate::Migrator;

pub async fn run_migrations(db: &SqlitePool) -> Result<()> {
    // Embedded migrations
    static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

    MIGRATOR.run(db).await?;

    Ok(())
}
```

### Migration Files

**migrations/001_initial.sql**:
```sql
-- Initial schema creation
-- (All CREATE TABLE statements from above)

-- Insert schema version
INSERT INTO schema_info (version, applied_at)
VALUES (1, CURRENT_TIMESTAMP);
```

**migrations/002_add_weekly_summaries.sql**:
```sql
-- Add weekly summaries table
CREATE TABLE weekly_summaries (...);

-- Update version
UPDATE schema_info SET version = 2, applied_at = CURRENT_TIMESTAMP;
```

## Data Retention and Archival

### Retention Policy Configuration

```toml
[retention]
enabled = true

# Detailed activity
activities_days = 30
network_activity_days = 30
terminal_activity_days = 30
events_days = 90

# Aggregated data
daily_summaries_days = 365
weekly_summaries_days = 730

# Audit log (never deleted automatically)
audit_log_days = -1  # Indefinite

# Archive before deletion
archive_before_delete = true
archive_path = "~/.local/share/dots-family/archives"
```

### Archival Process

```rust
pub async fn archive_old_data(db: &SqlitePool, config: &RetentionConfig) -> Result<()> {
    let now = Utc::now();

    // Calculate cutoff dates
    let activities_cutoff = now - Duration::days(config.activities_days);
    let events_cutoff = now - Duration::days(config.events_days);

    // Create archive database
    let archive_path = format!(
        "{}/archive-{}.db",
        config.archive_path,
        now.format("%Y-%m")
    );

    let archive_db = SqlitePool::connect(&archive_path).await?;

    // Copy data to archive
    sqlx::query(
        "ATTACH DATABASE ? AS archive"
    )
    .bind(&archive_path)
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO archive.activities
         SELECT * FROM main.activities
         WHERE timestamp < ?"
    )
    .bind(activities_cutoff)
    .execute(db)
    .await?;

    // Delete archived data from main database
    sqlx::query(
        "DELETE FROM activities WHERE timestamp < ?"
    )
    .bind(activities_cutoff)
    .execute(db)
    .await?;

    // Vacuum to reclaim space
    sqlx::query("VACUUM").execute(db).await?;

    Ok(())
}
```

## Query Patterns

### Common Queries

**Get today's screen time**:
```sql
SELECT COALESCE(SUM(duration_seconds), 0) as total_seconds
FROM activities
WHERE profile_id = ?
  AND date(timestamp) = date('now', 'localtime');
```

**Get active session**:
```sql
SELECT * FROM sessions
WHERE profile_id = ?
  AND end_time IS NULL
LIMIT 1;
```

**Top applications today**:
```sql
SELECT app_id, app_name, SUM(duration_seconds) as total_seconds
FROM activities
WHERE profile_id = ?
  AND date(timestamp) = date('now', 'localtime')
GROUP BY app_id, app_name
ORDER BY total_seconds DESC
LIMIT 10;
```

**Recent blocked attempts**:
```sql
SELECT event_type, timestamp, details
FROM events
WHERE profile_id = ?
  AND event_type IN ('app_blocked', 'website_blocked', 'command_blocked')
  AND timestamp > datetime('now', '-1 day')
ORDER BY timestamp DESC;
```

### Performance Optimization

**Prepared Statements**:
```rust
pub struct Queries {
    get_screen_time: Statement,
    record_activity: Statement,
    // ...
}

impl Queries {
    pub async fn new(db: &SqlitePool) -> Result<Self> {
        Ok(Self {
            get_screen_time: db.prepare(
                "SELECT COALESCE(SUM(duration_seconds), 0) FROM activities \
                 WHERE profile_id = ? AND date(timestamp) = date('now')"
            ).await?,
            // ...
        })
    }
}
```

**Connection Pooling**:
```rust
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect("sqlite:family.db")
    .await?;
```

## Backup and Recovery

### Automatic Backups

```rust
pub async fn backup_database(db_path: &Path, backup_dir: &Path) -> Result<()> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = backup_dir.join(format!("family-{}.db", timestamp));

    // SQLite backup API
    let src = Connection::open(db_path)?;
    let dst = Connection::open(&backup_path)?;

    let backup = backup::Backup::new(&src, &dst)?;
    backup.run_to_completion(5, Duration::from_millis(250), None)?;

    Ok(())
}
```

### Recovery

```rust
pub async fn restore_from_backup(backup_path: &Path, db_path: &Path) -> Result<()> {
    // Verify backup integrity
    let backup_db = SqlitePool::connect(backup_path).await?;
    sqlx::query("PRAGMA integrity_check")
        .execute(&backup_db)
        .await?;

    // Copy to main location
    fs::copy(backup_path, db_path)?;

    Ok(())
}
```

## Related Documentation

- ARCHITECTURE.md: Overall system design
- MONITORING.md: How data is collected
- RUST_APPLICATIONS.md: Database access patterns
- IMPLEMENTATION_ROADMAP.md: Schema evolution plan
