#!/usr/bin/env bash
# DOTS Family Mode - Production Deployment Setup Script
# Sets up D-Bus permissions, systemd services, and system integration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CURRENT_USER=$(whoami)
INSTALL_MODE="${1:-development}"  # 'development' or 'production'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

check_requirements() {
    log_info "Checking deployment requirements..."
    
    # Check if running as root for system files
    if [[ "$INSTALL_MODE" == "production" && "$CURRENT_USER" != "root" ]]; then
        log_error "Production installation requires root privileges"
        log_info "Run: sudo ./scripts/deploy_system_integration.sh production"
        exit 1
    fi
    
    # Check required directories exist
    for dir in dbus systemd; do
        if [[ ! -d "$PROJECT_ROOT/$dir" ]]; then
            log_error "Required directory not found: $PROJECT_ROOT/$dir"
            exit 1
        fi
    done
    
    # Check if systemd is available
    if ! command -v systemctl >/dev/null 2>&1; then
        log_error "systemd is required but not available"
        exit 1
    fi
    
    log_success "Requirements check passed"
}

install_dbus_configuration() {
    log_info "Installing D-Bus configuration..."
    
    if [[ "$INSTALL_MODE" == "production" ]]; then
        # Production: use system D-Bus directory
        DBUS_DIR="/etc/dbus-1/system.d"
        DBUS_SERVICES_DIR="/usr/share/dbus-1/system-services"
        
        cp "$PROJECT_ROOT/dbus/org.dots.FamilyDaemon.conf" "$DBUS_DIR/"
        cp "$PROJECT_ROOT/deployment/dbus/org.dots.FamilyDaemon.service" "$DBUS_SERVICES_DIR/"
        
        # Reload D-Bus configuration
        systemctl reload dbus.service
        
        log_success "Production D-Bus configuration installed"
    else
        # Development: use user session D-Bus
        USER_DBUS_DIR="$HOME/.local/share/dbus-1"
        USER_DBUS_SERVICES_DIR="$USER_DBUS_DIR/services"
        USER_DBUS_CONFIG_DIR="$HOME/.config/dbus-1/session.d"
        
        # Create directories if they don't exist
        mkdir -p "$USER_DBUS_SERVICES_DIR"
        mkdir -p "$USER_DBUS_CONFIG_DIR"
        
        # Copy development D-Bus configuration
        cp "$PROJECT_ROOT/dbus/org.dots.FamilyDaemon-dev.conf" "$USER_DBUS_CONFIG_DIR/org.dots.FamilyDaemon.conf"
        
        # Create user session service file
        cat > "$USER_DBUS_SERVICES_DIR/org.dots.FamilyDaemon.service" <<EOF
[D-BUS Service]
Name=org.dots.FamilyDaemon
Exec=$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/debug/dots-family-daemon
User=$CURRENT_USER
EOF
        
        # Reload user session D-Bus
        if pgrep -x "dbus-daemon" >/dev/null; then
            dbus-send --session --type=method_call --dest=org.freedesktop.DBus / org.freedesktop.DBus.ReloadConfig 2>/dev/null || true
        fi
        
        log_success "Development D-Bus configuration installed"
    fi
}

install_systemd_services() {
    log_info "Installing systemd service files..."
    
    if [[ "$INSTALL_MODE" == "production" ]]; then
        # Production: install to system location
        SYSTEMD_DIR="/etc/systemd/system"
        
        cp "$PROJECT_ROOT"/systemd/*.service "$SYSTEMD_DIR/"
        cp "$PROJECT_ROOT"/systemd/*.timer "$SYSTEMD_DIR/"
        cp "$PROJECT_ROOT"/systemd/*.target "$SYSTEMD_DIR/"
        
        # Reload systemd and enable services
        systemctl daemon-reload
        systemctl enable dots-family-daemon.service
        systemctl enable dots-family-maintenance.timer
        
        log_success "Production systemd services installed and enabled"
    else
        # Development: install to user location
        USER_SYSTEMD_DIR="$HOME/.config/systemd/user"
        mkdir -p "$USER_SYSTEMD_DIR"
        
        # Copy and modify service files for user session
        for service_file in "$PROJECT_ROOT"/systemd/*.service; do
            service_name=$(basename "$service_file")
            
            # Modify paths for development and remove capabilities that don't work with user services
            sed -e "s|/usr/bin/|$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/debug/|g" \
                -e "s|/var/lib/dots-family|$HOME/.local/share/dots-family|g" \
                -e "s|/etc/dots-family|$HOME/.config/dots-family|g" \
                -e "s|User=root|User=$CURRENT_USER|g" \
                -e "s|Group=root|Group=$(id -gn)|g" \
                -e '/CapabilityBoundingSet=/d' \
                -e '/AmbientCapabilities=/d' \
                -e '/SecureBits=/d' \
                -e '/ProtectSystem=/d' \
                -e '/ProtectHome=/d' \
                -e '/PrivateTmp=/d' \
                -e '/PrivateDevices=/d' \
                -e '/ProtectHostname=/d' \
                -e '/ProtectClock=/d' \
                -e '/ProtectKernelTunables=/d' \
                -e '/ProtectKernelModules=/d' \
                -e '/ProtectKernelLogs=/d' \
                -e '/ProtectControlGroups=/d' \
                -e '/IPAddressDeny=/d' \
                -e '/IPAddressAllow=/d' \
                -e '/RestrictAddressFamilies=/d' \
                -e '/SystemCallFilter=/d' \
                -e '/SystemCallErrorNumber=/d' \
                -e '/SystemCallArchitectures=/d' \
                -e '/MemoryDenyWriteExecute=/d' \
                -e '/RestrictRealtime=/d' \
                -e '/RestrictSUIDSGID=/d' \
                -e '/LockPersonality=/d' \
                -e '/StateDirectory=/d' \
                -e '/StateDirectoryMode=/d' \
                -e '/ConfigurationDirectory=/d' \
                -e '/ConfigurationDirectoryMode=/d' \
                -e '/CacheDirectory=/d' \
                -e '/CacheDirectoryMode=/d' \
                -e '/LogsDirectory=/d' \
                -e '/LogsDirectoryMode=/d' \
                -e '/ReadWritePaths=/d' \
                -e '/ReadOnlyPaths=/d' \
                -e '/BindReadOnlyPaths=/d' \
                -e 's|WantedBy=multi-user.target|WantedBy=default.target|g' \
                "$service_file" > "$USER_SYSTEMD_DIR/$service_name"
        done
        
        # Copy timers and targets
        cp "$PROJECT_ROOT"/systemd/*.timer "$USER_SYSTEMD_DIR/" 2>/dev/null || true
        cp "$PROJECT_ROOT"/systemd/*.target "$USER_SYSTEMD_DIR/" 2>/dev/null || true
        
        # Reload user systemd
        systemctl --user daemon-reload
        
        log_success "Development systemd services installed"
        log_info "To start services: systemctl --user start dots-family-daemon.service"
    fi
}

create_directories() {
    log_info "Creating required directories..."
    
    if [[ "$INSTALL_MODE" == "production" ]]; then
        # Production directories
        mkdir -p /var/lib/dots-family
        mkdir -p /etc/dots-family
        mkdir -p /var/log/dots-family
        chown root:root /var/lib/dots-family /etc/dots-family /var/log/dots-family
        chmod 750 /var/lib/dots-family /var/log/dots-family
        chmod 755 /etc/dots-family
    else
        # Development directories
        mkdir -p "$HOME/.local/share/dots-family"
        mkdir -p "$HOME/.config/dots-family"
        mkdir -p "$HOME/.local/state/dots-family"
    fi
    
    log_success "Directories created"
}

install_binaries() {
    if [[ "$INSTALL_MODE" == "production" ]]; then
        log_info "Installing binaries to /usr/bin/..."
        
        # Build release binaries
        cd "$PROJECT_ROOT"
        cargo build --release
        
        # Install binaries
        cp target/x86_64-unknown-linux-gnu/release/dots-family-daemon /usr/bin/
        cp target/x86_64-unknown-linux-gnu/release/dots-family-monitor /usr/bin/
        cp target/x86_64-unknown-linux-gnu/release/dots-family-ctl /usr/bin/
        cp target/x86_64-unknown-linux-gnu/release/dots-family-gui /usr/bin/
        
        # Set permissions
        chmod 755 /usr/bin/dots-family-*
        
        log_success "Binaries installed"
    else
        log_info "Development mode: using binaries from target directory"
        
        # Build debug binaries
        cd "$PROJECT_ROOT"
        cargo build
        
        log_success "Debug binaries built"
    fi
}

test_integration() {
    log_info "Testing system integration..."
    
    if [[ "$INSTALL_MODE" == "production" ]]; then
        # Test system services
        if systemctl is-active --quiet dots-family-daemon.service; then
            log_success "Daemon service is running"
        else
            log_warning "Daemon service is not running. Start with: systemctl start dots-family-daemon.service"
        fi
    else
        # Test development setup
        log_info "Testing D-Bus name ownership..."
        
        # Try to check D-Bus service availability
        if dbus-send --session --dest=org.freedesktop.DBus --type=method_call --print-reply /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | grep -q "org.dots.FamilyDaemon"; then
            log_success "D-Bus service is available"
        else
            log_info "D-Bus service not currently running (this is normal)"
        fi
    fi
    
    log_success "Integration test completed"
}

main() {
    log_info "=== DOTS Family Mode - System Integration Setup ==="
    log_info "Installation mode: $INSTALL_MODE"
    log_info "Current user: $CURRENT_USER"
    echo
    
    check_requirements
    echo
    
    create_directories
    echo
    
    install_dbus_configuration
    echo
    
    install_systemd_services
    echo
    
    install_binaries
    echo
    
    test_integration
    echo
    
    log_success "=== System integration setup completed ==="
    
    if [[ "$INSTALL_MODE" == "production" ]]; then
        echo
        log_info "Production deployment commands:"
        log_info "  Start daemon: systemctl start dots-family-daemon.service"
        log_info "  Enable on boot: systemctl enable dots-family-daemon.service"
        log_info "  View status: systemctl status dots-family-daemon.service"
        log_info "  View logs: journalctl -u dots-family-daemon.service -f"
    else
        echo
        log_info "Development deployment commands:"
        log_info "  Start daemon: systemctl --user start dots-family-daemon.service"
        log_info "  View status: systemctl --user status dots-family-daemon.service"
        log_info "  View logs: journalctl --user -u dots-family-daemon.service -f"
        log_info "  Test CLI: $PROJECT_ROOT/target/x86_64-unknown-linux-gnu/debug/dots-family-ctl status"
    fi
}

main "$@"