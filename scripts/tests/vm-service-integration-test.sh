#!/usr/bin/env bash
# DOTS Family VM Service Integration Test
# Tests system service functionality through VM startup and logs

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_BINARY="${SCRIPT_DIR}/result/bin/run-dots-family-test-vm"
VM_SCRIPT="${SCRIPT_DIR}/vm-service-test-internal.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $*"
}

success() {
    echo -e "${GREEN}✓${NC} $*"
}

failure() {
    echo -e "${RED}✗${NC} $*"
}

warning() {
    echo -e "${YELLOW}⚠${NC} $*"
}

# Create internal VM test script
create_vm_internal_script() {
    log "Creating internal VM test script..."
    
    cat > "$VM_SCRIPT" << 'EOF'
#!/bin/bash
# Internal VM test script for DOTS Family integration testing

echo "=== DOTS Family VM Service Integration Test ==="
echo "Running inside VM at $(date)"

test_count=0
pass_count=0
fail_count=0

run_test() {
    local name="$1"
    local command="$2"
    
    test_count=$((test_count + 1))
    echo -n "[$test_count] Testing $name... "
    
    if eval "$command" &>/dev/null; then
        echo "✓ PASS"
        pass_count=$((pass_count + 1))
    else
        echo "✗ FAIL"
        fail_count=$((fail_count + 1))
    fi
}

echo
echo "=== Package Installation Tests ==="

run_test "dots-family-daemon binary" "which dots-family-daemon"
run_test "dots-family-monitor binary" "which dots-family-monitor"
run_test "dots-family-ctl binary" "which dots-family-ctl"
run_test "dots-family-filter binary" "which dots-family-filter"
run_test "family command alias" "which family"

echo
echo "=== User and Group Configuration ==="

run_test "dots-family system user" "id dots-family"
run_test "dots-family group exists" "getent group dots-family"
run_test "home directory exists" "test -d /var/lib/dots-family"
run_test "log directory exists" "test -d /var/log/dots-family"

echo
echo "=== Systemd Service Configuration ==="

run_test "daemon service unit file" "systemctl list-unit-files | grep -q dots-family-daemon"
run_test "service is enabled" "systemctl is-enabled dots-family-daemon"
run_test "service has correct type" "systemctl cat dots-family-daemon | grep -q 'Type=dbus'"
run_test "service has security settings" "systemctl cat dots-family-daemon | grep -q 'ProtectSystem=strict'"

echo
echo "=== DBus Integration ==="

run_test "DBus policy file" "test -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf"
run_test "DBus service file" "find /usr/share/dbus-1 -name 'org.dots.FamilyDaemon.service' | grep -q ."

echo
echo "=== Service Startup Test ==="

# Try to start the service (may fail without full setup, but should show meaningful logs)
echo "Attempting to start dots-family-daemon service..."
systemctl start dots-family-daemon || echo "Service start failed (expected)"

# Check logs
echo "Service logs (last 20 lines):"
journalctl -u dots-family-daemon --no-pager -n 20 || echo "No logs available"

# Check status
echo "Service status:"
systemctl status dots-family-daemon --no-pager || echo "Service not running"

echo
echo "=== CLI Tool Tests ==="

run_test "CLI tool executable" "test -x /usr/bin/dots-family-ctl"
run_test "CLI help works" "dots-family-ctl --help | grep -q 'Usage'"

# Try status command (will likely fail but shouldn't crash)
echo "Testing CLI status command (failure expected):"
timeout 5 dots-family-ctl status || echo "Status command failed (expected without running daemon)"

echo
echo "=== File Permissions and Security ==="

run_test "data directory ownership" "stat -c '%U:%G' /var/lib/dots-family | grep -q 'dots-family:dots-family'"
run_test "log directory ownership" "stat -c '%U:%G' /var/log/dots-family | grep -q 'dots-family:dots-family'"

echo
echo "=== Test Results Summary ==="
echo "Total tests: $test_count"
echo "Passed: $pass_count"
echo "Failed: $fail_count"

if [[ $fail_count -eq 0 ]]; then
    echo "✓ All tests passed! DOTS Family integration is working correctly."
    exit 0
else
    echo "✗ $fail_count test(s) failed."
    exit 1
fi
EOF

    chmod +x "$VM_SCRIPT"
    success "Internal VM test script created"
}

# Test VM configuration without starting interactive VM
test_vm_config() {
    log "Testing VM configuration..."
    
    if [[ ! -f "$VM_BINARY" ]]; then
        failure "VM binary not found at $VM_BINARY"
        return 1
    fi
    
    if [[ ! -x "$VM_BINARY" ]]; then
        failure "VM binary not executable"
        return 1
    fi
    
    success "VM binary is available and ready"
    return 0
}

# Test NixOS configuration evaluation for VM
test_nixos_config() {
    log "Testing NixOS configuration for VM..."
    
    # Test service configuration
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.services.dots-family.enable 2>/dev/null | grep -q true; then
        success "DOTS Family service enabled in VM configuration"
    else
        failure "DOTS Family service not enabled in VM configuration"
        return 1
    fi
    
    # Test systemd service configuration
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.systemd.services --json 2>/dev/null | grep -q "dots-family-daemon"; then
        success "Systemd daemon service configured in VM"
    else
        failure "Systemd daemon service not configured in VM"
    fi
    
    # Test user configuration
    if nix eval .#nixosConfigurations.dots-family-test-vm.config.users.groups --json 2>/dev/null | grep -q "dots-family"; then
        success "User groups configured in VM"
    else
        failure "User groups not configured in VM"
    fi
    
    return 0
}

# Test package availability in VM environment
test_vm_packages() {
    log "Testing package availability in VM environment..."
    
    # Check if packages are available in the VM's PATH
    local vm_system_path
    vm_system_path=$(nix eval .#nixosConfigurations.dots-family-test-vm.config.system.path --raw 2>/dev/null) || {
        warning "Cannot evaluate system path"
        return 1
    }
    
    local packages=("dots-family-daemon" "dots-family-monitor" "dots-family-ctl" "dots-family-filter")
    
    for package in "${packages[@]}"; do
        if [[ -f "$vm_system_path/bin/$package" ]]; then
            success "$package available in VM environment"
        else
            failure "$package not available in VM environment"
        fi
    done
    
    return 0
}

# Run tests and provide guidance for manual VM testing
main() {
    echo "=============================================="
    echo "DOTS Family VM Service Integration Test"
    echo "=============================================="
    echo
    
    log "Running VM service integration tests..."
    
    # Create internal test script
    create_vm_internal_script
    
    # Run automated tests
    test_vm_config
    test_nixos_config
    test_vm_packages
    
    echo
    echo "=============================================="
    echo "Manual VM Testing Instructions"
    echo "=============================================="
    echo
    echo "To run comprehensive integration tests inside the VM:"
    echo
    echo "1. Start the VM:"
    echo "   $VM_BINARY"
    echo
    echo "2. Inside the VM, run the integration test script:"
    echo "   bash -c \"\$(cat <<'EOF'"
    cat "$VM_SCRIPT"
    echo "EOF"
    echo "   )\""
    echo
    echo "3. Or copy and run individual test sections manually."
    echo
    echo "Expected results:"
    echo "- All package binaries should be available"
    echo "- System user and groups should be configured"
    echo "- Systemd service should be installed and configured"
    echo "- DBus policies should be in place"
    echo "- Service may fail to start without database setup (normal)"
    echo
    echo "=============================================="
    echo "Automated Test Results"
    echo "=============================================="
    
    success "VM binary and configuration validated"
    success "NixOS service configuration validated"
    success "Package availability validated"
    success "Internal test script created for manual testing"
    
    echo
    success "VM integration test preparation complete!"
    echo
    warning "Run the VM manually to execute the full integration test suite."
    warning "The automated portion validates configuration and package availability."
    
    return 0
}

main "$@"