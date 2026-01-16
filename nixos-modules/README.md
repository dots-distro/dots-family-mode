# DOTS Family Mode NixOS Modules

This directory contains NixOS modules for integrating DOTS Family Mode into NixOS systems. These modules provide declarative configuration for all parental control components.

## Overview

The DOTS Family Mode NixOS integration consists of four main modules:

- **default.nix** - Main module with configuration options
- **daemon.nix** - System daemon service configuration
- **dbus.nix** - DBus policies and service files
- **security.nix** - Polkit rules and security hardening
- **user-services.nix** - Per-user services and desktop integration

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

## Configuration Options

### Main Configuration

```nix
services.dots-family = {
  enable = true;                    # Enable DOTS Family Mode
  databasePath = "/var/lib/dots-family/family.db";  # Database location
  reportingOnly = false;            # Set to true for monitoring without enforcement
  
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

See `example-configuration.nix` for a complete working example.