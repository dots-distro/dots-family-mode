# DOTS Family Mode NixOS Modules

This directory contains NixOS modules for integrating DOTS Family Mode into NixOS systems. These modules provide declarative configuration for all parental control components with **enhanced eBPF monitoring** and **secure DBus policies**.

## Overview

The DOTS Family Mode NixOS integration consists of five main modules:

- **default.nix** - Main module with configuration options
- **daemon.nix** - System daemon service with eBPF capabilities
- **dbus.nix** - Enhanced DBus policies with role-based security
- **security.nix** - Polkit rules, eBPF configuration, and security hardening
- **user-services.nix** - Per-user services and desktop integration

## New Features (Phase 8 Integration)

### eBPF Monitoring Integration
- **Kernel-level monitoring**: Process, network, and filesystem activity tracking
- **Enhanced performance**: JIT compilation and optimized collection
- **Security hardening**: Proper capability management for eBPF operations
- **Fallback support**: Graceful degradation when eBPF is unavailable

### Advanced DBus Security
- **Role-based access**: Separate permissions for parents, children, and monitors
- **Method-level control**: Granular permissions for each daemon function
- **Signal management**: Secure policy update and notification delivery
- **Monitor isolation**: Per-user monitoring with secure reporting

### Production-Ready Security
- **Capability isolation**: Minimal privileges for each component
- **Resource limits**: Memory and process restrictions for child users
- **AppArmor profiles**: Optional additional security layer
- **Tamper resistance**: Protected configuration and database access

## Quick Start

### 1. Add to your flake inputs

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dots-family-mode.url = "github:dots-distro/dots-family-mode";
  };
  
  outputs = { self, nixpkgs, dots-family-mode }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        dots-family-mode.nixosModules.default
      ];
    };
  };
}
```

### 2. Enable and configure in your NixOS configuration

```nix
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    parentUsers = [ "parent1" "parent2" ];
    childUsers = [ "child1" "child2" ];
    
    profiles.child1 = {
      name = "Alice";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "2h30m";
      timeWindows = [{
        start = "15:00";
        end = "18:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      allowedApplications = [ "firefox" "tuxmath" ];
      webFilteringLevel = "strict";
    };
  };
}
```

### 3. Create user accounts

```nix
users.users = {
  parent1 = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    hashedPassword = "...";
  };
  child1 = {
    isNormalUser = true;
    hashedPassword = "...";
  };
};
```

## Package Management

The DOTS Family Mode module uses **standard NixOS package building** - no overlays required:

### Automatic Package Building
- Packages are built automatically using `rustPlatform.buildRustPackage`
- All dependencies are handled automatically
- No external overlay dependencies
- Standard NixOS approach

### Optional Package Overrides
You can still override packages if needed:
```nix
services.dots-family = {
  enable = true;
  package = your-custom-daemon-package;
  monitorPackage = your-custom-monitor-package;
  ctlPackage = your-custom-ctl-package;
  # ... other config
};
```

```nix
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    parentUsers = [ "parent1" "parent2" ];
    childUsers = [ "child1" "child2" ];
    
    profiles.child1 = {
      name = "Alice";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "2h30m";
      timeWindows = [{
        start = "15:00";
        end = "18:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      allowedApplications = [ "firefox" "tuxmath" ];
      webFilteringLevel = "strict";
    };
  };
}
```

### 3. Create user accounts

```nix
users.users = {
  parent1 = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    hashedPassword = "...";
  };
  child1 = {
    isNormalUser = true;
    hashedPassword = "...";
  };
};
```

## Configuration Options

### Main Configuration

```nix
services.dots-family = {
  enable = true;                    # Enable DOTS Family Mode
  databasePath = "/var/lib/dots-family/family.db";  # Database location
  reportingOnly = false;            # Set to true for monitoring without enforcement
  runAsRoot = false;               # Run daemon as root (true) or dedicated user (false, default)
  
  # User groups
  parentUsers = [ "mom" "dad" ];    # Users with full control
  childUsers = [ "alice" "bob" ];   # Users subject to restrictions
  
  # Feature toggles
  enableWebFiltering = true;        # Web content filtering
  enableTerminalFiltering = false;  # Terminal command filtering (experimental)
  enableNotifications = true;       # Desktop notifications
};
```

### Profile Configuration

```nix
services.dots-family.profiles.<username> = {
  name = "Display Name";           # Human-readable name
  ageGroup = "8-12";              # "5-7", "8-12", "13-17", or "custom"
  dailyScreenTimeLimit = "2h30m"; # Daily time limit (optional)
  
  # Time windows when computer access is allowed
  timeWindows = [{
    start = "09:00";              # 24-hour format
    end = "17:00";
    days = [ "mon" "tue" "wed" "thu" "fri" ];  # Weekdays
  }];
  
  # Application restrictions
  allowedApplications = [ "firefox" "calculator" ];
  blockedApplications = [ "steam" "discord" ];
  
  # Web filtering level
  webFilteringLevel = "strict";   # "strict", "moderate", "minimal", "disabled"
};
```

### Age Group Defaults

The system provides sensible defaults based on age groups:

- **5-7**: Very restrictive, educational apps only, 1-hour daily limit
- **8-12**: Moderate restrictions, some games allowed, 2-hour daily limit  
- **13-17**: Lighter restrictions, social apps allowed, 4-hour daily limit
- **custom**: No defaults, fully manual configuration

## Security Features

### Authentication
- Parent users can manage all settings without additional authentication
- Child users can only view status and request exceptions
- All privileged operations require parent group membership

### System Hardening
- Daemon runs with minimal privileges and filesystem isolation
- Database encryption at rest with SQLCipher
- AppArmor profiles for additional security (when enabled)
- Resource limits for child user accounts

### DBus Security
- Strict DBus policies limiting child access to daemon
- Separate session bus configuration for monitors
- Service activation with proper user isolation

## Services and Components

### System Services
- **dots-family-daemon.service** - Core policy enforcement daemon
- Runs as dedicated system user with restricted permissions
- Automatically starts on boot and handles policy enforcement

### User Services  
- **dots-family-monitor.service** - Per-user activity monitoring
- **dots-family-gui.service** - Parent dashboard (for parent users)
- Automatically started in user sessions

### CLI Tools
- `family` command alias for `dots-family-ctl`
- Bash completion for all commands
- Desktop launcher for GUI access

## File Locations

### System Files
- `/var/lib/dots-family/` - Database and daemon state
- `/var/log/dots-family/` - Log files
- `/etc/dots-family/` - Configuration files (auto-generated)

### User Files
- `~/.config/dots-family/` - User-specific settings
- `/tmp/dots-family-<user>/` - Temporary monitoring data

## Troubleshooting

### Check Service Status
```bash
systemctl status dots-family-daemon.service
systemctl --user status dots-family-monitor.service
```

### View Logs
```bash
journalctl -u dots-family-daemon.service
journalctl --user -u dots-family-monitor.service
```

### Test Configuration
```bash
family status                    # Check overall status
family profile list             # List all profiles
family check firefox            # Check if app is allowed
```

### Common Issues

1. **DBus errors**: Ensure DBus is running and user is in correct groups
2. **Permission denied**: Check that users are in `dots-family-parents` or `dots-family-children` groups
3. **Monitor not starting**: Check that compositor is supported (Niri/Sway/Hyprland)

## Development and Testing

## Testing and Validation

### Quick Integration Test
```bash
# Build and run test VM with all features enabled
nix build -f integration-test.nix system
./result/bin/run-*-vm

# In the VM, run the integration test
sudo /etc/dots-family/integration-test.sh
```

### eBPF Capability Testing
```bash
# Test eBPF support before deployment
sudo /path/to/ebpf-config/configure-ebpf-capabilities.sh

# Validate system compatibility
/path/to/ebpf-config/test-ebpf-capabilities.sh
```

### DBus Security Validation
```bash
# Test DBus policies as different users
sudo -u testparent dbus-send --system --print-reply --dest=org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon.list_profiles
sudo -u testchild dbus-send --system --print-reply --dest=org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon.create_profile string:"test" string:"8-12"  # Should fail
```

### VM Testing
```bash
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
./result/bin/run-*-vm
```

### Module Development
The modules are organized for maintainability:
- Each component has its own module file
- Shared configuration passed via `internal` options
- Security policies isolated in dedicated module
- eBPF configuration integrated with kernel parameters
- DBus policies generated from role-based templates

## Production Deployment

### Prerequisites
- NixOS 23.05+ (for systemd eBPF support)
- Kernel 4.4+ with eBPF enabled
- 2GB+ RAM for eBPF monitoring
- DBus system and session buses

### Deployment Steps
1. **Add module to flake inputs** (see Quick Start above)
2. **Configure users and profiles** in NixOS configuration
3. **Deploy and activate** configuration
4. **Test eBPF functionality** with provided scripts
5. **Validate security policies** with test commands

### Production Monitoring
```bash
# Monitor daemon health
systemctl status dots-family-daemon.service
journalctl -u dots-family-daemon.service -f

# Check eBPF programs
sudo bpftool prog list | grep family
sudo bpftool map list | grep family

# Validate DBus security
dbus-monitor --system "interface=org.dots.FamilyDaemon"
```

## Integration Files

- **integration-test.nix** - Complete VM test configuration with all features
- **../dbus-policies/** - Enhanced DBus security policies (referenced by module)
- **../ebpf-config/** - eBPF kernel configuration and validation scripts
- **../systemd/** - Enhanced systemd services (referenced by module)

See `integration-test.nix` for a complete working example with test users and validation scripts.