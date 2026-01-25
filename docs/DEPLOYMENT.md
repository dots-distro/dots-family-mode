# DOTS Family Mode - Production Deployment Guide

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Initial Deployment](#initial-deployment)
5. [Profile Management](#profile-management)
6. [Monitoring and Logs](#monitoring-and-logs)
7. [Troubleshooting](#troubleshooting)
8. [Security Considerations](#security-considerations)
9. [Upgrading](#upgrading)

---

## Prerequisites

### System Requirements

- **Operating System**: NixOS 23.11 or later
- **Kernel**: Linux 6.1+ (for eBPF support)
- **Architecture**: x86_64-linux (ARM support planned)
- **Memory**: Minimum 512MB RAM for daemon
- **Disk Space**: 100MB for binaries + variable database size

### Required Kernel Features

The following kernel features must be enabled:

```bash
# Check eBPF support
zgrep CONFIG_BPF /proc/config.gz | grep -v "^#"

# Required features:
# CONFIG_BPF=y
# CONFIG_BPF_SYSCALL=y
# CONFIG_BPF_JIT=y
# CONFIG_HAVE_EBPF_JIT=y
```

NixOS kernels include these by default.

---

## Quick Start

### Step 1: Add to Flake Inputs

Add DOTS Family Mode to your system flake:

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dots-family-mode = {
      url = "github:yourusername/dots-family-mode";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, dots-family-mode, ... }: {
    nixosConfigurations.yourhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        dots-family-mode.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

### Step 2: Enable in Configuration

Add to your `configuration.nix`:

```nix
# configuration.nix
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    
    # IMPORTANT: Start in reporting-only mode
    reportingOnly = true;
    
    # Keep Tailscale exempt for remote access
    tailscaleExempt = true;
    
    # Database location
    databasePath = "/var/lib/dots-family/family.db";
  };
}
```

### Step 3: Rebuild and Test

```bash
# Rebuild system
sudo nixos-rebuild switch

# Verify services are running
systemctl status dots-family-daemon.service
systemctl status dots-family-monitor@$USER.service

# Check logs
journalctl -u dots-family-daemon.service -f
```

---

## Configuration

### Basic Configuration Options

```nix
services.dots-family = {
  enable = true;
  
  # Reporting mode (logs only, no enforcement)
  reportingOnly = true;  # Set to false for enforcement
  
  # Database configuration
  databasePath = "/var/lib/dots-family/family.db";
  
  # Network exemptions
  tailscaleExempt = true;  # Exempt Tailscale from filtering
  
  # Profiles (optional - can also manage via CLI)
  profiles = {
    alice = {
      dailyLimitMinutes = 120;  # 2 hours
      allowedApplications = [
        "firefox"
        "chromium"
        "code"
        "gimp"
      ];
      timeWindows = {
        weekday = [
          { start = "16:00"; end = "18:00"; }  # After school
          { start = "19:00"; end = "20:00"; }  # After dinner
        ];
        weekend = [
          { start = "09:00"; end = "12:00"; }
          { start = "14:00"; end = "18:00"; }
        ];
      };
    };
  };
};
```

### Advanced Configuration

#### Enable Web Filtering

```nix
services.dots-family = {
  enable = true;
  
  webFiltering = {
    enable = true;
    bindAddress = "127.0.0.1";
    port = 3128;
    
    # Safe search enforcement
    safeSearchEnforcement = true;
    
    # Category-based blocking
    blockedCategories = [
      "adult"
      "gambling"
      "violence"
    ];
    
    # Domain allowlist
    allowedDomains = [
      "*.edu"
      "*.gov"
      "khanacademy.org"
      "scratch.mit.edu"
    ];
  };
};
```

#### Configure eBPF Monitoring

```nix
services.dots-family = {
  enable = true;
  
  monitoring = {
    # Network monitoring
    networkMonitoring = {
      enable = true;
      trackConnections = true;
      logDeniedConnections = true;
    };
    
    # Process monitoring
    processMonitoring = {
      enable = true;
      trackExecution = true;
    };
    
    # Filesystem monitoring
    filesystemMonitoring = {
      enable = true;
      monitoredPaths = [
        "/home"
        "/tmp"
      ];
    };
  };
};
```

---

## Initial Deployment

### Phase 1: Observation (Week 1)

Deploy in **reporting-only mode** to establish baseline:

```nix
services.dots-family = {
  enable = true;
  reportingOnly = true;  # Logs activity, no blocking
};
```

**Activities:**
1. Deploy configuration
2. Monitor logs for 7 days
3. Review activity patterns
4. Identify legitimate applications
5. Note any issues or false positives

**Commands:**
```bash
# View all activity
sudo dots-family-ctl activity list --all

# Get activity for specific user
sudo dots-family-ctl activity list --user alice

# Export activity report
sudo dots-family-ctl activity export --days 7 > activity-report.json
```

### Phase 2: Soft Enforcement (Week 2)

Enable enforcement with generous limits:

```nix
services.dots-family = {
  enable = true;
  reportingOnly = false;  # Enable enforcement
  
  profiles.alice = {
    dailyLimitMinutes = 240;  # 4 hours (generous)
    # No time windows yet
    allowedApplications = [
      # Add all legitimate apps from Phase 1
    ];
  };
};
```

**Activities:**
1. Enable enforcement with high limits
2. Monitor for issues
3. Adjust allow lists as needed
4. Communicate with family members

### Phase 3: Full Enforcement (Week 3+)

Tighten limits to target values:

```nix
services.dots-family = {
  enable = true;
  reportingOnly = false;
  
  profiles.alice = {
    dailyLimitMinutes = 120;  # Target: 2 hours
    timeWindows = {
      weekday = [
        { start = "16:00"; end = "18:00"; }
      ];
    };
  };
};
```

---

## Profile Management

### CLI Profile Management

#### Create Profile

```bash
# Create profile for child
sudo dots-family-ctl profile create \
  --name "alice" \
  --age-group "late-elementary" \
  --daily-limit 120

# Create with username binding
sudo dots-family-ctl profile create \
  --name "bob" \
  --age-group "early-elementary" \
  --username "bob" \
  --daily-limit 90
```

#### List Profiles

```bash
# List all profiles
sudo dots-family-ctl profile list

# Get profile details
sudo dots-family-ctl profile get alice
```

#### Update Profile

```bash
# Update time limit
sudo dots-family-ctl profile update alice \
  --daily-limit 150

# Add time window
sudo dots-family-ctl profile add-window alice \
  --type weekday \
  --start "16:00" \
  --end "18:00"

# Add allowed application
sudo dots-family-ctl profile allow-app alice firefox
```

#### Activate Profile

```bash
# Activate profile (start enforcement)
sudo dots-family-ctl profile activate alice

# Deactivate profile (pause enforcement)
sudo dots-family-ctl profile deactivate alice
```

### GUI Profile Management

Launch the parent dashboard:

```bash
dots-family-gui
```

Features:
- Visual profile configuration
- Real-time activity monitoring
- Screen time graphs
- Approval request handling

---

## Monitoring and Logs

### Service Status

```bash
# Check daemon status
systemctl status dots-family-daemon.service

# Check monitor status (per-user)
systemctl --user status dots-family-monitor.service

# View all DOTS Family services
systemctl list-units | grep dots-family
```

### Logs

#### System Logs

```bash
# Follow daemon logs
journalctl -u dots-family-daemon.service -f

# Last 100 lines with timestamp
journalctl -u dots-family-daemon.service -n 100 -o short-iso

# Logs since boot
journalctl -u dots-family-daemon.service -b

# Logs for specific user's monitor
journalctl --user -u dots-family-monitor.service -f
```

#### Activity Logs

```bash
# View recent activity
sudo dots-family-ctl activity list --recent

# Activity for specific profile
sudo dots-family-ctl activity list --profile alice

# Activity for date range
sudo dots-family-ctl activity list \
  --start "2026-01-20" \
  --end "2026-01-25"

# Export activity data
sudo dots-family-ctl activity export \
  --format json \
  --output activity-export.json
```

### Database Queries

Direct database access (advanced):

```bash
# Connect to database
sudo sqlite3 /var/lib/dots-family/family.db

# Example queries:
sqlite> SELECT * FROM profiles;
sqlite> SELECT * FROM activities ORDER BY timestamp DESC LIMIT 10;
sqlite> SELECT profile_id, SUM(duration_seconds)/60 as minutes 
        FROM activities 
        WHERE date(timestamp) = date('now') 
        GROUP BY profile_id;
```

---

## Troubleshooting

### Common Issues

#### 1. Service Won't Start

**Symptom:** `systemctl status dots-family-daemon` shows failed

**Solution:**
```bash
# Check logs for error
journalctl -u dots-family-daemon.service -n 50

# Common causes:
# - Database permission issues
sudo chown dots-family:dots-family /var/lib/dots-family/
sudo chmod 750 /var/lib/dots-family/

# - eBPF loading failure (needs CAP_BPF or CAP_SYS_ADMIN)
# Verify in NixOS config:
# systemd.services.dots-family-daemon.serviceConfig.AmbientCapabilities
```

#### 2. eBPF Programs Not Loading

**Symptom:** Logs show "Failed to load eBPF program"

**Solution:**
```bash
# Check kernel eBPF support
zgrep CONFIG_BPF /proc/config.gz

# Check capabilities
systemctl show dots-family-daemon.service | grep Capabilities

# Verify eBPF programs exist in Nix store
nix-store -qR $(which dots-family-daemon) | grep ebpf

# Check for errors in kernel log
dmesg | grep -i bpf
```

#### 3. DBus Communication Failure

**Symptom:** CLI commands fail with "Failed to connect to DBus"

**Solution:**
```bash
# Verify DBus service is registered
busctl list | grep dots.family

# Check DBus policies
cat /etc/dbus-1/system.d/dots-family.conf

# Test DBus connection
busctl call com.dots.family /com/dots/family com.dots.family.Daemon1 GetStatus
```

#### 4. Monitor Not Starting for User

**Symptom:** User monitor service fails

**Solution:**
```bash
# Check user service status
systemctl --user status dots-family-monitor.service

# Restart user service
systemctl --user restart dots-family-monitor.service

# Check for XDG_RUNTIME_DIR
echo $XDG_RUNTIME_DIR

# Verify user in correct group
groups $USER | grep dots-family-users
```

#### 5. Web Filtering Not Working

**Symptom:** Blocked sites still accessible

**Solution:**
```bash
# Check proxy is running
netstat -tlnp | grep 3128

# Test proxy directly
curl -x http://127.0.0.1:3128 http://blocked-site.example

# Verify browser proxy settings
# Firefox: about:preferences#general → Network Settings
# Must be set to "Manual proxy configuration" with SOCKS5

# Check filter rules
sudo dots-family-ctl filter list-rules
```

### Debug Mode

Enable verbose logging:

```nix
services.dots-family = {
  enable = true;
  debug = true;  # Enable debug logging
};
```

Or temporarily:
```bash
# Set log level to trace
sudo systemctl set-environment RUST_LOG=dots_family=trace
sudo systemctl restart dots-family-daemon.service

# View trace logs
journalctl -u dots-family-daemon.service -f
```

---

## Security Considerations

### Privilege Separation

DOTS Family Mode uses a **least-privilege architecture**:

- **Daemon**: Runs as `dots-family` system user with limited capabilities
- **Monitor**: Runs as regular user, communicates via DBus
- **eBPF**: Loaded with CAP_BPF only (no root required)

### Capability Requirements

Required Linux capabilities:

```nix
# In NixOS module
systemd.services.dots-family-daemon.serviceConfig = {
  AmbientCapabilities = [
    "CAP_BPF"              # Load eBPF programs
    "CAP_NET_ADMIN"        # Network filtering
    "CAP_SYS_PTRACE"       # Process monitoring (optional)
  ];
  
  # Drop all other capabilities
  CapabilityBoundingSet = [
    "CAP_BPF"
    "CAP_NET_ADMIN"
  ];
};
```

### Database Encryption

Database is stored in `/var/lib/dots-family/` with restricted permissions:

```bash
# Verify permissions
ls -la /var/lib/dots-family/
# Should show: drwx------ dots-family dots-family

# Backup database securely
sudo tar czf family-backup.tar.gz /var/lib/dots-family/
sudo chmod 600 family-backup.tar.gz
```

### Remote Administration

**CRITICAL**: Always exempt remote access tools:

```nix
services.dots-family = {
  tailscaleExempt = true;  # Never block Tailscale
  
  # Or add to allowlist
  profiles.alice.allowedApplications = [
    "tailscaled"
    "ssh"
  ];
};
```

### Parent Authentication

Set parent password for approvals:

```bash
# Set password
sudo dots-family-ctl auth set-password

# Test authentication
sudo dots-family-ctl auth test
```

Password is hashed with Argon2id and stored in database.

---

## Upgrading

### Upgrade Process

```bash
# 1. Backup database
sudo systemctl stop dots-family-daemon.service
sudo cp /var/lib/dots-family/family.db /var/lib/dots-family/family.db.backup

# 2. Update flake input
nix flake update dots-family-mode

# 3. Rebuild system
sudo nixos-rebuild switch

# 4. Verify services restarted
systemctl status dots-family-daemon.service

# 5. Test functionality
sudo dots-family-ctl profile list
```

### Database Migration

DOTS Family Mode automatically migrates database schema on startup.

Check migration status:

```bash
# View migration logs
journalctl -u dots-family-daemon.service | grep -i migration

# Manual migration (if needed)
sudo dots-family-ctl db migrate --dry-run
sudo dots-family-ctl db migrate
```

### Rollback

If upgrade fails:

```bash
# Rollback NixOS generation
sudo nixos-rebuild switch --rollback

# Restore database backup
sudo systemctl stop dots-family-daemon.service
sudo cp /var/lib/dots-family/family.db.backup /var/lib/dots-family/family.db
sudo systemctl start dots-family-daemon.service
```

---

## Production Checklist

Before enabling enforcement:

- [ ] eBPF programs loading successfully
- [ ] Database initialized and accessible
- [ ] DBus policies configured
- [ ] CLI commands working
- [ ] Profiles created and configured
- [ ] Tested in reporting-only mode for 1 week
- [ ] Family members informed and trained
- [ ] Remote access exemption configured
- [ ] Parent password set
- [ ] Backup strategy established
- [ ] Monitoring alerts configured
- [ ] Documented custom rules and exemptions

---

## Support and Resources

### Documentation

- Architecture: `docs/ARCHITECTURE.md`
- Security: `docs/SECURITY_ARCHITECTURE.md`
- NixOS Integration: `docs/NIXOS_INTEGRATION.md`
- Monitoring: `docs/MONITORING.md`
- Content Filtering: `docs/CONTENT_FILTERING.md`

### Community

- GitHub Issues: Report bugs and feature requests
- Discussions: Ask questions and share experiences

### Professional Support

For production deployments, consider:
- Custom profile development
- Integration with existing systems
- Advanced filtering rules
- Training and onboarding

---

## Example: Complete Deployment

Here's a complete example configuration:

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    reportingOnly = false;  # Enforcement enabled
    tailscaleExempt = true;
    
    databasePath = "/var/lib/dots-family/family.db";
    
    profiles = {
      alice = {
        username = "alice";
        dailyLimitMinutes = 120;
        weekendBonusMinutes = 60;
        
        allowedApplications = [
          "firefox"
          "chromium"
          "code"
          "gimp"
          "inkscape"
          "blender"
        ];
        
        timeWindows = {
          weekday = [
            { start = "16:00"; end = "18:00"; }
            { start = "19:30"; end = "20:30"; }
          ];
          weekend = [
            { start = "09:00"; end = "12:00"; }
            { start = "14:00"; end = "19:00"; }
          ];
          holiday = [
            { start = "09:00"; end = "20:00"; }
          ];
        };
      };
      
      bob = {
        username = "bob";
        dailyLimitMinutes = 90;
        weekendBonusMinutes = 30;
        
        allowedApplications = [
          "firefox"
          "scratch-desktop"
          "tuxpaint"
        ];
        
        timeWindows = {
          weekday = [
            { start = "16:00"; end = "17:30"; }
          ];
          weekend = [
            { start = "10:00"; end = "12:00"; }
            { start = "14:00"; end = "16:00"; }
          ];
        };
      };
    };
    
    webFiltering = {
      enable = true;
      safeSearchEnforcement = true;
      blockedCategories = [ "adult" "gambling" "violence" ];
    };
  };
  
  # Ensure users exist
  users.users.alice = {
    isNormalUser = true;
    extraGroups = [ "dots-family-users" ];
  };
  
  users.users.bob = {
    isNormalUser = true;
    extraGroups = [ "dots-family-users" ];
  };
}
```

Deploy:

```bash
sudo nixos-rebuild switch
sudo dots-family-ctl profile activate alice
sudo dots-family-ctl profile activate bob
```

---

## License

DOTS Family Mode is open source software. See LICENSE file for details.
