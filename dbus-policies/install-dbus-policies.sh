#!/bin/bash

# DBus Policy Installation Script
# Installs DOTS Family Mode DBus policies to the system

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   error "This script must be run as root (use sudo)"
   exit 1
fi

# Define paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DBUS_POLICIES_DIR="$SCRIPT_DIR/dbus-policies"
SYSTEM_DBUS_DIR="/etc/dbus-1/system.d"
SESSION_DBUS_DIR="/etc/dbus-1/session.d"
SYSTEM_SERVICES_DIR="/usr/share/dbus-1/system-services"

log "Installing DOTS Family Mode DBus policies..."

# Check if source policies exist
if [[ ! -d "$DBUS_POLICIES_DIR" ]]; then
    error "DBus policies directory not found: $DBUS_POLICIES_DIR"
    exit 1
fi

# Create target directories if they don't exist
mkdir -p "$SYSTEM_DBUS_DIR"
mkdir -p "$SESSION_DBUS_DIR"
mkdir -p "$SYSTEM_SERVICES_DIR"

# Install system bus policies (daemon)
log "Installing system bus policies..."
if [[ -f "$DBUS_POLICIES_DIR/org.dots.FamilyDaemon.conf" ]]; then
    cp "$DBUS_POLICIES_DIR/org.dots.FamilyDaemon.conf" "$SYSTEM_DBUS_DIR/"
    chmod 644 "$SYSTEM_DBUS_DIR/org.dots.FamilyDaemon.conf"
    success "Installed org.dots.FamilyDaemon.conf to system bus"
else
    error "System policy file not found: $DBUS_POLICIES_DIR/org.dots.FamilyDaemon.conf"
    exit 1
fi

# Install session bus policies (monitor)
log "Installing session bus policies..."
if [[ -f "$DBUS_POLICIES_DIR/org.dots.FamilyMonitor.conf" ]]; then
    cp "$DBUS_POLICIES_DIR/org.dots.FamilyMonitor.conf" "$SESSION_DBUS_DIR/"
    chmod 644 "$SESSION_DBUS_DIR/org.dots.FamilyMonitor.conf"
    success "Installed org.dots.FamilyMonitor.conf to session bus"
else
    error "Session policy file not found: $DBUS_POLICIES_DIR/org.dots.FamilyMonitor.conf"
    exit 1
fi

# Install service activation file
log "Installing DBus service activation..."
if [[ -f "$SCRIPT_DIR/dbus/org.dots.FamilyDaemon.service" ]]; then
    cp "$SCRIPT_DIR/dbus/org.dots.FamilyDaemon.service" "$SYSTEM_SERVICES_DIR/"
    chmod 644 "$SYSTEM_SERVICES_DIR/org.dots.FamilyDaemon.service"
    success "Installed service activation file"
else
    error "Service activation file not found: $SCRIPT_DIR/dbus/org.dots.FamilyDaemon.service"
    exit 1
fi

# Create required system groups
log "Creating system groups..."
getent group dots-family >/dev/null 2>&1 || {
    groupadd -r dots-family
    success "Created dots-family group"
}

getent group dots-parents >/dev/null 2>&1 || {
    groupadd -r dots-parents
    success "Created dots-parents group"
}

# Create monitor user
log "Creating monitor user..."
id dots-monitor >/dev/null 2>&1 || {
    useradd -r -s /bin/false -d /nonexistent -c "DOTS Family Monitor" dots-monitor
    success "Created dots-monitor user"
}

# Reload DBus configuration
log "Reloading DBus configuration..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl reload dbus.service || warn "Failed to reload DBus service"
else
    warn "systemctl not available, manual DBus restart may be required"
fi

# Validate installation
log "Validating installation..."
ERRORS=0

# Check policy files
for policy in org.dots.FamilyDaemon.conf org.dots.FamilyMonitor.conf; do
    if [[ -f "$SYSTEM_DBUS_DIR/$policy" || -f "$SESSION_DBUS_DIR/$policy" ]]; then
        success "Policy file $policy is installed"
    else
        error "Policy file $policy is missing"
        ((ERRORS++))
    fi
done

# Check service file
if [[ -f "$SYSTEM_SERVICES_DIR/org.dots.FamilyDaemon.service" ]]; then
    success "Service activation file is installed"
else
    error "Service activation file is missing"
    ((ERRORS++))
fi

# Check groups
for group in dots-family dots-parents; do
    if getent group "$group" >/dev/null 2>&1; then
        success "Group $group exists"
    else
        error "Group $group is missing"
        ((ERRORS++))
    fi
done

# Check monitor user
if id dots-monitor >/dev/null 2>&1; then
    success "User dots-monitor exists"
else
    error "User dots-monitor is missing"
    ((ERRORS++))
fi

if [[ $ERRORS -eq 0 ]]; then
    success "DBus policies installed successfully!"
    log ""
    log "Next steps:"
    log "1. Add parent users to 'dots-parents' group:"
    log "   sudo usermod -a -G dots-parents <parent-username>"
    log ""
    log "2. Add child users to 'dots-family' group:"
    log "   sudo usermod -a -G dots-family <child-username>"
    log ""
    log "3. Install and start the DOTS Family systemd services"
    log ""
else
    error "Installation completed with $ERRORS errors"
    exit 1
fi