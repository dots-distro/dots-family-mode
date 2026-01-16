# Rust Applications Specification

## Overview

Family Mode consists of seven Rust applications, each with specific responsibilities. All applications follow common patterns for configuration, logging, error handling, and inter-process communication.

## Common Patterns

### Project Structure

Each application follows this structure:
```
dots-<app-name>/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs      # Configuration handling
│   ├── dbus.rs        # DBus interface
│   ├── error.rs       # Error types
│   └── <modules>/     # Application-specific modules
├── tests/
│   ├── integration/
│   └── unit/
├── benches/           # Performance benchmarks
└── README.md
```

### Common Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# DBus communication
zbus = "4.0"

# Configuration
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
config = "0.14"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Common utilities
chrono = "0.4"
uuid = { version = "1.6", features = ["v4"] }
```

### Configuration Pattern

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub dbus: DbusConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    // App-specific config
    // ...
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        let settings = config::Config::builder()
            .add_source(config::File::from(config_path))
            .add_source(config::Environment::with_prefix("DOTS_FAMILY"))
            .build()?;

        Ok(settings.try_deserialize()?)
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("No config directory"))?;
        Ok(config_dir.join("dots-family").join("config.toml"))
    }
}
```

### Logging Pattern

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber;

pub fn init_logging(config: &LoggingConfig) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&config.level))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    Ok(())
}
```

### Error Handling Pattern

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("DBus error: {0}")]
    Dbus(#[from] zbus::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

## 1. dots-family-daemon

### Overview
Central coordination service managing policies, authentication, and database operations.

### Cargo.toml
```toml
[package]
name = "dots-family-daemon"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
zbus = "4.0"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio", "chrono"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
config = "0.14"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
chrono = "0.4"
ring = "0.17"  # Cryptography
argon2 = "0.5"  # Password hashing
uuid = { version = "1.6", features = ["v4", "serde"] }

[dev-dependencies]
tempfile = "3.8"
```

### Main Components

**Main Service**:
```rust
use zbus::{Connection, ConnectionBuilder};
use sqlx::SqlitePool;

pub struct FamilyDaemon {
    config: Config,
    db: SqlitePool,
    policy_engine: PolicyEngine,
    auth: AuthManager,
}

impl FamilyDaemon {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize database
        let db = SqlitePool::connect(&config.database.path).await?;
        sqlx::migrate!().run(&db).await?;

        // Initialize components
        let policy_engine = PolicyEngine::new(&db).await?;
        let auth = AuthManager::new(&config.auth)?;

        Ok(Self {
            config,
            db,
            policy_engine,
            auth,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Build DBus service
        let conn = ConnectionBuilder::session()?
            .name("org.dots.FamilyDaemon")?
            .serve_at("/org/dots/FamilyDaemon", self.clone())?
            .build()
            .await?;

        info!("Family daemon started");

        // Keep running
        std::future::pending::<()>().await;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    init_logging(&config.logging)?;

    let mut daemon = FamilyDaemon::new(config).await?;
    daemon.run().await?;

    Ok(())
}
```

**DBus Interface**:
```rust
use zbus::dbus_interface;

#[dbus_interface(name = "org.dots.FamilyDaemon")]
impl FamilyDaemon {
    /// Check if application is allowed
    async fn check_application_allowed(
        &self,
        profile: &str,
        app_id: &str,
    ) -> Result<bool> {
        self.policy_engine.check_app_allowed(profile, app_id).await
    }

    /// Get remaining screen time
    async fn get_time_remaining(&self, profile: &str) -> Result<i64> {
        self.policy_engine.get_time_remaining(profile).await
    }

    /// Report activity
    async fn report_activity(
        &self,
        profile: &str,
        activity: Activity,
    ) -> Result<()> {
        self.db.record_activity(profile, activity).await
    }

    /// Authenticate parent
    async fn authenticate(
        &self,
        password: &str,
    ) -> Result<String> {
        self.auth.authenticate(password).await
    }

    /// Update policy (requires auth token)
    async fn update_policy(
        &self,
        token: &str,
        profile: &str,
        policy: Policy,
    ) -> Result<()> {
        self.auth.verify_token(token)?;
        self.policy_engine.update_policy(profile, policy).await?;
        self.policy_updated(profile).await?;
        Ok(())
    }

    /// Signal: Policy was updated
    #[dbus_interface(signal)]
    async fn policy_updated(&self, profile: &str) -> zbus::Result<()>;

    /// Signal: Time limit warning
    #[dbus_interface(signal)]
    async fn time_limit_warning(
        &self,
        profile: &str,
        minutes_remaining: u32,
    ) -> zbus::Result<()>;
}
```

**Policy Engine**:
```rust
pub struct PolicyEngine {
    db: SqlitePool,
    cache: Arc<RwLock<PolicyCache>>,
}

impl PolicyEngine {
    pub async fn check_app_allowed(
        &self,
        profile: &str,
        app_id: &str,
    ) -> Result<bool> {
        // 1. Check cache
        if let Some(result) = self.cache.read().await.get(profile, app_id) {
            return Ok(result);
        }

        // 2. Load policy from database
        let policy = self.load_policy(profile).await?;

        // 3. Evaluate
        let allowed = match policy.applications.mode {
            Mode::Allowlist => {
                policy.applications.allowed.contains(app_id) ||
                self.check_category_allowed(&policy, app_id).await?
            }
            Mode::Blocklist => {
                !policy.applications.blocked.contains(app_id) &&
                !self.check_category_blocked(&policy, app_id).await?
            }
        };

        // 4. Cache result
        self.cache.write().await.insert(profile, app_id, allowed);

        Ok(allowed)
    }

    pub async fn get_time_remaining(&self, profile: &str) -> Result<i64> {
        // Get today's usage from database
        let usage = self.get_today_usage(profile).await?;

        // Get policy limit
        let policy = self.load_policy(profile).await?;
        let limit = policy.screen_time.daily_limit_minutes * 60;

        // Calculate remaining
        let remaining = limit.saturating_sub(usage);

        Ok(remaining as i64)
    }
}
```

**Authentication Manager**:
```rust
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use ring::rand::{SystemRandom, SecureRandom};

pub struct AuthManager {
    password_hash: String,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    rng: SystemRandom,
}

impl AuthManager {
    pub async fn authenticate(&self, password: &str) -> Result<String> {
        // Verify password
        let parsed_hash = PasswordHash::new(&self.password_hash)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)?;

        // Generate session token
        let token = self.generate_token()?;

        // Store session
        let session = Session {
            token: token.clone(),
            created: Utc::now(),
            expires: Utc::now() + Duration::minutes(15),
        };
        self.sessions.write().await.insert(token.clone(), session);

        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<()> {
        let sessions = self.sessions.read().blocking_lock();
        let session = sessions.get(token)
            .ok_or_else(|| anyhow!("Invalid token"))?;

        if session.expires < Utc::now() {
            return Err(anyhow!("Token expired"));
        }

        Ok(())
    }

    fn generate_token(&self) -> Result<String> {
        let mut bytes = [0u8; 32];
        self.rng.fill(&mut bytes)?;
        Ok(hex::encode(bytes))
    }
}
```

### CLI Interface
```bash
# Run daemon
dots-family-daemon

# Options
dots-family-daemon --config /path/to/config.toml
dots-family-daemon --log-level debug
```

### Configuration
```toml
[general]
enabled = true
active_profile = "alex"

[database]
path = "~/.local/share/dots-family/family.db"
max_connections = 10

[auth]
password_hash = "$argon2id$..."
session_timeout_minutes = 15

[dbus]
bus = "session"
name = "org.dots.FamilyDaemon"

[logging]
level = "info"
```

### Testing
- Unit tests for policy evaluation
- Integration tests for DBus communication
- Performance tests for database queries
- Security tests for authentication

## 2. dots-family-monitor

### Overview
Tracks window activity and application usage across different window managers.

### Cargo.toml
```toml
[package]
name = "dots-family-monitor"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
zbus = "4.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
chrono = "0.4"

# Wayland protocol support
wayland-client = "0.31"
wayland-protocols = "0.31"

[dev-dependencies]
tokio-test = "0.4"
```

### Main Components

**Main Service**:
```rust
pub struct FamilyMonitor {
    config: Config,
    wm_bridge: Arc<dyn WindowManagerBridge>,
    daemon: DaemonProxy,
    state: Arc<RwLock<MonitorState>>,
}

impl FamilyMonitor {
    pub async fn new(config: Config) -> Result<Self> {
        // Connect to window manager
        let wm_bridge = create_wm_bridge(&config.window_manager).await?;

        // Connect to daemon
        let conn = Connection::session().await?;
        let daemon = DaemonProxy::new(&conn).await?;

        Ok(Self {
            config,
            wm_bridge,
            daemon,
            state: Arc::new(RwLock::new(MonitorState::new())),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(
            Duration::from_millis(self.config.poll_interval_ms)
        );

        loop {
            interval.tick().await;

            // Get current window
            let window = self.wm_bridge.get_active_window().await?;

            // Update state
            self.update_state(window).await?;

            // Report to daemon if needed
            if self.should_report().await {
                self.report_activity().await?;
            }
        }
    }

    async fn update_state(&mut self, window: Window) -> Result<()> {
        let mut state = self.state.write().await;

        // If window changed, finalize previous activity
        if state.current_window.as_ref() != Some(&window) {
            if let Some(activity) = state.finalize_current() {
                self.report_activity_to_daemon(activity).await?;
            }

            state.current_window = Some(window);
            state.current_start = Utc::now();
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    init_logging(&config.logging)?;

    let mut monitor = FamilyMonitor::new(config).await?;
    monitor.run().await?;

    Ok(())
}
```

**Window Manager Bridge Trait**:
```rust
#[async_trait]
pub trait WindowManagerBridge: Send + Sync {
    async fn get_active_window(&self) -> Result<Window>;
    async fn close_window(&self, window_id: &str) -> Result<()>;
    async fn subscribe_events(&self) -> Result<WindowEventStream>;
}

pub struct Window {
    pub id: String,
    pub title: String,
    pub app_id: String,
    pub app_name: String,
}
```

### CLI Interface
```bash
# Run monitor
dots-family-monitor

# Options
dots-family-monitor --wm niri
dots-family-monitor --poll-interval 1000
```

### Configuration
```toml
[general]
window_manager = "auto"  # or "niri", "swayfx", "hyprland"
poll_interval_ms = 1000

[monitoring]
track_window_titles = true
anonymize_titles = false
aggregate_interval_minutes = 5

[dbus]
daemon_name = "org.dots.FamilyDaemon"
```

### Testing
- Unit tests for state management
- Integration tests with mock WM
- Performance tests for polling efficiency

## 3. dots-family-filter

### Overview
HTTP/HTTPS proxy for web content filtering.

### Cargo.toml
```toml
[package]
name = "dots-family-filter"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
hyper = { version = "1.0", features = ["server", "http1", "http2"] }
hyper-util = "0.1"
rustls = "0.22"
zbus = "4.0"
url = "2.5"
regex = "1.10"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
anyhow = "1.0"
thiserror = "1.0"

# Filter list parsing
adblock = "0.8"  # AdBlock Plus format parser

# Bloom filter for fast lookups
probabilistic-collections = "0.7"
```

### Main Components

**Proxy Server**:
```rust
use hyper::{Request, Response, Body};
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;

pub struct FilterProxy {
    config: Config,
    filter_engine: Arc<FilterEngine>,
    daemon: DaemonProxy,
}

impl FilterProxy {
    pub async fn run(&self) -> Result<()> {
        let addr = format!("{}:{}",
            self.config.listen_address,
            self.config.listen_port
        ).parse()?;

        let filter_engine = self.filter_engine.clone();
        let daemon = self.daemon.clone();

        let make_svc = make_service_fn(move |_| {
            let filter_engine = filter_engine.clone();
            let daemon = daemon.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    handle_request(req, filter_engine.clone(), daemon.clone())
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        info!("Filter proxy listening on {}", addr);

        server.await?;

        Ok(())
    }
}

async fn handle_request(
    req: Request<Body>,
    filter_engine: Arc<FilterEngine>,
    daemon: DaemonProxy,
) -> Result<Response<Body>> {
    let url = req.uri().to_string();

    // Check filter
    match filter_engine.check_url(&url).await? {
        FilterResult::Allowed => {
            // Forward request
            forward_request(req).await
        }
        FilterResult::Blocked(reason) => {
            // Log block
            daemon.report_blocked_website(&url, &reason).await?;

            // Return block page
            Ok(create_block_page(&url, &reason))
        }
    }
}
```

**Filter Engine**:
```rust
use adblock::Engine as AdblockEngine;

pub struct FilterEngine {
    adblock: AdblockEngine,
    categories: HashMap<String, Vec<String>>,
    custom_rules: Vec<FilterRule>,
    bloom: BloomFilter,
}

impl FilterEngine {
    pub async fn check_url(&self, url: &str) -> Result<FilterResult> {
        let parsed = Url::parse(url)?;
        let domain = parsed.domain()
            .ok_or_else(|| anyhow!("No domain in URL"))?;

        // 1. Fast negative check with Bloom filter
        if !self.bloom.contains(domain) {
            return Ok(FilterResult::Allowed);
        }

        // 2. Check AdBlock rules
        if self.adblock.check_network_urls(url, "", "").matched {
            return Ok(FilterResult::Blocked("Blocklist".to_string()));
        }

        // 3. Check category
        if let Some(category) = self.get_category(domain).await? {
            if self.is_category_blocked(&category) {
                return Ok(FilterResult::Blocked(
                    format!("Category: {}", category)
                ));
            }
        }

        // 4. Check custom rules
        for rule in &self.custom_rules {
            if rule.matches(url) {
                return Ok(rule.action.clone());
            }
        }

        Ok(FilterResult::Allowed)
    }

    pub async fn reload_lists(&mut self) -> Result<()> {
        // Download and parse filter lists
        let lists = download_filter_lists().await?;
        self.adblock = AdblockEngine::from_rules(&lists);

        // Rebuild Bloom filter
        self.rebuild_bloom();

        Ok(())
    }
}
```

### CLI Interface
```bash
# Run filter
dots-family-filter

# Options
dots-family-filter --port 8118
dots-family-filter --update-lists
dots-family-filter --test-url https://example.com
```

### Configuration
```toml
[proxy]
listen_address = "127.0.0.1"
listen_port = 8118
https_inspection = false

[filtering]
mode = "strict"  # strict, moderate, minimal
safe_search_enforced = true

[lists]
auto_update = true
update_interval_hours = 24
builtin = ["family-mode-base", "family-mode-adult"]
```

### Testing
- Unit tests for filter matching
- Integration tests for HTTP/HTTPS proxying
- Performance tests for filter evaluation

## 4. dots-terminal-filter

### Overview
Terminal command filtering with shell integration.

### Cargo.toml
```toml
[package]
name = "dots-terminal-filter"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
zbus = "4.0"
nix = { version = "0.27", features = ["pty"] }
regex = "1.10"
shellwords = "1.1"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
anyhow = "1.0"
```

### Main Components

**Command Filter**:
```rust
pub struct CommandFilter {
    config: Config,
    rules: Vec<FilterRule>,
    daemon: DaemonProxy,
}

impl CommandFilter {
    pub async fn check_command(&self, command: &str) -> Result<FilterAction> {
        // Parse command
        let parsed = parse_command(command)?;

        // Classify risk
        let risk = self.classify_command(&parsed)?;

        match risk {
            CommandRisk::Safe => Ok(FilterAction::Allow),
            CommandRisk::Educational => {
                self.show_educational_warning(&parsed).await?;
                Ok(FilterAction::AllowWithWarning)
            }
            CommandRisk::Risky => {
                Ok(FilterAction::RequireApproval)
            }
            CommandRisk::Dangerous => {
                self.daemon.report_blocked_command(command).await?;
                Ok(FilterAction::Block)
            }
        }
    }

    fn classify_command(&self, parsed: &ParsedCommand) -> Result<CommandRisk> {
        // Check explicit rules first
        for rule in &self.rules {
            if rule.matches(parsed) {
                return Ok(rule.risk);
            }
        }

        // Heuristic classification
        if self.is_dangerous_pattern(parsed) {
            return Ok(CommandRisk::Dangerous);
        }

        if parsed.requires_sudo() {
            return Ok(CommandRisk::Risky);
        }

        if self.is_common_mistake(parsed) {
            return Ok(CommandRisk::Educational);
        }

        Ok(CommandRisk::Safe)
    }
}
```

**Shell Integration**:
```rust
// Generate shell wrapper script
pub fn generate_shell_wrapper(shell: &str) -> Result<String> {
    match shell {
        "bash" => Ok(BASH_WRAPPER.to_string()),
        "zsh" => Ok(ZSH_WRAPPER.to_string()),
        "fish" => Ok(FISH_WRAPPER.to_string()),
        _ => Err(anyhow!("Unsupported shell: {}", shell)),
    }
}

const BASH_WRAPPER: &str = r#"
# DOTS Family Mode Command Filter
if [[ -n "$DOTS_FAMILY_MODE" ]]; then
    preexec() {
        if ! dots-terminal-filter check "$1"; then
            return 1
        fi
    }
    trap 'preexec "$BASH_COMMAND"' DEBUG
fi
"#;
```

### CLI Interface
```bash
# Check command (used by shell wrapper)
dots-terminal-filter check "rm -rf /"

# Generate shell wrapper
dots-terminal-filter generate-wrapper bash > ~/.dots-family-bash.sh

# Test command classification
dots-terminal-filter classify "sudo apt install package"
```

### Configuration
```toml
[filtering]
enabled = true
mode = "filter"  # monitor, filter, block

[commands]
# Always blocked
blocked = [
  "rm -rf /",
  "dd if=/dev/zero of=/dev/sda",
]

# Require approval
approval_required = [
  "sudo *",
  "rm -rf *",
]
```

### Testing
- Unit tests for command parsing
- Unit tests for classification
- Integration tests with shells

## 5. dots-wm-bridge

### Overview
Abstraction layer for different window manager integrations.

### Cargo.toml
```toml
[package]
name = "dots-wm-bridge"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
zbus = "4.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
anyhow = "1.0"
thiserror = "1.0"

# Wayland
wayland-client = "0.31"

# Niri-specific
niri-ipc = { git = "https://github.com/YaLTeR/niri" }
```

### Main Components

**WM Bridge Service**:
```rust
use async_trait::async_trait;

pub struct WMBridge {
    adapter: Box<dyn WindowManagerAdapter>,
}

impl WMBridge {
    pub async fn new(wm: &str) -> Result<Self> {
        let adapter: Box<dyn WindowManagerAdapter> = match wm {
            "niri" => Box::new(NiriAdapter::new().await?),
            "swayfx" => Box::new(SwayfxAdapter::new().await?),
            "hyprland" => Box::new(HyprlandAdapter::new().await?),
            "auto" => Box::new(detect_wm().await?),
            _ => return Err(anyhow!("Unsupported WM: {}", wm)),
        };

        Ok(Self { adapter })
    }
}

#[async_trait]
pub trait WindowManagerAdapter: Send + Sync {
    async fn get_active_window(&self) -> Result<Window>;
    async fn list_windows(&self) -> Result<Vec<Window>>;
    async fn close_window(&self, id: &str) -> Result<()>;
    async fn subscribe_events(&self) -> Result<WindowEventStream>;
}
```

**Niri Adapter**:
```rust
use niri_ipc::{Socket, Request, Response};

pub struct NiriAdapter {
    socket: Socket,
}

#[async_trait]
impl WindowManagerAdapter for NiriAdapter {
    async fn get_active_window(&self) -> Result<Window> {
        let response = self.socket.send(Request::Windows).await?;

        match response {
            Response::Windows(windows) => {
                let active = windows.iter()
                    .find(|w| w.is_focused)
                    .ok_or_else(|| anyhow!("No active window"))?;

                Ok(Window {
                    id: active.id.to_string(),
                    title: active.title.clone(),
                    app_id: active.app_id.clone(),
                    app_name: active.app_id.clone(), // TODO: lookup
                })
            }
            _ => Err(anyhow!("Unexpected response")),
        }
    }
}
```

### CLI Interface
```bash
# Run bridge
dots-wm-bridge

# Options
dots-wm-bridge --wm niri
dots-wm-bridge --detect
```

### Testing
- Unit tests for adapters
- Integration tests with mock WMs

## 6. dots-family-ctl

### Overview
Command-line administration tool.

### Cargo.toml
```toml
[package]
name = "dots-family-ctl"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
zbus = "4.0"
ratatui = "0.25"
crossterm = "0.27"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
comfy-table = "7.1"
```

### Main Components

**CLI Structure**:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dots-family-ctl")]
#[command(about = "DOTS Family Mode Control")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Task management
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Profile management
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// View reports
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    /// Grant exceptions
    Exception {
        #[command(subcommand)]
        action: ExceptionAction,
    },
}
```

### CLI Interface
```bash
# View status
dots-family-ctl status --profile alex

# Grant exception
dots-family-ctl exception grant \
  --profile alex \
  --type extra-time \
  --amount 60 \
  --reason "Homework"

# View report
dots-family-ctl report daily --profile alex

# Update policy
dots-family-ctl policy update \
  --profile alex \
  --screen-time-limit 120
```

## 7. dots-family-gui

### Overview
GTK4-based graphical parent dashboard.

## 8. dots-family-lockscreen

### Purpose
Custom Wayland lockscreen with parental override capabilities

### Description
A Wayland-native lockscreen that supports dual authentication - child password for normal unlock and parent PIN/password for override access. Prevents bypass via virtual console switching.

### Key Features
- Child password authentication (PAM)
- Parent override PIN (short-term access)
- Parent full authentication (user switching)
- Emergency override with parent notification
- Time-limited parent sessions
- Audit logging of all unlock events
- Compatible with Niri, Swayfx, Hyprland

### Dependencies
```toml
[dependencies]
gtk4 = "0.7"
gtk4-layer-shell = "0.3"
pam-client = "0.5"
zbus = "4.0"
tokio = { version = "1.35", features = ["full"] }
wayland-client = "0.31"
wayland-protocols = { version = "0.31", features = ["unstable"] }
```

### DBus Interface
`org.dots.FamilyLockscreen`

**Methods**:
- `Unlock()`: Standard unlock with child password
- `ParentOverride(pin: String)`: Unlock with parent PIN
- `EmergencyRequest()`: Request emergency parent override

**Signals**:
- `UnlockAttempt(success: bool, user: String)`: Fired on unlock attempts
- `ParentOverrideActivated(duration_minutes: u32)`: Parent override activated
- `EmergencyRequested(timestamp: u64)`: Emergency unlock requested

### Configuration
**System**: `/etc/dots-family/lockscreen.toml`
```toml
[lockscreen]
enabled = true
idle_timeout_seconds = 300
allow_parent_override = true

[parent]
pin_encrypted = "$argon2id$..."
override_duration_minutes = 30

[security]
max_unlock_attempts = 5
lockout_duration_seconds = 60
log_all_attempts = true
```

**User override**: `~/.config/dots-family/lockscreen.toml`

### Security Considerations
- No password caching in memory
- Parent PIN stored encrypted
- Rate limiting on authentication attempts
- Cannot be killed by unprivileged users
- Integrates with family daemon for policy enforcement

### Implementation Details

**Main Components**:
```rust
pub struct FamilyLockscreen {
    config: Config,
    pam: PamAuthenticator,
    daemon: DaemonProxy,
    session: LockSession,
}

impl FamilyLockscreen {
    pub async fn lock(&mut self) -> Result<()> {
        // Acquire ext-session-lock-v1
        self.session.lock().await?;

        // Show lock UI
        self.show_ui().await?;

        // Wait for unlock
        loop {
            match self.wait_for_auth().await? {
                AuthResult::ChildUnlock => break,
                AuthResult::ParentOverride(duration) => {
                    self.grant_parent_access(duration).await?;
                    break;
                }
                AuthResult::Failed => continue,
            }
        }

        Ok(())
    }
}
```

### CLI Interface
```bash
# Start lockscreen
dots-family-lockscreen

# Lock immediately
dots-family-lockscreen --lock-now

# Test authentication
dots-family-lockscreen --test-auth
```

## 7. dots-family-gui (continued)

### Overview (continued)
GTK4-based graphical parent dashboard.

### Cargo.toml
```toml
[package]
name = "dots-family-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
gtk4 = "0.7"
libadwaita = "0.5"
zbus = "4.0"
plotters = "0.3"
serde = { version = "1.0", features = ["derive"] }
```

### Main Components

**Application Structure**:
```rust
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use libadwaita as adw;

pub struct FamilyGui {
    app: Application,
    daemon: DaemonProxy,
}

impl FamilyGui {
    pub fn new() -> Result<Self> {
        let app = Application::builder()
            .application_id("org.dots.FamilyGui")
            .build();

        app.connect_activate(|app| {
            build_ui(app);
        });

        Ok(Self {
            app,
            daemon: /* connect to daemon */,
        })
    }

    pub fn run(&self) -> Result<()> {
        self.app.run();
        Ok(())
    }
}
```

## Build and Distribution

### Nix Flake Integration

```nix
{
  outputs = { self, nixpkgs }: {
    packages = {
      dots-family-daemon = pkgs.rustPlatform.buildRustPackage {
        pname = "dots-family-daemon";
        version = "0.1.0";
        src = ./dots-family-daemon;
        cargoLock.lockFile = ./dots-family-daemon/Cargo.lock;
      };

      # ... other packages
    };
  };
}
```

## Common Testing Patterns

### Integration Test Template
```rust
#[tokio::test]
async fn test_app_integration() {
    // Setup
    let config = test_config();
    let daemon = spawn_test_daemon(config).await;

    // Test
    let result = daemon.check_application_allowed("test", "firefox").await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}
```

### Performance Benchmark Template
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_filter_check(c: &mut Criterion) {
    let filter = FilterEngine::new(/* ... */);

    c.bench_function("filter_check", |b| {
        b.iter(|| {
            filter.check_url(black_box("https://example.com"))
        });
    });
}

criterion_group!(benches, bench_filter_check);
criterion_main!(benches);
```

## Related Documentation

- ARCHITECTURE.md: Overall system design
- WM_INTEGRATION.md: Window manager details
- DATA_SCHEMA.md: Database schema
- IMPLEMENTATION_ROADMAP.md: Build order
