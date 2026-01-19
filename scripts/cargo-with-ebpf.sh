#!/usr/bin/env bash
# Cargo wrapper with eBPF development environment

set -euo pipefail

if [[ -z "${IN_NIX_SHELL:-}" ]]; then
    echo "Error: Must run inside 'nix develop' shell"
    echo "Run: nix develop"
    exit 1
fi

EBPF_RESULT=$(nix path-info .#dots-family-ebpf)

export BPF_PROCESS_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/process-monitor"
export BPF_NETWORK_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/network-monitor"  
export BPF_FILESYSTEM_MONITOR_PATH="$EBPF_RESULT/target/bpfel-unknown-none/release/filesystem-monitor"

echo "Running cargo with eBPF environment configured..."
exec cargo "$@"