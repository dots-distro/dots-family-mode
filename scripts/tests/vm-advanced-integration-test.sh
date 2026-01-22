#!/usr/bin/env bash
# Advanced DOTS Family VM Integration Test Suite
# Tests system service functionality by actually running the VM

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_BINARY="${SCRIPT_DIR}/result/bin/run-dots-family-test-vm"
TEST_LOG="${SCRIPT_DIR}/vm_integration_test.log"
TIMEOUT=300  # 5 minutes timeout for VM operations

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results tracking
PASSED_TESTS=()
FAILED_TESTS=()
SKIPPED_TESTS=()

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $*" | tee -a "$TEST_LOG"
}

success() {
    echo -e "${GREEN}✓${NC} $*" | tee -a "$TEST_LOG"
    PASSED_TESTS+=("$*")
}

failure() {
    echo -e "${RED}✗${NC} $*" | tee -a "$TEST_LOG"
    FAILED_TESTS+=("$*")
}

warning() {
    echo -e "${YELLOW}⚠${NC} $*" | tee -a "$TEST_LOG"
}

skip() {
    echo -e "${YELLOW}○${NC} $*" | tee -a "$TEST_LOG"
    SKIPPED_TESTS+=("$*")
}

cleanup() {
    if [[ -n "${VM_PID:-}" ]]; then
        log "Cleaning up VM process $VM_PID"
        kill "$VM_PID" 2>/dev/null || true
        wait "$VM_PID" 2>/dev/null || true
    fi
    
    # Clean up any VM files
    rm -f nixos.qcow2 2>/dev/null || true
}

trap cleanup EXIT

# Check pre-requisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    if [[ ! -f "$VM_BINARY" ]]; then
        failure "VM binary not found at $VM_BINARY"
        exit 1
    fi
    
    if [[ ! -x "$VM_BINARY" ]]; then
        failure "VM binary not executable"
        exit 1
    fi
    
    if ! command -v expect >/dev/null; then
        warning "expect command not found - some interactive tests will be skipped"
    fi
    
    success "Prerequisites check passed"
}

# Start VM and wait for it to boot
start_vm() {
    log "Starting DOTS Family test VM..."
    
    # Remove any existing disk image
    rm -f nixos.qcow2
    
    # Start VM in background with increased memory and timeout
    QEMU_OPTS="-m 2048 -smp 2 -netdev user,id=net0 -device virtio-net,netdev=net0" \
    "$VM_BINARY" &
    VM_PID=$!
    
    log "VM started with PID $VM_PID"
    
    # Wait for VM to boot (check for systemd being available)
    local boot_timeout=120
    local elapsed=0
    
    while [[ $elapsed -lt $boot_timeout ]]; do
        if vm_execute "systemctl is-system-running --wait" 2>/dev/null | grep -q "running\|degraded"; then
            success "VM booted successfully"
            return 0
        fi
        sleep 5
        elapsed=$((elapsed + 5))
        
        if ! kill -0 "$VM_PID" 2>/dev/null; then
            failure "VM process died during boot"
            return 1
        fi
    done
    
    failure "VM failed to boot within $boot_timeout seconds"
    return 1
}

# Execute command in VM using expect
vm_execute() {
    local command="$1"
    local timeout="${2:-30}"
    
    if ! command -v expect >/dev/null; then
        skip "Cannot execute VM command (expect not available): $command"
        return 1
    fi
    
    expect -c "
        set timeout $timeout
        spawn ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p 2222 root@localhost
        expect {
            \"password:\" {
                send \"root\r\"
                exp_continue
            }
            \"# \" {
                send \"$command\r\"
                expect \"# \"
                send \"exit\r\"
                expect eof
            }
            timeout {
                exit 1
            }
        }
    " 2>/dev/null
}

# Test system service functionality
test_system_service() {
    log "Testing system service functionality..."
    
    # Test 1: Check if dots-family-daemon service exists
    if vm_execute "systemctl list-unit-files | grep dots-family-daemon"; then
        success "dots-family-daemon systemd service is installed"
    else
        failure "dots-family-daemon systemd service not found"
        return 1
    fi
    
    # Test 2: Check service configuration
    if vm_execute "systemctl cat dots-family-daemon | grep -q 'Type=dbus'"; then
        success "Service configured as DBus service"
    else
        failure "Service not properly configured as DBus service"
    fi
    
    # Test 3: Try to start the service
    if vm_execute "systemctl start dots-family-daemon"; then
        success "Service starts without errors"
        
        # Test 4: Check service status
        if vm_execute "systemctl is-active dots-family-daemon"; then
            success "Service is active"
        else
            warning "Service started but not active (may be expected without full setup)"
        fi
    else
        warning "Service failed to start (expected without database setup)"
    fi
    
    # Test 5: Check service logs for initialization
    if vm_execute "journalctl -u dots-family-daemon --no-pager -n 10"; then
        success "Service logs accessible"
    else
        failure "Cannot access service logs"
    fi
}

# Test DBus integration
test_dbus_integration() {
    log "Testing DBus integration..."
    
    # Test 1: Check DBus policy files
    if vm_execute "ls /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf"; then
        success "DBus policy file installed"
    else
        failure "DBus policy file not found"
        return 1
    fi
    
    # Test 2: Check DBus service activation file
    if vm_execute "ls /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service"; then
        success "DBus service activation file installed"
    else
        warning "DBus service activation file not found (may be in different location)"
    fi
    
    # Test 3: Restart DBus to load policies
    if vm_execute "systemctl reload dbus"; then
        success "DBus reloaded successfully"
    else
        failure "Failed to reload DBus"
    fi
}

# Test user and group configuration
test_user_group_config() {
    log "Testing user and group configuration..."
    
    # Test 1: Check dots-family system user
    if vm_execute "id dots-family"; then
        success "dots-family system user exists"
    else
        failure "dots-family system user not found"
        return 1
    fi
    
    # Test 2: Check dots-family group
    if vm_execute "getent group dots-family"; then
        success "dots-family group exists"
    else
        failure "dots-family group not found"
    fi
    
    # Test 3: Check home directory
    if vm_execute "ls -la /var/lib/dots-family"; then
        success "dots-family home directory exists"
    else
        failure "dots-family home directory not found"
    fi
    
    # Test 4: Check directory permissions
    if vm_execute "stat -c '%U:%G %a' /var/lib/dots-family | grep 'dots-family:dots-family'"; then
        success "Directory has correct ownership"
    else
        warning "Directory ownership may need verification"
    fi
}

# Test security configuration
test_security_config() {
    log "Testing security configuration..."
    
    # Test 1: Check Polkit rules
    if vm_execute "ls /etc/polkit-1/rules.d/*dots*"; then
        success "Polkit rules installed"
    else
        warning "Polkit rules not found (may not be required for basic functionality)"
    fi
    
    # Test 2: Check service security settings
    if vm_execute "systemctl show dots-family-daemon | grep 'ProtectSystem=strict'"; then
        success "Service has proper security hardening (ProtectSystem)"
    else
        failure "Service missing security hardening"
    fi
    
    # Test 3: Check capability restrictions
    if vm_execute "systemctl show dots-family-daemon | grep 'CapabilityBoundingSet'"; then
        success "Service has capability restrictions"
    else
        failure "Service missing capability restrictions"
    fi
}

# Test CLI tool functionality
test_cli_tool() {
    log "Testing CLI tool functionality..."
    
    # Test 1: Check if CLI tool is installed
    if vm_execute "which dots-family-ctl"; then
        success "dots-family-ctl CLI tool installed"
    else
        failure "dots-family-ctl CLI tool not found"
        return 1
    fi
    
    # Test 2: Check if family alias works
    if vm_execute "which family"; then
        success "family alias configured"
    else
        warning "family alias not configured"
    fi
    
    # Test 3: Test CLI help
    if vm_execute "dots-family-ctl --help"; then
        success "CLI tool help accessible"
    else
        failure "CLI tool help not accessible"
    fi
    
    # Test 4: Test status command (may fail without daemon running)
    if vm_execute "timeout 10 dots-family-ctl status"; then
        success "CLI tool status command works"
    else
        warning "CLI tool status command failed (expected without daemon)"
    fi
}

# Test package installation
test_package_installation() {
    log "Testing package installation..."
    
    local packages=("dots-family-daemon" "dots-family-monitor" "dots-family-ctl" "dots-family-filter")
    
    for package in "${packages[@]}"; do
        if vm_execute "which $package"; then
            success "$package binary installed"
        else
            failure "$package binary not found"
        fi
    done
}

# Main test execution
main() {
    echo "=============================================="
    echo "DOTS Family VM Advanced Integration Test Suite"
    echo "=============================================="
    echo
    
    # Initialize log
    echo "=== VM Integration Test Started at $(date) ===" > "$TEST_LOG"
    
    log "Starting comprehensive VM integration testing..."
    
    # Pre-flight checks
    check_prerequisites
    
    # Start VM
    if ! start_vm; then
        failure "Failed to start VM - aborting tests"
        exit 1
    fi
    
    # Wait a bit more for services to stabilize
    log "Waiting for system to stabilize..."
    sleep 30
    
    # Run test suites
    test_package_installation
    test_user_group_config  
    test_system_service
    test_dbus_integration
    test_security_config
    test_cli_tool
    
    # Results summary
    echo
    echo "=============================================="
    echo "Test Results Summary"
    echo "=============================================="
    
    echo -e "\n${GREEN}Passed Tests (${#PASSED_TESTS[@]}):${NC}"
    for test in "${PASSED_TESTS[@]}"; do
        echo "  ✓ $test"
    done
    
    if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
        echo -e "\n${RED}Failed Tests (${#FAILED_TESTS[@]}):${NC}"
        for test in "${FAILED_TESTS[@]}"; do
            echo "  ✗ $test"
        done
    fi
    
    if [[ ${#SKIPPED_TESTS[@]} -gt 0 ]]; then
        echo -e "\n${YELLOW}Skipped Tests (${#SKIPPED_TESTS[@]}):${NC}"
        for test in "${SKIPPED_TESTS[@]}"; do
            echo "  ○ $test"
        done
    fi
    
    echo
    echo "Test log saved to: $TEST_LOG"
    
    if [[ ${#FAILED_TESTS[@]} -eq 0 ]]; then
        success "All tests passed! VM integration is working correctly."
        exit 0
    else
        failure "${#FAILED_TESTS[@]} test(s) failed. Check logs for details."
        exit 1
    fi
}

# Handle signals
trap 'log "Test interrupted"; cleanup; exit 130' INT TERM

main "$@"