#!/bin/bash

# DOTS Family Mode - Systemd Installation Script
# This script installs and configures the systemd services for DOTS Family Mode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSTEMD_SYSTEM_DIR="/etc/systemd/system"
BIN_DIR="/usr/bin"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)" >&2
   exit 1
fi

echo "Installing DOTS Family Mode systemd services..."

# Copy service files to systemd directory
echo "Installing systemd service files..."
cp "${SCRIPT_DIR}/systemd/dots-family-daemon.service" "${SYSTEMD_SYSTEM_DIR}/"
cp "${SCRIPT_DIR}/systemd/dots-family-monitor@.service" "${SYSTEMD_SYSTEM_DIR}/"
cp "${SCRIPT_DIR}/systemd/dots-family-filter.service" "${SYSTEMD_SYSTEM_DIR}/"
cp "${SCRIPT_DIR}/systemd/dots-family.target" "${SYSTEMD_SYSTEM_DIR}/"

# Create system users for services
echo "Creating system users..."

# User for web filter service
if ! id "dots-filter" &>/dev/null; then
    useradd -r -s /bin/false -d /var/lib/dots-family-filter dots-filter
    echo "Created user: dots-filter"
fi

# Create directories
echo "Creating service directories..."
mkdir -p /var/lib/dots-family
mkdir -p /var/lib/dots-family-filter
mkdir -p /etc/dots-family
mkdir -p /etc/dots-family-filter
mkdir -p /var/cache/dots-family-filter

# Set permissions
echo "Setting permissions..."
chown root:root /var/lib/dots-family /etc/dots-family
chmod 755 /var/lib/dots-family /etc/dots-family

chown dots-filter:dots-filter /var/lib/dots-family-filter /etc/dots-family-filter /var/cache/dots-family-filter
chmod 755 /var/lib/dots-family-filter /etc/dots-family-filter /var/cache/dots-family-filter

# Copy binary files (assuming they exist in target/release/)
echo "Installing binary files..."
RELEASE_DIR="${SCRIPT_DIR}/target/release"

if [[ -f "${RELEASE_DIR}/dots-family-daemon" ]]; then
    cp "${RELEASE_DIR}/dots-family-daemon" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-family-daemon"
    echo "Installed: dots-family-daemon"
else
    echo "Warning: dots-family-daemon binary not found in ${RELEASE_DIR}"
fi

if [[ -f "${RELEASE_DIR}/dots-family-monitor" ]]; then
    cp "${RELEASE_DIR}/dots-family-monitor" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-family-monitor"
    echo "Installed: dots-family-monitor"
else
    echo "Warning: dots-family-monitor binary not found in ${RELEASE_DIR}"
fi

if [[ -f "${RELEASE_DIR}/dots-family-filter" ]]; then
    cp "${RELEASE_DIR}/dots-family-filter" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-family-filter"
    echo "Installed: dots-family-filter"
else
    echo "Warning: dots-family-filter binary not found in ${RELEASE_DIR}"
fi

if [[ -f "${RELEASE_DIR}/dots-family-ctl" ]]; then
    cp "${RELEASE_DIR}/dots-family-ctl" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-family-ctl"
    echo "Installed: dots-family-ctl"
else
    echo "Warning: dots-family-ctl binary not found in ${RELEASE_DIR}"
fi

if [[ -f "${RELEASE_DIR}/dots-terminal-filter" ]]; then
    cp "${RELEASE_DIR}/dots-terminal-filter" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-terminal-filter"
    echo "Installed: dots-terminal-filter"
else
    echo "Warning: dots-terminal-filter binary not found in ${RELEASE_DIR}"
fi

if [[ -f "${RELEASE_DIR}/dots-family-gui" ]]; then
    cp "${RELEASE_DIR}/dots-family-gui" "${BIN_DIR}/"
    chmod 755 "${BIN_DIR}/dots-family-gui"
    echo "Installed: dots-family-gui"
else
    echo "Warning: dots-family-gui binary not found in ${RELEASE_DIR}"
fi

# Install DBus service file
echo "Installing DBus service file..."
DBUS_SERVICES_DIR="/usr/share/dbus-1/system-services"
mkdir -p "$DBUS_SERVICES_DIR"
cp "${SCRIPT_DIR}/dbus/org.dots.FamilyDaemon.service" "$DBUS_SERVICES_DIR/"

# Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "Available services:"
echo "  - dots-family-daemon.service     (Core daemon)"
echo "  - dots-family-filter.service     (Web content filter)"
echo "  - dots-family-monitor@USER.service  (Activity monitor for specific user)"
echo "  - dots-family.target             (All services together)"
echo ""
echo "To start the system:"
echo "  sudo systemctl enable --now dots-family.target"
echo ""
echo "To enable activity monitoring for a user:"
echo "  sudo systemctl enable --now dots-family-monitor@USERNAME.service"
echo ""
echo "To check status:"
echo "  sudo systemctl status dots-family.target"
echo "  sudo systemctl status dots-family-daemon"
echo "  sudo systemctl status dots-family-filter"
echo ""
echo "To view logs:"
echo "  sudo journalctl -f -u dots-family-daemon"
echo "  sudo journalctl -f -u dots-family-filter"
echo ""
echo "Configuration files:"
echo "  - Daemon: /etc/dots-family/"
echo "  - Filter: /etc/dots-family-filter/"
echo ""
echo "Log files can be viewed with journalctl or in /var/log/syslog"