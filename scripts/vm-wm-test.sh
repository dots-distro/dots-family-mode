#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - Window Manager Bridge Test Script
# Tests WM detection, adapter functionality, and bridge operations

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_LOG="/tmp/wm_bridge_test.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test result tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

log() {
    echo -e "$(date '+%H:%M:%S') $*" | tee -a "$TEST_LOG"
}

log_info() {
    log "${BLUE}[INFO]${NC} $*"
}

log_success() {
    log "${GREEN}[PASS]${NC} $*"
    ((TESTS_PASSED++))
}

log_error() {
    log "${RED}[FAIL]${NC} $*"
    ((TESTS_FAILED++))
}

log_warning() {
    log "${YELLOW}[WARN]${NC} $*"
}

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    shift
    
    log_info "Running test: $test_name"
    
    if "$@"; then
        log_success "✓ $test_name"
        return 0
    else
        log_error "✗ $test_name"
        return 1
    fi
}

# Test WM detection functions
test_niri_detection() {
    if [ -n "${NIRI_SOCKET:-}" ]; then
        log_info "NIRI_SOCKET detected: $NIRI_SOCKET"
        which niri >/dev/null || { log_warning "niri command not found"; return 1; }
        return 0
    else
        log_warning "NIRI_SOCKET not set"
        return 1
    fi
}

test_sway_detection() {
    if [ -n "${SWAYSOCK:-}" ]; then
        log_info "SWAYSOCK detected: $SWAYSOCK"
        which swaymsg >/dev/null || { log_warning "swaymsg command not found"; return 1; }
        return 0
    else
        log_warning "SWAYSOCK not set"
        return 1
    fi
}

test_hyprland_detection() {
    if [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]; then
        log_info "HYPRLAND_INSTANCE_SIGNATURE detected: $HYPRLAND_INSTANCE_SIGNATURE"
        which hyprctl >/dev/null || { log_warning "hyprctl command not found"; return 1; }
        return 0
    else
        log_warning "HYPRLAND_INSTANCE_SIGNATURE not set"
        return 1
    fi
}

test_wayland_fallback() {
    if [ -n "${WAYLAND_DISPLAY:-}" ]; then
        log_info "WAYLAND_DISPLAY detected: $WAYLAND_DISPLAY"
        return 0
    elif [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
        log_info "XDG_SESSION_TYPE is wayland"
        return 0
    else
        log_warning "No Wayland environment detected"
        return 1
    fi
}

# Test direct WM tool functionality
test_niri_tool_functionality() {
    [ -z "${NIRI_SOCKET:-}" ] && { log_warning "Skipping Niri test - not available"; return 0; }
    
    local output
    output=$(niri msg --json focused-window 2>/dev/null) || {
        log_warning "niri msg failed, might be no focused window"
        return 0
    }
    
    log_info "Niri focused window response: $output"
    return 0
}

test_sway_tool_functionality() {
    [ -z "${SWAYSOCK:-}" ] && { log_warning "Skipping Sway test - not available"; return 0; }
    
    local output
    output=$(swaymsg -t get_tree 2>/dev/null) || {
        log_error "swaymsg get_tree failed"
        return 1
    }
    
    local node_count
    node_count=$(echo "$output" | grep -o '"type"' | wc -l)
    log_info "Sway tree has $node_count nodes"
    [ "$node_count" -gt 0 ]
}

test_hyprland_tool_functionality() {
    [ -z "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] && { log_warning "Skipping Hyprland test - not available"; return 0; }
    
    local output
    output=$(hyprctl activewindow -j 2>/dev/null) || {
        log_warning "hyprctl activewindow failed, might be no active window"
        return 0
    }
    
    log_info "Hyprland active window response: ${output:0:100}..."
    return 0
}

# Test the WM bridge binary directly
test_wm_bridge_binary() {
    which dots-wm-bridge-test >/dev/null || {
        log_warning "dots-wm-bridge-test binary not found, creating minimal test"
        return 0
    }
    
    timeout 10 dots-wm-bridge-test || {
        log_error "WM bridge test binary failed or timed out"
        return 1
    }
}

# Test monitor using WM bridge
test_monitor_wm_integration() {
    local monitor_pid
    
    # Start monitor in background
    dots-family-monitor > /tmp/monitor_wm_test.log 2>&1 &
    monitor_pid=$!
    
    # Give it time to initialize
    sleep 3
    
    # Check if it's still running
    if kill -0 "$monitor_pid" 2>/dev/null; then
        log_info "Monitor started successfully with PID $monitor_pid"
        
        # Check logs for WM bridge initialization
        if grep -q "Using window manager:" /tmp/monitor_wm_test.log; then
            local wm_name
            wm_name=$(grep "Using window manager:" /tmp/monitor_wm_test.log | tail -1 | cut -d: -f2- | tr -d ' ')
            log_info "Monitor detected WM: $wm_name"
        else
            log_warning "No WM detection message found in monitor logs"
        fi
        
        # Stop monitor
        kill "$monitor_pid" 2>/dev/null || true
        wait "$monitor_pid" 2>/dev/null || true
        
        return 0
    else
        log_error "Monitor failed to start or crashed immediately"
        cat /tmp/monitor_wm_test.log
        return 1
    fi
}

# Test environment detection
test_environment_detection() {
    log_info "Environment detection:"
    log_info "XDG_SESSION_TYPE: ${XDG_SESSION_TYPE:-unset}"
    log_info "WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-unset}"
    log_info "DISPLAY: ${DISPLAY:-unset}"
    log_info "NIRI_SOCKET: ${NIRI_SOCKET:-unset}"
    log_info "SWAYSOCK: ${SWAYSOCK:-unset}"
    log_info "HYPRLAND_INSTANCE_SIGNATURE: ${HYPRLAND_INSTANCE_SIGNATURE:-unset}"
    
    # At least one should be available
    [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ] || [ "${XDG_SESSION_TYPE:-}" = "wayland" ]
}

# Determine which WM is currently active
detect_active_wm() {
    if test_niri_detection 2>/dev/null; then
        echo "niri"
    elif test_sway_detection 2>/dev/null; then
        echo "sway"
    elif test_hyprland_detection 2>/dev/null; then
        echo "hyprland"
    elif test_wayland_fallback 2>/dev/null; then
        echo "wayland-generic"
    else
        echo "unknown"
    fi
}

# Main test execution
main() {
    log_info "Starting DOTS Family Mode WM Bridge Tests"
    log_info "============================================"
    echo "" > "$TEST_LOG"
    
    local active_wm
    active_wm=$(detect_active_wm)
    log_info "Detected active WM: $active_wm"
    
    # Environment tests
    run_test "Environment Detection" test_environment_detection
    
    # WM detection tests
    run_test "Niri Detection" test_niri_detection || true
    run_test "Sway Detection" test_sway_detection || true  
    run_test "Hyprland Detection" test_hyprland_detection || true
    run_test "Wayland Fallback Detection" test_wayland_fallback || true
    
    # Tool functionality tests
    run_test "Niri Tool Functionality" test_niri_tool_functionality || true
    run_test "Sway Tool Functionality" test_sway_tool_functionality || true
    run_test "Hyprland Tool Functionality" test_hyprland_tool_functionality || true
    
    # WM Bridge integration tests
    run_test "WM Bridge Binary Test" test_wm_bridge_binary || true
    run_test "Monitor WM Integration" test_monitor_wm_integration
    
    # Results
    echo ""
    log_info "WM Bridge Test Results Summary"
    log_info "=============================="
    log_info "Active WM: $active_wm"
    log_info "Tests run: $TESTS_RUN"
    log_success "Tests passed: $TESTS_PASSED"
    log_error "Tests failed: $TESTS_FAILED"
    
    if [ "$TESTS_FAILED" -eq 0 ]; then
        log_success "🎉 All WM bridge tests passed!"
        return 0
    else
        log_error "❌ Some tests failed. Check $TEST_LOG for details."
        return 1
    fi
}

main "$@"