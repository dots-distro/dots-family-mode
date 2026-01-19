#!/usr/bin/env bash
# DOTS Family Mode NixOS Testing Framework
# Comprehensive validation runner for system services and security

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo
    echo "=================================================================="
    echo "$1"
    echo "=================================================================="
}

# Check if we're in a Nix development environment
check_nix_env() {
    print_status "Checking Nix development environment..."
    
    if [[ -z "${IN_NIX_SHELL:-}" ]]; then
        print_warning "Not in Nix development shell. Entering nix develop..."
        exec nix develop -c "$0" "$@"
    fi
    
    print_success "Nix development environment detected"
}

# Build validation VM
build_validation_vm() {
    print_header "Building DOTS Family Mode Validation VM"
    
    print_status "Building VM configuration..."
    if nix build .#nixosConfigurations.dots-family-validation-vm.config.system.build.vm; then
        print_success "VM build completed successfully"
    else
        print_error "VM build failed"
        exit 1
    fi
}

# Run validation tests in VM
run_validation_tests() {
    print_header "Running DOTS Family Mode Validation Tests"
    
    print_status "Starting validation VM..."
    
    # Check if VM already running
    if pgrep -f "dots-family-validation-vm" >/dev/null; then
        print_warning "VM appears to be running. Stopping it first..."
        pkill -f "dots-family-validation-vm" || true
        sleep 2
    fi
    
    # Run VM with validation
    print_status "Launching VM with automatic validation..."
    ./result/bin/run-dots-family-validation-vm &
    
    local vm_pid=$!
    
    print_status "Waiting for VM to boot..."
    sleep 30  # Give VM time to boot
    
    # Run validation inside VM (via SSH if available, or direct script execution)
    print_status "Running validation tests..."
    
    # Option 1: Direct validation (if VM supports automatic script execution)
    if timeout 300 ./result/bin/run-dots-family-validation-vm --command "/etc/dots-family/validation.sh"; then
        print_success "Validation tests completed successfully"
        return 0
    fi
    
    # Option 2: SSH-based validation (if networking is configured)
    if [[ -f "./vm-ssh-config" ]]; then
        print_status "Attempting SSH-based validation..."
        if ssh -F ./vm-ssh-config dots-validation "/etc/dots-family/validation.sh"; then
            print_success "SSH validation completed successfully"
            return 0
        else
            print_warning "SSH validation failed, trying manual instructions..."
        fi
    fi
    
    # Option 3: Manual validation instructions
    print_warning "Automatic validation not available. Manual validation required:"
    echo
    echo "1. Connect to VM console (should be open in another window)"
    echo "2. Login as 'parent' user (password: parent123)"
    echo "3. Run: sudo /etc/dots-family/validation.sh"
    echo "4. Check results for any FAILED tests"
    echo
    echo "VM will remain running for manual testing."
    
    # Wait for user to stop VM
    print_status "VM running. Press Ctrl+C to stop VM."
    wait $vm_pid
}

# Build all VM variants
build_all_vms() {
    print_header "Building All VM Variants"
    
    local vms=(
        "dots-family-test-vm"
        "dots-family-validation-vm"
    )
    
    for vm in "${vms[@]}"; do
        print_status "Building $vm..."
        if nix build ".#nixosConfigurations.$vm.config.system.build.vm"; then
            print_success "$vm build completed"
        else
            print_error "$vm build failed"
        fi
    done
}

# Test specific components
test_components() {
    print_header "Testing Individual Components"
    
    print_status "Testing daemon build..."
    if nix build .#dots-family-daemon; then
        print_success "Daemon build successful"
    else
        print_error "Daemon build failed"
    fi
    
    print_status "Testing monitor build..."
    if nix build .#dots-family-monitor; then
        print_success "Monitor build successful"
    else
        print_error "Monitor build failed"
    fi
    
    print_status "Testing CLI build..."
    if nix build .#dots-family-ctl; then
        print_success "CLI build successful"
    else
        print_error "CLI build failed"
    fi
    
    print_status "Testing eBPF build..."
    if nix build .#dots-family-ebpf; then
        print_success "eBPF build successful"
    else
        print_warning "eBPF build failed (may be expected in development)"
    fi
}

# Test system integration (without VM)
test_system_integration() {
    print_header "Testing System Integration (No VM)"
    
    print_status "Testing NixOS module evaluation..."
    if nix-instantiate --eval --expr 'let pkgs = import <nixpkgs> {}; in (import ./nixos-modules/dots-family/default.nix { inherit pkgs config lib; config = {}; })'; then
        print_success "NixOS module evaluation successful"
    else
        print_error "NixOS module evaluation failed"
    fi
    
    print_status "Testing package overlay..."
    if nix build --expr 'with import <nixpkgs> {}; [ dots-family-daemon dots-family-monitor dots-family-ctl ]'; then
        print_success "Package overlay working"
    else
        print_warning "Package overlay may have issues"
    fi
    
    print_status "Testing flake outputs..."
    if nix flake check; then
        print_success "Flake validation passed"
    else
        print_error "Flake validation failed"
    fi
}

# Cleanup VM artifacts
cleanup() {
    print_header "Cleaning Up VM Artifacts"
    
    print_status "Stopping any running VMs..."
    pkill -f "run-.*-vm" || true
    
    print_status "Removing build artifacts..."
    rm -f ./result
    rm -f ./result-*
    
    print_status "Cleaning temporary files..."
    rm -f ./vm-ssh-config
    
    print_success "Cleanup completed"
}

# Show usage
usage() {
    echo "DOTS Family Mode NixOS Testing Framework"
    echo
    echo "Usage: $0 [COMMAND]"
    echo
    echo "Commands:"
    echo "  validate          Build and run comprehensive validation VM"
    echo "  build-all         Build all VM variants"
    echo "  test-components   Test individual component builds"
    echo "  test-system       Test system integration (no VM)"
    echo "  cleanup           Clean up VM artifacts"
    echo "  help              Show this help message"
    echo
    echo "Examples:"
    echo "  $0 validate                    # Run full validation suite"
    echo "  $0 build-all                   # Build all VMs for testing"
    echo "  $0 test-components             # Test individual packages"
    echo "  $0 test-system                # Quick integration test"
    echo "  $0 cleanup                     # Clean build artifacts"
    echo
    echo "Environment Variables:"
    echo "  IN_NIX_SHELL      Set when in nix develop environment"
    echo "  VALIDATION_LOG    Log file for validation results"
}

# Main execution
main() {
    case "${1:-validate}" in
        "validate")
            check_nix_env
            build_validation_vm
            run_validation_tests
            ;;
        "build-all")
            check_nix_env
            build_all_vms
            ;;
        "test-components")
            check_nix_env
            test_components
            ;;
        "test-system")
            check_nix_env
            test_system_integration
            ;;
        "cleanup")
            cleanup
            ;;
        "help"|"-h"|"--help")
            usage
            ;;
        *)
            print_error "Unknown command: $1"
            usage
            exit 1
            ;;
    esac
}

# Handle signals gracefully
trap cleanup EXIT

# Run main function with all arguments
main "$@"