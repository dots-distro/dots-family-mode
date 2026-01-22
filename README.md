# DOTS Family Mode

DOTS Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Current Status: Phase 8 Complete - Production Ready

- **Daemon**: Fully functional with eBPF monitoring
- **Monitor**: Activity tracking service operational
- **CLI**: Complete administration tool
- **NixOS Integration**: Declarative module system
- **VM Testing**: Automated test framework available

## Quick Start

To get started with DOTS Family Mode, you need to have Nix installed.

### 1. Enter the development environment:
```bash
nix develop
```

### 2. Build all components:
```bash
nix build .#default
```

### 3. Build VM for testing:
```bash
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
./result/bin/run-dots-family-test-vm
```

### 4. Run the test suite:
```bash
nix run .#test
```

## Features

### Core Components
- **dots-family-daemon** - Core policy enforcement daemon with eBPF monitoring
- **dots-family-monitor** - Activity tracking service for user sessions
- **dots-family-ctl** - CLI administration tool
- **dots-family-filter** - Web content filtering proxy
- **dots-terminal-filter** - Terminal command filtering

### NixOS Integration
- Declarative configuration via Nix modules
- Systemd service integration
- DBus communication support
- eBPF kernel monitoring

### Security
- Capability-based permissions
- Filesystem protection (ProtectSystem=strict)
- Network restrictions (RestrictAddressFamilies)
- Memory protection (MemoryDenyWriteExecute)

## Documentation

For detailed information about the architecture, features, and development, please refer to the [documentation](./docs/INDEX.md).

For VM testing guide, see [VM_TESTING_GUIDE.md](./VM_TESTING_GUIDE.md).

## Commands

### Build and Test
```bash
# Build all packages
nix build .#default

# Build specific packages
nix build .#dots-family-daemon
nix build .#dots-family-monitor
nix build .#dots-family-ctl
nix build .#dots-family-filter

# Build eBPF programs
nix build .#dots-family-ebpf

# Run tests
nix run .#test

# Run clippy
nix build .#checks.x86_64-linux.clippy
```

### VM Testing
```bash
# Build VM
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm

# Start VM
./result/bin/run-dots-family-test-vm

# SSH to VM (password: root)
ssh -p 10022 root@localhost

# Run automated tests in VM
scp -P 10022 scripts/automated_user_simulation.sh root@localhost:/tmp/
ssh -p 10022 root@localhost "bash /tmp/automated_user_simulation.sh"
```

### Service Management (in VM)
```bash
# Start daemon
systemctl start dots-family-daemon.service

# Check status
systemctl status dots-family-daemon.service

# View logs
journalctl -u dots-family-daemon.service -f

# Test DBus
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon GetVersion

# CLI commands
dots-family-ctl status
dots-family-ctl profile list
dots-family-ctl session list
```

## Configuration

### NixOS Module
```nix
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    parentUsers = [ "parent" ];
    childUsers = [ "child" ];
    reportingOnly = true;  # Safe mode for testing
    
    profiles.child = {
      name = "Test Child";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "2h";
      timeWindows = [{
        start = "09:00";
        end = "17:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      allowedApplications = [ "firefox" "calculator" ];
      webFilteringLevel = "moderate";
    };
  };
}
```

### Manual Installation
```bash
# Install systemd services
sudo systemd/install.sh install

# Start services
sudo systemctl start dots-family-daemon.service
systemctl --user start dots-family-monitor.service

# Enable on boot
sudo systemctl enable dots-family-daemon.service
systemctl --user enable dots-family-monitor.service
```

## Project Structure

- **crates/** - Rust source code
  - dots-family-common - Shared types and utilities
  - dots-family-proto - DBus protocol definitions
  - dots-family-db - Database layer
  - dots-family-daemon - Core service
  - dots-family-monitor - Activity monitoring
  - dots-family-ctl - CLI tool
  - dots-family-filter - Web filtering
  - dots-terminal-filter - Terminal filtering

- **nixos-modules/** - NixOS integration
  - default.nix - Main module
  - daemon.nix - Service configuration
  - dbus.nix - DBus policies
  - security.nix - Security hardening
  - user-services.nix - User services

- **systemd/** - Systemd service files
  - dots-family-daemon.service
  - dots-family-monitor.service
  - install.sh - Installation script

- **docs/** - Documentation
  - See docs/INDEX.md for full documentation list

- **test-evidence/** - Test results and evidence

## License

AGPL-3.0-or-later
