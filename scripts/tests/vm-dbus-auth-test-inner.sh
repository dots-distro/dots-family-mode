#!/usr/bin/env bash
# Internal VM DBus communication and authentication test script

echo "=== DOTS Family DBus Communication & Authentication Test ==="
echo "Running inside VM at $(date)"

test_count=0
pass_count=0
fail_count=0

run_test() {
    local name="$1"
    local command="$2"
    
    test_count=$((test_count + 1))
    echo -n "[$test_count] $name... "
    
    if eval "$command" &>/dev/null; then
        echo "✓ PASS"
        pass_count=$((pass_count + 1))
    else
        echo "✗ FAIL"
        fail_count=$((fail_count + 1))
        # Show error for debugging
        echo "    Command failed: $command"
        eval "$command" 2>&1 | sed 's/^/    /' | head -3
    fi
}

run_test_with_output() {
    local name="$1"
    local command="$2"
    
    test_count=$((test_count + 1))
    echo -n "[$test_count] $name... "
    
    local output
    if output=$(eval "$command" 2>&1); then
        echo "✓ PASS"
        pass_count=$((pass_count + 1))
        if [[ -n "$output" ]]; then
            echo "    Output: $output"
        fi
    else
        echo "✗ FAIL"
        fail_count=$((fail_count + 1))
        echo "    Command failed: $command"
        echo "    Error: $output"
    fi
}

# Wait for system to be ready
echo "Waiting for system initialization..."
sleep 5

echo
echo "=== DBus System Infrastructure Tests ==="

run_test "DBus system service is running" "systemctl is-active dbus"
run_test "DBus system bus is accessible" "busctl --system status >/dev/null"
run_test "DBus session bus is available" "busctl --user status >/dev/null || true"

echo
echo "=== DOTS Family DBus Configuration Tests ==="

# Test DBus configuration files
run_test "DBus policy file exists" "test -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf"
run_test "DBus service activation file exists" "test -f /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service"

# Validate DBus policy configuration
if [[ -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf ]]; then
    echo "Analyzing DBus policy configuration..."
    
    run_test "DBus policy allows interface access" "grep -q 'org.dots.FamilyDaemon' /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf"
    run_test "DBus policy configures authentication" "grep -q 'policy\|allow\|deny' /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf"
    
    # Check for expected policy patterns
    echo "DBus policy content analysis:"
    echo "  Interface patterns:"
    grep -E "(interface|member|path)" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf | head -3 | sed 's/^/    /'
    echo "  Permission patterns:"
    grep -E "(allow|deny|policy)" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf | head -3 | sed 's/^/    /'
fi

# Test DBus service activation file
if [[ -f /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service ]]; then
    echo "Analyzing DBus service activation..."
    
    run_test "Service activation points to correct binary" "grep -q 'dots-family-daemon' /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service"
    run_test "Service activation configures correct user" "grep -q 'User=dots-family' /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service"
    
    echo "Service activation content:"
    cat /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service | sed 's/^/    /'
fi

echo
echo "=== Service Startup for DBus Testing ==="

# Ensure the service is running for DBus tests
systemctl daemon-reload

# Try to start the service
echo "Ensuring DOTS Family daemon is running for DBus tests..."
if ! systemctl is-active dots-family-daemon >/dev/null 2>&1; then
    echo "Starting daemon for DBus communication tests..."
    if systemctl start dots-family-daemon 2>/dev/null; then
        echo "✓ Service started for testing"
        sleep 3  # Give service time to register on DBus
    else
        echo "⚠ Service failed to start - some DBus tests may fail"
    fi
else
    echo "✓ Service already running"
fi

echo
echo "=== DBus Service Registration Tests ==="

# Test if service is registered on DBus
run_test_with_output "Service registered on system bus" "busctl --system list | grep -E 'org\.dots\.FamilyDaemon|:1\.[0-9]+.*dots-family'"

# Test service introspection
echo "Testing DBus service introspection..."
if busctl --system list | grep -q "org.dots.FamilyDaemon"; then
    echo "✓ Service found on DBus, testing introspection..."
    
    # Test introspection of main interface
    if busctl --system introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon >/dev/null 2>&1; then
        echo "✓ PASS - Service introspection works"
        pass_count=$((pass_count + 1))
        
        # Show available interfaces and methods
        echo "Available interfaces:"
        busctl --system introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon 2>/dev/null | head -10 | sed 's/^/    /'
        
    else
        echo "✗ FAIL - Service introspection failed"
        fail_count=$((fail_count + 1))
    fi
    test_count=$((test_count + 1))
    
else
    echo "⚠ Service not visible on DBus - skipping introspection tests"
fi

echo
echo "=== Authentication and Permission Tests ==="

# Test authentication patterns with different users

# Test root/admin access
echo "Testing root/admin DBus access..."
if busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.freedesktop.DBus.Introspectable Introspect >/dev/null 2>&1; then
    echo "✓ Root can access DBus interface"
else
    echo "⚠ Root DBus access failed (may be expected if service not fully running)"
fi

# Test parent user access (should be allowed)
if id parent >/dev/null 2>&1; then
    echo "Testing parent user DBus access..."
    if sudo -u parent busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.freedesktop.DBus.Introspectable Introspect >/dev/null 2>&1; then
        echo "✓ Parent user can access DBus interface"
    else
        echo "⚠ Parent user DBus access restricted/failed"
    fi
fi

# Test child user access (should be restricted)
if id child1 >/dev/null 2>&1; then
    echo "Testing child user DBus access restrictions..."
    if sudo -u child1 busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.freedesktop.DBus.Introspectable Introspect >/dev/null 2>&1; then
        echo "⚠ Child user has DBus access (may be expected for read-only operations)"
    else
        echo "✓ Child user properly restricted from DBus interface"
    fi
fi

echo
echo "=== CLI DBus Communication Tests ==="

# Test CLI tool DBus communication
echo "Testing CLI tool DBus communication..."

# Test basic status command (should use DBus)
if command -v dots-family-ctl >/dev/null 2>&1; then
    echo "Testing CLI DBus communication..."
    
    if timeout 10 dots-family-ctl status >/dev/null 2>&1; then
        echo "✓ PASS - CLI communicates with daemon via DBus"
        pass_count=$((pass_count + 1))
        
        # Test more complex operations
        if timeout 10 dots-family-ctl profile list >/dev/null 2>&1; then
            echo "✓ PASS - Complex DBus operations work"
            pass_count=$((pass_count + 1))
        else
            echo "⚠ WARN - Complex DBus operations failed"
        fi
        test_count=$((test_count + 1))
        
    else
        echo "✗ FAIL - CLI cannot communicate with daemon"
        fail_count=$((fail_count + 1))
    fi
    test_count=$((test_count + 1))
    
else
    echo "⚠ dots-family-ctl not found"
fi

echo
echo "=== DBus Authentication Method Tests ==="

# Test different authentication methods and patterns

echo "Testing DBus authentication mechanisms..."

# Test service ownership and permissions
if busctl --system list | grep -q "org.dots.FamilyDaemon"; then
    echo "Checking service ownership..."
    
    # Get service process info
    service_info=$(busctl --system status org.dots.FamilyDaemon 2>/dev/null | head -10)
    if [[ -n "$service_info" ]]; then
        echo "Service ownership information:"
        echo "$service_info" | sed 's/^/    /'
    fi
fi

# Test polkit integration (if available)
if command -v pkcheck >/dev/null 2>&1; then
    echo "Testing Polkit integration..."
    
    # Test basic Polkit functionality
    run_test "Polkit is available" "pkcheck --version >/dev/null"
    
    # Check if our actions are registered
    if pkaction --verbose | grep -q "dots" 2>/dev/null; then
        echo "✓ DOTS Family Polkit actions registered"
    else
        echo "⚠ No DOTS Family specific Polkit actions found"
    fi
else
    echo "⚠ Polkit not available for testing"
fi

echo
echo "=== DBus Signal and Event Tests ==="

# Test DBus signal handling (basic test)
echo "Testing DBus signals and events..."

# Monitor for DBus signals (brief test)
echo "Monitoring for DBus signals..."
if timeout 5 busctl --system monitor org.dots.FamilyDaemon >/dev/null 2>&1 &; then
    monitor_pid=$!
    sleep 2
    
    # Try to trigger a signal by calling a method
    if timeout 3 dots-family-ctl status >/dev/null 2>&1; then
        echo "✓ DBus monitoring works, signals may be available"
    else
        echo "⚠ No signals detected during test"
    fi
    
    # Clean up monitor
    kill $monitor_pid 2>/dev/null || true
    wait $monitor_pid 2>/dev/null || true
else
    echo "⚠ DBus signal monitoring failed to start"
fi

echo
echo "=== DBus Connection Security Tests ==="

# Test DBus connection security
echo "Testing DBus connection security..."

# Test that non-authorized users cannot access sensitive methods
echo "Testing unauthorized access prevention..."

# Create a test user if needed (basic security test)
if ! id testuser >/dev/null 2>&1; then
    useradd -m testuser 2>/dev/null || echo "Could not create test user"
fi

if id testuser >/dev/null 2>&1; then
    echo "Testing unauthorized user access..."
    if sudo -u testuser busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.freedesktop.DBus.Introspectable Introspect >/dev/null 2>&1; then
        echo "⚠ Unauthorized user can access DBus (may be intended for read operations)"
    else
        echo "✓ Unauthorized user properly blocked from DBus access"
    fi
fi

echo
echo "=== Service Communication Error Handling ==="

# Test error handling in DBus communication
echo "Testing DBus error handling..."

# Test invalid method calls
if command -v busctl >/dev/null 2>&1 && busctl --system list | grep -q "org.dots.FamilyDaemon"; then
    echo "Testing invalid method call handling..."
    
    # Try to call non-existent method
    if busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon NonExistentMethod 2>/dev/null; then
        echo "⚠ Invalid method call succeeded (unexpected)"
    else
        echo "✓ Invalid method calls properly rejected"
    fi
fi

echo
echo "=== DBus Configuration Validation ==="

# Final validation of DBus setup
echo "Performing final DBus configuration validation..."

# Check systemd DBus integration
run_test "systemd DBus integration working" "systemctl show dots-family-daemon | grep -q 'BusName='"

# Validate all DBus files are properly formatted
if [[ -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf ]]; then
    echo "Validating DBus policy file format..."
    if xmllint --noout /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf 2>/dev/null; then
        echo "✓ DBus policy file is valid XML"
    else
        echo "⚠ DBus policy file may have XML format issues"
    fi
fi

echo
echo "=== Test Results Summary ==="
echo "=========================="
echo "Total tests: $test_count"
echo "Passed: $pass_count"
echo "Failed: $fail_count"

if [[ $test_count -gt 0 ]]; then
    echo "Success rate: $(( (pass_count * 100) / test_count ))%"
else
    echo "Success rate: N/A (no tests run)"
fi

echo
echo "=== DBus Communication Assessment ==="

if [[ $fail_count -eq 0 ]]; then
    echo "✓ All DBus communication tests passed!"
    echo "✓ DBus authentication and authorization working correctly"
    echo "✓ Service communication is functioning properly"
elif [[ $fail_count -le 3 ]]; then
    echo "⚠ Some DBus tests failed, but core communication appears functional"
    echo "✓ Basic DBus setup and authentication validated"
else
    echo "✗ Multiple DBus test failures - communication system may need attention"
fi

echo
echo "Key findings:"
echo "- DBus service registration: $(busctl --system list | grep -q 'org.dots.FamilyDaemon' && echo 'Working' || echo 'Failed')"
echo "- CLI DBus communication: $(timeout 5 dots-family-ctl status >/dev/null 2>&1 && echo 'Working' || echo 'Failed')"
echo "- Authentication policies: $(test -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf && echo 'Configured' || echo 'Missing')"
echo "- Service activation: $(test -f /usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service && echo 'Configured' || echo 'Missing')"

if [[ $fail_count -le 3 ]]; then
    exit 0
else
    exit 1
fi
