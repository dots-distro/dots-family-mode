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
