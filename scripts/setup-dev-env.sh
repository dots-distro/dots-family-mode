#!/usr/bin/env bash
# Setup development environment with eBPF program paths

set -euo pipefail

# Ensure we're in nix develop
if [[ -z "${IN_NIX_SHELL:-}" ]]; then
    echo "Error: Must run inside 'nix develop' shell"
    echo "Run: nix develop"
    exit 1
fi

# Build eBPF programs first
echo "Building eBPF programs..."
nix build .#dots-family-ebpf --no-link

# Find the latest eBPF build result
EBPF_RESULT=$(nix path-info .#dots-family-ebpf)

# Set environment variables for development
export BPF_PROCESS_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/process-monitor"
export BPF_NETWORK_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/network-monitor"
export BPF_FILESYSTEM_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/filesystem-monitor"

# Verify the files exist
echo "eBPF programs available at:"
echo "  Process: $BPF_PROCESS_MONITOR_PATH"
echo "  Network: $BPF_NETWORK_MONITOR_PATH"
echo "  Filesystem: $BPF_FILESYSTEM_MONITOR_PATH"

# Verify they're actual files
for path in "$BPF_PROCESS_MONITOR_PATH" "$BPF_NETWORK_MONITOR_PATH" "$BPF_FILESYSTEM_MONITOR_PATH"; do
    if [[ ! -f "$path" ]]; then
        echo "Warning: $path does not exist"
    fi
done

echo ""
echo "Development environment ready!"
echo "eBPF environment variables are now set for this shell session."
echo ""
echo "Test with: cargo build --package dots-family-daemon"
echo "Expected: 'Permission denied' warnings (normal in Nix), but no 'simulation mode' warnings"