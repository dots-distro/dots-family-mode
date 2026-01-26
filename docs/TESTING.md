# DOTS Family Mode - Testing Guide

This document describes the testing strategy and available testing tools for DOTS Family Mode.

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Types](#test-types)
3. [Running Tests](#running-tests)
4. [Test Environments](#test-environments)
5. [Continuous Integration](#continuous-integration)
6. [Writing Tests](#writing-tests)
7. [Troubleshooting](#troubleshooting)

---

## Testing Philosophy

DOTS Family Mode follows a pragmatic testing approach that balances thorough coverage with developer productivity:

**Key Principles:**
- **Fast Feedback**: Most tests should complete in seconds, not minutes
- **Layered Testing**: Multiple test types for different purposes
- **CI/CD Ready**: Automated tests that run reliably in CI environments
- **Developer Friendly**: Easy to run locally without complex setup
- **Production Validation**: Tests that verify actual deployment behavior

**Test Pyramid:**
```
        /\
       /  \      E2E Tests (CI only, slow)
      /    \
     /------\    Integration Tests (occasional, medium)
    /        \
   /----------\  Unit Tests (always, fast)
  /__________  \ Smoke Tests (deployment validation, instant)
```

---

## Test Types

### 1. Unit Tests (Primary Development Tool)

**Purpose**: Test individual components in isolation  
**Runtime**: ~90 seconds  
**When to Use**: Always, before every commit  

**Run Command:**
```bash
cargo test --workspace --lib --bins
```

**Coverage:**
- Policy engine logic
- Profile management
- Time window calculations
- Application filtering
- Web filtering rules
- Terminal command analysis
- Database operations
- DBus protocol
- Window manager adapters

**Statistics:**
- 216 tests total
- 21 ignored (integration-only)
- 100% pass rate maintained

**Example:**
```bash
# Run all tests
cargo test --workspace --lib --bins

# Run specific package tests
cargo test -p dots-family-daemon

# Run with output
cargo test -- --nocapture

# Run single test
cargo test test_time_window_active
```

### 2. Smoke Tests (Deployment Validation)

**Purpose**: Quick validation that system is correctly installed  
**Runtime**: < 5 seconds  
**When to Use**: After deployment, before going live  

**Run Command:**
```bash
./tests/smoke-test.sh
```

**What It Checks:**
- Binary availability (`dots-family-daemon`, `dots-family-ctl`)
- eBPF programs present and valid format
- (Future) Database directory exists
- (Future) Systemd service installed
- (Future) DBus service registered

**Exit Codes:**
- `0`: All critical tests passed
- `1`: One or more tests failed

**Example Output:**
```
DOTS Family Mode - Smoke Test
==============================

[PASS] daemon binary exists
[PASS] ctl binary exists
[PASS] eBPF programs present

Passed: 3 / 3
```

**When to Run:**
1. After installing via NixOS configuration
2. After system updates
3. Before enabling on production systems
4. During troubleshooting

### 3. Integration Tests (Occasional Use)

**Purpose**: Test interactions between components  
**Runtime**: Variable (5-30 minutes)  
**When to Use**: Before releases, after major changes  

**Available Tests:**
- `tests/nix/full-deployment-test.nix` - Complete NixOS deployment
- `tests/bdd/` - Behavior-driven tests
- Shell scripts in `tests/` - Various integration scenarios

**Warning**: Full VM tests are resource-intensive and should be run on CI or dedicated test machines, not during development.

**Run Command:**
```bash
# BDD tests
cargo test --package dots-family-bdd-tests

# Nix deployment test (WARNING: slow, ~15-30 min)
nix build .#checks.x86_64-linux.nixos-deployment --print-build-logs
```

### 4. eBPF Program Tests

**Purpose**: Verify eBPF programs compile and have valid format  
**Runtime**: ~25 seconds  
**When to Use**: After modifying eBPF code  

**Run Command:**
```bash
# Build eBPF programs
cd crates/dots-family-ebpf
export PATH="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH"
cargo build --release --target bpfel-unknown-none -Z build-std=core

# Verify output
file target/bpfel-unknown-none/release/process-monitor
# Should output: "ELF 64-bit LSB relocatable, eBPF, version 1 (SYSV), not stripped"
```

**Update Prebuilt Binaries:**
```bash
cp target/bpfel-unknown-none/release/*-monitor ../../prebuilt-ebpf/
```

### 5. Manual Testing (Real-World Validation)

**Purpose**: Verify actual functionality in real-world scenarios  
**When to Use**: Before releases, for UX validation  

**Test Scenarios:**
1. **Profile Creation**: Create parent and child users, configure profiles
2. **Time Limits**: Verify screen time limits work correctly
3. **Application Blocking**: Test that blocked apps cannot launch
4. **Web Filtering**: Check that inappropriate sites are blocked
5. **Approval Workflow**: Request and grant approval for blocked content
6. **Terminal Filtering**: Verify command filtering in shell
7. **Multi-WM Support**: Test with Sway, Hyprland, Niri

**Manual Test Checklist:**
- [ ] Daemon starts successfully
- [ ] DBus service is available
- [ ] CLI commands work (`dots-family-ctl status`, `list-profiles`, etc.)
- [ ] eBPF programs load without errors
- [ ] GUI dashboard displays correctly
- [ ] Logs show no critical errors
- [ ] Profile changes take effect immediately
- [ ] Time limits enforce correctly

---

## Running Tests

### Quick Start (Developer Workflow)

```bash
# 1. Run unit tests (always)
cargo test --workspace --lib --bins

# 2. Build eBPF programs (if changed)
nix build .#dots-family-ebpf

# 3. Run smoke test (after installation)
./tests/smoke-test.sh

# 4. Manual testing on actual NixOS
# Deploy to test machine and verify functionality
```

### Full Test Suite (Pre-Release)

```bash
# 1. Unit tests
cargo test --workspace --lib --bins

# 2. eBPF build test
nix build .#dots-family-ebpf

# 3. All package builds
nix build .#dots-family-daemon .#dots-family-monitor .#dots-family-ctl

# 4. BDD integration tests
cargo test --package dots-family-bdd-tests

# 5. Smoke test
./tests/smoke-test.sh

# 6. Manual testing on NixOS
# Follow manual test checklist
```

### CI/CD Pipeline (Automated)

```bash
# Minimal CI (fast, runs on every commit)
- cargo test --workspace --lib --bins
- cargo clippy --all-targets --all-features
- cargo fmt --all -- --check
- nix flake check --no-build

# Full CI (comprehensive, runs on PRs)
- All minimal CI checks
- nix build .#dots-family-daemon (and all packages)
- nix build .#checks.x86_64-linux.nixos-deployment
- Integration tests
```

---

## Test Environments

### 1. Local Development Machine

**Best For**: Unit tests, eBPF compilation, smoke tests

**Requirements:**
- Rust toolchain (stable)
- Rustup with nightly (for eBPF)
- Nix with flakes enabled
- 8GB+ RAM
- NixOS (for smoke tests)

**Limitations:**
- Cannot run full VM integration tests (too resource-intensive)
- Some features require actual NixOS deployment

### 2. NixOS Test Machine

**Best For**: Manual testing, end-to-end validation

**Requirements:**
- NixOS 23.11 or later
- Wayland compositor (Sway, Hyprland, or Niri)
- Multiple user accounts (parent and child)
- Root access for daemon configuration

**Setup:**
```nix
# /etc/nixos/configuration.nix
{
  imports = [ ./dots-family-mode/nixos-modules/dots-family ];
  
  services.dots-family = {
    enable = true;
    parentUsers = [ "parent" ];
    childUsers = [ "child" ];
  };
}
```

### 3. CI Environment (GitHub Actions)

**Best For**: Automated testing, regression detection

**Configuration:** (see `.github/workflows/ci.yml`)

**Resources:**
- 2-4 CPU cores
- 8GB RAM
- Nix caching enabled
- Matrix testing across NixOS versions

---

## Continuous Integration

### Recommended CI Strategy

**On Every Commit:**
1. Unit tests
2. Linting (clippy, rustfmt)
3. Flake check
4. Build all packages

**On Pull Requests:**
1. All commit checks
2. Integration tests (BDD)
3. VM deployment test (if resources allow)
4. Manual review checklist

**On Releases:**
1. All PR checks
2. Full VM integration test
3. Manual testing on actual hardware
4. Security audit review
5. Documentation review

### GitHub Actions Example

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v22
        with:
          nix_path: nixpkgs=channel:nixos-unstable
      
      - name: Run unit tests
        run: |
          nix develop --command cargo test --workspace --lib --bins
      
      - name: Build packages
        run: |
          nix build .#dots-family-daemon
          nix build .#dots-family-ebpf
      
      - name: Run smoke test
        run: |
          nix develop --command ./tests/smoke-test.sh
```

---

## Writing Tests

### Unit Test Guidelines

**Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

**Best Practices:**
1. **Descriptive Names**: `test_time_window_allows_access_during_allowed_hours`
2. **Arrange-Act-Assert**: Clear test structure
3. **One Assertion Per Test**: Focus on single behavior
4. **Use Test Fixtures**: Reusable test data
5. **Mock External Dependencies**: Don't rely on system state
6. **Test Edge Cases**: Boundary values, empty inputs, etc.

**Example:**
```rust
#[test]
fn test_profile_validates_age_group() {
    let valid_groups = vec!["5-7", "8-12", "13-17"];
    
    for group in valid_groups {
        let profile = Profile {
            age_group: group.to_string(),
            ..Default::default()
        };
        assert!(profile.validate().is_ok());
    }
    
    let invalid = Profile {
        age_group: "invalid".to_string(),
        ..Default::default()
    };
    assert!(invalid.validate().is_err());
}
```

### Integration Test Guidelines

**Location**: `tests/` directory or `tests/bdd/`

**Structure:**
```rust
#[test]
fn test_daemon_cli_communication() {
    // Start daemon (in test mode)
    let daemon = start_test_daemon();
    
    // Execute CLI command
    let output = run_cli_command(&["status"]);
    
    // Verify communication worked
    assert!(output.contains("Daemon running"));
    
    // Cleanup
    daemon.stop();
}
```

### Smoke Test Guidelines

**Location**: `tests/smoke-test.sh`

**Format:**
```bash
#!/usr/bin/env bash
set -e

# Test N: Description
tests_total=$((tests_total + 1))
if [ condition ]; then
    echo "[PASS] test description"
    tests_passed=$((tests_passed + 1))
else
    echo "[FAIL] test description"
fi
```

**Rules:**
1. Must complete in < 10 seconds
2. Exit code 0 for success, non-zero for failure
3. No external dependencies (beyond Nix/NixOS)
4. Idempotent (can run multiple times)
5. Clear pass/fail indicators

---

## Troubleshooting

### Common Test Failures

#### 1. "Command not found: dots-family-daemon"

**Cause**: Binaries not installed or not in PATH

**Fix:**
```bash
# Check installation
ls /run/current-system/sw/bin/dots-family*

# Rebuild NixOS configuration
sudo nixos-rebuild switch

# Or build in development
nix develop
```

#### 2. "eBPF program loading failed"

**Cause**: eBPF programs not built or incorrect format

**Fix:**
```bash
# Rebuild eBPF programs
cd crates/dots-family-ebpf
export PATH="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH"
cargo build --release --target bpfel-unknown-none -Z build-std=core

# Verify format
file target/bpfel-unknown-none/release/process-monitor

# Update prebuilt
cp target/bpfel-unknown-none/release/*-monitor ../../prebuilt-ebpf/
```

#### 3. "Test timed out"

**Cause**: System under load or test hanging

**Fix:**
- Close resource-intensive applications
- Increase test timeout
- Run tests one at a time
- Check for background processes

#### 4. "Permission denied" errors

**Cause**: Tests require root or specific user

**Fix:**
```bash
# Run with sudo if needed
sudo ./tests/smoke-test.sh

# Or configure polkit rules
# See docs/DEPLOYMENT.md for details
```

#### 5. Unit tests fail after eBPF changes

**Cause**: LSP errors are harmless for eBPF, but code changes may have real issues

**Check:**
- Compile eBPF programs successfully
- Verify binary format is valid eBPF
- LSP errors in eBPF files are expected (no_std environment)
- Real errors will prevent compilation

### Test Performance Issues

**VM Tests Taking Too Long:**
- Run on CI, not locally
- Use smoke tests for quick validation
- Build packages individually first to leverage caching

**Unit Tests Slow:**
- Run specific package: `cargo test -p package-name`
- Use `--lib` to skip integration tests
- Parallel execution: `cargo test -- --test-threads=4`

**eBPF Compilation Slow:**
- Use prebuilt binaries during development
- Only recompile when eBPF code changes
- Cache Nix builds: `cachix use dots-family` (if available)

---

## Test Coverage

### Current Coverage (as of Session 8)

**Unit Tests:**
- Policy Engine: ✅ Comprehensive
- Profile Management: ✅ Comprehensive  
- Time Windows: ✅ Comprehensive
- Application Filtering: ✅ Good
- Web Filtering: ✅ Good
- Terminal Filtering: ✅ Comprehensive
- Database: ✅ Good
- DBus Protocol: ✅ Basic
- Window Manager: ✅ Good

**Integration Tests:**
- CLI ↔ Daemon: ✅ Basic
- Daemon ↔ Monitor: ⏳ Partial
- eBPF Loading: ⏳ Partial
- End-to-End Workflows: ⏳ Limited

**Manual Tests:**
- Basic Functionality: ✅ Verified
- Multi-WM Support: ⏳ Partial
- Edge Cases: ⏳ Limited

### Coverage Goals

**Short-term:**
- [ ] Increase DBus protocol test coverage
- [ ] Add eBPF data extraction verification
- [ ] More end-to-end workflow tests

**Long-term:**
- [ ] Automated GUI testing
- [ ] Performance regression tests
- [ ] Security vulnerability scanning
- [ ] Stress testing (high load scenarios)

---

## Resources

### Documentation
- [DEPLOYMENT.md](DEPLOYMENT.md) - Installation and configuration
- [USER_GUIDE.md](USER_GUIDE.md) - Parent-facing guide
- [EBPF_ENHANCEMENTS.md](EBPF_ENHANCEMENTS.md) - eBPF development

### Tools
- **cargo-test**: Rust test runner
- **nix build**: Build packages
- **nix develop**: Development shell
- **systemctl**: Service management
- **journalctl**: Log viewing

### External Resources
- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Nix Testing Guide](https://nixos.org/manual/nixos/stable/#sec-nixos-tests)
- [eBPF Testing](https://ebpf.io/what-is-ebpf/#testing)

---

## Quick Reference

```bash
# Daily development
cargo test --workspace --lib --bins        # Unit tests
./tests/smoke-test.sh                      # Smoke test

# After eBPF changes
cd crates/dots-family-ebpf
cargo build --release --target bpfel-unknown-none -Z build-std=core
cp target/bpfel-unknown-none/release/*-monitor ../../prebuilt-ebpf/

# Before commits
cargo test --workspace --lib --bins        # All tests pass
cargo clippy --all-targets                  # No warnings
cargo fmt --all                              # Code formatted

# Pre-release
cargo test --workspace                      # All tests
nix build .#dots-family-daemon             # Builds succeed
./tests/smoke-test.sh                       # Smoke test passes
# Manual testing on NixOS                   # Real-world validation
```

---

**Last Updated**: Session 8 (January 26, 2026)  
**Maintainer**: DOTS Family Mode Team
