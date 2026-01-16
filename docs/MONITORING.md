# Monitoring and Reporting System

## Overview

The Family Mode monitoring system provides parents with visibility into their children's computer usage while respecting privacy boundaries. All data is stored locally, with configurable retention policies and transparency requirements.

## Monitoring Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Activity Sources                           │
├─────────────┬─────────────┬─────────────┬────────────────────┤
│   Windows   │ Applications│   Network   │   File System      │
└──────┬──────┴──────┬──────┴──────┬──────┴──────┬─────────────┘
       │             │             │             │
       │             │             │             │
┌──────▼─────────────▼─────────────▼─────────────▼─────────────┐
│           dots-family-monitor (Rust)                          │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Window     │  │  Application │  │   Network    │       │
│  │   Tracker    │  │   Monitor    │  │   Monitor    │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │                 │
│         └─────────────────┼─────────────────┘                 │
│                           │                                   │
│                    ┌──────▼──────┐                            │
│                    │  Activity   │                            │
│                    │  Aggregator │                            │
│                    └──────┬──────┘                            │
│                           │                                   │
└───────────────────────────┼───────────────────────────────────┘
                            │
                            │ DBus
                            │
┌───────────────────────────▼───────────────────────────────────┐
│           dots-family-daemon (Rust)                           │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Activity Database (SQLite)                  │ │
│  │                                                           │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐             │ │
│  │  │ Sessions │  │ Activities│  │  Events  │             │ │
│  │  └──────────┘  └──────────┘  └──────────┘             │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Report Generator                            │ │
│  │                                                           │ │
│  │  - Daily summaries                                       │ │
│  │  - Weekly reports                                        │ │
│  │  - Custom analytics                                      │ │
│  └─────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

## Data Collection

### Window Activity Tracking

**Purpose**: Track which applications are actively used

**Collection Method**:
- Poll active window every 1 second
- Record window title, app ID, timestamp
- Calculate focus duration
- Aggregate into 5-minute buckets

**Configuration**:
```toml
[monitoring.windows]
enabled = true
poll_interval_ms = 1000
aggregate_interval_minutes = 5
track_window_titles = true  # Privacy: may contain sensitive info
anonymize_titles = false    # Replace titles with app name only
```

**Data Collected**:
```rust
pub struct WindowActivity {
    timestamp: DateTime<Utc>,
    profile: String,
    app_id: String,
    app_name: String,
    window_title: Option<String>,  // Optional for privacy
    duration_seconds: u64,
    window_manager: String,
}
```

**Privacy Options**:
- Disable window title tracking
- Anonymize titles (show app name only)
- Exclude specific applications from tracking
- Configurable retention period

### Application Usage Tracking

**Purpose**: Understand application usage patterns

**Data Collected**:
```rust
pub struct ApplicationUsage {
    date: NaiveDate,
    profile: String,
    app_id: String,
    app_name: String,
    category: String,
    total_duration_seconds: u64,
    launch_count: u32,
    first_launched: DateTime<Utc>,
    last_launched: DateTime<Utc>,
}
```

**Configuration**:
```toml
[monitoring.applications]
enabled = true
track_launch_count = true
track_duration = true
excluded_apps = [
  "org.gnome.Shell",  # System apps not useful to track
  "org.freedesktop.Notifications"
]
```

**Aggregation**:
- Real-time tracking during use
- Aggregated daily for reports
- Historical trends calculated weekly

### Network Activity Monitoring

**Purpose**: Understand web browsing patterns (high-level only)

**Data Collected**:
```rust
pub struct NetworkActivity {
    timestamp: DateTime<Utc>,
    profile: String,
    domain: String,           // Not full URL for privacy
    category: ContentCategory,
    duration_seconds: u64,
    blocked: bool,
    action: FilterAction,
}
```

**Configuration**:
```toml
[monitoring.network]
enabled = true
track_domains = true
track_categories = true
log_blocked_only = false  # Or only log blocked attempts
anonymize_domains = false # Replace with category only
max_domains_per_day = 1000
```

**Privacy Considerations**:
- Only domain logged, not full URLs
- Query parameters never logged
- Option to log categories only
- Optional complete disablement

### Screen Time Calculation

**Purpose**: Accurate screen time tracking

**Calculation**:
```rust
pub struct ScreenTime {
    date: NaiveDate,
    profile: String,
    total_seconds: u64,
    active_seconds: u64,    // Actually using computer
    idle_seconds: u64,      // Computer on but idle
    by_category: HashMap<String, u64>,
    by_application: HashMap<String, u64>,
}
```

**Idle Detection**:
```toml
[monitoring.screen_time]
idle_threshold_seconds = 300  # 5 minutes no input = idle
count_idle_as_screen_time = false
pause_tracking_when_locked = true
```

**Implementation**:
- Monitor keyboard/mouse input
- Detect screen lock
- Handle sleep/suspend correctly
- Account for time zone changes

### Event Logging

**Purpose**: Log significant events for audit and reports

**Event Types**:
```rust
pub enum MonitoringEvent {
    // Application events
    ApplicationLaunched { app_id: String, timestamp: DateTime<Utc> },
    ApplicationBlocked { app_id: String, reason: String },
    ApplicationClosed { app_id: String, duration: Duration },

    // Time limit events
    TimeLimitWarning { minutes_remaining: u32 },
    TimeLimitReached,
    TimeLimitExtended { minutes_added: u32, reason: String },

    // Filter events
    WebsiteBlocked { domain: String, category: ContentCategory },
    CommandBlocked { command: String, reason: String },

    // Policy events
    PolicyViolation { policy: String, details: String },
    ParentOverride { reason: String, duration: Duration },

    // System events
    MonitoringStarted,
    MonitoringStopped { reason: String },
    ProfileSwitched { from: String, to: String },
}
```

**Configuration**:
```toml
[monitoring.events]
enabled = true
log_level = "info"  # "debug", "info", "warn", "error"

# What to log
log_app_launches = true
log_app_blocks = true
log_time_warnings = true
log_filter_blocks = true
log_policy_violations = true
log_parent_overrides = true

# Privacy
redact_sensitive_info = true
```

## Data Storage

### Database Schema

**Sessions Table**:
```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    duration_seconds INTEGER,
    screen_time_seconds INTEGER,
    idle_time_seconds INTEGER,
    ended_reason TEXT  -- logout, time_limit, bedtime, crash
);

CREATE INDEX idx_sessions_profile_date ON sessions(profile, start_time);
```

**Activities Table**:
```sql
CREATE TABLE activities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    profile TEXT NOT NULL,
    app_id TEXT NOT NULL,
    app_name TEXT NOT NULL,
    category TEXT,
    window_title TEXT,
    duration_seconds INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_activities_profile_date ON activities(profile, timestamp);
CREATE INDEX idx_activities_app ON activities(app_id, timestamp);
```

**Network Activity Table**:
```sql
CREATE TABLE network_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL,
    profile TEXT NOT NULL,
    domain TEXT NOT NULL,
    category TEXT,
    duration_seconds INTEGER,
    blocked BOOLEAN NOT NULL,
    action TEXT NOT NULL
);

CREATE INDEX idx_network_profile_date ON network_activity(profile, timestamp);
```

**Events Table**:
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    profile TEXT NOT NULL,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,  -- info, warning, error
    details TEXT,  -- JSON
    metadata TEXT  -- JSON
);

CREATE INDEX idx_events_profile_date ON events(profile, timestamp);
CREATE INDEX idx_events_type ON events(event_type, timestamp);
```

**Daily Summaries Table** (pre-aggregated for performance):
```sql
CREATE TABLE daily_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date DATE NOT NULL,
    profile TEXT NOT NULL,
    screen_time_seconds INTEGER NOT NULL,
    active_time_seconds INTEGER NOT NULL,
    idle_time_seconds INTEGER NOT NULL,
    app_launches INTEGER NOT NULL,
    websites_visited INTEGER NOT NULL,
    blocks_count INTEGER NOT NULL,
    top_apps TEXT,  -- JSON array
    top_categories TEXT,  -- JSON array
    summary_generated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(date, profile)
);

CREATE INDEX idx_summaries_profile ON daily_summaries(profile, date DESC);
```

### Data Retention

**Configuration**:
```toml
[monitoring.retention]
enabled = true

# Detailed activity data
activities_days = 30
network_activity_days = 30
events_days = 90

# Aggregated data
daily_summaries_days = 365
weekly_summaries_days = 730  # 2 years

# Archive before deletion
archive_before_delete = true
archive_path = "~/.local/share/dots-family/archives"
archive_format = "sqlite"  # or "json", "csv"
```

**Retention Process**:
1. Nightly job checks retention policies
2. Identify expired data
3. Optionally archive to separate file
4. Delete from main database
5. VACUUM database to reclaim space
6. Log retention actions

### Data Export

**Purpose**: Allow parents to export data for external analysis

**Export Formats**:
```bash
# Export to JSON
dots-family-ctl export --profile alex --format json \
  --start 2024-01-01 --end 2024-03-31 \
  --output alex-q1-2024.json

# Export to CSV
dots-family-ctl export --profile alex --format csv \
  --type activities \
  --output activities.csv

# Export to PDF report
dots-family-ctl export --profile alex --format pdf \
  --type weekly-report \
  --output report.pdf
```

**Export Configuration**:
```toml
[monitoring.export]
enabled = true
max_export_days = 365
redact_sensitive_data = true  # Remove window titles, exact URLs
include_charts = true
```

## Reporting System

### Daily Summary

**Content**:
- Total screen time
- Top 5 applications used
- Top 5 websites visited
- Number of policy violations
- Time limit status
- Notable events

**Configuration**:
```toml
[reports.daily]
enabled = true
generation_time = "20:00"  # Generate at 8 PM
delivery_method = "notification"  # or "email"
```

**Example Output**:
```
Daily Summary - Alex - March 15, 2024
=====================================

Screen Time: 1h 45m / 2h 00m (88%)
  Active: 1h 30m
  Idle: 15m

Top Applications:
  1. Firefox (45m) - Web browsing
  2. Inkscape (30m) - Graphics
  3. VS Code (20m) - Development
  4. Tuxmath (10m) - Education

Top Websites:
  1. wikipedia.org (15m)
  2. khanacademy.org (12m)
  3. github.com (8m)

Policy Status:
  ✅ No violations today
  ✅ Time limit respected
  ⚠️  1 blocked website attempt

Notable Events:
  - Extended screen time by 30m for homework
  - Approved Discord access for 1 hour
```

### Weekly Report

**Content**:
- Week-over-week comparison
- Daily screen time trends
- Most used applications
- Category breakdown
- Policy compliance
- Recommendations

**Configuration**:
```toml
[reports.weekly]
enabled = true
generation_day = "sunday"
generation_time = "18:00"
delivery_method = "email"
include_charts = true
```

**Example Sections**:

**Screen Time Trend**:
```
Weekly Screen Time - March 11-17, 2024
======================================

Mon: ████████████░░░░ 1h 45m
Tue: ████████████████ 2h 00m (limit)
Wed: ██████████░░░░░░ 1h 30m
Thu: ████████████████ 2h 00m (limit)
Fri: ████████████░░░░ 1h 45m
Sat: ████████████████████░░░░ 3h 00m (weekend bonus)
Sun: ██████████████████████░░ 2h 45m

Total: 14h 45m
Daily Average: 2h 06m
Compared to last week: -15m (-11%)
```

**Category Breakdown**:
```
Time by Category
================

Education      ████████░░ 4h 15m (29%)
Web Browsing   ███████░░░ 3h 30m (24%)
Graphics       ██████░░░░ 2h 45m (19%)
Development    ████░░░░░░ 2h 00m (14%)
Games          ███░░░░░░░ 1h 30m (10%)
Other          █░░░░░░░░░ 0h 45m (5%)
```

**Recommendations**:
```
Recommendations
===============

✅ Excellent balance between educational and recreational activities
✅ Consistent adherence to time limits
✅ Good variety of application categories

💡 Suggestions:
  - Consider allowing social media access for 30m on weekends
  - Alex is interested in game development (6 searches this week)
  - Ready for terminal access based on computer skills progression
```

### Custom Reports

**Purpose**: Generate reports for specific needs

**Examples**:
```bash
# Application usage report
dots-family-ctl report --type app-usage \
  --profile alex \
  --start 2024-01-01 \
  --end 2024-03-31 \
  --group-by category

# Time limit compliance
dots-family-ctl report --type time-compliance \
  --profile alex \
  --period month

# Filter effectiveness
dots-family-ctl report --type filter-stats \
  --profile alex \
  --period week
```

**Report Configuration**:
```toml
[reports.custom]
enabled = true

[[reports.custom.templates]]
name = "monthly-overview"
schedule = "monthly"
sections = [
  "screen-time-trend",
  "top-applications",
  "category-breakdown",
  "policy-compliance",
  "filter-effectiveness",
  "recommendations"
]
```

## Alerts and Notifications

### Alert Types

**Time-Based Alerts**:
```toml
[alerts.time]
enabled = true

# Warn parent when child approaches limit
approach_limit_threshold = 15  # minutes before limit
notify_parent = true

# Alert when limit exceeded (shouldn't happen)
limit_exceeded_alert = true

# Alert on unusual usage patterns
unusual_usage_threshold = 2.0  # 2x normal usage
```

**Violation Alerts**:
```toml
[alerts.violations]
enabled = true

# Alert severity levels
immediate = [
  "blocked-dangerous-command",
  "attempted-policy-modification",
  "suspicious-activity"
]

hourly_digest = [
  "blocked-website",
  "blocked-application"
]

daily_digest = [
  "time-limit-warning",
  "safe-search-enforcement"
]
```

**Behavioral Alerts**:
```toml
[alerts.behavioral]
enabled = true

# Alert on concerning patterns
patterns = [
  { type = "excessive-block-attempts", threshold = 10, period = "hour" },
  { type = "unusual-time-of-day", description = "Using computer at 2 AM" },
  { type = "rapid-app-switching", threshold = 20, period = "minute" },
  { type = "repeated-override-requests", threshold = 5, period = "day" }
]

# Machine learning-based anomaly detection (optional)
ml_anomaly_detection = false
```

### Notification Delivery

**Desktop Notifications**:
```toml
[notifications.desktop]
enabled = true
urgency = "normal"  # low, normal, critical
timeout_ms = 5000
action_buttons = true  # Allow quick actions from notification
```

**Email Notifications**:
```toml
[notifications.email]
enabled = true
to = "parent@example.com"
from = "family-mode@localhost"
subject_prefix = "[Family Mode]"

# Aggregate to reduce noise
aggregate_minutes = 15
max_per_hour = 4
```

**Mobile Notifications** (future):
```toml
[notifications.mobile]
enabled = false
service = "ntfy"  # or "pushover", "gotify"
topic = "dots-family-alerts"
```

## Dashboard

### Real-Time View

**Features**:
- Current active window/application
- Screen time remaining today
- Recent activity timeline
- Active alerts
- Quick actions (grant time, approve request)

**Implementation** (GTK4):
```rust
pub struct RealtimeDashboard {
    current_activity: Label,
    time_remaining: ProgressBar,
    recent_activities: ListBox,
    alerts: ListBox,
    quick_actions: Box,
}

impl RealtimeDashboard {
    pub fn update(&self, state: &MonitoringState) {
        // Update current activity
        self.current_activity.set_text(&format!(
            "{} - {}",
            state.current_app_name,
            state.current_window_title
        ));

        // Update time remaining
        let fraction = state.time_used as f64 / state.time_limit as f64;
        self.time_remaining.set_fraction(fraction);

        // Update recent activities
        self.update_recent_activities(&state.recent);

        // Update alerts
        self.update_alerts(&state.alerts);
    }
}
```

### Historical View

**Features**:
- Date range selector
- Screen time chart
- Application breakdown
- Category analysis
- Export options

**Chart Types**:
- Line chart: Screen time over time
- Bar chart: Daily application usage
- Pie chart: Category breakdown
- Heatmap: Time of day usage patterns

### Insights View

**Features**:
- Usage patterns analysis
- Compliance trends
- Behavioral insights
- Comparison with age group averages (anonymized)
- Progression recommendations

**Example Insights**:
```
Insights for Alex - March 2024
==============================

📊 Usage Patterns:
  - Most active time: 16:00-18:00 weekdays
  - Preferred activities: Educational content (45%), Creative (30%), Gaming (25%)
  - Average session length: 42 minutes

📈 Trends:
  - Screen time decreased 12% from last month (positive)
  - Educational app usage increased 28% (positive)
  - Gaming time consistent at ~1.5h/week

🎯 Compliance:
  - Time limit adherence: 96% (28/29 days)
  - Policy violations: 3 (all minor, web filters)
  - Override requests: 2 (both approved)

💡 Recommendations:
  - Consider expanding allowed application categories
  - Alex ready for more autonomy in 3 months based on compliance
  - Strong interest in coding - recommend development tools access
```

## Privacy and Transparency

### Child Access to Reports

**Configuration**:
```toml
[monitoring.child_access]
enabled = true

# What child can see about their own activity
can_view_screen_time = true
can_view_top_applications = true
can_view_daily_summary = true
can_view_detailed_logs = false  # Window titles, exact times
can_export_data = false
```

**Purpose**:
- Teach self-awareness
- Encourage healthy habits
- Build trust through transparency

### Privacy Controls

**Configuration**:
```toml
[monitoring.privacy]
# Exclude specific apps from tracking
excluded_apps = [
  "org.keepassxc.KeePassXC",  # Password manager
  "therapy-app",              # Mental health apps
]

# Exclude specific time periods (therapy, private time)
excluded_times = [
  { day = "tuesday", start = "16:00", end = "17:00", reason = "therapy" },
]

# Automatic pause triggers
auto_pause_triggers = [
  "therapy-app-launched",
  "parent-present-mode",  # Parent and child using together
]
```

### Audit Logging

**Purpose**: Track who accessed monitoring data

**Log Entries**:
```rust
pub struct AuditLogEntry {
    timestamp: DateTime<Utc>,
    user: String,  // "parent" or "child"
    action: String,  // "viewed-report", "exported-data", "modified-policy"
    resource: String,  // What was accessed
    ip_address: Option<String>,
    success: bool,
}
```

**Configuration**:
```toml
[monitoring.audit]
enabled = true
log_parent_access = true
log_child_access = true
retention_days = 365
immutable = true  # Cannot be deleted, only expired
```

## Performance Optimization

### Efficient Data Collection

**Strategies**:
- Aggregate in-memory before database writes
- Batch database inserts every 5 minutes
- Use prepared statements
- Index frequently queried fields

**Memory Management**:
```toml
[monitoring.performance]
max_memory_mb = 50
buffer_size = 1000  # Activities buffered before write
write_interval_seconds = 300
```

### Report Generation

**Optimization**:
- Pre-aggregate daily summaries
- Cache common queries
- Generate reports asynchronously
- Use database views for complex queries

**Background Processing**:
```rust
pub async fn generate_reports_background() {
    // Run at low priority
    tokio::task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3600));

        loop {
            interval.tick().await;

            // Generate pending reports
            generate_pending_summaries().await;
            aggregate_daily_data().await;
            cleanup_old_data().await;
        }
    });
}
```

## Testing Strategy

### Unit Tests
- Activity aggregation logic
- Time calculations
- Report generation
- Data retention policies

### Integration Tests
- End-to-end activity tracking
- Database operations
- Report delivery
- Performance benchmarks

### Manual Testing
- Real-world usage scenarios
- Report accuracy verification
- Dashboard responsiveness
- Privacy compliance

## Related Documentation

- PARENTAL_CONTROLS.md: Time limits and restrictions
- CONTENT_FILTERING.md: What gets blocked/logged
- DATA_SCHEMA.md: Database schema details
- RUST_APPLICATIONS.md: Monitor application specs
