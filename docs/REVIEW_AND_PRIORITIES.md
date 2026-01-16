# Family Mode Review and Priorities

## Critical Issues Found

### 1. Systemd Service Configuration Error

**Current State (INCORRECT)**:
- ARCHITECTURE.md line 92: "Runs as user systemd service"
- ARCHITECTURE.md lines 551-574: User services configuration
- RUST_APPLICATIONS.md line 210: "ConnectionBuilder::session()" (session bus)
- RUST_APPLICATIONS.md line 432: "bus = 'session'"

**Required State (CORRECT)**:
- **Daemon MUST run as system-wide systemd service** (global/system service)
- **DBus MUST use system bus** for cross-user enforcement

**Technical Rationale**:
Family Mode provides parental controls that enforce policies across all user accounts on the system. A per-user service would only affect that user's session and could be easily stopped by the child. System-wide enforcement requires:

1. **System service**: Runs as privileged process independent of user sessions
2. **System DBus**: Allows cross-user communication and enforcement
3. **System-wide configuration**: Stored in `/etc/` hierarchy, not `~/.config/`
4. **Root or dedicated user**: Service needs elevated privileges for enforcement

**Impact Assessment**:

| Component | Current | Required | Changes Needed |
|-----------|---------|----------|----------------|
| Systemd units | User service | System service | Rewrite unit files |
| DBus bus | Session bus | System bus | Change connection type |
| Config location | `~/.config/` | `/etc/` | Update all paths |
| Database location | `~/.local/share/` | `/var/lib/` | Update paths, permissions |
| Service user | User account | Root or `dots-family` | Add user, capabilities |
| Permissions model | User-level | System-level | Redesign access control |

**Files Requiring Updates**:
1. `docs/improvements/family_mode/ARCHITECTURE.md`
   - Section "1. dots-family-daemon" (lines 73-103)
   - Section "Systemd Services" (lines 549-574)
   - Section "Security Architecture" (lines 417-436)
2. `docs/improvements/family_mode/RUST_APPLICATIONS.md`
   - DBus connection code (lines 210-214)
   - Configuration examples (lines 418-437)
3. `docs/improvements/family_mode/IMPLEMENTATION_ROADMAP.md`
   - Phase 7: NixOS Integration (lines 394-463)

### 2. DBus Bus Selection

**Current State (INCORRECT)**:
```rust
// WRONG: Session bus is per-user
let conn = ConnectionBuilder::session()?
    .name("org.dots.FamilyDaemon")?
    .build()
    .await?;
```

**Required State (CORRECT)**:
```rust
// RIGHT: System bus for cross-user enforcement
let conn = ConnectionBuilder::system()?
    .name("org.dots.FamilyDaemon")?
    .build()
    .await?;
```

**DBus Policy Configuration Required**:
```xml
<!-- /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf -->
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="dots-family">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
  </policy>

  <policy user="root">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
  </policy>

  <policy context="default">
    <allow send_destination="org.dots.FamilyDaemon"
           send_interface="org.dots.FamilyDaemon"/>
  </policy>
</busconfig>
```

**Impact**: All DBus interfaces must be redesigned for system bus, affecting all components.

## Feature Prioritization

### Priority Rationale

Based on the discovery of systemd service issues and user feedback, implementation priority is:

1. **Foundation First**: Core daemon as system service (must be correct from start)
2. **CLI Before GUI**: Administrators need tools immediately, GUI is polish
3. **Multi-WM Support Early**: Window manager integration is foundational for monitoring - must come immediately after daemon before filtering features
4. **Terminal Filtering**: Prevents easy bypass of restrictions
5. **Content Safety**: Web filtering more critical than pretty dashboards
6. **Dashboard Last**: Nice to have, not essential for functionality

### Phase 1: Foundation (HIGHEST PRIORITY)

**Timeline**: Weeks 1-6

**Critical Path**:
1. **System Service Architecture** (NEW - Week 1)
   - Design system service permissions model
   - Define dedicated user vs root approach
   - Document capability requirements (CAP_SYS_PTRACE, CAP_NET_ADMIN)
   - Design `/etc/` configuration structure
   - Design `/var/lib/` database structure

2. **dots-family-daemon** (Weeks 2-4)
   - Core service as **system service**
   - DBus on **system bus**
   - Authentication across user boundaries
   - Basic policy engine (time limits, app allow/block)
   - SQLite database with proper permissions

3. **dots-family-cli** (Week 5)
   - Parent authentication via DBus
   - Policy configuration commands
   - Status reporting
   - Profile management

4. **Basic Monitoring** (Week 6)
   - dots-family-monitor (system service component)
   - Window tracking for Niri (one WM to start)
   - Communication with daemon via system DBus

**Deliverables**:
- System-wide daemon with correct architecture
- CLI for administration
- Basic enforcement (time limits, app blocking)
- Single WM monitoring (Niri)

**Success Criteria**:
- Daemon runs as system service
- Child users cannot stop daemon
- Policies enforce across all user accounts
- CLI accessible with parent authentication

### Phase 2: Multi-WM Support (HIGHEST PRIORITY)

**Timeline**: Weeks 7-10

**Rationale**: Multi-WM support (window manager integration) is foundational for monitoring and enforcement. Without proper WM integration, the monitoring system cannot track windows and applications across different window managers. This needs to come immediately after the core daemon is established, before terminal filtering and content filtering which depend on window context.

**Tasks**:
1. **dots-wm-bridge** (Weeks 7-9)
   - Swayfx adapter
   - Hyprland adapter
   - Automatic WM detection
   - Window event monitoring
   - Application identification across WMs

2. **Monitor Integration** (Week 10)
   - Extend dots-family-monitor for multi-WM support
   - Unified window tracking API
   - Cross-WM application detection
   - Test on all three WMs

3. **dots-family-lockscreen** (Weeks 9-10)
   - Custom Wayland lockscreen with parental override support
   - Critical for preventing console bypass
   - Integrates with ext-session-lock-v1 protocol
   - Works on Niri, Swayfx, Hyprland

**Deliverables**:
- All three WMs supported (Niri, Swayfx, Hyprland)
- Automatic detection working
- Unified monitoring interface
- Custom lockscreen with parental override

### Phase 3: Terminal Filtering (HIGH PRIORITY)

**Timeline**: Weeks 11-14

**Rationale**: Terminal filtering prevents easy bypass of restrictions (e.g., `systemctl --user stop dots-family-daemon` or `killall dots-family-monitor`). This must come AFTER WM support establishes window context.

**Tasks**:
1. **dots-terminal-filter** (Weeks 11-13)
   - Command parsing and classification
   - Shell integration (bash, zsh, fish)
   - Educational feedback system
   - Integration with daemon via system DBus

2. **Bypass Prevention** (Week 14)
   - Block service control commands
   - Block process killing
   - Block config file modification
   - Block DBus manipulation

**Deliverables**:
- Terminal command filtering functional
- Shell integration for bash, zsh, fish
- Bypass attempts logged and blocked

### Phase 4: Content Filtering (HIGH PRIORITY)

**Timeline**: Weeks 15-18

**Rationale**: Web content safety is more critical than GUI dashboards. Parents need this protection before pretty interfaces.

**Tasks**:
1. **dots-family-proxy** (Weeks 15-17)
   - HTTP/HTTPS proxy server
   - Category-based filtering
   - URL/domain blocking
   - Safe search enforcement

2. **DNS Filtering** (Week 18)
   - Network-level blocking
   - Safe DNS enforcement
   - DNS-over-HTTPS prevention

**Deliverables**:
- Working web content filter
- DNS-level filtering
- Safe search enforced
- Category-based blocking

### Phase 5: Multi-Child Support (MEDIUM PRIORITY)

**Timeline**: Weeks 19-21

**Tasks**:
1. **Profile Management** (Weeks 19-20)
   - Multiple child profiles
   - Per-profile policies
   - Profile switching
   - Age-appropriate templates

2. **Multi-User Enforcement** (Week 21)
   - Cross-user policy application
   - Per-user activity tracking
   - Profile detection at login

**Deliverables**:
- Multiple child profiles supported
- Policies per profile
- Age-based templates

### Phase 6: User Interface (LOWER PRIORITY)

**Timeline**: Weeks 22-26

**Tasks**:
1. **dots-family-tray** (Week 22-23)
   - System tray indicator
   - Quick status view
   - Quick actions

2. **dots-family-dashboard** (Weeks 24-26)
   - GTK4 GUI
   - Activity reports
   - Policy configuration UI
   - Charts and visualizations

**Deliverables**:
- System tray indicator
- GTK4 dashboard for parents
- Visual reports and charts

## Required Documentation Updates

### 1. ARCHITECTURE.md

**Section: "1. dots-family-daemon" (lines 73-103)**

Replace:
```
**Security**:
- Runs as user systemd service
- Config files encrypted with parent password
- Database encrypted at rest
- Capability-based access control
```

With:
```
**Security**:
- Runs as system-wide systemd service (global service, not per-user)
- Service user: dedicated 'dots-family' account with limited capabilities
- Config files: /etc/dots-family/ (root-owned, 0600 permissions)
- Database: /var/lib/dots-family/ (dots-family user-owned)
- DBus: system bus (not session bus)
- Capability-based access control via polkit
- Required capabilities: CAP_SYS_PTRACE (process monitoring)
```

**Section: "Systemd Services" (lines 549-574)**

Replace entire section with:
```
### Systemd Services

**System services** (global, one instance per machine):
- `dots-family-daemon.service` (system service)

**User services** (per-user instances):
- `dots-family-monitor.service`
- `dots-family-filter.service`
- `dots-wm-bridge.service`

**Rationale**:
- Daemon MUST be system service for cross-user enforcement
- Monitoring components run per-user for session integration
- Child users cannot stop system service

**System Service Configuration**:
```ini
[Unit]
Description=DOTS Family Mode Daemon (System-Wide)
Documentation=https://github.com/dots-nix/framework
After=dbus.service
Requires=dbus.service

[Service]
Type=dbus
BusName=org.dots.FamilyDaemon
User=dots-family
Group=dots-family
ExecStart=/usr/bin/dots-family-daemon
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/dots-family

# Capabilities
AmbientCapabilities=CAP_SYS_PTRACE
CapabilityBoundingSet=CAP_SYS_PTRACE

[Install]
WantedBy=multi-user.target
```
```

**Section: "Communication Protocols" (lines 319-347)**

Update DBus interface to note system bus:
```
### DBus Interfaces

All components expose DBus interfaces for inter-process communication.

**IMPORTANT**: dots-family-daemon uses **system bus** for cross-user enforcement.
Other components use session bus for per-user functionality.
```

### 2. RUST_APPLICATIONS.md

**Section: DBus Interface (lines 237-298)**

Replace line 210-214:
```rust
// OLD (WRONG):
let conn = ConnectionBuilder::session()?
    .name("org.dots.FamilyDaemon")?
    .build()
    .await?;

// NEW (CORRECT):
let conn = ConnectionBuilder::system()?
    .name("org.dots.FamilyDaemon")?
    .build()
    .await?;
```

**Section: Configuration (lines 417-437)**

Update paths:
```toml
[general]
enabled = true
active_profile = "alex"

[database]
# System-wide database
path = "/var/lib/dots-family/family.db"
max_connections = 10

[auth]
# Password hash in system config
password_hash = "$argon2id$..."
session_timeout_minutes = 15

[dbus]
# System bus for cross-user enforcement
bus = "system"
name = "org.dots.FamilyDaemon"

[logging]
level = "info"
# Logs to journald (system)
```

### 3. IMPLEMENTATION_ROADMAP.md

**Update Phase 1 (lines 53-110)**:

Add new first task:
```
**System Service Architecture** (Week 1):
- [ ] Design system-wide service model
- [ ] Document privilege requirements
- [ ] Define dedicated user vs root approach
- [ ] Design DBus policy configuration
- [ ] Design file system layout (/etc/, /var/lib/)
- [ ] Document capability requirements
- [ ] Design polkit policies for authentication
```

Update daemon tasks to emphasize system service:
```
**dots-family-daemon** (Weeks 2-4):
- [ ] Implement daemon as SYSTEM SERVICE (not user service)
- [ ] DBus interface on SYSTEM BUS
- [ ] Cross-user policy enforcement
- [ ] Basic policy engine
  - Time limit enforcement
  - Application allow/block lists
- [ ] Authentication system with polkit
  - Password hashing (Argon2)
  - Session management
  - Cross-user authorization
- [ ] Database operations in /var/lib/dots-family
  - Activity recording
  - Policy storage
  - Event logging
- [ ] Configuration in /etc/dots-family
```

**Reorder Phases**:
```
Phase 1: Foundation (Weeks 1-6)
Phase 2: Multi-WM Support (Weeks 7-10)      # MOVED UP - Critical for monitoring
Phase 3: Terminal Filtering (Weeks 11-14)   # MOVED DOWN
Phase 4: Content Filtering (Weeks 15-18)    # MOVED DOWN
Phase 5: Multi-Child Support (Weeks 19-21)  # MOVED DOWN
Phase 6: User Interface (Weeks 22-26)       # MOVED DOWN
Phase 7: Reporting (Weeks 27-30)            # NEW
Phase 8: NixOS Integration (Weeks 31-34)    # ADJUSTED
Phase 9: Polish (Weeks 35-38)               # ADJUSTED
Phase 10: Beta Testing (Weeks 39-42)        # ADJUSTED
```

### 4. Create SYSTEMD_ARCHITECTURE.md (NEW)

Create comprehensive document explaining:
- System vs user services
- When each is appropriate
- Security implications
- Multi-user enforcement strategies
- DBus policy configuration
- Polkit integration
- Capability requirements
- File system layout

(See detailed content in following section)

## New Documentation: SYSTEMD_ARCHITECTURE.md

Create `/docs/improvements/family_mode/SYSTEMD_ARCHITECTURE.md`:

```markdown
# Systemd Architecture for Family Mode

## Overview

Family Mode requires a hybrid systemd architecture:
- **System service**: Core daemon for cross-user enforcement
- **User services**: Per-user monitoring and filtering components

This document explains the rationale, implementation, and security model.

## System Service vs User Services

### When to Use System Services

Use system services when:
- Cross-user functionality required
- Service must survive user logout
- Elevated privileges needed
- Single instance per machine
- Must be protected from user control

### When to Use User Services

Use user services when:
- Per-user functionality
- Session integration needed
- User-level permissions sufficient
- Multiple instances (one per user)

### Family Mode Service Types

| Service | Type | Rationale |
|---------|------|-----------|
| dots-family-daemon | System | Cross-user enforcement, single policy engine |
| dots-family-monitor | User | Per-user window tracking, session integration |
| dots-family-filter | User | Per-user proxy, session networking |
| dots-wm-bridge | User | Per-user WM connection |

## System Service Architecture

### Service User

**Option 1: Dedicated User (RECOMMENDED)**
```bash
# Create service user
useradd -r -s /bin/nologin -d /var/lib/dots-family dots-family
```

Advantages:
- Principle of least privilege
- Limited capabilities
- Clear ownership model
- Easier auditing

Disadvantages:
- More complex setup
- Capability configuration required

**Option 2: Root User**

Advantages:
- Simple configuration
- All permissions available

Disadvantages:
- Security risk
- Violates principle of least privilege
- Harder to audit

**Recommendation**: Use dedicated `dots-family` user with limited capabilities.

### Required Capabilities

```ini
[Service]
# Required for process monitoring
AmbientCapabilities=CAP_SYS_PTRACE
CapabilityBoundingSet=CAP_SYS_PTRACE

# Optional: network filtering
# AmbientCapabilities=CAP_NET_ADMIN
# CapabilityBoundingSet=CAP_NET_ADMIN
```

**CAP_SYS_PTRACE**: Required to:
- Monitor process tree
- Detect application launches
- Enforce application blocking

### File System Layout

```
/etc/dots-family/
├── daemon.toml                 # Main config (root:root, 0600)
├── profiles/
│   ├── profile1.toml          # Child profile (root:root, 0600)
│   └── profile2.toml
└── policies/
    └── default.toml           # Default policies

/var/lib/dots-family/
├── family.db                   # SQLite database (dots-family:dots-family, 0600)
├── filter-lists/              # Filter list cache
└── backups/                   # Automatic backups

/usr/lib/systemd/system/
└── dots-family-daemon.service # System service unit

/usr/lib/systemd/user/
├── dots-family-monitor.service
├── dots-family-filter.service
└── dots-wm-bridge.service
```

### DBus Configuration

**System Bus Policy** (`/etc/dbus-1/system.d/org.dots.FamilyDaemon.conf`):
```xml
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <!-- Service ownership -->
  <policy user="dots-family">
    <allow own="org.dots.FamilyDaemon"/>
  </policy>

  <!-- Root can own and control -->
  <policy user="root">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
  </policy>

  <!-- All users can send queries -->
  <policy context="default">
    <allow send_destination="org.dots.FamilyDaemon"
           send_interface="org.dots.FamilyDaemon"
           send_member="CheckApplicationAllowed"/>
    <allow send_destination="org.dots.FamilyDaemon"
           send_interface="org.dots.FamilyDaemon"
           send_member="GetTimeRemaining"/>
    <allow send_destination="org.dots.FamilyDaemon"
           send_interface="org.dots.FamilyDaemon"
           send_member="ReportActivity"/>
  </policy>

  <!-- Policy updates require authentication (via polkit) -->
  <policy context="default">
    <deny send_destination="org.dots.FamilyDaemon"
          send_interface="org.dots.FamilyDaemon"
          send_member="UpdatePolicy"/>
    <deny send_destination="org.dots.FamilyDaemon"
          send_interface="org.dots.FamilyDaemon"
          send_member="GrantException"/>
  </policy>
</busconfig>
```

## Polkit Integration

### Authentication for Administrative Actions

**Polkit Policy** (`/usr/share/polkit-1/actions/org.dots.FamilyDaemon.policy`):
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1.0/policyconfig.dtd">
<policyconfig>
  <action id="org.dots.FamilyDaemon.UpdatePolicy">
    <description>Update family mode policies</description>
    <message>Authentication is required to update family mode policies</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>auth_admin</allow_active>
    </defaults>
  </action>

  <action id="org.dots.FamilyDaemon.GrantException">
    <description>Grant temporary exception</description>
    <message>Authentication is required to grant temporary exceptions</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>auth_admin</allow_active>
    </defaults>
  </action>
</policyconfig>
```

### Rust Polkit Integration

```rust
use zbus::dbus_interface;
use polkit::Authority;

#[dbus_interface(name = "org.dots.FamilyDaemon")]
impl FamilyDaemon {
    async fn update_policy(
        &self,
        #[zbus(header)] hdr: zbus::MessageHeader<'_>,
        profile: &str,
        policy: Policy,
    ) -> Result<()> {
        // Get caller information
        let sender = hdr.sender()
            .ok_or_else(|| anyhow!("No sender"))?;

        // Check authorization via polkit
        let authority = Authority::new().await?;
        let subject = Subject::new_for_busname(sender.as_str());

        let authorized = authority.check_authorization(
            &subject,
            "org.dots.FamilyDaemon.UpdatePolicy",
            &HashMap::new(),
            CheckAuthorizationFlags::ALLOW_USER_INTERACTION,
            None,
        ).await?;

        if !authorized.is_authorized() {
            return Err(anyhow!("Not authorized"));
        }

        // Proceed with policy update
        self.policy_engine.update_policy(profile, policy).await?;

        Ok(())
    }
}
```

## Multi-User Enforcement

### Cross-User Policy Application

```rust
pub struct PolicyEngine {
    db: SqlitePool,
    // Map: system username -> profile name
    user_profiles: Arc<RwLock<HashMap<String, String>>>,
}

impl PolicyEngine {
    pub async fn check_app_allowed(
        &self,
        system_user: &str,  // e.g., "alex" (system account)
        app_id: &str,
    ) -> Result<bool> {
        // 1. Lookup profile for system user
        let profiles = self.user_profiles.read().await;
        let profile = profiles.get(system_user)
            .ok_or_else(|| anyhow!("No profile for user"))?;

        // 2. Load policy for profile
        let policy = self.load_policy(profile).await?;

        // 3. Evaluate policy
        let allowed = match policy.applications.mode {
            Mode::Allowlist => policy.applications.allowed.contains(app_id),
            Mode::Blocklist => !policy.applications.blocked.contains(app_id),
        };

        Ok(allowed)
    }
}
```

### User-to-Profile Mapping

**Configuration** (`/etc/dots-family/daemon.toml`):
```toml
[profiles]
# System username -> profile name
"alex" = "profile-alex-8-12"
"jamie" = "profile-jamie-13-17"
```

### Activity Tracking Across Users

```rust
pub async fn report_activity(
    &self,
    system_user: &str,
    activity: Activity,
) -> Result<()> {
    // Associate activity with profile
    let profiles = self.user_profiles.read().await;
    let profile = profiles.get(system_user)
        .ok_or_else(|| anyhow!("No profile for user"))?;

    // Store in database with profile association
    sqlx::query!(
        "INSERT INTO activities (profile, app_id, duration, timestamp)
         VALUES (?, ?, ?, ?)",
        profile,
        activity.app_id,
        activity.duration,
        activity.timestamp,
    )
    .execute(&self.db)
    .await?;

    Ok(())
}
```

## Security Considerations

### Preventing Child Bypass Attempts

**System Service Protection**:
```ini
[Service]
# Prevent stopping by non-root users
ProtectSystem=strict
ProtectHome=true
NoNewPrivileges=true

# Restart if killed
Restart=on-failure
RestartSec=5s
```

**File Permission Protection**:
```bash
# Config files: root-owned, not writable
chown root:root /etc/dots-family/*.toml
chmod 0600 /etc/dots-family/*.toml

# Database: service-owned, not user-accessible
chown dots-family:dots-family /var/lib/dots-family/family.db
chmod 0600 /var/lib/dots-family/family.db
```

**Process Monitoring**:
```rust
// Detect if daemon is stopped and alert
pub async fn watchdog() {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        // Check if daemon is running
        if !is_daemon_running().await {
            // Emergency alert
            send_emergency_alert().await;
        }
    }
}
```

### Audit Logging

All administrative actions logged:
```rust
pub async fn log_admin_action(
    &self,
    action: &str,
    caller: &str,
    details: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO audit_log (timestamp, action, caller, details)
         VALUES (datetime('now'), ?, ?, ?)",
        action,
        caller,
        details,
    )
    .execute(&self.db)
    .await?;

    // Also log to journald
    tracing::info!(
        action = action,
        caller = caller,
        details = details,
        "Admin action performed"
    );

    Ok(())
}
```

## Testing System Services

### VM Testing

```bash
# Build VM with system service
nix build .#nixosConfigurations.family-mode-test.config.system.build.vm

# Run VM
./result/bin/run-nixos-vm

# Inside VM, test system service
sudo systemctl status dots-family-daemon
sudo journalctl -u dots-family-daemon -f

# Test as child user
su - childuser
dots-family-ctl status  # Should work (queries allowed)
systemctl --user stop dots-family-daemon  # Should fail (system service)
```

### Integration Tests

```rust
#[tokio::test]
async fn test_system_service_enforcement() {
    // Setup: Start daemon as system service
    let daemon = spawn_system_daemon().await;

    // Setup: Create child user
    let child_user = create_test_user("childuser").await;

    // Test: Child user cannot stop daemon
    let result = run_as_user(&child_user, "systemctl stop dots-family-daemon").await;
    assert!(result.is_err());
    assert!(daemon.is_running().await);

    // Test: Policy enforcement works
    let result = daemon.check_app_allowed("childuser", "blocked-app").await;
    assert_eq!(result, false);
}
```

## NixOS Integration

### System Service Module

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
in
{
  options.services.dots-family = {
    enable = lib.mkEnableOption "DOTS Family Mode daemon";

    profiles = lib.mkOption {
      type = lib.types.attrsOf (lib.types.submodule {
        options = {
          systemUser = lib.mkOption {
            type = lib.types.str;
            description = "System username";
          };
          ageGroup = lib.mkOption {
            type = lib.types.enum ["5-7" "8-12" "13-17"];
          };
          # ... other profile options
        };
      });
      default = {};
    };
  };

  config = lib.mkIf cfg.enable {
    # Create service user
    users.users.dots-family = {
      isSystemUser = true;
      group = "dots-family";
      home = "/var/lib/dots-family";
      createHome = true;
    };
    users.groups.dots-family = {};

    # System service
    systemd.services.dots-family-daemon = {
      description = "DOTS Family Mode Daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" ];
      requires = [ "dbus.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "org.dots.FamilyDaemon";
        User = "dots-family";
        Group = "dots-family";
        ExecStart = "${pkgs.dots-family-daemon}/bin/dots-family-daemon";
        Restart = "on-failure";
        RestartSec = "5s";

        # Security
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = "/var/lib/dots-family";

        # Capabilities
        AmbientCapabilities = "CAP_SYS_PTRACE";
        CapabilityBoundingSet = "CAP_SYS_PTRACE";
      };
    };

    # DBus policy
    services.dbus.packages = [ pkgs.dots-family-daemon ];

    # Polkit policy
    security.polkit.extraConfig = ''
      polkit.addRule(function(action, subject) {
        if (action.id == "org.dots.FamilyDaemon.UpdatePolicy") {
          return subject.isInGroup("parents") ? polkit.Result.YES : polkit.Result.AUTH_ADMIN;
        }
      });
    '';

    # Generate config file
    environment.etc."dots-family/daemon.toml".text = ''
      [profiles]
      ${lib.concatStringsSep "\n" (lib.mapAttrsToList (name: profile:
        ''"${profile.systemUser}" = "${name}"''
      ) cfg.profiles)}
    '';
  };
}
```

## Related Documentation

- ARCHITECTURE.md: Overall system design
- RUST_APPLICATIONS.md: Application specifications
- IMPLEMENTATION_ROADMAP.md: Development phases
```

## Questions for Discussion

### 1. Daemon Privileges

**✅ DECIDED**: Daemon runs as dedicated user `dots-family`

**Decision**:

| Approach | Pros | Cons |
|----------|------|------|
| Root | Simple setup, all permissions | Security risk, violates least privilege |
| **Dedicated user (CHOSEN)** | Better security, limited capabilities | More complex, capability configuration |

**Implementation**: Dedicated `dots-family` user with `CAP_SYS_PTRACE` capability.

**Rationale**: Follows principle of least privilege, better security audit trail, easier to sandbox.

**Reference**: See ARCHITECTURE_DECISIONS.md for detailed analysis.

### 2. Configuration Location

**Question**: Where should system-wide config live?

**Options**:
- `/etc/dots-family/` (traditional)
- `/etc/opt/dots-family/` (optional software)
- `/usr/share/dots-family/` (shared data)

**Recommendation**: `/etc/dots-family/` for mutable config, `/usr/share/dots-family/` for defaults.

**File Layout**:
```
/etc/dots-family/
├── daemon.toml              # Main config (system-wide)
├── profiles/
│   ├── <profile>.toml      # Per-profile policies
└── policies/
    └── default.toml         # Default policies

/var/lib/dots-family/
├── family.db               # Main database
├── filter-lists/           # Filter list cache
└── backups/                # DB backups

/usr/share/dots-family/
├── templates/              # Profile templates
├── filter-lists/           # Default filter lists
└── docs/                   # Documentation
```

### 3. Per-User vs Global Settings

**Question**: Which settings should be per-user vs system-wide?

**Recommendation**:

| Setting | Scope | Rationale |
|---------|-------|-----------|
| Profile definitions | System-wide | Parents control all profiles |
| User-to-profile mapping | System-wide | Prevents reassignment |
| Time limits | Per-profile | Different for each child |
| Application lists | Per-profile | Age-appropriate |
| Web filtering | Per-profile | Age-appropriate |
| Filter lists (data) | System-wide | Shared resource |
| Parent password | System-wide | Single authentication |
| Activity database | System-wide | Centralized tracking |

### 4. Bypass Prevention

**Question**: How to prevent root users from disabling daemon?

**Answer**: Cannot fully prevent root bypass (by design), but can:

1. **Detect and Alert**:
```rust
// Monitor daemon health
pub async fn daemon_watchdog() {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        if !is_daemon_running().await {
            // Alert via email, notification, etc.
            send_alert_daemon_stopped().await;
        }
    }
}
```

2. **Audit Logging**:
```bash
# Log all root actions
auditctl -w /etc/dots-family/ -p wa -k dots-family-config
auditctl -w /var/lib/dots-family/ -p wa -k dots-family-data
```

3. **Integrity Monitoring**:
```rust
// Verify config files haven't been modified
pub async fn verify_integrity() {
    let current_hash = hash_config_files().await?;
    let stored_hash = load_stored_hash().await?;

    if current_hash != stored_hash {
        alert_config_tampering().await;
    }
}
```

4. **Parent Communication**:
- Regular reports to parent email
- Alerts on service stops
- Alerts on config changes
- Dashboard shows service status

**Note**: Root access is intentionally privileged. A parent with root can always disable controls. The goal is to make child bypass difficult while alerting parents to tampering attempts.

### 5. Emergency Access

**Question**: How should emergency parent override work with system service?

**Options**:

**Option 1: Emergency CLI**:
```bash
# Parent authentication required
sudo dots-family-ctl emergency-override --duration 1h

# Or temporary disable
sudo systemctl stop dots-family-daemon  # Requires root + alert sent
```

**Option 2: Emergency DBus Method**:
```rust
#[dbus_interface(name = "org.dots.FamilyDaemon")]
impl FamilyDaemon {
    async fn emergency_override(
        &self,
        duration_minutes: u32,
        reason: &str,
    ) -> Result<()> {
        // Requires polkit authentication
        // Logs to audit
        // Sends notification
        // Temporarily disables enforcement
    }
}
```

**Option 3: Emergency Config**:
```toml
# /etc/dots-family/daemon.toml
[emergency]
enabled = false  # Parent can edit this

# When enabled:
# - All restrictions lifted
# - Activity still logged
# - Alert sent to parent
```

**Recommendation**: Combination of options:
1. DBus method for GUI/CLI access (most user-friendly)
2. Config file as fallback (if GUI unavailable)
3. Service stop as last resort (alerts parent)

## Summary

### Critical Changes Required

1. **Architecture**: Daemon MUST be system service, not user service
2. **DBus**: MUST use system bus, not session bus
3. **Configuration**: Move from `~/.config/` to `/etc/`
4. **Database**: Move from `~/.local/share/` to `/var/lib/`
5. **Permissions**: Implement proper system-wide permissions model
6. **Documentation**: Update all architecture docs

### Implementation Priority

1. **Phase 1**: Core daemon (system service), CLI, basic monitoring (Weeks 1-6)
2. **Phase 2**: Multi-WM support (foundational for monitoring) (Weeks 7-10)
3. **Phase 3**: Terminal filtering (bypass prevention) (Weeks 11-14)
4. **Phase 4**: Web filtering (content safety) (Weeks 15-18)
5. **Phase 5**: Multi-child support (Weeks 19-21)
6. **Phase 6**: GUI dashboard (Weeks 22-26)

### Next Steps

1. Create `SYSTEMD_ARCHITECTURE.md` with detailed design
2. Update `ARCHITECTURE.md` with corrections
3. Update `RUST_APPLICATIONS.md` with system bus usage
4. Update `IMPLEMENTATION_ROADMAP.md` with new priorities
5. Begin Phase 1 implementation with correct architecture
