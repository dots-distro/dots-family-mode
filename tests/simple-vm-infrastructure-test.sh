#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - Simple VM Infrastructure Test
# Tests that VMs can be built and started without requiring SSH connectivity

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output  
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $*"
}

main() {
    log_info "DOTS Family Mode - VM Infrastructure Test"
    log_info "========================================="
    
    # Step 1: Build DOTS components using cargo (known working)
    log_info "Building DOTS Family Mode components..."
    cd "$PROJECT_DIR"
    
    if ! cargo build --release --bin dots-family-daemon --bin dots-family-ctl --bin dots-family-monitor; then
        log_error "Failed to build DOTS components"
        return 1
    fi
    
    log_success "Built DOTS components successfully"
    
    # Verify binaries exist
    for binary in dots-family-daemon dots-family-ctl dots-family-monitor; do
        if [[ ! -f "target/x86_64-unknown-linux-gnu/release/$binary" ]]; then
            log_error "Binary not found: target/x86_64-unknown-linux-gnu/release/$binary"
            return 1
        fi
    done
    
    log_success "All binaries confirmed built"
    
    # Step 2: Build VM using nix-build approach
    log_info "Building test VM..."
    cd "$PROJECT_DIR/tests"
    
    if ! nix-build '<nixpkgs/nixos>' -A vm -I nixos-config=./simple-test-vm.nix; then
        log_error "Failed to build VM"
        return 1
    fi
    
    log_success "VM built successfully"
    
    # Step 3: Verify VM runner exists
    log_info "Verifying VM infrastructure..."
    if [[ ! -f "result/bin/run-dots-test-vm-vm" ]]; then
        log_error "VM runner not found"
        return 1
    fi
    
    log_success "VM runner found"
    
    # Step 4: Test VM startup for 10 seconds (just to verify it can start)
    log_info "Testing VM startup (10 second test)..."
    timeout 10s result/bin/run-dots-test-vm-vm &
    VM_PID=$!
    
    sleep 8
    
    if ps -p "$VM_PID" > /dev/null 2>&1; then
        log_success "VM started and is running"
        kill "$VM_PID" 2>/dev/null || true
        wait "$VM_PID" 2>/dev/null || true
    else
        log_error "VM failed to start or exited early"
        return 1
    fi
    
    # Step 5: Verify disk image was created
    log_info "Checking VM disk image creation..."
    if [[ -f "dots-test-vm.qcow2" ]]; then
        log_success "VM disk image created successfully"
        log_info "Disk image size: $(du -h dots-test-vm.qcow2 | cut -f1)"
    else
        log_error "VM disk image not created"
        return 1
    fi
    
    # Summary
    log_success "================================"
    log_success "VM INFRASTRUCTURE TEST PASSED!"
    log_success "================================"
    log_success "✓ DOTS components build successfully"
    log_success "✓ VM configuration builds"
    log_success "✓ VM can start and create disk image"
    log_success "✓ Infrastructure ready for advanced testing"
    
    return 0
}

main "$@"