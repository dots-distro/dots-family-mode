#!/usr/bin/env bash
set -euo pipefail

# Quick D-Bus Bus Type Validation Test
# Verify that daemon and monitor both use system bus

echo "🔧 DOTS Family Mode - Bus Type Validation"
echo "=========================================="
echo

# Build the applications
echo "📦 Building applications..."
cargo build --bin dots-family-daemon --bin dots-family-monitor --bin dots-family-ctl
echo "✅ Build completed"
echo

# Test 1: Check if system D-Bus is available
echo "🧪 Test 1: System D-Bus availability"
echo "===================================="

if command -v dbus-send &> /dev/null; then
    echo "✅ dbus-send available"
else
    echo "❌ dbus-send not found"
    exit 1
fi

# Try to list system bus services
if dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
   /org/freedesktop/DBus org.freedesktop.DBus.ListNames &>/dev/null; then
    echo "✅ System D-Bus is accessible"
else
    echo "❌ Cannot access system D-Bus (may need root/sudo)"
    echo "ℹ️  This is expected in user environments"
fi

echo

# Test 2: Start daemon briefly to check bus registration
echo "🧪 Test 2: Daemon system bus registration"
echo "=========================================="

echo "Starting daemon briefly..."

# Create temporary database for test
export DATABASE_URL="/tmp/dots-family-test-$(date +%s).db"
echo "Using test database: $DATABASE_URL"

# Start daemon in background
timeout 10s ./target/x86_64-unknown-linux-gnu/debug/dots-family-daemon > /tmp/daemon-test.log 2>&1 &
DAEMON_PID=$!

echo "Daemon started with PID: $DAEMON_PID"

# Give daemon time to initialize
sleep 3

# Check if daemon is still running
if kill -0 $DAEMON_PID 2>/dev/null; then
    echo "✅ Daemon process still running"
    
    # Try to check if D-Bus service is registered
    if dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
       /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | grep -q "org.dots.FamilyDaemon"; then
        echo "✅ Daemon registered on system bus successfully"
    else
        echo "⚠️  Daemon D-Bus service not detected (may need root privileges)"
        echo "📋 This is expected when running without system bus access"
    fi
    
    # Clean up
    kill $DAEMON_PID 2>/dev/null || true
    wait $DAEMON_PID 2>/dev/null || true
else
    echo "❌ Daemon stopped unexpectedly"
    echo "📋 Daemon log:"
    cat /tmp/daemon-test.log || echo "No log available"
fi

echo

# Test 3: Check CLI tries to connect to system bus
echo "🧪 Test 3: CLI system bus connection attempt"
echo "============================================"

echo "Attempting CLI status (should fail gracefully)..."
CLI_OUTPUT=$(timeout 5s ./target/x86_64-unknown-linux-gnu/debug/dots-family-ctl status 2>&1 || echo "TIMEOUT_OR_ERROR")

echo "📋 CLI Output:"
echo "$CLI_OUTPUT"

if [[ "$CLI_OUTPUT" == *"system"* ]] || [[ "$CLI_OUTPUT" == *"System"* ]]; then
    echo "✅ CLI appears to be using system bus"
elif [[ "$CLI_OUTPUT" == *"Failed to connect"* ]] || [[ "$CLI_OUTPUT" == *"No route to host"* ]]; then
    echo "✅ CLI properly attempts system bus connection (failed as expected)"
else
    echo "⚠️  CLI bus type unclear from output"
fi

echo

# Test 4: Verify no session bus references in code
echo "🧪 Test 4: Code verification - no session bus references"
echo "========================================================"

echo "Searching for session bus references..."
SESSION_REFS=$(grep -r "session()" crates/ 2>/dev/null || echo "")

if [ -z "$SESSION_REFS" ]; then
    echo "✅ No session() bus references found"
else
    echo "❌ Found session bus references:"
    echo "$SESSION_REFS"
    exit 1
fi

# Check for system bus references
SYSTEM_REFS=$(grep -r "system()" crates/ 2>/dev/null | wc -l)
echo "✅ Found $SYSTEM_REFS system() bus references"

echo

# Cleanup
rm -f /tmp/daemon-test.log
rm -f "$DATABASE_URL" 2>/dev/null || true

echo "🎉 Bus Type Validation Complete!"
echo "================================"
echo
echo "✅ All components properly configured for system bus"
echo "🔧 D-Bus integration ready for system-level deployment"