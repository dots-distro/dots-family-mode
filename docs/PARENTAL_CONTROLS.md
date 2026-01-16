# Parental Controls Specification

## Overview

Parental controls in Family Mode provide parents with tools to manage their children's computer usage through time limits, application restrictions, and progressive access policies. The system balances safety with autonomy, adapting to children's developmental stages.

## Control Categories

### 1. Time Management

#### Daily Screen Time Limits

**Purpose**: Enforce healthy screen time habits

**Configuration**:
```toml
[screen_time]
enabled = true
daily_limit_minutes = 120
weekend_bonus_minutes = 60
rollover_unused = false
```

**Behavior**:
- Countdown begins when first application launches
- Time tracked per child profile across all applications
- Warnings at 30, 15, 5 minutes remaining
- At limit: All applications closed gracefully, desktop locked
- Time resets at midnight (configurable timezone)

**Advanced Options**:
```toml
[screen_time.advanced]
# Don't count educational apps toward limit
exempt_categories = ["education", "development"]
exempt_applications = ["anki", "libreoffice"]

# Pause time during specific activities
pause_during = ["homework-mode"]

# Bonus time earned for goals
bonus_time_per_book_read = 30
bonus_time_per_chore = 15
max_bonus_per_day = 60
```

#### Time Windows

**Purpose**: Restrict computer use to specific times of day

**Configuration**:
```toml
[screen_time.windows]
enabled = true

# Weekday windows
weekday = [
  { start = "06:00", end = "08:00", label = "morning" },
  { start = "15:00", end = "19:00", label = "evening" }
]

# Weekend windows
weekend = [
  { start = "08:00", end = "21:00", label = "daytime" }
]

# Holiday windows (uses calendar)
holiday = [
  { start = "08:00", end = "21:00", label = "holiday" }
]
```

**Behavior**:
- Outside windows: Cannot login or existing session locked
- 5-minute warning before window closes
- Grace period for saving work (configurable)
- Manual override available to parent

#### Bedtime Mode

**Purpose**: Enforce healthy sleep schedules

**Configuration**:
```toml
[screen_time.bedtime]
enabled = true
school_night = "21:00"
weekend_night = "22:00"
wake_time = "06:00"
enforce_logout = true
```

**Behavior**:
- 30-minute warning before bedtime
- At bedtime: Forced logout
- Login disabled until wake time
- Blue light filter activated 2 hours before bedtime (optional)

### 2. Application Control

#### Allow/Block Lists

**Configuration**:
```toml
[applications]
mode = "allowlist" # or "blocklist"

# Explicit application IDs
allowed = [
  "org.mozilla.firefox",
  "org.inkscape.Inkscape",
  "org.libreoffice.LibreOffice.Writer"
]

blocked = [
  "com.valvesoftware.Steam",
  "com.discordapp.Discord"
]

# Pattern matching
allowed_patterns = [
  "org.gnome.*",  # All GNOME apps
  "*.education.*" # Educational apps
]

blocked_patterns = [
  "*.game.*"
]
```

**Behavior**:
- Allowlist mode: Only explicitly allowed apps can run
- Blocklist mode: All apps except blocked can run
- Attempted launch of blocked app: Friendly denial message
- Unknown apps in allowlist mode: Parent approval required

#### Category-Based Filtering

**Purpose**: Control app classes without listing every app

**Configuration**:
```toml
[applications.categories]
allowed = [
  "education",
  "productivity",
  "graphics",
  "music"
]

blocked = [
  "games",
  "social-media",
  "video-streaming",
  "chat"
]

# Override category for specific apps
overrides = [
  { app = "org.tuxpaint.Tuxpaint", category = "education" } # game but educational
]
```

**Categories** (XDG Desktop standard plus custom):
- `education`: Educational software
- `development`: Programming tools
- `office`: Office productivity
- `graphics`: Image editing, drawing
- `audio`: Music creation, editing
- `video`: Video editing (not streaming)
- `games`: Gaming software
- `social-media`: Social networking apps
- `chat`: Instant messaging
- `video-streaming`: YouTube, Netflix, etc.
- `web-browser`: Web browsers
- `email`: Email clients

**Category Detection**:
1. Check desktop file `Categories` field
2. Apply manual overrides
3. Heuristic analysis of app name/description
4. Parent classification for unknown apps

#### Time-Based Application Access

**Configuration**:
```toml
[applications.schedules]
# Gaming only on weekends
"steam" = { weekday = [], weekend = ["08:00-12:00", "14:00-18:00"] }

# Social media for older kids, limited hours
"discord" = { weekday = ["17:00-18:00"], weekend = ["10:00-12:00", "14:00-16:00"] }

# Educational apps always available
"anki" = { weekday = ["any"], weekend = ["any"] }
```

**Behavior**:
- App launches outside schedule: Denied with next available time shown
- Running app when schedule ends: 5-minute warning, then graceful close
- Schedule overrides daily time limits for exempt apps

#### Launch Approval

**Purpose**: Allow flexibility while maintaining oversight

**Configuration**:
```toml
[applications.approval]
enabled = true
mode = "ask-parent" # or "temporary-allow" or "auto-approve"

# Auto-approve educational apps
auto_approve_categories = ["education"]

# Request valid for 24 hours
approval_duration_hours = 24

# Notification method
notify_via = ["desktop", "email", "mobile"] # mobile requires setup
```

**Workflow**:
1. Child attempts to launch blocked/unknown app
2. Dialog explains app is restricted
3. "Request Permission" button shown
4. Parent receives notification
5. Parent approves/denies with optional temporary grant
6. Child notified of decision

### 3. Age-Based Profiles

#### Profile Templates

**Ages 5-7 (Early Elementary)**:
```toml
[profile]
name = "Early Elementary (5-7)"
description = "Heavily supervised environment for young children"

[screen_time]
daily_limit_minutes = 60
exempt_categories = []
windows.weekday = [
  { start = "16:00", end = "17:00" }
]
windows.weekend = [
  { start = "09:00", end = "10:00" },
  { start = "14:00", end = "15:00" }
]

[applications]
mode = "allowlist"
allowed_categories = ["education"]
allowed = [
  "tuxpaint",
  "gcompris",
  "tuxmath"
]

[web_filtering]
enabled = false # No web access at this age
allowed_sites = [] # Or parent-selected educational sites only

[terminal]
enabled = false
```

**Ages 8-12 (Late Elementary/Middle School)**:
```toml
[profile]
name = "Middle Years (8-12)"
description = "Guided independence with content filtering"

[screen_time]
daily_limit_minutes = 120
weekend_bonus_minutes = 60
exempt_categories = ["education"]

[applications]
mode = "allowlist"
allowed_categories = [
  "education",
  "productivity",
  "graphics",
  "music",
  "games-educational"
]

[web_filtering]
enabled = true
mode = "strict"
safe_search_enforced = true
blocked_categories = [
  "adult",
  "violence",
  "gambling",
  "social-media",
  "chat"
]

[terminal]
enabled = false
```

**Ages 13-17 (High School)**:
```toml
[profile]
name = "Teen (13-17)"
description = "Increasing autonomy with boundary enforcement"

[screen_time]
daily_limit_minutes = 180
weekend_bonus_minutes = 120
exempt_categories = ["education", "development"]

[applications]
mode = "blocklist"
blocked_categories = [
  "gambling",
  "adult"
]
allowed_categories = ["*"] # Everything else allowed

[web_filtering]
enabled = true
mode = "moderate"
safe_search_enforced = true
blocked_categories = [
  "adult",
  "violence",
  "gambling"
]
# Social media allowed but monitored

[terminal]
enabled = true
mode = "filtered"
blocked_commands = [
  "rm -rf /",
  "sudo rm",
  ":(){ :|:& };:", # fork bomb
]
educational_warnings = true
```

#### Progressive Access

**Purpose**: Automatically expand access as child matures

**Configuration**:
```toml
[profile.progression]
enabled = true
birthday = "2015-03-15"

# Automatically adjust restrictions based on age
auto_adjust = true
review_interval_months = 6

# Milestones
milestones = [
  { age = 13, action = "enable-terminal", note = "Terminal access granted" },
  { age = 14, action = "enable-social-media", note = "Social media access granted" },
  { age = 15, action = "extend-screen-time", amount = 60, note = "Additional hour of screen time" },
  { age = 16, action = "switch-to-blocklist", note = "Switched to blocklist mode" },
  { age = 18, action = "disable-family-mode", note = "Family mode disabled" }
]
```

**Behavior**:
- On birthday: Parent notification of upcoming changes
- Review period: Parent confirms or modifies progression
- Manual override: Parent can accelerate or delay milestones
- Activity report included with progression recommendation

### 4. Exceptions and Overrides

#### Temporary Exceptions

**Purpose**: Allow flexibility for special circumstances

**Configuration via CLI**:
```bash
# Grant extra time for school project
dots-family-ctl exception grant \
  --profile alex \
  --type extra-time \
  --amount 120 \
  --reason "Science fair project" \
  --expires "2024-03-15 23:59"

# Allow blocked application temporarily
dots-family-ctl exception grant \
  --profile alex \
  --type allow-app \
  --app steam \
  --duration 4h \
  --reason "Friend's birthday LAN party"

# Suspend monitoring for privacy
dots-family-ctl exception grant \
  --profile alex \
  --type suspend-monitoring \
  --duration 1h \
  --reason "Therapy session" \
  --scope "all" # or "web-only", "applications-only"
```

**GUI Workflow**:
1. Child requests exception via notification action
2. Parent receives request with reason
3. Parent approves with optional modifications
4. Exception applied immediately
5. Automatic expiration
6. Exception logged in activity report

#### Emergency Override

**Purpose**: Bypass all restrictions immediately

**Activation**:
- Physical button on parent's device (future)
- CLI: `dots-family-ctl override enable --duration 24h --reason "Emergency"`
- GUI: Emergency button with password confirmation

**Behavior**:
- All restrictions immediately lifted
- Child notified that override is active
- Override logged in audit trail
- Automatic expiration or manual deactivation

#### Homework Mode

**Purpose**: Optimize environment for focused work

**Configuration**:
```toml
[modes.homework]
enabled = true

# Suspend time limits
pause_screen_time = true

# Restrict to productivity apps
allowed_categories = [
  "education",
  "productivity",
  "office",
  "development"
]

# Allow specific websites
web_filtering.mode = "allowlist"
web_filtering.allowed_domains = [
  "wikipedia.org",
  "khanacademy.org",
  "school-lms.edu"
]

# Disable notifications
suppress_notifications = true
```

**Activation**:
- Child: "Homework Mode" button in system tray
- Parent: Schedule homework mode automatically
- Auto-detect: When homework apps launched (optional)

### 5. Multi-Child Management

#### Profile Switching

**Purpose**: Support multiple children on one system

**Configuration**:
```toml
[general]
multi_user_mode = true
fast_user_switching = true

[profiles]
active = "auto" # or specific profile name

# Profiles
[profiles.alex]
age_group = "8-12"
# ... profile settings ...

[profiles.jordan]
age_group = "13-17"
# ... profile settings ...
```

**User Association**:
```toml
# Map system users to profiles
[user_mapping]
"alex" = "alex"
"jordan" = "jordan"
"parent" = null # No family mode restrictions
```

**Behavior**:
- Profile auto-detected from system user
- Profile switching requires parent password
- Separate activity tracking per profile
- Shared restrictions can be configured (family screen time pool)

#### Shared Time Pool

**Configuration**:
```toml
[screen_time.shared]
enabled = true
pool_type = "family" # or "device"
daily_pool_minutes = 240 # Total for all children combined

# Distribute pool
distribution = "fair" # or "proportional" or "first-come"

# Individual maximums
[profiles.alex.screen_time]
max_from_pool = 120 # Can't use more than 2h even if pool available

[profiles.jordan.screen_time]
max_from_pool = 180
```

**Use Cases**:
- Limit total screen time across all children
- Encourage negotiation and time sharing
- Prevent one child monopolizing device

### 6. Parent Authentication

#### Password Authentication

**Setup**:
```bash
# Initial setup
dots-family-ctl auth setup-password

# Change password
dots-family-ctl auth change-password
```

**Configuration**:
```toml
[authentication]
method = "password"
complexity_required = true
min_length = 12
session_timeout_minutes = 15
max_attempts = 3
lockout_duration_minutes = 30
```

**Security**:
- Argon2id hashing with secure parameters
- Salt unique per installation
- Rate limiting on authentication attempts
- Session tokens with automatic expiration

#### Multi-Factor Authentication (Optional)

**Configuration**:
```toml
[authentication.mfa]
enabled = true
method = "totp" # or "webauthn"
required_for = [
  "disable-family-mode",
  "delete-profile",
  "view-activity-details"
]
```

**TOTP Setup**:
```bash
dots-family-ctl auth setup-mfa --method totp
# Displays QR code for authenticator app
```

**WebAuthn Setup**:
```bash
dots-family-ctl auth setup-mfa --method webauthn
# Prompts for security key registration
```

#### Parent PIN (Quick Access)

**Purpose**: Fast authentication for common tasks

**Configuration**:
```toml
[authentication.pin]
enabled = true
length = 6
allowed_operations = [
  "grant-extra-time",
  "approve-app-request",
  "temporary-exception"
]
expires_minutes = 5
```

**Usage**:
- PIN for low-risk operations (extra time, app approval)
- Full password for high-risk (disable monitoring, delete logs)
- Parent chooses which operations require PIN vs password

### 7. Notification and Communication

#### Parent Notifications

**Configuration**:
```toml
[notifications.parent]
enabled = true
channels = ["desktop", "email"]

# What to notify about
events = [
  "app-request",
  "time-limit-reached",
  "policy-violation",
  "blocked-website",
  "suspicious-activity"
]

# Notification frequency
immediate = ["app-request", "suspicious-activity"]
digest_hourly = ["blocked-website"]
digest_daily = ["time-limit-reached"]

[notifications.parent.email]
address = "parent@example.com"
smtp_server = "smtp.gmail.com"
smtp_port = 587
smtp_username = "user@gmail.com"
smtp_password_command = "pass show email/smtp" # Use password manager
```

#### Child Notifications

**Configuration**:
```toml
[notifications.child]
enabled = true

# Time limit warnings
time_warnings = [30, 15, 5] # minutes before limit

# Friendly reminders
friendly_messages = true
tone = "encouraging" # or "neutral" or "firm"

# Example messages
messages = [
  "You have {minutes} minutes left today. Great job managing your time!",
  "Getting close to your limit. Time to wrap up!",
  "Time to take a break. See you tomorrow!"
]
```

#### Activity Reports

**Configuration**:
```toml
[reports]
enabled = true
frequency = "weekly" # or "daily", "monthly"
delivery_day = "sunday"
delivery_time = "18:00"
include_charts = true

[reports.content]
include = [
  "screen-time-summary",
  "top-applications",
  "websites-visited",
  "time-of-day-usage",
  "policy-violations",
  "progression-recommendations"
]

exclude_sensitive = true # Don't include specific URLs, etc.
```

### 8. Policy Testing

#### Dry Run Mode

**Purpose**: Test policies before enforcement

**Configuration**:
```toml
[general]
dry_run_mode = true
dry_run_profile = "alex"
dry_run_duration_days = 7
```

**Behavior**:
- Policies evaluated but not enforced
- Violations logged but not blocked
- Report generated showing what would have been blocked
- Parent can adjust policies before enabling

#### Simulation Mode

**Purpose**: Preview impact of policy changes

**CLI**:
```bash
# Simulate policy change
dots-family-ctl simulate \
  --profile alex \
  --change "applications.mode=blocklist" \
  --using-history-days 30

# Output: Shows what would have been blocked in past 30 days
```

**Output**:
```
Policy Simulation Results
=========================
Profile: alex
Simulation Period: 2024-02-15 to 2024-03-15
Change: applications.mode = blocklist (from allowlist)

Impact Summary:
- Applications that would have been blocked: 3
  * Steam (used 15 times, 12.5 hours)
  * Discord (used 45 times, 8.2 hours)
  * Minecraft (used 8 times, 6.3 hours)

- Activities that would have been allowed: 127
  * All previously allowed apps remain allowed

Recommendation: Consider allowing Steam on weekends only instead
of complete block to avoid dramatic change.
```

## Implementation Notes

### Performance Optimization

- Policy evaluation cached for 1 minute
- Database queries use prepared statements
- Activity aggregation done in background
- Notifications rate-limited to prevent spam

### User Experience

- Denial messages should be friendly and educational
- Warnings should be clear and actionable
- Parent interface should be efficient (common tasks quick)
- Child interface should be age-appropriate

### Accessibility

- All notifications available via screen reader
- High contrast mode for visibility
- Keyboard navigation for all features
- Font scaling support

### Privacy

- Activity logs configurable retention period
- Logs can be encrypted
- Parent can delete specific log entries
- Child can request log review with parent

## Related Documentation

- MONITORING.md: Activity tracking and reporting details
- CONTENT_FILTERING.md: Web and content filtering
- DATA_SCHEMA.md: Database schema for policies
