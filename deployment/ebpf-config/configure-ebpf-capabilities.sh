#!/run/current-system/sw/bin/bash

# eBPF Capabilities Configuration Script
# Configures kernel parameters and validates eBPF support for DOTS Family Mode

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root (use sudo)"
        exit 1
    fi
}

check_kernel_support() {
    log "Checking kernel eBPF support..."
    
    # Check if /sys/kernel/debug exists (debugfs)
    if [[ ! -d /sys/kernel/debug ]]; then
        warn "Debugfs not mounted, mounting to /sys/kernel/debug"
        mount -t debugfs none /sys/kernel/debug 2>/dev/null || {
            error "Failed to mount debugfs - eBPF may not work properly"
            return 1
        }
    fi
    
    # Check if BPF syscall is available
    if [[ -f /proc/sys/kernel/unprivileged_bpf_disabled ]]; then
        success "eBPF syscall support detected"
    else
        error "eBPF syscall support not found - kernel may be too old"
        return 1
    fi
    
    # Check if BPF JIT is available
    if [[ -f /proc/sys/net/core/bpf_jit_enable ]]; then
        success "eBPF JIT compiler available"
    else
        warn "eBPF JIT compiler not available - performance may be reduced"
    fi
    
    return 0
}

install_sysctl_config() {
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local sysctl_file="$script_dir/sysctl-ebpf.conf"
    local target_dir="/etc/sysctl.d"
    local target_file="$target_dir/99-dots-family-ebpf.conf"
    
    log "Installing eBPF sysctl configuration..."
    
    if [[ ! -f "$sysctl_file" ]]; then
        error "Source sysctl config not found: $sysctl_file"
        return 1
    fi
    
    # Create target directory
    mkdir -p "$target_dir"
    
    # Install sysctl config
    cp "$sysctl_file" "$target_file"
    chmod 644 "$target_file"
    success "Installed sysctl config to $target_file"
    
    # Apply configuration
    log "Applying sysctl configuration..."
    sysctl -p "$target_file" || {
        error "Failed to apply sysctl configuration"
        return 1
    }
    
    success "eBPF sysctl configuration applied"
}

check_capabilities() {
    log "Checking required capabilities for DOTS Family Mode..."
    
    # List of capabilities we need
    local required_caps=(
        "CAP_SYS_ADMIN"     # General eBPF operations
        "CAP_NET_ADMIN"     # Network monitoring
        "CAP_SYS_PTRACE"    # Process monitoring  
        "CAP_DAC_READ_SEARCH" # File system access
    )
    
    local missing_caps=()
    
    for cap in "${required_caps[@]}"; do
        # Check if capability exists in kernel headers
        if grep -q "$cap" /usr/include/linux/capability.h 2>/dev/null; then
            success "Capability $cap is supported"
        else
            warn "Capability $cap may not be supported"
            missing_caps+=("$cap")
        fi
    done
    
    if [[ ${#missing_caps[@]} -gt 0 ]]; then
        error "Some required capabilities are missing: ${missing_caps[*]}"
        return 1
    fi
    
    success "All required capabilities are supported"
}

validate_ebpf_features() {
    log "Validating eBPF feature support..."
    
    # Check available eBPF features
    local features_checked=0
    local features_available=0
    
    # Check if bpftool is available for feature detection
    if command -v bpftool >/dev/null 2>&1; then
        log "Using bpftool for feature detection..."
        
        # Get feature list
        local features=$(bpftool feature 2>/dev/null | grep -E "(Scanning|Available)" || true)
        if [[ -n "$features" ]]; then
            success "eBPF features detected via bpftool"
            echo "$features" | head -10
            ((features_available++))
        fi
        ((features_checked++))
    fi
    
    # Check kernel config if available
    if [[ -f /proc/config.gz ]]; then
        log "Checking kernel config for eBPF support..."
        
        local config_checks=(
            "CONFIG_BPF=y"
            "CONFIG_BPF_SYSCALL=y" 
            "CONFIG_BPF_JIT=y"
            "CONFIG_HAVE_EBPF_JIT=y"
        )
        
        for check in "${config_checks[@]}"; do
            if zcat /proc/config.gz | grep -q "^$check" 2>/dev/null; then
                success "Kernel config: $check"
                ((features_available++))
            else
                warn "Kernel config missing: $check"
            fi
            ((features_checked++))
        done
    elif [[ -f "/boot/config-$(uname -r)" ]]; then
        log "Checking boot config for eBPF support..."
        
        local config_checks=(
            "CONFIG_BPF=y"
            "CONFIG_BPF_SYSCALL=y"
        )
        
        for check in "${config_checks[@]}"; do
            if grep -q "^$check" "/boot/config-$(uname -r)" 2>/dev/null; then
                success "Boot config: $check"
                ((features_available++))
            else
                warn "Boot config missing: $check"
            fi
            ((features_checked++))
        done
    fi
    
    log "eBPF feature validation: $features_available/$features_checked checks passed"
    
    if [[ $features_available -eq 0 ]]; then
        error "No eBPF features could be validated"
        return 1
    fi
    
    return 0
}

create_test_program() {
    log "Creating simple eBPF test program..."
    
    # Create a minimal test to verify eBPF works
    cat > /tmp/ebpf_test.sh << 'EOF'
#!/bin/bash
# Simple test for eBPF functionality
echo "Testing basic eBPF availability..."

# Check if we can read BPF-related files
test -r /proc/sys/kernel/unprivileged_bpf_disabled && echo "✓ BPF syscall accessible"
test -r /proc/sys/net/core/bpf_jit_enable && echo "✓ BPF JIT accessible" 

# Try to use bpftool if available
if command -v bpftool >/dev/null 2>&1; then
    echo "✓ bpftool available"
    bpftool prog list >/dev/null 2>&1 && echo "✓ BPF program listing works"
fi

echo "Basic eBPF test completed"
EOF

    chmod +x /tmp/ebpf_test.sh
    success "Created eBPF test program at /tmp/ebpf_test.sh"
    
    # Run the test
    log "Running eBPF test..."
    if /tmp/ebpf_test.sh; then
        success "eBPF test completed successfully"
    else
        warn "eBPF test completed with warnings"
    fi
    
    rm -f /tmp/ebpf_test.sh
}

main() {
    log "DOTS Family Mode eBPF Configuration"
    log "==================================="
    
    check_root
    
    local errors=0
    
    check_kernel_support || ((errors++))
    check_capabilities || ((errors++))
    install_sysctl_config || ((errors++))
    validate_ebpf_features || ((errors++))
    create_test_program || ((errors++))
    
    if [[ $errors -eq 0 ]]; then
        success "eBPF configuration completed successfully!"
        log ""
        log "Next steps:"
        log "1. Reboot system to ensure all kernel parameters take effect"
        log "2. Install DOTS Family systemd services with eBPF capabilities"
        log "3. Test eBPF functionality with actual monitoring programs"
        log ""
        log "Current eBPF status:"
        cat /proc/sys/kernel/unprivileged_bpf_disabled 2>/dev/null | sed 's/^/  unprivileged_bpf_disabled: /' || echo "  Status check failed"
        cat /proc/sys/net/core/bpf_jit_enable 2>/dev/null | sed 's/^/  bpf_jit_enable: /' || echo "  JIT check failed"
    else
        error "eBPF configuration completed with $errors errors"
        log ""
        log "Please resolve the errors above before using eBPF functionality"
        exit 1
    fi
}

main "$@"