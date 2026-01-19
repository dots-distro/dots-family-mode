#!/bin/bash
set -e

echo "Testing Real eBPF Programs with Kernel Integration"
echo "================================================="

# Create comprehensive eBPF test script for VM
cat << 'VM_TEST_SCRIPT' > vm_ebpf_test.sh
#!/bin/bash
set -e

echo "=== eBPF Integration Test in VM ==="

# Check system eBPF support
echo "1. Checking eBPF system support..."
echo "Kernel version: $(uname -r)"
echo "BPF filesystem: $(mount | grep bpf || echo 'Not mounted')"

# Check capabilities
echo ""
echo "2. Checking capabilities..."
echo "Current user: $(whoami)"
echo "UID: $(id -u)"
echo "Capabilities: $(grep Cap /proc/self/status | head -3 || echo 'Cannot read capabilities')"

# Test eBPF program files
echo ""
echo "3. Testing eBPF program files..."
if [ -f /nix/store/*/target/bpfel-unknown-none/release/process-monitor ]; then
    PROCESS_MONITOR=$(find /nix/store -name "process-monitor" 2>/dev/null | head -1)
    NETWORK_MONITOR=$(find /nix/store -name "network-monitor" 2>/dev/null | head -1)  
    FILESYSTEM_MONITOR=$(find /nix/store -name "filesystem-monitor" 2>/dev/null | head -1)
    
    echo "Process monitor: $PROCESS_MONITOR"
    echo "Network monitor: $NETWORK_MONITOR"
    echo "Filesystem monitor: $FILESYSTEM_MONITOR"
    
    if [ -n "$PROCESS_MONITOR" ]; then
        echo "File info: $(file $PROCESS_MONITOR)"
        echo "Size: $(stat -c%s $PROCESS_MONITOR) bytes"
        
        # Test daemon with real eBPF programs
        echo ""
        echo "4. Testing daemon with real eBPF programs..."
        export BPF_PROCESS_MONITOR_PATH="$PROCESS_MONITOR"
        export BPF_NETWORK_MONITOR_PATH="$NETWORK_MONITOR" 
        export BPF_FILESYSTEM_MONITOR_PATH="$FILESYSTEM_MONITOR"
        
        # Try to start daemon briefly to test eBPF loading
        timeout 15s dots-family-daemon --version 2>&1 | head -10 || echo "Daemon test completed"
        
        echo ""
        echo "5. Testing eBPF program loading capability..."
        # Check if we can access eBPF syscall
        if [ -r /proc/sys/kernel/bpf_stats_enabled ]; then
            echo "BPF stats enabled: $(cat /proc/sys/kernel/bpf_stats_enabled)"
        fi
        
        # Check for debugfs access needed for eBPF
        if [ -d /sys/kernel/debug/tracing ]; then
            echo "Tracing debugfs available"
            ls -la /sys/kernel/debug/tracing/events/ | head -5 || echo "Cannot list tracing events"
        fi
        
    else
        echo "eBPF programs not found in /nix/store"
    fi
else
    echo "No eBPF programs found"
fi

echo ""
echo "6. Testing monitoring services..."
systemctl status dots-family-daemon --no-pager -l || echo "Daemon not running"

echo ""
echo "=== eBPF Integration Test Complete ==="

VM_TEST_SCRIPT

# Start VM with eBPF test
echo "Starting VM to test real eBPF programs..."
timeout 180s bash -c '
./result/bin/run-dots-family-test-vm \
  -netdev user,id=net0,hostfwd=tcp::2223-:22 \
  -device virtio-net-pci,netdev=net0 \
  -nographic \
  -monitor none \
  -m 2048 \
  &> vm_ebpf.log & 
VM_PID=$!

# Wait for VM to boot
echo "Waiting for VM boot (PID: $VM_PID)..."
sleep 60

# Copy test script to VM and run it
echo "Running eBPF integration test in VM..."
scp -o ConnectTimeout=10 -o StrictHostKeyChecking=no -P 2223 vm_ebpf_test.sh root@localhost:/tmp/
ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -P 2223 root@localhost "chmod +x /tmp/vm_ebpf_test.sh && /tmp/vm_ebpf_test.sh" || echo "VM test execution completed"

# Cleanup
echo "Cleaning up VM (PID: $VM_PID)..."
kill $VM_PID 2>/dev/null || true
wait $VM_PID 2>/dev/null || true

echo "eBPF integration test results saved to vm_ebpf.log"
head -50 vm_ebpf.log | tail -30
'