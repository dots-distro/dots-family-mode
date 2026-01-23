# DOTS Family Mode - Security Architecture

## Architecture Validation

The pivot to a **System Service** architecture is critical for security. Expert review confirms this is the standard pattern for Linux control systems (similar to NetworkManager, upower).

### Why System Service is Mandatory

Running as user service creates fatal vulnerabilities:

1. **Process Control**: Child can `systemctl --user stop` the daemon
2. **Resource Competition**: OOM killer may prioritize killing user services during heavy resource usage
3. **Multi-User Bypass**: Cannot enforce rules on other user accounts child might create
4. **Permission Scope**: User services lack authority to enforce system-level restrictions

### Validated Architecture Pattern

```
System Bus (Root Authority)
    ↓
dots-family-daemon (System Service, UID: dots-family)
    ↓ DBus IPC
Per-User Agents:
    - dots-family-monitor (User Session)
    - dots-wm-bridge (Wayland Compositor IPC)
    - dots-family-filter (Network Proxy)
```

## Critical Attack Vectors & Mitigations

### 1. "Kill the Messenger" Attack

**Attack**: Child kills `dots-family-monitor` or `dots-wm-bridge` process to stop activity reporting.

**Risk**: If daemon interprets silence as "idle", child gains infinite screen time.

**Mitigation - Fail-Closed Design**:
```rust
// Daemon must expect heartbeats
struct MonitorHeartbeat {
    last_seen: Instant,
    session_active: bool, // Check via loginctl
}

impl MonitorHeartbeat {
    fn is_tampered(&self) -> bool {
        // If graphical session active but no heartbeat for 30s
        self.session_active && self.last_seen.elapsed() > Duration::from_secs(30)
    }
}

// On tamper detection:
// 1. Assume ACTIVE usage (continue deducting time)
// 2. Trigger "Tamper Lock" via dots-family-lockscreen
// 3. Log audit event
```

**Implementation Status**: Not yet implemented (Phase 1 priority)

### 2. Time Travel (NTP Bypass)

**Attack**: Child changes system clock to rewind time or jump forward.

**Risk**: Time limits become meaningless if based on wall-clock time.

**Mitigation - Monotonic Clock**:
```rust
// NEVER use SystemTime::now() for duration tracking
use std::time::Instant; // Monotonic, cannot be rewound

struct SessionTracking {
    start_monotonic: Instant,
    start_realtime: SystemTime,
    // On startup: detect clock jumps by comparing offset
}

// Persist monotonic + realtime offset in DB
// On startup: if realtime jumped but monotonic didn't = clock manipulation
```

**Implementation Status**: Not yet implemented (Critical for Phase 1)

### 3. Network Filter Bypass

**Attack**:
- Install static binary (Tor Browser, custom game client)
- Use non-HTTP protocols
- Bypass HTTP proxy entirely

**Risk**: `dots-family-filter` as HTTP proxy can be circumvented.

**Mitigation - Kernel-Level Enforcement**:
```bash
# NFTables rules (requires CAP_NET_ADMIN on daemon)
# Force ALL traffic from child's UID through proxy
nft add rule ip filter OUTPUT meta skuid $CHILD_UID tcp dport != 8080 drop
nft add rule ip filter OUTPUT meta skuid $CHILD_UID udp dport != 53 drop

# Allow only:
# - Proxy port (8080)
# - DNS to NextDNS (53)
# Drop everything else
```

**Implementation Status**: Not yet designed (Phase 2)

**Requirements**:
- Daemon needs `CAP_NET_ADMIN` capability
- Must handle network namespace isolation
- Fail-safe: If rules cannot be applied, lock network entirely

### 4. Process Injection / Memory Modification

**Attack**: Advanced users might use `ptrace` or `LD_PRELOAD` to modify monitor behavior.

**Mitigation**:
```rust
// Daemon monitors its child processes
// Requires CAP_SYS_PTRACE

// Check for suspicious parent PIDs
// Verify monitor binary hash on startup
// Detect LD_PRELOAD in monitor environment
```

**Implementation Status**: Not yet designed (Phase 3)

### 5. Secondary Boot / Live USB

**Attack**: Boot from USB drive to bypass system entirely.

**Mitigation**:
- BIOS/UEFI password (physical security)
- Secure Boot enforcement
- Disk encryption (prevents tampering with system files)
- Boot order locked in firmware

**Implementation Status**: Deployment concern, not code

## NixOS Integration - Security Hardening

### Dedicated System User

```nix
users.users.dots-family = {
  isSystemUser = true;
  group = "dots-family";
  home = "/var/lib/dots-family";
  createHome = true;
  shell = pkgs.shadow; # No shell access
};
```

### System Service with Capabilities

The daemon runs as a dedicated, unprivileged `dots-family` system user with specific capabilities, following the principle of least privilege. It does **not** run as root.

```nix
systemd.services.dots-family-daemon = {
  serviceConfig = {
    User = "dots-family";

    # Security Capabilities
    AmbientCapabilities = [
      "CAP_SYS_PTRACE"  # Monitor child processes
      # Future: "CAP_NET_ADMIN" for firewall rules
    ];
    CapabilityBoundingSet = [ "CAP_SYS_PTRACE" ];

    # Filesystem Isolation
    ProtectSystem = "strict";
    ProtectHome = true;  # Cannot see /home
    ReadWritePaths = [ "/var/lib/dots-family" ];

    # Network Access
    PrivateNetwork = false;  # Needs network for future filtering

    # Prevent Tampering
    Restart = "always";  # Auto-restart if killed
  };
};
```

### DBus Security Policy

```xml
<!-- System Bus: /share/dbus-1/system.d/org.dots.FamilyDaemon.conf -->
<busconfig>
  <!-- Only dots-family user can own the name -->
  <policy user="dots-family">
    <allow own="org.dots.FamilyDaemon"/>
  </policy>

  <!-- Root can administer -->
  <policy user="root">
    <allow send_destination="org.dots.FamilyDaemon"/>
  </policy>

  <!-- Regular users: read-only access -->
  <policy context="default">
    <!-- Allow queries -->
    <allow send_destination="org.dots.FamilyDaemon"
           send_member="CheckApplicationAllowed"/>
    <allow send_destination="org.dots.FamilyDaemon"
           send_member="GetTimeRemaining"/>

    <!-- Deny policy changes -->
    <deny send_destination="org.dots.FamilyDaemon"
          send_member="UpdatePolicy"/>
  </policy>
</busconfig>
```

## Implementation Priorities - REVISED

### Phase 1 (Critical Security Foundation)
1. **dots-family-daemon** (System Service)
2. **dots-family-lockscreen** (Wayland) ← MOVED FROM PHASE 2
3. **Heartbeat mechanism** (Fail-closed enforcement)
4. **Monotonic time tracking** (Anti-time-travel)

**Rationale**: Without lockscreen, daemon is toothless. Must be able to enforce "time up" immediately.

### Phase 2 (Granular Monitoring)
1. **dots-wm-bridge** (Wayland compositor IPC)
2. **dots-family-monitor** (Activity tracking)
3. **Application-level enforcement**

### Phase 3 (Network Security)
1. **dots-family-filter** (HTTP proxy)
2. **NFTables integration** (Kernel-level enforcement)
3. **Network namespace isolation**

## Deployment Strategy (Remote System)

### Failsafe Rollout

Since physical access is impossible:

1. **Tailscale Priority**:
   ```nix
   # Ensure Tailscale is NOT filtered
   systemd.services.tailscaled.after = [ "network.target" ];
   # Mark Tailscale traffic to bypass filtering
   ```

2. **Reporting-Only Mode First**:
   ```toml
   # /var/lib/dots-family/config.toml
   [enforcement]
   enabled = false  # Week 1: Just log, don't enforce
   reporting_only = true
   ```

3. **SSH Override**:
   ```rust
   // In lockscreen logic
   fn should_lock(user: &str) -> bool {
       if user == "root" {
           return false; // Never lock root via SSH
       }
       // Normal enforcement
   }
   ```

4. **Gradual Activation**:
   - Week 1: Reporting only, verify logs
   - Week 2: Enable soft limits (warnings)
   - Week 3: Enable hard limits (lockscreen)

## Database Security

### SQLCipher Encryption

```rust
// Database must be encrypted at rest
DatabaseConfig {
    path: "/var/lib/dots-family/family.db",
    encryption_key: Some(derived_from_hardware_id()),
}

// Prevent child from:
// 1. Reading activity history
// 2. Modifying time limits
// 3. Creating fake parent account
```

### Key Derivation

```rust
// Derive encryption key from hardware ID
// Child cannot copy database to another machine
use sha2::{Sha256, Digest};

fn derive_db_key() -> String {
    let machine_id = fs::read_to_string("/etc/machine-id").unwrap();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.trim());
    hasher.update(b"dots-family-v1");
    format!("{:x}", hasher.finalize())
}
```

## Audit Logging

All security-sensitive operations must be logged:

```rust
enum AuditEvent {
    DaemonStarted,
    DaemonStopped { reason: String },
    MonitorHeartbeatLost,
    ClockJumpDetected { before: SystemTime, after: SystemTime },
    LockscreenTriggered { reason: String },
    PolicyUpdated { by: String, changes: Vec<String> },
    TamperAttempt { details: String },
}

// Logs stored in append-only table
// Child cannot delete history
```

## Next Implementation Tasks

1. ✅ ActivityTracker with duration (Phase 1, Task 4-5)
2. ⏳ Heartbeat mechanism with fail-closed logic (Task 6 - in progress)
3. ⏳ Monotonic time tracking (Task 11)
4. ⏳ Lockscreen integration (Move to Phase 1)
5. 🔲 Clock jump detection
6. 🔲 NFTables integration (Phase 3)
7. 🔲 Process monitoring with CAP_SYS_PTRACE (Phase 3)

## Security Review Checklist

Before production deployment:

- [ ] Daemon runs as system service (not user service)
- [ ] DBus policies enforce read-only access for users
- [ ] Heartbeat mechanism with fail-closed behavior
- [ ] Monotonic clock used for all duration tracking
- [ ] Clock jump detection implemented
- [ ] Database encrypted with hardware-derived key
- [ ] Lockscreen cannot be killed by child
- [ ] Audit log is append-only
- [ ] Tailscale exempt from filtering
- [ ] SSH root access always available
- [ ] Reporting-only mode tested for 1 week
- [ ] NixOS module handles capability grants
- [ ] systemd hardening applied (ProtectSystem, ProtectHome)

## References

- Expert review: 2026-01-12
- Original architecture: `docs/REVIEW_AND_PRIORITIES.md`
- System service pattern: Similar to NetworkManager, upower, systemd-logind
