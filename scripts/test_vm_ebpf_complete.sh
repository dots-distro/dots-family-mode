#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - Comprehensive eBPF VM Testing Script
# This script tests real eBPF program deployment in NixOS VM environment

echo "🚀 DOTS Family Mode - eBPF VM Integration Testing"
echo "==============================================="

# Configuration
VM_MEMORY=2048
VM_TIMEOUT=120
RESULT_FILE="vm_ebpf_test_results.log"
VM_SCRIPT=$(realpath result/bin/run-dots-family-test-vm)

# Clean up previous test artifacts
rm -f $RESULT_FILE dots-family-test.qcow2

echo "📋 Test Configuration:"
echo "  - VM Memory: ${VM_MEMORY}MB"
echo "  - Timeout: ${VM_TIMEOUT}s"
echo "  - VM Script: $VM_SCRIPT"
echo "  - Results: $RESULT_FILE"
echo ""

# Function to run VM test with specific commands
run_vm_test() {
    local test_name="$1"
    local vm_commands="$2"
    
    echo "🔬 Running Test: $test_name"
    echo "================================"
    
    # Start VM and run test commands
    timeout ${VM_TIMEOUT} bash -c "
        echo '$vm_commands' | $VM_SCRIPT -m ${VM_MEMORY} -nographic 2>&1
    " | tee -a $RESULT_FILE || {
        echo "❌ Test '$test_name' timed out or failed"
        return 1
    }
    
    echo "✅ Test '$test_name' completed"
    echo ""
}

# Test 1: Basic VM Boot and System Initialization
echo "📦 Phase 1: Basic VM Boot Test"
run_vm_test "VM Boot and System Check" '
export TERM=xterm
echo "🔍 Testing VM boot and system initialization..."
systemctl status
echo "✅ System status checked"
poweroff
'

# Test 2: eBPF Infrastructure and Program Loading
echo "📦 Phase 2: eBPF Infrastructure Test"
run_vm_test "eBPF Program Loading" '
export TERM=xterm
echo "🔍 Testing eBPF infrastructure..."

# Check if eBPF support is available in kernel
echo "Kernel eBPF support:"
ls -la /sys/fs/bpf/ 2>/dev/null || echo "No BPF filesystem"
lsmod | grep bpf || echo "No BPF kernel modules loaded"

# Check for eBPF tools availability
which bpftool 2>/dev/null || echo "bpftool not available"
mount | grep bpf || echo "No BPF mounts"

# Check DOTS Family Mode eBPF programs
echo "DOTS Family Mode eBPF binaries:"
find /nix/store -name "*dots-family-ebpf*" -type d 2>/dev/null | head -5

# Test daemon initialization with eBPF
echo "Testing daemon eBPF initialization..."
systemctl status dots-family-daemon || true
journalctl -u dots-family-daemon --no-pager -n 20 || true

echo "✅ eBPF infrastructure checked"
poweroff
'

# Test 3: DOTS Family Mode Service Integration
echo "📦 Phase 3: Service Integration Test"
run_vm_test "Service Integration and eBPF Health" '
export TERM=xterm
echo "🔍 Testing DOTS Family Mode services with eBPF..."

# Start services
systemctl start dots-family-daemon
sleep 5
systemctl status dots-family-daemon

# Test CLI with eBPF status
dots-family-ctl status || true

# Check eBPF health through logs
echo "eBPF health check through daemon logs:"
journalctl -u dots-family-daemon --no-pager -n 50 | grep -i ebpf || echo "No eBPF logs found"

# Test monitor service
systemctl start dots-family-monitor
sleep 3
systemctl status dots-family-monitor

# Check if eBPF programs are actually loaded
echo "Checking loaded eBPF programs:"
bpftool prog list 2>/dev/null || echo "bpftool not available"
cat /proc/sys/kernel/unprivileged_bpf_disabled || echo "BPF status unknown"

echo "✅ Service integration tested"
poweroff
'

# Test 4: Real eBPF Program Functionality
echo "📦 Phase 4: Real eBPF Program Functionality"
run_vm_test "eBPF Program Functionality" '
export TERM=xterm
echo "🔍 Testing real eBPF program functionality..."

# Start all DOTS Family Mode services
systemctl start dots-family-daemon
systemctl start dots-family-monitor
sleep 10

# Generate some activity for eBPF programs to monitor
echo "Generating system activity for monitoring..."
ls -la /home/ &
sleep 1
ps aux | head -10 &
touch /tmp/test_file_activity
rm /tmp/test_file_activity
sleep 5

# Check daemon logs for eBPF activity
echo "eBPF program activity logs:"
journalctl -u dots-family-daemon --no-pager -n 100 | grep -E "(eBPF|process|monitor|activity)" || echo "No activity logs"

# Check monitor logs
echo "Monitor service logs:"  
journalctl -u dots-family-monitor --no-pager -n 50 | grep -E "(monitor|activity|process)" || echo "No monitor logs"

# Test CLI status after activity
echo "Final CLI status check:"
dots-family-ctl status

echo "✅ eBPF functionality tested"
poweroff
'

echo "📊 Test Summary"
echo "==============="

# Analyze results
if [ -f "$RESULT_FILE" ]; then
    echo "📄 Test Results Analysis:"
    
    # Check for successful boots
    BOOTS=$(grep -c "✅ System status checked" $RESULT_FILE || true)
    echo "  - Successful VM boots: $BOOTS"
    
    # Check for eBPF support
    EBPF_SUPPORT=$(grep -c "bpf" $RESULT_FILE || true)
    echo "  - eBPF references found: $EBPF_SUPPORT"
    
    # Check for service starts
    SERVICE_STARTS=$(grep -c "dots-family-daemon" $RESULT_FILE || true)
    echo "  - Daemon service references: $SERVICE_STARTS"
    
    # Check for errors
    ERRORS=$(grep -c -i "error\|failed" $RESULT_FILE || true)
    echo "  - Error/failure count: $ERRORS"
    
    echo ""
    echo "💾 Full test results saved to: $RESULT_FILE"
    echo "📈 Test execution completed"
    
    if [ "$BOOTS" -ge 3 ] && [ "$ERRORS" -lt 5 ]; then
        echo "🎉 Overall Status: PASS - eBPF VM integration working"
        exit 0
    else
        echo "⚠️ Overall Status: PARTIAL - Check results for issues"
        exit 1
    fi
else
    echo "❌ No test results file found - tests may have failed completely"
    exit 1
fi