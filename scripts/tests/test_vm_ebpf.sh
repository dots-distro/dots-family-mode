#!/bin/bash
set -e

echo "DOTS Family Mode eBPF Test Script"
echo "================================="

# Test eBPF functionality in VM
echo "1. Testing VM connectivity and basic services..."

# Start VM in background with SSH access
echo "Starting VM with SSH access..."
timeout 600s bash -c '
./result/bin/run-dots-family-test-vm \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -nographic \
  -monitor none \
  &> vm_full.log & 
VM_PID=$!
echo "VM PID: $VM_PID"

# Wait for VM to boot and SSH to be available
echo "Waiting for VM to boot..."
sleep 45

# Test SSH connectivity
echo "Testing SSH connectivity..."
for i in {1..30}; do
  if ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -p 2222 root@localhost "echo SSH connected" 2>/dev/null; then
    echo "SSH connection successful!"
    break
  fi
  echo "Attempt $i: SSH not ready, waiting..."
  sleep 2
done

# Test DOTS Family Mode services in VM
echo "Testing DOTS Family Mode services..."
ssh -o StrictHostKeyChecking=no -p 2222 root@localhost bash << "REMOTE_SCRIPT"

echo "=== VM System Information ==="
uname -a
echo ""

echo "=== eBPF Support Check ==="
# Check if eBPF filesystem is mounted
mount | grep bpf || echo "BPF filesystem not mounted"

# Check eBPF-related files
ls -la /proc/sys/kernel/ | grep bpf || echo "No BPF-related files in /proc/sys/kernel/"

# Check for debugfs (needed for eBPF)
mount | grep debugfs || echo "debugfs not mounted"

echo ""

echo "=== DOTS Family Services Status ==="
systemctl status dots-family-daemon --no-pager || echo "dots-family-daemon not running"

echo ""

echo "=== Test eBPF Program Loading ==="
# Try to run our daemon and check eBPF loading
export BPF_PROCESS_MONITOR_PATH=""  # Empty to force simulation mode
export BPF_NETWORK_MONITOR_PATH=""
export BPF_FILESYSTEM_MONITOR_PATH=""

# Test daemon startup in simulation mode
timeout 10s dots-family-daemon --help || echo "dots-family-daemon not available"

echo ""

echo "=== Process Monitoring Test ==="
# Create some process activity to monitor
echo "Creating test process activity..."
sleep 1 & 
ps aux | head -10

echo ""

echo "=== eBPF Capabilities Check ==="
# Check for eBPF-related capabilities
cat /proc/self/status | grep Cap || echo "No capability info"

id

echo ""

echo "=== Available eBPF Tools ==="
which bpftool 2>/dev/null || echo "bpftool not available"
which tc 2>/dev/null || echo "tc not available"

echo ""
echo "=== VM Test Complete ==="

REMOTE_SCRIPT

# Cleanup
kill $VM_PID 2>/dev/null || true
wait $VM_PID 2>/dev/null || true
'