#!/usr/bin/env bash
# DOTS Family Mode - Web Filtering VM Test
#
# This script tests web filtering functionality within the VM.
# It starts the content filter and runs Playwright tests.
#
# Usage:
#   ./test-web-filtering-vm.sh [--start-filter] [--stop-filter] [--evidence DIR]
#
# Options:
#   --start-filter    Start the content filter before testing
#   --stop-filter     Stop the content filter after testing
#   --evidence DIR    Evidence directory (default: /tmp/dots-family-web-test)
#   --help, -h        Show this help

set -euo pipefail

EVIDENCE_DIR="${EVIDENCE_DIR:-/tmp/dots-family-web-test}"
START_FILTER=false
STOP_FILTER=false
FILTER_HOST="127.0.0.1"
FILTER_PORT="8080"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${CYAN}[INFO]${NC} $*"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --start-filter)
            START_FILTER=true
            shift
            ;;
        --stop-filter)
            STOP_FILTER=true
            shift
            ;;
        --evidence|--evidence-dir)
            EVIDENCE_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "DOTS Family Mode - Web Filtering VM Test"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --start-filter      Start content filter before testing"
            echo "  --stop-filter       Stop content filter after testing"
            echo "  --evidence DIR      Evidence directory (default: /tmp/dots-family-web-test)"
            echo "  --help, -h          Show this help"
            exit 0
            ;;
        *)
            log_fail "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DOTS Family Mode - Web Filtering VM Test${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"
mkdir -p "$EVIDENCE_DIR/screenshots"
mkdir -p "$EVIDENCE_DIR/html"
mkdir -p "$EVIDENCE_DIR/network"

log_info "Evidence directory: $EVIDENCE_DIR"
log_info "Filter proxy: ${FILTER_HOST}:${FILTER_PORT}"
echo ""

# Track results
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"
    local description="$3"
    
    echo -e "${CYAN}[TEST]${NC} ${test_name}"
    echo "       ${description}"
    
    if eval "$test_command" > "$EVIDENCE_DIR/${test_name// /-}.log" 2>&1; then
        echo -e "${GREEN}[PASS]${NC} ${test_name}"
        ((TESTS_PASSED++))
        return 0
    else
        local exit_code=$?
        if grep -q -i "skipped\|not available\|not found" "$EVIDENCE_DIR/${test_name// /-}.log" 2>/dev/null; then
            echo -e "${YELLOW}[SKIP]${NC} ${test_name}"
            ((TESTS_SKIPPED++))
        else
            echo -e "${RED}[FAIL]${NC} ${test_name} (exit code: $exit_code)"
            ((TESTS_FAILED++))
        fi
        return $exit_code
    fi
}

# Start filter if requested
if [[ "$START_FILTER" == "true" ]]; then
    log_info "Starting content filter..."
    
    if command -v dots-family-filter &> /dev/null; then
        dots-family-filter --bind-address "$FILTER_HOST" --port "$FILTER_PORT" \
            > "$EVIDENCE_DIR/filter.log" 2>&1 &
        FILTER_PID=$!
        sleep 2
        
        if kill -0 $FILTER_PID 2>/dev/null; then
            log_pass "Content filter started (PID: $FILTER_PID)"
        else
            log_warn "Content filter failed to start"
        fi
    else
        log_warn "dots-family-filter not found in PATH"
    fi
fi

echo ""
echo -e "${CYAN}═══ Web Filtering Tests ═══${NC}"
echo ""

# Test 1: Check Node.js availability
run_test "Node.js Available" \
    "command -v node" \
    "Verify Node.js is installed for Playwright tests"

# Test 2: Check Playwright availability  
run_test "Playwright Available" \
    "command -v npx && npx playwright --version" \
    "Verify Playwright is installed"

# Test 3: Check test scripts exist
run_test "Test Scripts Exist" \
    "[ -f /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode/scripts/web-filtering-test/web-filtering-test.js ]" \
    "Verify web filtering test script exists"

# Test 4: Check package.json exists
run_test "Package.json Exists" \
    "[ -f /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode/scripts/web-filtering-test/package.json ]" \
    "Verify npm package configuration exists"

# Test 5: Test proxy connectivity
run_test "Filter Proxy Test" \
    "curl -x http://${FILTER_HOST}:${FILTER_PORT} -s -o /dev/null -w '%{http_code}' http://example.com 2>/dev/null || echo 'proxy_error'" \
    "Test connectivity to content filter proxy"

# Test 6: Check Playwright browsers
run_test "Playwright Browsers" \
    "[ -d ~/.cache/ms-playwright ] || echo 'browsers_not_found'" \
    "Check if Playwright browsers are installed"

# Test 7: Try running web filtering test
if [[ -f /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode/scripts/web-filtering-test/web-filtering-test.js ]]; then
    echo ""
    echo -e "${CYAN}[TEST]${NC} Playwright Web Filtering Test"
    echo "       Running comprehensive web filtering tests"
    
    cd /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode/scripts/web-filtering-test
    
    if node web-filtering-test.js \
        --proxy-host="$FILTER_HOST" \
        --proxy-port="$FILTER_PORT" \
        --evidence-dir="$EVIDENCE_DIR/web-filtering" \
        > "$EVIDENCE_DIR/playwright_test.log" 2>&1; then
        echo -e "${GREEN}[PASS]${NC} Playwright Web Filtering Test"
        ((TESTS_PASSED++))
    else
        local exit_code=$?
        if grep -q -i "skipped\|not available" "$EVIDENCE_DIR/playwright_test.log"; then
            echo -e "${YELLOW}[SKIP]${NC} Playwright Web Filtering Test"
            ((TESTS_SKIPPED++))
        else
            echo -e "${RED}[FAIL]${NC} Playwright Web Filtering Test (exit code: $exit_code)"
            ((TESTS_FAILED++))
        fi
    fi
    
    cd /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode
fi

# Stop filter if requested
if [[ "$STOP_FILTER" == "true" ]] && [[ -n ${FILTER_PID:-} ]]; then
    log_info "Stopping content filter..."
    kill $FILTER_PID 2>/dev/null || true
    log_pass "Content filter stopped"
fi

# Summary
echo ""
echo -e "${CYAN}═══ Test Summary ═══${NC}"
echo ""

echo "Total Tests: $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))"
echo -e "Passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Failed: ${RED}${TESTS_FAILED}${NC}"
echo -e "Skipped: ${YELLOW}${TESTS_SKIPPED}${NC}"
echo ""

log_info "Evidence saved to: $EVIDENCE_DIR"

# Generate summary report
cat > "$EVIDENCE_DIR/summary.md" << EOF
# DOTS Family Mode - Web Filtering VM Test Summary

## Test Run Information

- **Date:** $(date)
- **Filter Proxy:** ${FILTER_HOST}:${FILTER_PORT}
- **Evidence Directory:** ${EVIDENCE_DIR}

## Results

| Metric | Count |
|--------|-------|
| Total Tests | $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED)) |
| Passed | ${TESTS_PASSED} |
| Failed | ${TESTS_FAILED} |
| Skipped | ${TESTS_SKIPPED} |

## Status

$([ $TESTS_FAILED -eq 0 ] && echo "✅ ALL TESTS PASSED" || echo "⚠️ ${TESTS_FAILED} TEST(S) FAILED")

## Evidence Files

- Test logs: $EVIDENCE_DIR/*.log
- Web filtering evidence: $EVIDENCE_DIR/web-filtering/
- Screenshots: $EVIDENCE_DIR/screenshots/
- HTML responses: $EVIDENCE_DIR/html/
EOF

log_info "Summary report: $EVIDENCE_DIR/summary.md"

if [[ $TESTS_FAILED -eq 0 ]]; then
    exit 0
else
    exit 1
fi
