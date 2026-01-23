#!/usr/bin/env bash
# Validate full DOTS Family Mode deployment in VM
# This script tests that all services are running and functional

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Evidence directory
EVIDENCE_DIR="${1:-./test-evidence/full-deployment-$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$EVIDENCE_DIR"

log_file="$EVIDENCE_DIR/validation.log"

log() {
    echo -e "$@" | tee -a "$log_file"
}

log_success() {
    log "${GREEN}✓${NC} $1"
}

log_error() {
    log "${RED}✗${NC} $1"
}

log_warning() {
    log "${YELLOW}⚠${NC} $1"
}

log_info() {
    log "$1"
}

# Check if running inside VM or need to SSH
if [ -f /etc/NIXOS ]; then
    # We're inside the VM
    RUN_CMD=""
else
    # We're outside, need to SSH
    log_info "Running tests via SSH to VM..."
    RUN_CMD="ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p 10022 root@localhost"
fi

log_info "=========================================="
log_info "DOTS Family Mode - Full Deployment Test"
log_info "=========================================="
log_info "Evidence directory: $EVIDENCE_DIR"
log_info "Timestamp: $(date)"
log_info ""

# Test 1: System service status
log_info "Test 1: Checking systemd service status..."
if $RUN_CMD systemctl is-active dots-family-daemon.service >/dev/null 2>&1; then
    log_success "dots-family-daemon.service is active"
    $RUN_CMD systemctl status dots-family-daemon.service --no-pager > "$EVIDENCE_DIR/daemon-status.txt" 2>&1
else
    log_error "dots-family-daemon.service is NOT active"
    $RUN_CMD systemctl status dots-family-daemon.service --no-pager > "$EVIDENCE_DIR/daemon-status-failed.txt" 2>&1 || true
fi

# Test 2: DBus service availability
log_info ""
log_info "Test 2: Checking DBus service..."
if $RUN_CMD busctl status org.dots.FamilyDaemon >/dev/null 2>&1; then
    log_success "DBus service org.dots.FamilyDaemon is available"
    $RUN_CMD busctl introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon > "$EVIDENCE_DIR/dbus-introspect.txt" 2>&1 || true
else
    log_error "DBus service org.dots.FamilyDaemon is NOT available"
fi

# Test 3: CLI tool availability
log_info ""
log_info "Test 3: Checking CLI tool..."
if $RUN_CMD which dots-family-ctl >/dev/null 2>&1; then
    log_success "dots-family-ctl is installed"
    $RUN_CMD dots-family-ctl --version > "$EVIDENCE_DIR/ctl-version.txt" 2>&1 || true
    $RUN_CMD dots-family-ctl status > "$EVIDENCE_DIR/ctl-status.txt" 2>&1 || log_warning "CLI status command failed (may need initialization)"
else
    log_error "dots-family-ctl is NOT installed"
fi

# Test 4: Database file
log_info ""
log_info "Test 4: Checking database..."
if $RUN_CMD test -f /var/lib/dots-family/family.db; then
    log_success "Database file exists at /var/lib/dots-family/family.db"
    $RUN_CMD ls -lh /var/lib/dots-family/ > "$EVIDENCE_DIR/database-info.txt" 2>&1
else
    log_warning "Database file does not exist yet (may be created on first use)"
    $RUN_CMD ls -lh /var/lib/dots-family/ > "$EVIDENCE_DIR/var-lib-dots-family.txt" 2>&1 || true
fi

# Test 5: User accounts and groups
log_info ""
log_info "Test 5: Checking user accounts and groups..."
if $RUN_CMD getent group dots-family-parents >/dev/null 2>&1; then
    log_success "dots-family-parents group exists"
else
    log_error "dots-family-parents group does NOT exist"
fi

if $RUN_CMD getent group dots-family-children >/dev/null 2>&1; then
    log_success "dots-family-children group exists"
else
    log_error "dots-family-children group does NOT exist"
fi

if $RUN_CMD id parent >/dev/null 2>&1; then
    log_success "Parent user exists"
    $RUN_CMD id parent > "$EVIDENCE_DIR/parent-user-info.txt" 2>&1
else
    log_error "Parent user does NOT exist"
fi

if $RUN_CMD id child >/dev/null 2>&1; then
    log_success "Child user exists"
    $RUN_CMD id child > "$EVIDENCE_DIR/child-user-info.txt" 2>&1
else
    log_error "Child user does NOT exist"
fi

# Test 6: Service logs
log_info ""
log_info "Test 6: Collecting service logs..."
$RUN_CMD journalctl -u dots-family-daemon.service -n 50 --no-pager > "$EVIDENCE_DIR/daemon-logs.txt" 2>&1 || log_warning "Could not collect daemon logs"

# Test 7: Configuration files
log_info ""
log_info "Test 7: Checking configuration..."
if $RUN_CMD test -d /etc/dots-family; then
    log_success "/etc/dots-family directory exists"
    $RUN_CMD ls -lR /etc/dots-family > "$EVIDENCE_DIR/etc-dots-family.txt" 2>&1 || true
else
    log_warning "/etc/dots-family directory does not exist (configuration may be generated dynamically)"
fi

# Test 8: DBus policies
log_info ""
log_info "Test 8: Checking DBus policies..."
if $RUN_CMD test -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf; then
    log_success "DBus policy file exists"
    $RUN_CMD cat /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf > "$EVIDENCE_DIR/dbus-policy.conf" 2>&1 || true
else
    log_warning "DBus policy file not found"
fi

# Test 9: Runtime directories
log_info ""
log_info "Test 9: Checking runtime directories..."
if $RUN_CMD test -d /var/lib/dots-family; then
    log_success "/var/lib/dots-family exists"
fi

if $RUN_CMD test -d /var/log/dots-family; then
    log_success "/var/log/dots-family exists"
fi

# Test 10: Package installation
log_info ""
log_info "Test 10: Checking installed packages..."
$RUN_CMD nix-store -qR /run/current-system | grep dots-family > "$EVIDENCE_DIR/installed-packages.txt" 2>&1 || log_warning "Could not list installed packages"

# Summary
log_info ""
log_info "=========================================="
log_info "Validation Complete"
log_info "=========================================="
log_info "Evidence saved to: $EVIDENCE_DIR"
log_info ""
log_info "To start the VM (if not already running):"
log_info "  ./result/bin/run-dots-family-test-vm -display gtk -m 2048"
log_info ""
log_info "To connect via SSH:"
log_info "  ssh -p 10022 root@localhost  (password: root)"
log_info ""
log_info "To test manually in VM:"
log_info "  systemctl status dots-family-daemon.service"
log_info "  dots-family-ctl status"
log_info "  busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon.GetVersion"
