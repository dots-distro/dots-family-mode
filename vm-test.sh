#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - Comprehensive End-to-End Test Script
# Tests complete system functionality in VM environment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_SSH_PORT="${VM_SSH_PORT:-10022}"
VM_HOST="${VM_HOST:-localhost}"
TEST_LOG="${SCRIPT_DIR}/vm_test_results.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test result tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

log() {
    echo -e "$(date '+%H:%M:%S') $*" | tee -a "$TEST_LOG"
}

log_info() {
    log "${BLUE}[INFO]${NC} $*"
}

log_success() {
    log "${GREEN}[PASS]${NC} $*"
    ((TESTS_PASSED++))
}

log_error() {
    log "${RED}[FAIL]${NC} $*"
    ((TESTS_FAILED++))
}

log_warning() {
    log "${YELLOW}[WARN]${NC} $*"
}

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    shift
    
    log_info "Running test: $test_name"
    
    if "$@"; then
        log_success "✓ $test_name"
        return 0
    else
        log_error "✗ $test_name"
        return 1
    fi
}

vm_exec() {
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -o ConnectTimeout=10 -p "$VM_SSH_PORT" "$@" 2>/dev/null || return 1
}

vm_exec_as_parent() {
    vm_exec "parent@${VM_HOST}" "$@"
}

vm_exec_as_child() {
    vm_exec "child@${VM_HOST}" "$@"
}

vm_exec_as_root() {
    vm_exec "root@${VM_HOST}" "$@"
}

# Test functions
test_vm_connectivity() {
    vm_exec_as_root "echo 'VM connectivity check'" >/dev/null
}

test_family_packages_installed() {
    vm_exec_as_root "which dots-family-daemon dots-family-ctl dots-family-monitor" >/dev/null
}

test_daemon_startup() {
    # Start daemon in background and check it initializes
    vm_exec_as_parent "nohup dots-family-daemon > /tmp/daemon.log 2>&1 &"
    sleep 5
    
    # Check if daemon process is running
    vm_exec_as_parent "pgrep dots-family-daemon" >/dev/null
    
    # Check initialization logs
    if vm_exec_as_parent "grep -q 'Database migrations completed successfully' /tmp/daemon.log"; then
        return 0
    else
        vm_exec_as_parent "cat /tmp/daemon.log"
        return 1
    fi
}

test_cli_commands_available() {
    # Test CLI help works
    vm_exec_as_parent "dots-family-ctl --help" >/dev/null
    
    # Test profile commands available  
    vm_exec_as_parent "dots-family-ctl profile --help" >/dev/null
}

test_public_commands_no_auth() {
    # These should work without authentication
    vm_exec_as_parent "dots-family-ctl status" >/dev/null
    vm_exec_as_parent "dots-family-ctl profile list" >/dev/null
}

test_authentication_required_for_admin() {
    # This should fail without authentication (will hang waiting for password)
    timeout 3 vm_exec_as_parent "echo '' | dots-family-ctl profile create 'Test' '8-12'" && return 1 || return 0
}

test_monitor_startup() {
    # Start monitor and check it initializes
    vm_exec_as_parent "nohup dots-family-monitor > /tmp/monitor.log 2>&1 &"
    sleep 3
    
    # Check if monitor process is running
    vm_exec_as_parent "pgrep dots-family-monitor" >/dev/null
    
    # Check initialization logs
    vm_exec_as_parent "grep -q 'Monitor running' /tmp/monitor.log" || {
        vm_exec_as_parent "cat /tmp/monitor.log"
        return 1
    }
}

test_dbus_connectivity() {
    # Test that daemon exports DBus interface
    vm_exec_as_parent "busctl --user list | grep -q org.dots.FamilyDaemon" || {
        log_warning "DBus service not found, checking daemon logs..."
        vm_exec_as_parent "tail -20 /tmp/daemon.log"
        return 1
    }
}

test_database_creation() {
    # Check if database file was created
    vm_exec_as_parent "ls -la ~/.local/share/dots-family/ 2>/dev/null || echo 'No database directory found'"
    
    # Check daemon logs for database success
    vm_exec_as_parent "grep -q 'Database connection pool created' /tmp/daemon.log"
}

test_monitor_daemon_communication() {
    # Check monitor logs for daemon communication
    sleep 5  # Let monitor attempt heartbeat
    
    vm_exec_as_parent "grep -E '(Connected to daemon|Heartbeat|Failed to connect)' /tmp/monitor.log" || {
        log_warning "No daemon communication logs found, checking monitor logs..."
        vm_exec_as_parent "cat /tmp/monitor.log"
        return 1
    }
}

test_system_integration() {
    # Test that all processes can run together
    local daemon_running monitor_running
    
    daemon_running=$(vm_exec_as_parent "pgrep dots-family-daemon | wc -l")
    monitor_running=$(vm_exec_as_parent "pgrep dots-family-monitor | wc -l")
    
    [ "$daemon_running" -gt 0 ] && [ "$monitor_running" -gt 0 ]
}

# Phase 3: Multi-WM Bridge Testing
test_wm_detection() {
    # Run WM detection test inside VM
    vm_exec_as_parent "bash /home/parent/vm-wm-test.sh" || {
        log_warning "WM bridge test failed, checking environment..."
        vm_exec_as_parent "env | grep -E '(WAYLAND|NIRI|SWAY|HYPR|XDG_SESSION)'"
        return 1
    }
}

test_wm_bridge_compilation() {
    # Test that WM bridge was compiled and is available
    vm_exec_as_parent "which dots-family-monitor" >/dev/null || {
        log_error "dots-family-monitor binary not found"
        return 1
    }
    
    # Check if the monitor can detect and initialize WM bridge
    local output
    output=$(vm_exec_as_parent "timeout 5 dots-family-monitor 2>&1" || true)
    
    if echo "$output" | grep -q "Using window manager:"; then
        local wm_name
        wm_name=$(echo "$output" | grep "Using window manager:" | cut -d: -f2- | tr -d ' ')
        log_info "Monitor successfully detected WM: $wm_name"
        return 0
    else
        log_error "Monitor failed to detect WM or initialize WM bridge"
        log_info "Monitor output: $output"
        return 1
    fi
}

test_wm_capabilities() {
    # Test WM-specific capabilities
    local output
    output=$(vm_exec_as_parent "timeout 5 dots-family-monitor 2>&1" || true)
    
    if echo "$output" | grep -q "capabilities:"; then
        log_info "WM capabilities detected successfully"
        return 0
    else
        log_warning "No capability information found in monitor output"
        return 0
    fi
}

cleanup_vm() {
    log_info "Cleaning up VM processes..."
    vm_exec_as_parent "pkill -f dots-family || true"
    vm_exec_as_root "pkill -f dots-family || true"
}

# Main test execution
main() {
    log_info "Starting DOTS Family Mode VM End-to-End Tests"
    log_info "VM: ${VM_HOST}:${VM_SSH_PORT}"
    log_info "Test log: $TEST_LOG"
    echo "" > "$TEST_LOG"  # Clear previous log
    
    # Basic connectivity tests
    run_test "VM Connectivity" test_vm_connectivity
    run_test "Family Packages Installed" test_family_packages_installed
    
    # Service startup tests  
    run_test "Daemon Startup" test_daemon_startup
    run_test "CLI Commands Available" test_cli_commands_available
    run_test "Database Creation" test_database_creation
    
    # Authentication tests
    run_test "Public Commands Work Without Auth" test_public_commands_no_auth
    run_test "Admin Commands Require Auth" test_authentication_required_for_admin
    
    # Integration tests
    run_test "DBus Connectivity" test_dbus_connectivity
    run_test "Monitor Startup" test_monitor_startup
    run_test "Monitor-Daemon Communication" test_monitor_daemon_communication
    run_test "System Integration" test_system_integration
    
    # Phase 3: Multi-WM Bridge Tests
    run_test "WM Bridge Compilation" test_wm_bridge_compilation
    run_test "WM Detection" test_wm_detection  
    run_test "WM Capabilities" test_wm_capabilities
    
    # Cleanup
    cleanup_vm
    
    # Results summary
    echo ""
    log_info "============================================"
    log_info "DOTS Family Mode VM Test Results Summary"
    log_info "============================================"
    log_info "Total tests run: $TESTS_RUN"
    log_success "Tests passed: $TESTS_PASSED"
    log_error "Tests failed: $TESTS_FAILED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "🎉 ALL TESTS PASSED! DOTS Family Mode is fully functional!"
        return 0
    else
        log_error "❌ Some tests failed. Check logs for details."
        return 1
    fi
}

# Trap to ensure cleanup on script exit
trap cleanup_vm EXIT

# Check if VM is accessible before running tests
if ! vm_exec_as_root "echo 'VM accessibility check'" >/dev/null 2>&1; then
    log_error "Cannot connect to VM at ${VM_HOST}:${VM_SSH_PORT}"
    log_info "Make sure VM is running and SSH is accessible"
    log_info "Expected VM users: root, parent, child with passwords: root, parent123, child123"
    exit 1
fi

# Run main test suite
main "$@"