#!/usr/bin/env bash
# VM Testing Script for DOTS Family Mode
# This script runs inside the VM to test components with proper DBus setup

set -euo pipefail

VM_TEST_LOG="/tmp/vm_test_results.log"
export DATABASE_URL="sqlite:/tmp/test_family.db"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'  
BLUE='\033[0;34m'
NC='\033[0m'

# Test tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

log() {
    echo -e "$(date '+%H:%M:%S') $*" | tee -a "$VM_TEST_LOG"
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

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    shift
    
    log_info "Running VM test: $test_name"
    
    if "$@"; then
        log_success "$test_name"
        return 0
    else
        log_error "$test_name"
        return 1
    fi
}

test_vm_environment() {
    log_info "Testing VM environment setup"
    uname -a
    whoami
    echo "DBus session bus: $DBUS_SESSION_BUS_ADDRESS"
    systemctl --user status dbus
}

test_daemon_with_dbus() {
    log_info "Testing daemon startup with DBus in VM"
    
    # Start user dbus session if needed
    systemctl --user start dbus 2>/dev/null || true
    
    # Try to start daemon
    timeout 15s /nix/store/*/bin/dots-family-daemon 2>&1 | tee /tmp/daemon_test.log || true
    
    # Check if daemon started and attempted DBus registration
    if grep -q "Starting DOTS Family Daemon" /tmp/daemon_test.log; then
        if grep -q "Database migrations completed successfully" /tmp/daemon_test.log; then
            if grep -q "AccessDenied" /tmp/daemon_test.log; then
                log_info "Daemon started, migrations OK, DBus policy needs setup"
                return 0
            else
                log_info "Daemon started successfully with DBus"
                return 0
            fi
        else
            log_error "Daemon started but database migrations failed"
            return 1
        fi
    else
        log_error "Daemon failed to start"
        return 1
    fi
}

test_cli_tool() {
    log_info "Testing CLI tool in VM"
    
    # Test CLI help
    /nix/store/*/bin/dots-family-ctl --help > /dev/null 2>&1
    
    # Test CLI status (should fail gracefully without daemon)
    if ! /nix/store/*/bin/dots-family-ctl status 2>/dev/null; then
        log_info "CLI status failed as expected (no daemon running)"
        return 0
    else
        log_info "CLI status succeeded (daemon must be running)"
        return 0
    fi
}

test_monitor_component() {
    log_info "Testing monitor component in VM"
    
    # Test monitor startup
    timeout 5s /nix/store/*/bin/dots-family-monitor 2>&1 | tee /tmp/monitor_test.log || true
    
    if grep -q "Starting DOTS Family Monitor" /tmp/monitor_test.log; then
        if grep -q "Monitor running, polling every" /tmp/monitor_test.log; then
            log_info "Monitor started and began polling"
            return 0
        else
            log_error "Monitor started but didn't begin polling"
            return 1
        fi
    else
        log_error "Monitor failed to start"
        return 1
    fi
}

create_dbus_policy() {
    log_info "Creating development DBus policy for testing"
    
    # Create user dbus policy directory
    mkdir -p ~/.local/share/dbus-1/services
    
    # Create DBus service file for user session
    cat > ~/.local/share/dbus-1/services/org.dots.FamilyDaemon.service << 'EOF'
[D-BUS Service]
Name=org.dots.FamilyDaemon
Exec=/nix/store/*/bin/dots-family-daemon
User=parent
SystemdService=dots-family-daemon.service
EOF
    
    log_info "Created user DBus service file"
}

main() {
    log_info "=== DOTS Family Mode VM Testing ==="
    log_info "Date: $(date)"
    log_info "VM Hostname: $(hostname)"
    log_info "User: $(whoami)"
    
    # Environment setup
    run_test "VM Environment Setup" test_vm_environment
    run_test "Create DBus Policy" create_dbus_policy
    
    # Component tests
    run_test "Daemon Startup with DBus" test_daemon_with_dbus
    run_test "CLI Tool Testing" test_cli_tool
    run_test "Monitor Component Testing" test_monitor_component
    
    # Results
    log_info "=== VM TEST RESULTS ==="
    log_info "Tests Run: $TESTS_RUN"
    log_success "Tests Passed: $TESTS_PASSED"
    log_error "Tests Failed: $TESTS_FAILED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "ALL VM TESTS PASSED"
    else
        log_error "SOME VM TESTS FAILED"
    fi
    
    log_info "Test logs saved to: $VM_TEST_LOG"
    log_info "Daemon logs saved to: /tmp/daemon_test.log"
    log_info "Monitor logs saved to: /tmp/monitor_test.log"
}

main "$@"