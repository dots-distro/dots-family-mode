-- Add summary tables for reporting and analytics

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
