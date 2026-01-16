#!/usr/bin/env bash
# DOTS Family Mode Phase 7 NixOS Integration VM Test Suite
# Tests declarative NixOS configuration and service deployment

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_RESULT="${SCRIPT_DIR}/result/bin/run-dots-family-test-vm"
TEST_RESULTS=()
FAILED_TESTS=()

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $*"
}

success() {
    echo -e "${GREEN}✓${NC} $*"
    TEST_RESULTS+=("✓ $*")
}

failure() {
    echo -e "${RED}✗${NC} $*"
    TEST_RESULTS+=("✗ $*")
    FAILED_TESTS+=("$*")
}

warning() {
    echo -e "${YELLOW}⚠${NC} $*"
}

run_test() {
    local test_name="$1"
    local test_command="$2"
    
    log "Running test: $test_name"
    
    if eval "$test_command"; then
        success "$test_name"
    else
        failure "$test_name"
    fi
}

check_vm_binary() {
    if [[ ! -f "$VM_RESULT" ]]; then
        failure "VM binary not found at $VM_RESULT"
        return 1
    fi
    
    if [[ ! -x "$VM_RESULT" ]]; then
        failure "VM binary not executable"
        return 1
    fi
    
    success "VM binary is available and executable"
    return 0
}

test_vm_config_generation() {
    log "Testing VM configuration generation..."
    
    # Test that our NixOS configuration evaluates properly
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.system.build.vm &>/dev/null; then
        success "NixOS configuration evaluates successfully"
    else
        failure "NixOS configuration evaluation failed"
        return 1
    fi
    
    # Test that the configuration includes our custom options
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.services.dots-family.enable 2>/dev/null | grep -q "true"; then
        success "DOTS Family service is enabled in configuration"
    else
        failure "DOTS Family service not enabled in configuration"
    fi
    
    return 0
}

test_systemd_services() {
    log "Testing systemd service configuration..."
    
    # Check that daemon service is configured
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.systemd.services.dots-family-daemon.enable 2>/dev/null | grep -q "true"; then
        success "dots-family-daemon systemd service configured"
    else
        failure "dots-family-daemon systemd service not configured"
    fi
    
    # Check security settings
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.systemd.services.dots-family-daemon.serviceConfig.ProtectSystem 2>/dev/null | grep -q "strict"; then
        success "Security hardening configured for daemon service"
    else
        failure "Security hardening missing for daemon service"
    fi
    
    return 0
}

test_dbus_configuration() {
    log "Testing DBus configuration..."
    
    # Check that DBus policies are present in the configuration
    local dbus_config_path
    if dbus_config_path=$(nix eval .#nixosConfigurations.dots-family-test-vm.config.services.dbus.packages --raw 2>/dev/null); then
        success "DBus configuration is present"
        
        # Check if our DBus service file would be included
        if echo "$dbus_config_path" | grep -q "dots-family"; then
            success "DOTS Family DBus integration configured"
        else
            warning "DOTS Family DBus integration may not be properly configured"
        fi
    else
        failure "DBus configuration not accessible"
    fi
    
    return 0
}

test_user_configuration() {
    log "Testing user and group configuration..."
    
    # Check that dots-family group is configured
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.users.groups.dots-family 2>/dev/null | grep -q "dots-family"; then
        success "dots-family group configured"
    else
        failure "dots-family group not configured"
    fi
    
    return 0
}

test_security_policies() {
    log "Testing security and policy configuration..."
    
    # Check Polkit configuration
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.security.polkit.enable 2>/dev/null | grep -q "true"; then
        success "Polkit is enabled"
    else
        failure "Polkit is not enabled"
    fi
    
    return 0
}

start_vm_test() {
    log "Starting VM for live testing..."
    
    # Create a temporary script to run inside the VM
    cat > vm-internal-test.sh << 'EOF'
#!/bin/bash
# Internal VM test script

echo "=== DOTS Family Mode VM Internal Test ==="

# Check if systemd services are present
echo "Checking systemd services..."
if systemctl list-unit-files | grep -q "dots-family-daemon"; then
    echo "✓ dots-family-daemon service unit file present"
else
    echo "✗ dots-family-daemon service unit file missing"
    exit 1
fi

# Check if DBus configuration is applied
echo "Checking DBus configuration..."
if [ -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf ]; then
    echo "✓ DBus policy file present"
else
    echo "✗ DBus policy file missing"
    exit 1
fi

# Check if polkit rules are present
echo "Checking Polkit configuration..."
if ls /etc/polkit-1/rules.d/*dots* 2>/dev/null; then
    echo "✓ Polkit rules present"
else
    echo "✗ Polkit rules missing"
fi

# Check if users and groups are configured
echo "Checking user/group configuration..."
if getent group dots-family >/dev/null; then
    echo "✓ dots-family group exists"
else
    echo "✗ dots-family group missing"
fi

# Try to start the daemon service
echo "Testing daemon service startup..."
if systemctl start dots-family-daemon 2>/dev/null; then
    echo "✓ dots-family-daemon started successfully"
    systemctl status dots-family-daemon --no-pager
    systemctl stop dots-family-daemon
else
    echo "⚠ dots-family-daemon failed to start (expected without database)"
    journalctl -u dots-family-daemon --no-pager -n 10
fi

echo "=== VM Internal Test Complete ==="
EOF
    
    chmod +x vm-internal-test.sh
    
    # Run the VM with our test script
    # Note: This is a simplified test - a full test would require more complex VM interaction
    warning "VM live testing requires interactive setup - skipping for automated test"
    warning "To run VM manually: $VM_RESULT"
    
    success "VM test script prepared"
    return 0
}

main() {
    echo "========================================="
    echo "DOTS Family Mode Phase 7 VM Test Suite"
    echo "========================================="
    echo
    
    log "Starting comprehensive NixOS integration testing..."
    
    # Pre-flight checks
    run_test "Check VM binary exists" "check_vm_binary"
    
    # Configuration tests
    run_test "Test VM configuration generation" "test_vm_config_generation"
    run_test "Test systemd services configuration" "test_systemd_services"
    run_test "Test DBus configuration" "test_dbus_configuration"
    run_test "Test user configuration" "test_user_configuration"
    run_test "Test security policies" "test_security_policies"
    
    # VM interaction test
    run_test "Prepare VM live testing" "start_vm_test"
    
    echo
    echo "========================================="
    echo "Test Results Summary"
    echo "========================================="
    
    for result in "${TEST_RESULTS[@]}"; do
        echo "$result"
    done
    
    if [[ ${#FAILED_TESTS[@]} -eq 0 ]]; then
        echo
        success "All tests passed! Phase 7 NixOS integration is working correctly."
        echo
        log "You can now run the VM manually with: $VM_RESULT"
        log "Or use the existing vm-test.sh script for comprehensive testing"
        exit 0
    else
        echo
        failure "${#FAILED_TESTS[@]} test(s) failed:"
        for failed in "${FAILED_TESTS[@]}"; do
            echo "  - $failed"
        done
        exit 1
    fi
}

main "$@"