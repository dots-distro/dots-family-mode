#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

EVIDENCE_DIR="${1:-./test-evidence/automated-deployment-$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$EVIDENCE_DIR"

log() {
    echo -e "[$(date +'%H:%M:%S')] $*" | tee -a "$EVIDENCE_DIR/test.log"
}

log_success() {
    log "${GREEN}✓${NC} $1"
}

log_error() {
    log "${RED}✗${NC} $1"
}

log_info() {
    log "${BLUE}ℹ${NC} $1"
}

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

log_info "=========================================="
log_info "DOTS Family Mode - Automated Deployment Test"
log_info "=========================================="
log_info "Evidence directory: $EVIDENCE_DIR"
log_info "Temp directory: $TEMP_DIR"

cat > "$TEMP_DIR/vm-test-script.sh" << 'VMSCRIPT'
#!/usr/bin/env bash
set -euo pipefail

RESULTS_FILE="/tmp/xchg/test-results.txt"
exec > "$RESULTS_FILE" 2>&1

echo "===== DOTS Family Mode VM Validation ====="
echo "Timestamp: $(date)"
echo ""

TEST_PASSED=0
TEST_FAILED=0

test_service() {
    local service=$1
    echo "TEST: Service $service status"
    if systemctl is-active "$service" >/dev/null 2>&1; then
        echo "PASS: $service is active"
        systemctl status "$service" --no-pager || true
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: $service is not active"
        systemctl status "$service" --no-pager || true
        journalctl -u "$service" -n 50 --no-pager || true
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_dbus() {
    echo "TEST: DBus service availability"
    if busctl status org.dots.FamilyDaemon >/dev/null 2>&1; then
        echo "PASS: DBus service org.dots.FamilyDaemon is available"
        busctl introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon || true
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: DBus service org.dots.FamilyDaemon is not available"
        busctl list | grep -i dots || echo "No DOTS services found"
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_cli() {
    echo "TEST: CLI tool availability"
    if command -v dots-family-ctl >/dev/null 2>&1; then
        echo "PASS: dots-family-ctl is installed"
        dots-family-ctl --version || true
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: dots-family-ctl not found"
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_database() {
    echo "TEST: Database file"
    if [ -f /var/lib/dots-family/family.db ]; then
        echo "PASS: Database exists at /var/lib/dots-family/family.db"
        ls -lh /var/lib/dots-family/family.db
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "INFO: Database not yet created (may be created on first use)"
        ls -lh /var/lib/dots-family/ || echo "Directory not found"
    fi
    echo ""
}

test_ssl_certs() {
    echo "TEST: SSL certificate generation"
    if [ -f /var/lib/dots-family/ssl/ca.crt ] && [ -f /var/lib/dots-family/ssl/ca.key ]; then
        echo "PASS: SSL certificates exist"
        ls -lh /var/lib/dots-family/ssl/
        openssl x509 -in /var/lib/dots-family/ssl/ca.crt -noout -text | head -20
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: SSL certificates not found"
        ls -lh /var/lib/dots-family/ssl/ || echo "SSL directory not found"
        journalctl -u dots-family-ssl-ca.service -n 50 --no-pager || true
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_users() {
    echo "TEST: User accounts and groups"
    local user_tests_passed=0
    local user_tests_failed=0
    
    if getent group dots-family-parents >/dev/null 2>&1; then
        echo "PASS: dots-family-parents group exists"
        user_tests_passed=$((user_tests_passed + 1))
    else
        echo "FAIL: dots-family-parents group not found"
        user_tests_failed=$((user_tests_failed + 1))
    fi
    
    if getent group dots-family-children >/dev/null 2>&1; then
        echo "PASS: dots-family-children group exists"
        user_tests_passed=$((user_tests_passed + 1))
    else
        echo "FAIL: dots-family-children group not found"
        user_tests_failed=$((user_tests_failed + 1))
    fi
    
    if id parent >/dev/null 2>&1; then
        echo "PASS: parent user exists"
        id parent
        user_tests_passed=$((user_tests_passed + 1))
    else
        echo "FAIL: parent user not found"
        user_tests_failed=$((user_tests_failed + 1))
    fi
    
    if id child >/dev/null 2>&1; then
        echo "PASS: child user exists"
        id child
        user_tests_passed=$((user_tests_passed + 1))
    else
        echo "FAIL: child user not found"
        user_tests_failed=$((user_tests_failed + 1))
    fi
    
    if [ $user_tests_failed -eq 0 ]; then
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_packages() {
    echo "TEST: Package installation"
    if nix-store -qR /run/current-system | grep -q dots-family; then
        echo "PASS: DOTS Family packages installed"
        nix-store -qR /run/current-system | grep dots-family
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: DOTS Family packages not found in system closure"
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

test_dbus_policy() {
    echo "TEST: DBus policy files"
    if ls /etc/dbus-1/system.d/*dots* >/dev/null 2>&1; then
        echo "PASS: DBus policy files exist"
        ls -l /etc/dbus-1/system.d/*dots*
        TEST_PASSED=$((TEST_PASSED + 1))
    else
        echo "FAIL: DBus policy files not found"
        ls -l /etc/dbus-1/system.d/ || true
        TEST_FAILED=$((TEST_FAILED + 1))
    fi
    echo ""
}

echo "===== Running Tests ====="
echo ""

sleep 5

test_users
test_packages
test_dbus_policy
test_ssl_certs
test_database
test_service "dots-family-daemon.service"
test_dbus
test_cli

echo "===== Test Summary ====="
echo "Tests Passed: $TEST_PASSED"
echo "Tests Failed: $TEST_FAILED"
echo ""

if [ $TEST_FAILED -eq 0 ]; then
    echo "OVERALL: SUCCESS - All tests passed"
    exit 0
else
    echo "OVERALL: PARTIAL SUCCESS - Some tests failed"
    exit 1
fi
VMSCRIPT

chmod +x "$TEMP_DIR/vm-test-script.sh"

log_info "Creating test VM configuration..."

cat > "$TEMP_DIR/vm-test-config.nix" << 'EOF'
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [ 
    "${modulesPath}/virtualisation/qemu-vm.nix"
    ./vm-simple.nix
  ];

  virtualisation.qemu.options = [
    "-virtfs local,path=${toString ./..}/xchg,security_model=none,mount_tag=xchg"
  ];

  systemd.services.dots-family-automated-test = {
    description = "DOTS Family Mode - Automated Validation Test";
    after = [ "multi-user.target" "dots-family-daemon.service" ];
    wants = [ "dots-family-daemon.service" ];
    wantedBy = [ "multi-user.target" ];
    
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = "yes";
      ExecStart = "${pkgs.bash}/bin/bash /tmp/xchg/vm-test-script.sh";
      StandardOutput = "journal";
      StandardError = "journal";
    };
  };
}
EOF

log_info "Building test VM with automated validation..."

nix build ".#nixosConfigurations.dots-family-test-vm-root.config.system.build.vm" -o "$TEMP_DIR/vm" 2>&1 | tee "$EVIDENCE_DIR/build.log"

if [ ! -e "$TEMP_DIR/vm/bin/run-dots-family-test-vm" ]; then
    log_error "VM build failed"
    exit 1
fi

log_success "VM built successfully"

mkdir -p "$TEMP_DIR/xchg"
cp "$TEMP_DIR/vm-test-script.sh" "$TEMP_DIR/xchg/"

log_info "Starting VM with automated tests..."

timeout 180 "$TEMP_DIR/vm/bin/run-dots-family-test-vm" \
    -nographic \
    -m 2048 \
    -virtfs "local,path=$TEMP_DIR/xchg,security_model=none,mount_tag=xchg" \
    > "$EVIDENCE_DIR/vm-console.log" 2>&1 || true

log_info "VM execution completed (or timed out after 180s)"

if [ -f "$TEMP_DIR/xchg/test-results.txt" ]; then
    log_success "Test results retrieved from VM"
    cp "$TEMP_DIR/xchg/test-results.txt" "$EVIDENCE_DIR/test-results.txt"
    
    cat "$EVIDENCE_DIR/test-results.txt"
    
    if grep -q "OVERALL: SUCCESS" "$EVIDENCE_DIR/test-results.txt"; then
        log_success "All automated tests PASSED"
        exit 0
    elif grep -q "OVERALL: PARTIAL SUCCESS" "$EVIDENCE_DIR/test-results.txt"; then
        log_error "Some automated tests FAILED"
        exit 1
    else
        log_error "Test results incomplete"
        exit 1
    fi
else
    log_error "Could not retrieve test results from VM"
    log_info "Check console log: $EVIDENCE_DIR/vm-console.log"
    exit 1
fi
