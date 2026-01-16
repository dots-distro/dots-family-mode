# Family Mode Architecture

## System Architecture Overview

Family Mode uses a distributed architecture with multiple Rust applications communicating via DBus. This design ensures:
- Separation of concerns for security
- Independent component updates
- Graceful degradation on failure
- Cross-desktop compatibility

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Space                               │
│                                                                   │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │   Terminal   │     │ Applications │     │ Web Browser  │    │
│  │  (Ghostty)   │     │              │     │  (Firefox)   │    │
│  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘    │
│         │                    │                    │              │
│         │ PTY                │ Process            │ Content      │
│         │ Monitor            │ Monitor            │ Filter       │
│         │                    │                    │              │
│  ┌──────▼──────────────────▼────────────────────▼──────────┐   │
│  │         dots-terminal-filter (Rust)                      │   │
│  │         - Command validation                              │   │
│  │         - Shell integration                               │   │
│  └──────────────────────────┬───────────────────────────────┘   │
│                              │                                    │
│  ┌──────────────────────────▼───────────────────────────────┐   │
│  │         dots-family-monitor (Rust)                        │   │
│  │         - Window tracking                                 │   │
│  │         - Application lifecycle                           │   │
│  │         - Screen time calculation                         │   │
│  └──────────────────────────┬───────────────────────────────┘   │
│                              │                                    │
│                              │ DBus                               │
│                              │                                    │
│  ┌──────────────────────────▼───────────────────────────────┐   │
│  │         dots-family-daemon (Rust)                         │   │
│  │         - Policy engine                                   │   │
│  │         - Enforcement coordinator                         │   │
│  │         - Database management                             │   │
│  │         - Parent authentication                           │   │
│  └──────┬───────────────────┬────────────────────────────────┘  │
│         │                   │                                    │
│    ┌────▼────┐         ┌───▼─────┐                              │
│    │ SQLite  │         │ Config  │                              │
│    │   DB    │         │ Files   │                              │
│    └─────────┘         └─────────┘                              │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │         dots-wm-bridge (Rust)                              │ │
│  │         - WM-specific adapters                             │ │
│  │         - Niri/Swayfx/Hyprland integration                 │ │
│  └──────────────────────────────────────────────────────────┬─┘ │
│                                                               │   │
└───────────────────────────────────────────────────────────────┼──┘
                                                                │
┌───────────────────────────────────────────────────────────────▼──┐
│                    Window Manager Layer                          │
│                                                                   │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │     Niri     │     │   Swayfx     │     │  Hyprland    │    │
│  │    (Rust)    │     │   (C++)      │     │    (C++)     │    │
│  └──────────────┘     └──────────────┘     └──────────────┘    │
└───────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. dots-family-daemon

**Purpose**: Central coordination and policy enforcement service

**Responsibilities**:
- Load and validate configuration
- Maintain active policy state
- Coordinate between monitoring and filtering components
- Handle parent authentication
- Manage database operations
- Enforce time-based restrictions
- Generate alerts and reports

**Interfaces**:
- DBus service: `org.dots.FamilyDaemon`
- Config: `/etc/dots-family/daemon.toml` (system), `~/.config/dots-family/daemon.toml` (user)
- Database: `~/.local/share/dots-family/family.db`

**Security**:
- Runs as user systemd service
- Config files encrypted with parent password
- Database encrypted at rest
- Capability-based access control

**Dependencies**:
- `tokio` for async runtime
- `zbus` for DBus communication
- `sqlx` for database access
- `ring` for cryptography
- `serde` for configuration

### 2. dots-family-monitor

**Purpose**: Track application usage and window activity

**Responsibilities**:
- Monitor active windows and applications
- Calculate screen time per application
- Detect policy violations (blocked apps, time limits)
- Report activity to daemon
- Integrate with window managers

**Interfaces**:
- DBus service: `org.dots.FamilyMonitor`
- Communicates with: `dots-family-daemon`, `dots-wm-bridge`

**Implementation Details**:
- Poll window manager state every 1 second
- Aggregate activity into 5-minute buckets
- Efficient state diffing to minimize overhead
- Graceful handling of WM disconnections

**Dependencies**:
- `tokio` for async runtime
- `zbus` for DBus
- Window manager protocol libraries

### 3. dots-family-filter

**Purpose**: Content filtering engine

**Responsibilities**:
- Filter web content via proxy
- Inspect application network traffic
- Block based on URL, domain, category
- Update filter lists
- Log blocked attempts

**Interfaces**:
- HTTP proxy on localhost:8118 (configurable)
- DBus service: `org.dots.FamilyFilter`
- Communicates with: `dots-family-daemon`

**Implementation Details**:
- Transparent HTTP/HTTPS proxy
- Category-based filtering via filter lists
- Custom rules support
- Safe search enforcement
- Certificate pinning for HTTPS inspection (opt-in)

**Dependencies**:
- `hyper` for HTTP proxy
- `rustls` for TLS
- `url` for URL parsing
- Filter list format compatible with AdBlock Plus

### 4. dots-terminal-filter

**Purpose**: Terminal command filtering and validation

**Responsibilities**:
- Intercept shell commands before execution
- Validate against allowed/blocked patterns
- Provide educational feedback on blocked commands
- Log terminal activity
- Support multiple shells (bash, zsh, fish)

**Interfaces**:
- Shell plugin/wrapper
- DBus service: `org.dots.TerminalFilter`
- Communicates with: `dots-family-daemon`

**Implementation Details**:
- PTY-level interception for universal compatibility
- Command parsing and classification
- Pattern matching for dangerous commands
- Allow/block/warn modes
- Educational prompts for learning

**Dependencies**:
- `nix` for PTY handling
- `shellwords` for command parsing
- `regex` for pattern matching

### 5. dots-wm-bridge

**Purpose**: Window manager integration abstraction

**Responsibilities**:
- Provide unified interface to different WMs
- Translate WM-specific events to common format
- Support window manipulation (close, focus, etc.)
- Handle WM-specific quirks

**Interfaces**:
- DBus service: `org.dots.WMBridge`
- WM-specific IPC/sockets

**Implementation Details**:
- Trait-based adapter pattern
- Runtime WM detection
- Fallback to generic Wayland when WM unsupported
- Event stream transformation

**Supported WMs**:
- **Niri**: Native Rust IPC via Unix socket
- **Swayfx**: IPC via sway socket (JSON messages)
- **Hyprland**: Unix socket with Hyprland protocol

**Dependencies**:
- `tokio` for async
- `serde_json` for IPC messages
- WM-specific protocol crates

### 6. dots-family-ctl

**Purpose**: Command-line administration tool

**Responsibilities**:
- Configure policies
- View reports and statistics
- Manage profiles
- Grant temporary exceptions
- Test filters and rules

**Interfaces**:
- CLI application
- Communicates with: `dots-family-daemon` via DBus

**Implementation Details**:
- Rich terminal UI with `ratatui`
- Interactive and non-interactive modes
- JSON output for scripting
- Parent authentication required

**Dependencies**:
- `clap` for CLI parsing
- `ratatui` for TUI
- `zbus` for DBus

### 7. dots-family-gui

**Purpose**: Graphical parent dashboard

**Responsibilities**:
- View real-time activity
- Configure policies visually
- Review reports and alerts
- Manage multiple child profiles
- Export reports

**Interfaces**:
- GTK4 application
- Communicates with: `dots-family-daemon` via DBus

**Implementation Details**:
- Modern GTK4/Libadwaita UI
- Real-time activity updates
- Chart visualization with `plotters`
- Responsive design

**Dependencies**:
- `gtk4-rs` for UI
- `libadwaita` for modern widgets
- `plotters` for charts
- `zbus` for DBus

## Data Flow

### Application Launch

```
1. User launches application
2. WM notifies dots-wm-bridge
3. dots-wm-bridge sends event to dots-family-monitor
4. dots-family-monitor queries dots-family-daemon for policy
5. dots-family-daemon checks policy and time limits
6. If blocked:
   a. dots-family-daemon instructs dots-wm-bridge to close window
   b. Alert logged and notification shown
7. If allowed:
   a. Activity tracking begins
   b. Timer started for time limit enforcement
```

### Web Request

```
1. Browser makes HTTP request through proxy
2. dots-family-filter intercepts request
3. URL/domain checked against filter lists
4. If blocked:
   a. Block page returned
   b. Attempt logged to dots-family-daemon
5. If allowed:
   a. Request forwarded
   b. Response inspected for content patterns
   c. If safe, returned to browser
```

### Terminal Command

```
1. User types command in terminal
2. Shell wrapper intercepts before execution
3. dots-terminal-filter parses command
4. Command checked against rules
5. If blocked:
   a. Educational message shown
   b. Command not executed
   c. Logged to dots-family-daemon
6. If allowed:
   a. Command executed normally
   b. Activity logged (if configured)
```

## Communication Protocols

### DBus Interfaces

All components expose DBus interfaces for inter-process communication.

**org.dots.FamilyDaemon**:
```rust
interface FamilyDaemon {
    // Policy queries
    fn check_application_allowed(app_id: &str) -> Result<bool>;
    fn check_time_remaining() -> Result<Duration>;
    fn get_active_profile() -> Result<Profile>;

    // Activity reporting
    fn report_activity(activity: Activity) -> Result<()>;
    fn report_violation(violation: Violation) -> Result<()>;

    // Administration (requires auth)
    fn update_policy(policy: Policy) -> Result<()>;
    fn grant_exception(exception: Exception) -> Result<()>;
    fn authenticate_parent(password: &str) -> Result<Session>;

    // Signals
    signal policy_updated(profile: String);
    signal time_limit_warning(minutes_remaining: u32);
    signal time_limit_reached();
}
```

### Configuration Format

Configuration uses TOML for human readability and strong typing.

**daemon.toml**:
```toml
[general]
enabled = true
active_profile = "alex"
parent_password_hash = "argon2id$..."

[database]
path = "~/.local/share/dots-family/family.db"
retention_days = 90

[monitoring]
window_poll_interval_ms = 1000
activity_bucket_size_minutes = 5

[notifications]
enabled = true
time_limit_warnings = [30, 15, 5] # minutes before limit
```

**profile.toml**:
```toml
[profile]
name = "alex"
age_group = "8-12"

[screen_time]
daily_limit_minutes = 120
allowed_windows = [
  { start = "06:00", end = "08:00" },
  { start = "15:00", end = "19:00" }
]
weekend_bonus_minutes = 60

[applications]
mode = "allowlist" # or "blocklist"
allowed = [
  "firefox",
  "inkscape",
  "tuxmath"
]
blocked_categories = [
  "games",
  "social-media"
]

[web_filtering]
enabled = true
safe_search_enforced = true
blocked_categories = [
  "adult",
  "violence",
  "gambling",
  "social-media"
]
custom_blocked_domains = [
  "example.com"
]

[terminal]
enabled = false # disabled for this age group
blocked_commands = ["rm", "sudo", "chmod"]
```

## Security Architecture

### Threat Model

**Threats Considered**:
1. Child attempting to bypass restrictions
2. Child attempting to modify configuration
3. Child attempting to stop services
4. Child attempting to view monitoring data
5. Malicious application attempting to evade detection
6. Privilege escalation to modify policies

**Mitigations**:
1. Configuration encryption with parent password
2. File permissions restricting access
3. Systemd service protection (no user control)
4. Database encryption at rest
5. Process monitoring via kernel interfaces
6. Capability-based authentication for policy changes

### Authentication Flow

```
1. Parent initiates policy change via GUI/CLI
2. Application requests authentication from daemon
3. Daemon prompts for password
4. Password verified against Argon2id hash
5. Short-lived session token generated
6. Token used for subsequent operations
7. Token expires after 15 minutes of inactivity
```

### Encryption

- **Configuration files**: AES-256-GCM with key derived from parent password
- **Database**: SQLCipher with separate key
- **Session tokens**: Random 256-bit tokens
- **Password hashing**: Argon2id with secure parameters

### Audit Logging

All policy changes and authentication attempts logged:
- Timestamp
- Action attempted
- Success/failure
- Source (GUI, CLI, user)
- Changed values (before/after)

## Performance Considerations

### Resource Usage Targets

- **CPU**: <1% average, <5% peak
- **Memory**: <50MB per component
- **Disk I/O**: Minimal, batched writes
- **Network**: Proxy adds <10ms latency

### Optimization Strategies

1. **Polling Efficiency**: Only check changed windows
2. **Database Batching**: Write activity in 5-minute aggregates
3. **Filter List Caching**: Keep filter lists in memory
4. **Event Debouncing**: Aggregate rapid window changes
5. **Lazy Loading**: Load policies on-demand

### Scalability

- Single child: Negligible overhead
- Multiple children: Linear scaling per profile
- Large filter lists: Bloom filter for fast lookups
- Long activity history: Database archival and pruning

## Error Handling

### Failure Modes

**Component Crash**:
- Systemd automatically restarts service
- Temporary state recovered from database
- Parent notified of service interruption

**Database Corruption**:
- Automatic backup restoration
- Safe mode with default policies
- Parent notification required

**WM Disconnect**:
- Retry connection with exponential backoff
- Fallback to generic Wayland monitoring
- Reduced functionality until reconnected

**Network Issues** (filter updates):
- Use stale filter lists
- Retry in background
- Parent notification if >7 days stale

### Graceful Degradation

Priority ordering:
1. Time limits (enforced even if monitoring fails)
2. Application blocking (enforced via WM if possible)
3. Activity logging (best effort)
4. Web filtering (fails open or closed based on config)

## Testing Strategy

### Unit Tests
- Policy evaluation logic
- Filter matching algorithms
- Time calculation functions
- Encryption/decryption

### Integration Tests
- DBus communication between components
- Database operations
- Configuration loading
- WM adapter behavior

### System Tests
- End-to-end policy enforcement
- Time limit accuracy
- Filter effectiveness
- Performance benchmarks

### Manual Testing
- User experience testing with children
- Parent workflow testing
- Edge case scenarios
- Multiple profile scenarios

## Deployment Architecture

### Systemd Services

**User services** (per-user):
- `dots-family-daemon.service`
- `dots-family-monitor.service`
- `dots-family-filter.service`
- `dots-terminal-filter.service`
- `dots-wm-bridge.service`

**Dependencies**:
```ini
[Unit]
Description=DOTS Family Mode Daemon
After=graphical-session.target
Wants=dots-family-monitor.service

[Service]
Type=dbus
BusName=org.dots.FamilyDaemon
ExecStart=/usr/bin/dots-family-daemon
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
```

### NixOS Module Structure

```nix
{
  features.family-mode = {
    enable = true;

    daemon = {
      enable = true;
      config = { ... };
    };

    monitor = {
      enable = true;
      windowManagers = [ "niri" "swayfx" ];
    };

    filter = {
      enable = true;
      proxyPort = 8118;
      httpsInspection = false;
    };

    terminal = {
      enable = false; # per profile
      shells = [ "bash" "zsh" ];
    };

    profiles = {
      child = { ... };
    };
  };
}
```

## Future Extensibility

### Plugin System

Designed for future plugins:
- Custom filter providers
- Additional WM support
- Alternative authentication methods
- Third-party monitoring tools

### API Stability

- DBus interfaces versioned
- Config format backwards compatible
- Database schema migrations supported
- Deprecated features warned for 2 releases

### Internationalization

- All strings externalized
- UI translated via gettext
- RTL language support in GUI
- Date/time formatting localized

## Related Documentation

- RUST_APPLICATIONS.md: Detailed per-application specs
- WM_INTEGRATION.md: Window manager integration details
- DATA_SCHEMA.md: Database schema specification
- IMPLEMENTATION_ROADMAP.md: Development phases
