#!/usr/bin/env bash
set -euo pipefail

# DOTS Family Mode - Real Component Testing Script
# Tests what actually works with evidence collection

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_LOG="${SCRIPT_DIR}/component_test_results.log"

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

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    shift
    
    log_info "Running test: $test_name"
    
    if "$@"; then
        log_success "$test_name"
        return 0
    else
        log_error "$test_name"
        return 1
    fi
}

test_workspace_build() {
    log_info "Testing workspace compilation"
    cargo build --workspace --quiet
}

test_unit_tests() {
    log_info "Running all unit tests"
    cargo test --workspace --quiet
}

test_daemon_startup() {
    log_info "Testing daemon startup and database migrations"
    export DATABASE_URL="sqlite:./test_family.db"
    timeout 10s cargo run -p dots-family-daemon --quiet 2>&1 | grep -q "Database migrations completed successfully"
}

test_monitor_startup() {
    log_info "Testing monitor component startup"
    timeout 5s cargo run -p dots-family-monitor --quiet 2>&1 | grep -q "Monitor running, polling every"
}

test_cli_compilation() {
    log_info "Testing CLI compilation and help"
    cargo run -p dots-family-ctl -- --help > /dev/null 2>&1
}

cleanup() {
    log_info "Cleaning up test artifacts"
    rm -f ./test_family.db ./test_family.db-shm ./test_family.db-wal
}

main() {
    log_info "=== DOTS Family Mode Component Testing ==="
    log_info "Date: $(date)"
    log_info "Environment: $(uname -a)"
    log_info "Rust version: $(rustc --version)"
    
    cd "$SCRIPT_DIR"
    
    # Ensure clean state
    cleanup
    
    # Run component tests
    run_test "Workspace Build" test_workspace_build
    run_test "Unit Test Suite" test_unit_tests  
    run_test "Daemon Startup & DB Migrations" test_daemon_startup
    run_test "Monitor Component Startup" test_monitor_startup
    run_test "CLI Tool Compilation" test_cli_compilation
    
    # Summary
    log_info "=== TEST RESULTS SUMMARY ==="
    log_info "Tests Run: $TESTS_RUN"
    log_success "Tests Passed: $TESTS_PASSED"
    
    if [ $TESTS_FAILED -gt 0 ]; then
        log_error "Tests Failed: $TESTS_FAILED"
        log_error "Overall Status: SOME TESTS FAILED"
        cleanup
        exit 1
    else
        log_success "Tests Failed: $TESTS_FAILED"
        log_success "Overall Status: ALL COMPONENT TESTS PASSED"
        
        log_info ""
        log_info "=== CURRENT SYSTEM STATUS ==="
        log_info "✅ Workspace compiles cleanly"
        log_info "✅ All unit tests passing"
        log_info "✅ Database migrations functional"
        log_info "✅ Monitor starts with graceful fallback"
        log_info "✅ CLI tool compiles and shows help"
        log_info ""
        log_info "⚠️  DBus integration requires proper policy setup"
        log_info "⚠️  End-to-end testing needs VM environment"
        log_info "⚠️  Core family safety features still in development"
        log_info ""
        log_info "STATUS: Development prototype with solid foundation"
    fi
    
    cleanup
}

if [ "${BASH_SOURCE[0]}" == "${0}" ]; then
    main "$@"
fi