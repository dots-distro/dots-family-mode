#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "DOTS Family Mode - VM Manual Validation"
echo "=========================================="
echo "Timestamp: $(date)"
echo ""

echo "Test 1: System Service Status"
echo "------------------------------"
systemctl status dots-family-daemon.service --no-pager || echo "Daemon service check failed"
echo ""

echo "Test 2: DBus Service"
echo "--------------------"
busctl status org.dots.FamilyDaemon 2>&1 || echo "DBus service not available yet"
echo ""

echo "Test 3: CLI Tool"
echo "----------------"
which dots-family-ctl && dots-family-ctl --version || echo "CLI not found"
echo ""

echo "Test 4: Database"
echo "----------------"
ls -lh /var/lib/dots-family/ || echo "Database directory not found"
echo ""

echo "Test 5: User Groups"
echo "-------------------"
getent group dots-family-parents && echo "✓ Parents group exists"
getent group dots-family-children && echo "✓ Children group exists"
id parent && echo "✓ Parent user exists"
id child && echo "✓ Child user exists"
echo ""

echo "Test 6: Service Logs"
echo "--------------------"
journalctl -u dots-family-daemon.service -n 20 --no-pager
echo ""

echo "Test 7: DBus Policy"
echo "-------------------"
ls -l /etc/dbus-1/system.d/*dots* 2>/dev/null || echo "No DBus policies found"
echo ""

echo "Test 8: Packages Installed"
echo "---------------------------"
nix-store -qR /run/current-system | grep -i dots | head -10 || echo "No DOTS packages found in system closure"
echo ""

echo "=========================================="
echo "Validation Complete"
echo "=========================================="
