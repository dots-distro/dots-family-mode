#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode Installation Script
# Installs systemd services and configuration files

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_PREFIX="/usr"
CONFIG_DIR="/etc/dots-family"
STATE_DIR="/var/lib/dots-family"
SYSTEMD_SYSTEM_DIR="/etc/systemd/system"
SYSTEMD_USER_DIR="/etc/systemd/user"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root for system installation"
        exit 1
    fi
}

check_dependencies() {
    local deps=("systemd" "dbus")
    local missing=()
    
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing+=("$dep")
        fi
    done
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        exit 1
    fi
}

install_binaries() {
    log_info "Installing DOTS Family Mode binaries..."
    
    # Check if Nix build output exists
    if [[ -f "$SCRIPT_DIR/../result/bin/dots-family-daemon" ]]; then
        cp "$SCRIPT_DIR/../result/bin/dots-family-daemon" "$INSTALL_PREFIX/bin/"
        cp "$SCRIPT_DIR/../result/bin/dots-family-monitor" "$INSTALL_PREFIX/bin/"
        cp "$SCRIPT_DIR/../result/bin/dots-family-ctl" "$INSTALL_PREFIX/bin/"
        
        # Install eBPF programs
        mkdir -p "$INSTALL_PREFIX/lib/dots-family/ebpf"
        if [[ -d "$SCRIPT_DIR/../result/target/bpfel-unknown-none/release" ]]; then
            cp "$SCRIPT_DIR/../result/target/bpfel-unknown-none/release"/* \
               "$INSTALL_PREFIX/lib/dots-family/ebpf/"
        fi
        
        log_success "Binaries installed from Nix build"
    else
        log_error "Nix build output not found. Run 'nix build .#default' first"
        exit 1
    fi
}

install_systemd_services() {
    log_info "Installing systemd service files..."
    
    # Install system service (daemon)
    cp "$SCRIPT_DIR/dots-family-daemon.service" "$SYSTEMD_SYSTEM_DIR/"
    
    # Install user service (monitor)  
    mkdir -p "$SYSTEMD_USER_DIR"
    cp "$SCRIPT_DIR/dots-family-monitor.service" "$SYSTEMD_USER_DIR/"
    
    # Install D-Bus service file
    mkdir -p /usr/share/dbus-1/system-services
    cp "$SCRIPT_DIR/../dbus/org.dots.FamilyDaemon.service" \
       /usr/share/dbus-1/system-services/
    
    log_success "Systemd service files installed"
}

install_configuration() {
    log_info "Installing configuration files..."
    
    # Create configuration directory
    mkdir -p "$CONFIG_DIR"
    
    # Install configuration files
    cp "$SCRIPT_DIR/daemon.conf" "$CONFIG_DIR/"
    cp "$SCRIPT_DIR/monitor.conf" "$CONFIG_DIR/"
    
    # Create state directory with proper permissions
    mkdir -p "$STATE_DIR"
    chown root:root "$STATE_DIR"
    chmod 750 "$STATE_DIR"
    
    log_success "Configuration files installed"
}

reload_systemd() {
    log_info "Reloading systemd configuration..."
    
    systemctl daemon-reload
    
    log_success "Systemd configuration reloaded"
}

enable_services() {
    log_info "Enabling DOTS Family Mode services..."
    
    # Enable system service
    systemctl enable dots-family-daemon.service
    
    log_success "Services enabled"
    log_info "To start services:"
    echo "  sudo systemctl start dots-family-daemon"
    echo "  systemctl --user enable --now dots-family-monitor"
}

show_status() {
    log_info "Installation summary:"
    echo "  Binaries: $INSTALL_PREFIX/bin/dots-family-*"
    echo "  eBPF programs: $INSTALL_PREFIX/lib/dots-family/ebpf/"
    echo "  Configuration: $CONFIG_DIR/"
    echo "  State directory: $STATE_DIR/"
    echo "  System service: $SYSTEMD_SYSTEM_DIR/dots-family-daemon.service"
    echo "  User service: $SYSTEMD_USER_DIR/dots-family-monitor.service"
    echo ""
    log_info "Next steps:"
    echo "  1. Configure parent password: dots-family-ctl auth set-password"
    echo "  2. Create child profiles: dots-family-ctl profile create child1 8-12"
    echo "  3. Start services: sudo systemctl start dots-family-daemon"
    echo "  4. Monitor activity: dots-family-ctl status"
}

uninstall() {
    log_warning "Uninstalling DOTS Family Mode..."
    
    # Stop services
    systemctl stop dots-family-daemon.service || true
    systemctl --user stop dots-family-monitor.service || true
    
    # Disable services
    systemctl disable dots-family-daemon.service || true
    
    # Remove files
    rm -f "$INSTALL_PREFIX/bin/dots-family-"*
    rm -rf "$INSTALL_PREFIX/lib/dots-family"
    rm -f "$SYSTEMD_SYSTEM_DIR/dots-family-daemon.service"
    rm -f "$SYSTEMD_USER_DIR/dots-family-monitor.service"
    rm -f /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service
    
    # Keep configuration and state for safety
    log_warning "Configuration ($CONFIG_DIR) and state ($STATE_DIR) preserved"
    log_warning "Remove manually if desired"
    
    systemctl daemon-reload
    
    log_success "DOTS Family Mode uninstalled"
}

main() {
    case "${1:-install}" in
        install)
            log_info "Installing DOTS Family Mode..."
            check_root
            check_dependencies
            install_binaries
            install_systemd_services
            install_configuration
            reload_systemd
            enable_services
            show_status
            ;;
        uninstall)
            check_root
            uninstall
            ;;
        *)
            echo "Usage: $0 [install|uninstall]"
            exit 1
            ;;
    esac
}

main "$@"