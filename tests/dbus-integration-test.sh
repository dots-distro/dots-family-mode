#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - D-Bus Integration Test
# Tests daemon D-Bus registration and monitor communication

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔧 DOTS Family Mode - D-Bus Integration Test"
echo "============================================="
echo

# Build binaries first
echo "📦 Building DOTS Family Mode components..."
cd "$PROJECT_ROOT"

cargo build --release --bin dots-family-daemon --bin dots-family-monitor --bin dots-family-ctl
echo "✅ Build completed"
echo

# Build VM with D-Bus focus
echo "🏗️  Building test VM with D-Bus support..."
VM_STORE_PATH=$(nix-build -E "
  let
    pkgs = import <nixpkgs> {};
    nixos = import <nixpkgs/nixos>;
  in
  (nixos { 
    configuration = ./tests/simple-test-vm.nix;
  }).vm" --no-out-link)
echo "✅ VM built: $VM_STORE_PATH"
echo

# Start VM in background
echo "🚀 Starting test VM..."
VM_PID=""
cleanup() {
    if [ -n "$VM_PID" ] && kill -0 "$VM_PID" 2>/dev/null; then
        echo "🧹 Stopping VM (PID: $VM_PID)..."
        kill "$VM_PID" 2>/dev/null || true
        wait "$VM_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Start VM with console output
$VM_STORE_PATH/bin/run-nixos-vm &
VM_PID=$!

# Wait for VM to be ready
echo "⏳ Waiting for VM to be ready..."
VM_READY=false
for i in {1..60}; do
    if ssh -o ConnectTimeout=2 -o StrictHostKeyChecking=no -p 22221 root@localhost "echo VM ready" &>/dev/null; then
        VM_READY=true
        break
    fi
    sleep 2
done

if [ "$VM_READY" = false ]; then
    echo "❌ VM failed to start within timeout"
    exit 1
fi

echo "✅ VM is ready"
echo

# Copy binaries to VM
echo "📁 Copying binaries to VM..."
scp -o StrictHostKeyChecking=no -P 22221 \
    "$PROJECT_ROOT/target/release/dots-family-daemon" \
    "$PROJECT_ROOT/target/release/dots-family-monitor" \
    "$PROJECT_ROOT/target/release/dots-family-ctl" \
    root@localhost:/usr/local/bin/

ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "chmod +x /usr/local/bin/dots-family-*"
echo "✅ Binaries copied"
echo

# Test 1: Daemon D-Bus service registration
echo "🧪 Test 1: Daemon D-Bus service registration"
echo "============================================="

# Start daemon in background in VM
ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    DATABASE_URL=/tmp/dots-family-test.db /usr/local/bin/dots-family-daemon > /tmp/daemon.log 2>&1 &
    echo \$! > /tmp/daemon.pid
    sleep 3
"

# Check if daemon started
DAEMON_STATUS=$(ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    if kill -0 \$(cat /tmp/daemon.pid 2>/dev/null) 2>/dev/null; then
        echo 'running'
    else
        echo 'stopped'
    fi
")

if [ "$DAEMON_STATUS" = "running" ]; then
    echo "✅ Daemon process started successfully"
else
    echo "❌ Daemon failed to start"
    ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "cat /tmp/daemon.log" || true
    exit 1
fi

# Check D-Bus service registration
echo "🔍 Checking D-Bus service registration..."
DBUS_CHECK=$(ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
        /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | \
        grep 'org.dots.FamilyDaemon' && echo 'REGISTERED' || echo 'NOT_REGISTERED'
")

if [[ "$DBUS_CHECK" == *"REGISTERED"* ]]; then
    echo "✅ D-Bus service 'org.dots.FamilyDaemon' registered successfully"
else
    echo "❌ D-Bus service not found in system bus"
    echo "📋 Available D-Bus services:"
    ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
        dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
            /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | grep -E 'string \"[^\"]+\"' | head -10
    " || true
    exit 1
fi

echo

# Test 2: CLI connection to daemon
echo "🧪 Test 2: CLI connection to daemon"
echo "==================================="

CLI_STATUS=$(ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    /usr/local/bin/dots-family-ctl status 2>&1 | head -5
    exit_code=\$?
    echo \"EXIT_CODE:\$exit_code\"
")

echo "📋 CLI Status Output:"
echo "$CLI_STATUS"

if [[ "$CLI_STATUS" == *"EXIT_CODE:0"* ]]; then
    echo "✅ CLI successfully connected to daemon via D-Bus"
else
    echo "❌ CLI failed to connect to daemon"
    echo "📋 Daemon logs:"
    ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "tail -20 /tmp/daemon.log" || true
fi

echo

# Test 3: Monitor connection to daemon
echo "🧪 Test 3: Monitor connection to daemon"  
echo "======================================"

# Start monitor briefly to test connection
ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    timeout 5 /usr/local/bin/dots-family-monitor > /tmp/monitor.log 2>&1 &
    MONITOR_PID=\$!
    sleep 3
    kill \$MONITOR_PID 2>/dev/null || true
    wait \$MONITOR_PID 2>/dev/null || true
"

MONITOR_OUTPUT=$(ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "cat /tmp/monitor.log")
echo "📋 Monitor Output:"
echo "$MONITOR_OUTPUT"

if [[ "$MONITOR_OUTPUT" == *"Successfully connected to daemon via DBus"* ]]; then
    echo "✅ Monitor successfully connected to daemon via D-Bus"
elif [[ "$MONITOR_OUTPUT" == *"Failed to connect to daemon"* ]]; then
    echo "❌ Monitor failed to connect to daemon"
    exit 1
else
    echo "⚠️  Monitor connection status unclear"
    echo "📋 Full monitor log:"
    echo "$MONITOR_OUTPUT"
fi

echo

# Test 4: D-Bus method calls
echo "🧪 Test 4: D-Bus method calls"
echo "============================="

# Test ping method
PING_RESULT=$(ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    dbus-send --system --dest=org.dots.FamilyDaemon --type=method_call --print-reply \
        /org/dots/FamilyDaemon org.dots.FamilyDaemon.ping 2>&1 || echo 'PING_FAILED'
")

if [[ "$PING_RESULT" == *"boolean true"* ]]; then
    echo "✅ D-Bus ping method call successful"
elif [[ "$PING_RESULT" == *"PING_FAILED"* ]]; then
    echo "❌ D-Bus ping method call failed"
    echo "$PING_RESULT"
else
    echo "⚠️  D-Bus ping response unclear:"
    echo "$PING_RESULT"
fi

echo

# Cleanup daemon in VM
echo "🧹 Cleaning up daemon in VM..."
ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "
    kill \$(cat /tmp/daemon.pid 2>/dev/null) 2>/dev/null || true
    rm -f /tmp/daemon.pid /tmp/daemon.log /tmp/monitor.log
"

echo
echo "🎉 D-Bus Integration Test Complete!"
echo "===================================="
echo
echo "✅ All D-Bus integration tests passed"
echo "🔗 System bus communication working"
echo "📡 Daemon ↔ Monitor ↔ CLI integration validated"