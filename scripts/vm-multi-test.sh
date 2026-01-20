#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_help() {
    cat << EOF
DOTS Family Mode VM Test Runner

Usage: $0 [WM] [ACTION]

Window Managers:
  niri        Test with Niri window manager
  sway        Test with Sway window manager  
  hyprland    Test with Hyprland window manager
  all         Test all window managers sequentially

Actions:
  build       Build VM for specified WM
  test        Run tests on existing VM
  run         Build and test VM
  clean       Clean up VM artifacts

Examples:
  $0 niri build      Build Niri VM
  $0 sway test       Test existing Sway VM
  $0 hyprland run    Build and test Hyprland VM
  $0 all test        Test all existing VMs

Environment Variables:
  VM_MEMORY       VM memory in MB (default: 4096)
  VM_CPUS         VM CPU count (default: 4)
  SSH_PORT_BASE   SSH port base (default: 10022)
EOF
}

# Configuration
VM_MEMORY="${VM_MEMORY:-4096}"
VM_CPUS="${VM_CPUS:-4}"
SSH_PORT_BASE="${SSH_PORT_BASE:-10022}"

get_vm_config() {
    local wm="$1"
    case "$wm" in
        niri)     echo "vm-config-niri.nix" ;;
        sway)     echo "vm-config-sway.nix" ;;
        hyprland) echo "vm-config-hyprland.nix" ;;
        *)        echo "vm-config.nix" ;;
    esac
}

get_ssh_port() {
    local wm="$1"
    case "$wm" in
        niri)     echo $((SSH_PORT_BASE + 0)) ;;
        sway)     echo $((SSH_PORT_BASE + 1)) ;;
        hyprland) echo $((SSH_PORT_BASE + 2)) ;;
        *)        echo "$SSH_PORT_BASE" ;;
    esac
}

build_vm() {
    local wm="$1"
    local config
    config=$(get_vm_config "$wm")
    
    echo "Building VM for $wm using $config..."
    
    if [ ! -f "$config" ]; then
        echo "Error: Configuration file $config not found"
        return 1
    fi
    
    nixos-rebuild build-vm --flake .#"$wm-test" || {
        echo "VM build failed for $wm"
        return 1
    }
    
    echo "VM built successfully for $wm"
}

start_vm() {
    local wm="$1"
    local ssh_port
    ssh_port=$(get_ssh_port "$wm")
    
    echo "Starting VM for $wm on SSH port $ssh_port..."
    
    # Start VM in background
    QEMU_NET_OPTS="hostfwd=tcp::$ssh_port-:22" \
    ./result/bin/run-*-vm &
    
    local vm_pid=$!
    echo "VM started with PID $vm_pid"
    
    # Wait for SSH to be available
    echo "Waiting for VM to be ready..."
    for i in {1..60}; do
        if ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no \
               -p "$ssh_port" root@localhost "echo 'VM ready'" >/dev/null 2>&1; then
            echo "VM is ready on port $ssh_port"
            return 0
        fi
        sleep 5
    done
    
    echo "VM failed to start or SSH not available"
    kill "$vm_pid" 2>/dev/null || true
    return 1
}

test_vm() {
    local wm="$1"
    local ssh_port
    ssh_port=$(get_ssh_port "$wm")
    
    echo "Testing VM for $wm on port $ssh_port..."
    
    # Copy test script to VM
    scp -P "$ssh_port" -o StrictHostKeyChecking=no \
        vm-wm-test.sh root@localhost:/tmp/ || {
        echo "Failed to copy test script"
        return 1
    }
    
    # Run tests
    VM_SSH_PORT="$ssh_port" ./vm-test.sh || {
        echo "VM tests failed for $wm"
        return 1
    }
    
    echo "Tests completed successfully for $wm"
}

cleanup_vm() {
    local wm="$1"
    local ssh_port
    ssh_port=$(get_ssh_port "$wm")
    
    echo "Cleaning up VM for $wm..."
    
    # Try to shutdown VM gracefully
    ssh -p "$ssh_port" -o ConnectTimeout=5 -o StrictHostKeyChecking=no \
        root@localhost "shutdown now" 2>/dev/null || true
    
    # Kill any remaining VM processes
    pkill -f "run-.*-vm" || true
    
    # Clean build artifacts
    rm -f result*
    
    echo "Cleanup completed for $wm"
}

run_wm_test() {
    local wm="$1"
    local action="$2"
    
    echo "Running $action for $wm window manager..."
    
    case "$action" in
        build)
            build_vm "$wm"
            ;;
        test)
            test_vm "$wm"
            ;;
        run)
            build_vm "$wm" && start_vm "$wm" && test_vm "$wm"
            cleanup_vm "$wm"
            ;;
        clean)
            cleanup_vm "$wm"
            ;;
        *)
            echo "Unknown action: $action"
            show_help
            return 1
            ;;
    esac
}

run_all_tests() {
    local action="$1"
    local wms=("niri" "sway" "hyprland")
    local failed_wms=()
    
    echo "Running $action for all window managers..."
    
    for wm in "${wms[@]}"; do
        echo ""
        echo "================================================"
        echo "Processing $wm..."
        echo "================================================"
        
        if ! run_wm_test "$wm" "$action"; then
            failed_wms+=("$wm")
            echo "FAILED: $wm $action"
        else
            echo "SUCCESS: $wm $action"
        fi
    done
    
    echo ""
    echo "================================================"
    echo "Summary"
    echo "================================================"
    
    if [ ${#failed_wms[@]} -eq 0 ]; then
        echo "All window managers passed: ${wms[*]}"
        return 0
    else
        echo "Failed window managers: ${failed_wms[*]}"
        echo "Successful window managers: $(printf '%s\n' "${wms[@]}" | grep -v "$(printf '%s\n' "${failed_wms[@]}")" | tr '\n' ' ')"
        return 1
    fi
}

main() {
    cd "$SCRIPT_DIR"
    
    local wm="${1:-}"
    local action="${2:-run}"
    
    if [ -z "$wm" ]; then
        show_help
        return 1
    fi
    
    case "$wm" in
        niri|sway|hyprland)
            run_wm_test "$wm" "$action"
            ;;
        all)
            run_all_tests "$action"
            ;;
        -h|--help|help)
            show_help
            ;;
        *)
            echo "Unknown window manager: $wm"
            show_help
            return 1
            ;;
    esac
}

main "$@"