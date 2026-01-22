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
