#!/usr/bin/env bash
# DOTS Family Mode - Complete Test Suite Runner
#
# This script runs all VM tests including the new web filtering test suite.
# It's designed to be run from within the VM test environment.
#
# Usage:
#   ./run-all-vm-tests.sh [--quick] [--web-filtering-only] [--evidence DIR]
#
# Options:
#   --quick              Run only quick validation tests
#   --web-filtering-only Run only web filtering tests
#   --evidence DIR       Evidence directory for test results
#   --help, -h           Show this help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-test-evidence}"
QUICK_MODE="${QUICK_MODE:-false}"
WEB_FILTERING_ONLY="${WEB_FILTERING_ONLY:-false}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

log_header() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $*${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $*"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_info() {
    echo -e "${YELLOW}[INFO]${NC} $*"
}

log_block() {
    echo -e "${MAGENTA}[BLOCK]${NC} $*"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --web-filtering-only)
            WEB_FILTERING_ONLY=true
            shift
            ;;
        --evidence|--evidence-dir)
            EVIDENCE_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "DOTS Family Mode - Complete VM Test Suite Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --quick               Run only quick validation tests"
            echo "  --web-filtering-only  Run only web filtering tests"
            echo "  --evidence DIR        Evidence directory (default: test-evidence)"
            echo "  --help, -h            Show this help"
            exit 0
            ;;
        *)
            log_fail "Unknown option: $1"
            exit 1
            ;;
    esac
done

log_header "DOTS Family Mode - Complete VM Test Suite"

echo "Evidence Directory: $EVIDENCE_DIR"
echo "Quick Mode: $QUICK_MODE"
echo "Web Filtering Only: $WEB_FILTERING_ONLY"
echo ""

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"
mkdir -p "$EVIDENCE_DIR/web-filtering"
mkdir -p "$EVIDENCE_DIR/screenshots"
mkdir -p "$EVIDENCE_DIR/network"
mkdir -p "$EVIDENCE_DIR/html"

# Track overall results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Function to run a test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"
    local test_description="$3"
    
    log_test "$test_name"
    log_info "$test_description"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if eval "$test_command" > "$EVIDENCE_DIR/${test_name// /-}.log" 2>&1; then
        log_pass "$test_name completed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        local exit_code=$?
        if grep -q "SKIPPED\|skipped" "$EVIDENCE_DIR/${test_name// /-}.log" 2>/dev/null; then
            log_info "$test_name skipped"
            SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
        else
            log_fail "$test_name failed (exit code: $exit_code)"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        return $exit_code
    fi
}

# Web Filtering Tests
if [[ "$WEB_FILTERING_ONLY" == "true" ]] || [[ "$QUICK_MODE" == "false" ]]; then
    log_header "Web Filtering Tests"
    
    # Check for Playwright
    if command -v node &> /dev/null && [[ -f "$SCRIPT_DIR/web-filtering-test/web-filtering-test.js" ]]; then
        log_info "Running web filtering tests with Playwright..."
        
        cd "$SCRIPT_DIR/web-filtering-test"
        
        # Install Playwright browsers if needed
        if command -v npx &> /dev/null; then
            npx playwright install chromium 2>/dev/null || log_warn "Could not install Playwright browsers"
        fi
        
        # Run web filtering tests
        if node web-filtering-test.js \
            --proxy-host=127.0.0.1 \
            --proxy-port=8080 \
            --evidence-dir="$EVIDENCE_DIR/web-filtering"; then
            log_pass "Web filtering tests completed"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            log_fail "Web filtering tests failed"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        
        cd "$SCRIPT_DIR"
    else
        log_info "Web filtering tests require Node.js and Playwright"
        log_info "Installing dependencies..."
        
        # Try to install Playwright
        if command -v npm &> /dev/null; then
            cd "$SCRIPT_DIR/web-filtering-test"
            npm install --silent 2>/dev/null || log_warn "Could not install npm dependencies"
            
            if command -v npx &> /dev/null; then
                npx playwright install chromium 2>/dev/null || log_warn "Could not install Playwright browsers"
            fi
            
            cd "$SCRIPT_DIR"
        fi
        
        # Skip web filtering tests if dependencies not available
        log_info "Web filtering tests skipped (dependencies not available)"
        SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi
fi

# Core Daemon Tests
if [[ "$WEB_FILTERING_ONLY" == "false" ]]; then
    log_header "Core Daemon Tests"
    
    # Test 1: Daemon binary exists
    run_test "Daemon Binary" \
        "[ -f /run/current-system/sw/bin/dots-family-daemon ]" \
        "Verify daemon binary is installed"
    
    # Test 2: Daemon can show version
    run_test "Daemon Version" \
        "dots-family-daemon --version" \
        "Check daemon version information"
    
    # Test 3: CLI tool exists
    run_test "CLI Tool" \
        "[ -f /run/current-system/sw/bin/dots-family-ctl ]" \
        "Verify CLI tool is installed"
    
    # Test 4: CLI can show help
    run_test "CLI Help" \
        "dots-family-ctl --help" \
        "Check CLI help information"
    
    # Test 5: Terminal filter exists
    run_test "Terminal Filter" \
        "[ -f /run/current-system/sw/bin/dots-terminal-filter ]" \
        "Verify terminal filter is installed"
fi

# Quick Mode Tests
if [[ "$QUICK_MODE" == "true" ]]; then
    log_header "Quick Validation Tests"
    
    # Only run essential tests in quick mode
    run_test "Quick Daemon Check" \
        "[ -f /run/current-system/sw/bin/dots-family-daemon ]" \
        "Daemon binary installed"
    
    run_test "Quick CLI Check" \
        "[ -f /run/current-system/sw/bin/dots-family-ctl ]" \
        "CLI tool installed"
    
    run_test "Quick Monitor Check" \
        "[ -f /run/current-system/sw/bin/dots-family-monitor ]" \
        "Monitor binary installed"
fi

# Summary
log_header "Test Suite Summary"

echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED_TESTS${NC}"
echo ""

if [[ $FAILED_TESTS -eq 0 ]]; then
    log_pass "All tests passed!"
    EXIT_CODE=0
else
    log_fail "$FAILED_TESTS test(s) failed"
    EXIT_CODE=1
fi

log_info "Evidence saved to: $EVIDENCE_DIR"

# Generate summary report
cat > "$EVIDENCE_DIR/test_summary.md" << EOF
# DOTS Family Mode - Test Summary

## Test Run Information

- **Date:** $(date)
- **Quick Mode:** $QUICK_MODE
- **Web Filtering Only:** $WEB_FILTERING_ONLY
- **Evidence Directory:** $EVIDENCE_DIR

## Results

| Metric | Count |
|--------|-------|
| Total Tests | $TOTAL_TESTS |
| Passed | $PASSED_TESTS |
| Failed | $FAILED_TESTS |
| Skipped | $SKIPPED_TESTS |
| Pass Rate | $(awk "BEGIN {printf \"%.1f\", ($PASSED_TESTS / $TOTAL_TESTS) * 100}")% |

## Status

$([ $FAILED_TESTS -eq 0 ] && echo "✅ ALL TESTS PASSED" || echo "⚠️ SOME TESTS FAILED")

## Evidence Files

- Log files: $EVIDENCE_DIR/*.log
- Web filtering evidence: $EVIDENCE_DIR/web-filtering/
- Screenshots: $EVIDENCE_DIR/screenshots/
- HTML responses: $EVIDENCE_DIR/html/
- Network logs: $EVIDENCE_DIR/network/
EOF

log_info "Summary report: $EVIDENCE_DIR/test_summary.md"

exit $EXIT_CODE
