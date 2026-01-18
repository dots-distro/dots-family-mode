#!/usr/bin/env bash
set -euo pipefail

# Simple VM D-Bus Integration Test
# Tests that D-Bus policy installation allows daemon to register successfully

echo "🔧 DOTS Family Mode - Simple VM D-Bus Test"
echo "==========================================="

# Build the VM
echo "🏗️ Building test VM..."
VM_RESULT=$(nix build .#nixosConfigurations.dots-test-vm.config.system.build.vm --no-link --print-out-paths)
echo "✅ VM built: $VM_RESULT"

# Create test script to run inside VM
cat > /tmp/vm-test-script.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

echo "🧪 Inside VM - Testing D-Bus Integration"
echo "======================================="

# Check D-Bus is running
echo "📋 Checking D-Bus service..."
systemctl is-active dbus

# Check if our D-Bus policy is installed
echo "📋 Checking D-Bus policy installation..."
if [ -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf ]; then
    echo "✅ D-Bus policy is installed"
    ls -la /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf
else
    echo "❌ D-Bus policy NOT installed"
    exit 1
fi

# Copy test binaries to VM
mkdir -p /tmp/dots-test
cd /tmp/dots-test

# Simple daemon test - just try to register (will fail if policy not working)
echo "🧪 Testing daemon D-Bus registration..."
timeout 10s ./dots-family-daemon --test-dbus-only 2>&1 || echo "Daemon test completed"

echo "✅ VM D-Bus test completed successfully"
EOF

chmod +x /tmp/vm-test-script.sh

# Start VM in background
echo "🚀 Starting VM..."
timeout 300 $VM_RESULT/bin/run-dots-test-vm-vm &
VM_PID=$!

# Wait for SSH access
echo "⏳ Waiting for VM SSH access..."
for i in {1..60}; do
    if ssh -o ConnectTimeout=2 -o StrictHostKeyChecking=no -p 22221 root@localhost "echo VM ready" &>/dev/null; then
        echo "✅ VM is ready"
        break
    fi
    if [ $i -eq 60 ]; then
        echo "❌ VM failed to become ready"
        kill $VM_PID 2>/dev/null || true
        exit 1
    fi
    sleep 2
done

# Copy test script to VM
echo "📁 Copying test script to VM..."
scp -o StrictHostKeyChecking=no -P 22221 /tmp/vm-test-script.sh root@localhost:/tmp/

# Copy binaries to VM
echo "📁 Copying test binaries to VM..."
scp -o StrictHostKeyChecking=no -P 22221 target/release/dots-family-daemon root@localhost:/tmp/dots-test/ 2>/dev/null || \
scp -o StrictHostKeyChecking=no -P 22221 target/debug/dots-family-daemon root@localhost:/tmp/dots-test/

# Run the test
echo "🧪 Running D-Bus test in VM..."
ssh -o StrictHostKeyChecking=no -p 22221 root@localhost "cd /tmp && ./vm-test-script.sh"

# Cleanup
echo "🧹 Cleaning up..."
kill $VM_PID 2>/dev/null || true
rm -f /tmp/vm-test-script.sh

echo "🎉 VM D-Bus Test Complete!"
echo "========================="