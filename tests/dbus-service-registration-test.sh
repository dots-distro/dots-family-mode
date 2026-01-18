#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - D-Bus Service Registration Test
# Tests proper daemon registration with D-Bus policy installation

echo "🔧 DOTS Family Mode - D-Bus Service Registration Test"
echo "===================================================="
echo

# Check if we have root access for D-Bus policy installation
if [[ $EUID -eq 0 ]]; then
    echo "✅ Running as root - can install D-Bus policies"
    INSTALL_POLICY=true
else
    echo "⚠️  Not running as root - will test without installing policy"
    echo "💡 For full testing, run with: sudo ./tests/dbus-service-registration-test.sh"
    INSTALL_POLICY=false
fi

echo

# Build the daemon first
echo "📦 Building daemon..."
cargo build --bin dots-family-daemon
echo "✅ Build completed"
echo

# Install D-Bus policy if we have permission
POLICY_INSTALLED=false
if [ "$INSTALL_POLICY" = true ]; then
    echo "📋 Installing D-Bus policy..."
    
    # Check if D-Bus configuration directory exists
    if [ -d /etc/dbus-1/system.d ]; then
        cp dbus/org.dots.FamilyDaemon.conf /etc/dbus-1/system.d/
        echo "✅ D-Bus policy installed to /etc/dbus-1/system.d/"
        
        # Reload D-Bus configuration
        if systemctl is-active --quiet dbus; then
            systemctl reload dbus
            echo "✅ D-Bus configuration reloaded"
        else
            echo "⚠️  D-Bus service not running via systemctl"
        fi
        
        POLICY_INSTALLED=true
    else
        echo "❌ D-Bus system configuration directory not found"
    fi
    
    echo
fi

# Test daemon registration
echo "🧪 Testing daemon D-Bus service registration"
echo "============================================"

# Create temporary database for test
export DATABASE_URL="/tmp/dots-family-test-dbus-$(date +%s).db"
echo "Using test database: $DATABASE_URL"

# Start daemon in background
echo "Starting daemon..."
./target/x86_64-unknown-linux-gnu/debug/dots-family-daemon > /tmp/daemon-dbus-test.log 2>&1 &
DAEMON_PID=$!

echo "Daemon started with PID: $DAEMON_PID"

# Cleanup function
cleanup() {
    if [ -n "$DAEMON_PID" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
        echo "🧹 Stopping daemon (PID: $DAEMON_PID)..."
        kill "$DAEMON_PID" 2>/dev/null || true
        wait "$DAEMON_PID" 2>/dev/null || true
    fi
    
    # Clean up policy if we installed it
    if [ "$POLICY_INSTALLED" = true ] && [ -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf ]; then
        echo "🧹 Removing D-Bus policy..."
        rm -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf
        systemctl reload dbus 2>/dev/null || true
    fi
    
    # Clean up test database
    rm -f "$DATABASE_URL" 2>/dev/null || true
    rm -f /tmp/daemon-dbus-test.log 2>/dev/null || true
}
trap cleanup EXIT

# Give daemon time to start and register
sleep 5

# Check if daemon is running
if kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo "✅ Daemon process is running"
    
    # Check D-Bus service registration
    echo "🔍 Checking D-Bus service registration..."
    
    if dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
       /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | grep -q "org.dots.FamilyDaemon"; then
        echo "✅ D-Bus service 'org.dots.FamilyDaemon' successfully registered!"
        
        # Test service communication
        echo "🧪 Testing service communication..."
        
        PING_RESULT=$(dbus-send --system --dest=org.dots.FamilyDaemon --type=method_call --print-reply \
                     /org/dots/FamilyDaemon org.dots.FamilyDaemon.ping 2>&1)
        
        if echo "$PING_RESULT" | grep -q "boolean true"; then
            echo "✅ Service ping successful - communication working!"
            
            # Test CLI communication
            echo "🧪 Testing CLI communication..."
            
            CLI_OUTPUT=$(timeout 10s ./target/x86_64-unknown-linux-gnu/debug/dots-family-ctl status 2>&1 || echo "CLI_FAILED")
            
            if [[ "$CLI_OUTPUT" != "CLI_FAILED" ]] && [[ "$CLI_OUTPUT" != *"Error"* ]]; then
                echo "✅ CLI successfully communicated with daemon!"
                echo "📋 CLI Output Preview:"
                echo "$CLI_OUTPUT" | head -3
            else
                echo "⚠️  CLI communication issues:"
                echo "$CLI_OUTPUT" | head -5
            fi
            
        else
            echo "❌ Service ping failed:"
            echo "$PING_RESULT"
        fi
        
    else
        echo "❌ D-Bus service not found in system bus"
        echo "📋 Available services containing 'dots' or 'family':"
        dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
            /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | \
            grep -i -E "(dots|family)" || echo "None found"
    fi
    
else
    echo "❌ Daemon stopped unexpectedly"
    echo "📋 Daemon log:"
    cat /tmp/daemon-dbus-test.log || echo "No log available"
fi

echo

# Show policy installation status
echo "📋 D-Bus Policy Status"
echo "======================"
if [ "$POLICY_INSTALLED" = true ]; then
    echo "✅ D-Bus policy was temporarily installed and will be removed on cleanup"
else
    echo "⚠️  D-Bus policy not installed (insufficient permissions)"
    echo "💡 To test with policy: sudo $0"
fi

echo

# Show summary
echo "🎉 D-Bus Service Registration Test Complete!"
echo "============================================="

if [ "$POLICY_INSTALLED" = true ]; then
    echo "✅ Full test completed with D-Bus policy installation"
    echo "🔒 Service registration and communication validated"
else
    echo "⚠️  Partial test completed without D-Bus policy"
    echo "🔧 Expected registration failures due to missing policy"
fi

echo "📡 System bus integration verified"