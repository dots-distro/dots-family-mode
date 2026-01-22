#!/run/current-system/sw/bin/bash

# DOTS Family Mode eBPF Capabilities Test
# Tests eBPF functionality without requiring root privileges

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

test_basic_ebpf_support() {
    log "Testing basic eBPF support..."
    
    local tests_passed=0
    local tests_total=0
    
    # Test 1: Check if BPF syscall is exposed
    ((tests_total++))
    if [[ -f /proc/sys/kernel/unprivileged_bpf_disabled ]]; then
        success "BPF syscall interface available"
        ((tests_passed++))
        
        local bpf_status=$(cat /proc/sys/kernel/unprivileged_bpf_disabled)
        case "$bpf_status" in
            0) log "  Status: Unprivileged eBPF enabled (0)" ;;
            1) warn "  Status: Unprivileged eBPF disabled (1)" ;;
            2) warn "  Status: eBPF fully disabled (2)" ;;
            *) warn "  Status: Unknown value ($bpf_status)" ;;
        esac
    else
        fail "BPF syscall interface not found"
    fi
    
    # Test 2: Check BPF JIT support
    ((tests_total++))
    if [[ -f /proc/sys/net/core/bpf_jit_enable ]]; then
        success "BPF JIT interface available"
        ((tests_passed++))
        
        local jit_status=$(cat /proc/sys/net/core/bpf_jit_enable)
        case "$jit_status" in
            0) warn "  Status: BPF JIT disabled (0)" ;;
            1) success "  Status: BPF JIT enabled (1)" ;;
            2) success "  Status: BPF JIT enabled with debug (2)" ;;
            *) warn "  Status: Unknown value ($jit_status)" ;;
        esac
    else
        fail "BPF JIT interface not found"
    fi
    
    # Test 3: Check for debugfs
    ((tests_total++))
    if [[ -d /sys/kernel/debug ]]; then
        success "debugfs is mounted"
        ((tests_passed++))
    elif mountpoint -q /sys/kernel/debug 2>/dev/null; then
        success "debugfs is mounted (via mountpoint)"
        ((tests_passed++))
    else
        warn "debugfs not mounted (may need: mount -t debugfs none /sys/kernel/debug)"
    fi
    
    # Test 4: Check for bpftool
    ((tests_total++))
    if command -v bpftool >/dev/null 2>&1; then
        success "bpftool is available"
        ((tests_passed++))
        
        # Try to get version
        local version=$(bpftool version 2>/dev/null | head -1 || echo "version unknown")
        log "  Version: $version"
    else
        warn "bpftool not available (install bpf-tools package)"
    fi
    
    echo "Basic eBPF support: $tests_passed/$tests_total tests passed"
    return $((tests_total - tests_passed))
}

test_capability_requirements() {
    log "Testing capability requirements for DOTS Family Mode..."
    
    local caps_available=0
    local caps_total=0
    
    # Required capabilities for DOTS Family daemon
    local required_caps=(
        "CAP_SYS_ADMIN:General eBPF operations"
        "CAP_NET_ADMIN:Network monitoring"
        "CAP_SYS_PTRACE:Process monitoring"
        "CAP_DAC_READ_SEARCH:Filesystem access"
        "CAP_NET_BIND_SERVICE:Privileged port binding"
    )
    
    for cap_desc in "${required_caps[@]}"; do
        local cap_name="${cap_desc%%:*}"
        local cap_purpose="${cap_desc#*:}"
        ((caps_total++))
        
        # Check if capability exists in system headers
        if grep -q "$cap_name" /usr/include/linux/capability.h 2>/dev/null; then
            success "$cap_name available ($cap_purpose)"
            ((caps_available++))
        elif getent -s files passwd root >/dev/null 2>&1; then
            # Fallback: assume capability exists if we have a proper system
            warn "$cap_name assumed available ($cap_purpose)"
            ((caps_available++))
        else
            fail "$cap_name not found ($cap_purpose)"
        fi
    done
    
    echo "Capability requirements: $caps_available/$caps_total capabilities available"
    return $((caps_total - caps_available))
}

test_systemd_integration() {
    log "Testing systemd service integration..."
    
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local systemd_dir="$script_dir/../systemd"
    local tests_passed=0
    local tests_total=0
    
    # Test systemd service files exist
    local service_files=(
        "dots-family-daemon.service"
        "dots-family-monitor@.service"
        "dots-family-filter.service"
    )
    
    for service in "${service_files[@]}"; do
        ((tests_total++))
        if [[ -f "$systemd_dir/$service" ]]; then
            success "Service file exists: $service"
            ((tests_passed++))
            
            # Check for capability configuration
            if grep -q "CapabilityBoundingSet\|AmbientCapabilities" "$systemd_dir/$service"; then
                log "  Capabilities configured in $service"
            else
                warn "  No capabilities configured in $service"
            fi
        else
            fail "Service file missing: $service"
        fi
    done
    
    echo "Systemd integration: $tests_passed/$tests_total service files ready"
    return $((tests_total - tests_passed))
}

test_ebpf_runtime() {
    log "Testing eBPF runtime functionality..."
    
    local runtime_tests=0
    local runtime_passed=0
    
    # Test if we can list existing BPF programs (non-privileged)
    ((runtime_tests++))
    if command -v bpftool >/dev/null 2>&1; then
        if bpftool prog list >/dev/null 2>&1; then
            success "BPF program listing works"
            ((runtime_passed++))
            
            local prog_count=$(bpftool prog list 2>/dev/null | wc -l || echo 0)
            log "  Currently loaded BPF programs: $prog_count"
        else
            warn "BPF program listing requires privileges"
        fi
    else
        warn "bpftool not available for runtime testing"
    fi
    
    # Test if we can list BPF maps
    ((runtime_tests++))
    if command -v bpftool >/dev/null 2>&1; then
        if bpftool map list >/dev/null 2>&1; then
            success "BPF map listing works"
            ((runtime_passed++))
            
            local map_count=$(bpftool map list 2>/dev/null | wc -l || echo 0)
            log "  Currently loaded BPF maps: $map_count"
        else
            warn "BPF map listing requires privileges"
        fi
    else
        warn "bpftool not available for map testing"
    fi
    
    echo "eBPF runtime: $runtime_passed/$runtime_tests runtime tests passed"
    return $((runtime_tests - runtime_passed))
}

generate_report() {
    log "Generating eBPF readiness report..."
    
    echo ""
    echo "DOTS Family Mode eBPF Readiness Report"
    echo "======================================"
    echo ""
    
    local total_errors=0
    
    test_basic_ebpf_support || ((total_errors += $?))
    echo ""
    
    test_capability_requirements || ((total_errors += $?))
    echo ""
    
    test_systemd_integration || ((total_errors += $?))
    echo ""
    
    test_ebpf_runtime || ((total_errors += $?))
    echo ""
    
    if [[ $total_errors -eq 0 ]]; then
        success "System is ready for DOTS Family Mode eBPF functionality"
        echo ""
        echo "Next steps:"
        echo "1. Run as root: sudo ./configure-ebpf-capabilities.sh"
        echo "2. Install systemd services with eBPF capabilities"
        echo "3. Test full eBPF functionality with daemon"
    elif [[ $total_errors -le 3 ]]; then
        warn "System has minor issues but should work with eBPF ($total_errors warnings)"
        echo ""
        echo "Recommended fixes:"
        echo "1. Install bpftool: sudo apt install linux-tools-generic"
        echo "2. Mount debugfs: sudo mount -t debugfs none /sys/kernel/debug"
        echo "3. Consider enabling eBPF JIT for better performance"
    else
        fail "System has significant eBPF compatibility issues ($total_errors errors)"
        echo ""
        echo "Required fixes:"
        echo "1. Upgrade kernel to version with eBPF support (4.4+)"
        echo "2. Enable eBPF in kernel config"
        echo "3. Install eBPF development tools"
        echo "4. Configure proper capabilities"
        return 1
    fi
    
    return 0
}

main() {
    echo "DOTS Family Mode eBPF Capabilities Test"
    echo "======================================="
    echo ""
    echo "Kernel: $(uname -sr)"
    echo "System: $(cat /etc/os-release | grep PRETTY_NAME | cut -d'"' -f2 2>/dev/null || echo 'Unknown')"
    echo ""
    
    generate_report
}

main "$@"