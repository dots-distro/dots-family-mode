# DOTS Family Mode

DOTS Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Current Status: Phase 3 eBPF Complete - Production Ready Monitoring

### eBPF Monitoring System ✅
- **5 Production eBPF Monitors** (27.4KB total)
  - `process-monitor` (4.8K): Process exec/exit with PPID and executable paths
  - `filesystem-monitor` (6.8K): File open/read/write/close with full paths
  - `network-monitor` (5.5K): TCP connect/send/recv with bandwidth tracking
  - `memory-monitor` (5.7K): Memory allocations (kmalloc/kfree, page alloc/free)
  - `disk-io-monitor` (4.6K): Block I/O with nanosecond latency tracking
- **16 Probe Functions**: Tracepoints and kprobes for comprehensive monitoring
- **Advanced Features**: HashMap-based latency tracking, bandwidth monitoring
- **All Tests Passing**: 216 unit tests (100% pass rate)

### Core System
- **Daemon**: Fully functional with eBPF monitoring integration ready
- **Monitor**: Activity tracking service operational
- **CLI**: Complete administration tool
- **NixOS Integration**: Declarative module system
- **VM Testing**: Automated test framework available

## Known Limitations

- **Browser Testing**: Playwright-based browser tests are limited in the NixOS development environment due to browser binary compatibility issues. For full browser testing capabilities, use the VM environment.
- **eBPF Kernel Version**: Requires kernel struct offsets (no BTF/CO-RE in aya-ebpf 0.1)
- **IPv6**: Network monitoring currently IPv4 only

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

### eBPF Monitoring (Phase 3 Complete)
- **Process Monitoring**: Exec/exit events with PPID, UID/GID, executable paths
- **Network Monitoring**: TCP connect/send/recv with socket details and bandwidth tracking
- **Filesystem Monitoring**: File operations with full paths and byte counts
- **Memory Monitoring**: Kernel memory allocations (kmalloc/kfree, page alloc/free)
- **Disk I/O Monitoring**: Block device I/O with latency measurement using HashMap
- **Real-time Tracking**: 16 probe functions capturing system-wide activity
- **Low Overhead**: 27.4KB total eBPF code, efficient kernel-space monitoring

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
    
    # Optional: Custom paths
    databasePath = "/var/lib/dots-family/family.db";
    ebpfPackage = pkgs.dots-family-ebpf;  # Enable eBPF monitoring
    
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

### Environment Variables

The daemon supports configuration via environment variables:
- `DOTS_FAMILY_DB_PATH` - Database file location
- `DOTS_FAMILY_CONFIG_DIR` - Configuration directory
- `BPF_NETWORK_MONITOR_PATH` - eBPF network monitor (if enabled)
- `BPF_FILESYSTEM_MONITOR_PATH` - eBPF filesystem monitor (if enabled)

See [NIXOS_INTEGRATION.md](./docs/NIXOS_INTEGRATION.md#environment-variables) for details.

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
  - dots-family-ebpf - eBPF programs
  - dots-family-gui - Graphical user interface
  - dots-wm-bridge - Window manager integration

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

- **scripts/** - Utility scripts and automation
  - ci/ - CI/CD scripts
  - legacy/ - Legacy scripts
  - setup/ - Setup and installation scripts
  - tests/ - Test automation scripts

- **docs/** - Documentation
  - See docs/INDEX.md for full documentation list

- **test-evidence/** - Test results and evidence

## License

AGPL-3.0-or-later
