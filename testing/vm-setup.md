# VM Testing Environment Setup

## Overview

This document provides setup instructions for creating isolated testing environments to validate the DOTS Family Mode system with real eBPF capabilities and proper DBus permissions.

## Testing Environment Options

### Option 1: NixOS Container (Recommended)

Create a lightweight NixOS container with proper privileges for eBPF testing.

#### Prerequisites
- NixOS host system
- Container support enabled
- Root access for container creation

#### Setup Steps

1. **Create Container Configuration**
```nix
# /etc/nixos/containers/dots-testing.nix
{
  containers.dots-testing = {
    autoStart = false;
    enableTun = true;
    config = { config, pkgs, ... }: {
      system.stateVersion = "23.11";
      
      # Enable eBPF support
      boot.kernelPackages = pkgs.linuxPackages_latest;
      boot.kernelModules = [ "bpf" ];
      
      # DBus system bus
      services.dbus.enable = true;
      
      # Development environment
      environment.systemPackages = with pkgs; [
        rustc
        cargo
        git
        sqlite
        sqlcipher
        pkg-config
        openssl
        libbpf
        elfutils
        clang
        llvm
      ];
      
      # User for testing
      users.users.testuser = {
        isNormalUser = true;
        extraGroups = [ "wheel" ];
        password = "test123";
      };
      
      # Allow container networking
      networking.firewall.enable = false;
      networking.dhcpcd.enable = false;
      networking.defaultGateway = "192.168.100.1";
      networking.interfaces.eth0.ipv4.addresses = [{
        address = "192.168.100.10";
        prefixLength = 24;
      }];
    };
  };
}
```

2. **Start Container**
```bash
sudo nixos-rebuild switch
sudo nixos-container start dots-testing
sudo nixos-container login dots-testing
```

### Option 2: QEMU VM with NixOS

For full isolation and kernel-level testing.

#### VM Configuration
```nix
# vm-config.nix
{ config, pkgs, ... }:
{
  imports = [ <nixpkgs/nixos/modules/virtualisation/qemu-vm.nix> ];
  
  virtualisation = {
    memorySize = 2048;
    cores = 2;
    graphics = false;
    diskSize = 8192;
  };
  
  # eBPF and monitoring support
  boot.kernelPackages = pkgs.linuxPackages_latest;
  boot.kernelModules = [ "bpf" ];
  boot.extraModulePackages = with config.boot.kernelPackages; [ ];
  
  # System services
  services.dbus.enable = true;
  systemd.services.dbus.wantedBy = [ "multi-user.target" ];
  
  # Development tools
  environment.systemPackages = with pkgs; [
    rustc
    cargo
    git
    sqlite
    sqlcipher
    pkg-config
    openssl
    libbpf
    elfutils
    clang
    llvm
    htop
    strace
    tcpdump
    lsof
  ];
  
  # Test user
  users.users.dots = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    password = "test";
  };
  
  # Networking
  networking.hostName = "dots-test-vm";
  networking.firewall.enable = false;
}
```

#### Build and Run VM
```bash
nix-build '<nixpkgs/nixos>' -A vm -I nixos-config=vm-config.nix
./result/bin/run-nixos-vm
```

## Testing Scenarios

### 1. eBPF Functionality Testing

**Test Cases:**
- Process monitoring with real eBPF hooks
- Network connection tracking  
- Filesystem access monitoring
- Fallback mechanism validation

**Validation Script:**
```bash
#!/bin/bash
# test-ebpf.sh

echo "Testing eBPF functionality..."

# Test with root privileges
sudo cargo run -p dots-family-daemon &
DAEMON_PID=$!

sleep 5

# Generate test activity
echo "Generating test processes..."
sleep 10 &
dd if=/dev/zero of=/tmp/test-file bs=1M count=10 &
curl -s https://httpbin.org/get > /dev/null &

sleep 5

# Check monitoring data via DBus
echo "Checking monitoring data..."
# TODO: Add DBus monitoring snapshot calls

kill $DAEMON_PID
echo "eBPF test completed"
```

### 2. Integration Testing

**Full System Workflow:**
1. Start daemon with root privileges
2. Launch monitor service
3. Connect CLI tool
4. Generate controlled activity
5. Verify data flow: eBPF → Daemon → Database → CLI

**Integration Test Script:**
```bash
#!/bin/bash
# integration-test.sh

set -e

echo "Starting integration tests..."

export DATABASE_URL="sqlite:///tmp/integration-test.db"

# Clean environment
rm -f /tmp/integration-test.db
killall dots-family-daemon || true

# Start daemon
echo "Starting daemon..."
sudo -E cargo run -p dots-family-daemon &
DAEMON_PID=$!

sleep 5

# Test CLI functionality
echo "Testing CLI functionality..."
cargo run -p dots-family-ctl -- status
cargo run -p dots-family-ctl -- profile list

# Generate test activity
echo "Generating test activity..."
firefox &
FIREFOX_PID=$!
sleep 10
kill $FIREFOX_PID || true

# Check data collection
echo "Verifying data collection..."
cargo run -p dots-family-ctl -- status

# Cleanup
kill $DAEMON_PID
echo "Integration tests completed"
```

### 3. Performance Testing

**Load Testing:**
- High-frequency process creation
- Network connection stress testing
- Large file operations
- Memory usage monitoring

**Performance Test Script:**
```bash
#!/bin/bash
# performance-test.sh

echo "Starting performance tests..."

# Start monitoring
sudo -E cargo run -p dots-family-daemon &
DAEMON_PID=$!

# CPU stress test
for i in {1..50}; do
    sleep 1 &
done

# Network stress test
for i in {1..20}; do
    curl -s https://httpbin.org/get > /dev/null &
done

# File I/O stress test
for i in {1..10}; do
    dd if=/dev/zero of=/tmp/stress-$i bs=1M count=10 &
done

# Monitor resource usage
echo "Monitoring for 30 seconds..."
sleep 30

# Cleanup
kill $DAEMON_PID
rm -f /tmp/stress-*
echo "Performance tests completed"
```

## DBus Testing

### Policy File Installation

For testing DBus service ownership, install policy files:

```xml
<!-- /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf -->
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
  
  <policy group="wheel">
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
  
  <policy context="default">
    <deny send_destination="org.dots.FamilyDaemon"/>
  </policy>
</busconfig>
```

### DBus Testing Commands

```bash
# Test service registration
dbus-send --system --print-reply --dest=org.freedesktop.DBus \
  /org/freedesktop/DBus org.freedesktop.DBus.ListNames

# Test daemon communication
dbus-send --system --print-reply --dest=org.dots.FamilyDaemon \
  /org/dots/FamilyDaemon org.dots.FamilyDaemon.get_monitoring_snapshot
```

## Automated Testing Pipeline

### Test Runner Script

```bash
#!/bin/bash
# run-all-tests.sh

set -e

TEST_DIR="$(dirname "$0")"
cd "$TEST_DIR/.."

echo "Running DOTS Family Mode Test Suite"
echo "==================================="

# Compilation tests
echo "1. Testing compilation..."
export DATABASE_URL="sqlite:///tmp/test-family.db"
cargo build --workspace --release

# Unit tests
echo "2. Running unit tests..."
cargo test --workspace

# Integration tests (requires VM/container)
if [ "$1" = "--integration" ]; then
    echo "3. Running integration tests..."
    ./testing/integration-test.sh
    
    echo "4. Running performance tests..."
    ./testing/performance-test.sh
fi

echo "All tests completed successfully!"
```

## Environment Variables

```bash
# Testing configuration
export DOTS_TEST_MODE=1
export DATABASE_URL="sqlite:///tmp/test-family.db"
export RUST_LOG=debug
export RUST_BACKTRACE=1

# eBPF testing
export EBPF_TEST_MODE=1
export DOTS_EBPF_FALLBACK=0  # Force eBPF mode for testing
```

## Validation Checklist

- [ ] Container/VM boots successfully
- [ ] eBPF modules load correctly
- [ ] Daemon starts without errors
- [ ] Database migrations apply
- [ ] Monitor collects real data
- [ ] CLI tools connect to daemon
- [ ] DBus communication works
- [ ] Performance meets requirements
- [ ] Error handling functions correctly
- [ ] Resource usage is acceptable

## Troubleshooting

### Common Issues

1. **eBPF Load Failures**
   - Check kernel version: `uname -r`
   - Verify CAP_SYS_ADMIN: `capsh --print`
   - Check BPF filesystem: `mount | grep bpf`

2. **DBus Permission Errors**
   - Verify policy file installation
   - Check DBus service status: `systemctl status dbus`
   - Test with root privileges

3. **Database Issues**
   - Verify SQLCipher installation
   - Check file permissions
   - Validate migration files

### Debug Commands

```bash
# Check eBPF support
ls /sys/kernel/debug/tracing/

# Monitor DBus activity
dbus-monitor --system

# Check process capabilities
getpcaps $$

# Verify database
sqlite3 /tmp/test-family.db ".tables"
```