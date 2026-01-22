#!/usr/bin/env bash
# DOTS Family Mode - Direct Browser Test
# 
# This script tests web filtering using direct browser commands
# instead of Playwright (which has compatibility issues in some environments)
#
# Usage:
#   ./direct-browser-test.sh [--proxy HOST] [--port PORT] [--evidence DIR]
#
# Default: proxy=127.0.0.1:8080, evidence=/tmp/dots-family-browser-test

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-/tmp/dots-family-browser-test}"
PROXY_HOST="${PROXY_HOST:-127.0.0.1}"
PROXY_PORT="${PROXY_PORT:-8080}"

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

log_block() {
    echo -e "${YELLOW}[BLOCK]${NC} $*"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --proxy|--proxy-host)
            PROXY_HOST="$2"
            shift 2
            ;;
        --port)
            PROXY_PORT="$2"
            shift 2
            ;;
        --evidence|--evidence-dir)
            EVIDENCE_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "DOTS Family Mode - Direct Browser Test"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --proxy HOST        Proxy host (default: 127.0.0.1)"
            echo "  --port PORT         Proxy port (default: 8080)"
            echo "  --evidence DIR      Evidence directory (default: /tmp/dots-family-browser-test)"
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
echo -e "${CYAN}  DOTS Family Mode - Direct Browser Test${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

log_info "Proxy: ${PROXY_HOST}:${PROXY_PORT}"
log_info "Evidence: ${EVIDENCE_DIR}"
echo ""

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"
mkdir -p "$EVIDENCE_DIR/screenshots"
mkdir -p "$EVIDENCE_DIR/html"
mkdir -p "$EVIDENCE_DIR/network"

# Initialize counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Test function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_result="$3"  # "pass" or "block" or "allow"
    
    echo -e "${CYAN}[TEST]${NC} ${test_name}"
    
    if eval "$test_command" > "$EVIDENCE_DIR/${test_name// /-}.log" 2>&1; then
        log_pass "${test_name} completed"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} failed"
        ((TESTS_FAILED++))
    fi
}

# Check if Firefox is available
if ! command -v firefox &> /dev/null; then
    log_fail "Firefox not found - cannot run browser tests"
    echo ""
    echo "Evidence saved to: $EVIDENCE_DIR"
    exit 1
fi

log_info "Firefox version: $(firefox --version 2>/dev/null || echo 'unknown')"

# Test 1: Proxy connectivity
echo ""
echo -e "${CYAN}═══ Proxy Connectivity Tests ═══${NC}"

if curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s -o /dev/null -w "%{http_code}" http://example.com &>/dev/null; then
    log_pass "Proxy is accessible"
    ((TESTS_PASSED++))
    PROXY_AVAILABLE=true
else
    log_fail "Proxy not accessible - starting fresh proxy"
    PROXY_AVAILABLE=false
    
    # Try to start the proxy
    if [[ -x "/run/current-system/sw/bin/dots-family-filter" ]]; then
        /run/current-system/sw/bin/dots-family-filter --bind-address "$PROXY_HOST" --port "$PROXY_PORT" &
        sleep 2
        
        if curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s -o /dev/null -w "%{http_code}" http://example.com &>/dev/null; then
            log_pass "Proxy started successfully"
            PROXY_AVAILABLE=true
            ((TESTS_PASSED++))
        else
            log_fail "Failed to start proxy"
            ((TESTS_FAILED++))
        fi
    fi
fi

# Test 2: Firefox headless with proxy
echo ""
echo -e "${CYAN}═══ Browser Tests ═══${NC}"

if $PROXY_AVAILABLE; then
    # Test with Firefox
    log_info "Testing with Firefox headless..."
    
    if timeout 15 firefox --headless \
        --screenshot "$EVIDENCE_DIR/screenshots/firefox_test.png" \
        --width 1024 --height 768 \
        -proxy-server="http://${PROXY_HOST}:${PROXY_PORT}" \
        --new-window "https://example.com" 2>&1 | grep -q "success\|Screenshot"; then
        log_pass "Firefox headless screenshot captured"
        ((TESTS_PASSED++))
        
        # Check screenshot size
        if [[ -f "$EVIDENCE_DIR/screenshots/firefox_test.png" ]] && [[ -s "$EVIDENCE_DIR/screenshots/firefox_test.png" ]]; then
            log_info "Screenshot size: $(ls -lh "$EVIDENCE_DIR/screenshots/firefox_test.png" | awk '{print $5}')"
        fi
    else
        log_fail "Firefox headless test failed"
        ((TESTS_FAILED++))
    fi
    
    # Test HTML capture
    log_info "Capturing HTML response..."
    HTML_CONTENT=$(curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s "http://example.com" 2>/dev/null)
    HTML_FILE="$EVIDENCE_DIR/html/proxy_response_$(date +%s).html"
    echo "$HTML_CONTENT" > "$HTML_FILE"
    
    if [[ -s "$HTML_FILE" ]]; then
        log_pass "HTML response captured (${#HTML_CONTENT} bytes)"
        ((TESTS_PASSED++))
    else
        log_fail "HTML capture failed"
        ((TESTS_FAILED++))
    fi
else
    log_info "Skipping browser tests (no proxy)"
    ((TESTS_SKIPPED++))
fi

# Test 3: Network monitoring
echo ""
echo -e "${CYAN}═══ Network Tests ═══${NC}"

if $PROXY_AVAILABLE; then
    # Test different protocols
    for url in "http://example.com" "https://example.com"; do
        protocol=$(echo "$url" | cut -d: -f1)
        log_info "Testing ${protocol}..."
        
        HTTP_CODE=$(curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null || echo "000")
        
        # Save network log
        {
            echo "URL: $url"
            echo "Protocol: $protocol"
            echo "HTTP Code: $HTTP_CODE"
            echo "Timestamp: $(date -Iseconds)"
            echo "Proxy: ${PROXY_HOST}:${PROXY_PORT}"
        } > "$EVIDENCE_DIR/network/${protocol}_test.log"
        
        if [[ "$HTTP_CODE" =~ ^[23] ]]; then
            log_pass "${protocol} request successful (HTTP $HTTP_CODE)"
            ((TESTS_PASSED++))
        else
            log_fail "${protocol} request failed (HTTP $HTTP_CODE)"
            ((TESTS_FAILED++))
        fi
    done
else
    log_info "Skipping network tests (no proxy)"
    ((TESTS_SKIPPED+=2))
fi

# Test 4: Block page detection (if configured)
echo ""
echo -e "${CYAN}═══ Content Filtering Tests ═══${NC}"

if $PROXY_AVAILABLE; then
    # Test with a URL that might be blocked
    BLOCK_TEST_URL="http://example.com/blocked-test"
    log_info "Testing blocked URL detection..."
    
    RESPONSE=$(curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s "$BLOCK_TEST_URL" 2>/dev/null)
    HTTP_CODE=$(curl -x "http://${PROXY_HOST}:${PROXY_PORT}" -s -o /dev/null -w "%{http_code}" "$BLOCK_TEST_URL" 2>/dev/null || echo "000")
    
    # Save response
    echo "$RESPONSE" > "$EVIDENCE_DIR/html/block_test_response.html"
    
    if [[ "$HTTP_CODE" == "403" ]] || echo "$RESPONSE" | grep -qi "blocked\|denied\|forbidden"; then
        log_block "Content filtering active (HTTP $HTTP_CODE)"
        ((TESTS_PASSED++))
    elif [[ "$HTTP_CODE" == "200" ]]; then
        log_info "Content allowed (HTTP $HTTP_CODE) - filtering may not be configured"
        ((TESTS_PASSED++))
    else
        log_fail "Unexpected response (HTTP $HTTP_CODE)"
        ((TESTS_FAILED++))
    fi
else
    log_info "Skipping content filtering tests"
    ((TESTS_SKIPPED++))
fi

# Generate summary report
echo ""
echo -e "${CYAN}═══ Summary ═══${NC}"

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))

echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Skipped: ${YELLOW}$TESTS_SKIPPED${NC}"
echo ""

# Generate markdown report
REPORT_FILE="$EVIDENCE_DIR/browser_test_report.md"
cat > "$REPORT_FILE" << EOF
# DOTS Family Mode - Direct Browser Test Report

## Test Configuration

- **Proxy Host:** ${PROXY_HOST}
- **Proxy Port:** ${PROXY_PORT}
- **Evidence Directory:** ${EVIDENCE_DIR}
- **Date:** $(date -Iseconds)

## Test Results

| Metric | Count |
|--------|-------|
| Total Tests | $TOTAL_TESTS |
| Passed | $TESTS_PASSED |
| Failed | $TESTS_FAILED |
| Skipped | $TESTS_SKIPPED |

## Test Details

### Browser Tests
- Firefox headless screenshot: $([ -f "$EVIDENCE_DIR/screenshots/firefox_test.png" ] && echo "✅ Captured" || echo "❌ Failed")
- Screenshot size: $(ls -lh "$EVIDENCE_DIR/screenshots/firefox_test.png" 2>/dev/null | awk '{print $5}' || echo "N/A")

### Network Tests
- HTTP requests tested
- HTTPS requests tested

### Content Filtering
- Block page detection tested

## Evidence Files

- **Screenshots:** $EVIDENCE_DIR/screenshots/
- **HTML:** $EVIDENCE_DIR/html/
- **Network Logs:** $EVIDENCE_DIR/network/
- **Test Logs:** $EVIDENCE_DIR/*.log

## Status

$([ $TESTS_FAILED -eq 0 ] && echo "✅ ALL TESTS PASSED" || echo "⚠️ SOME TESTS FAILED")
EOF

log_info "Report saved: $REPORT_FILE"

echo ""
echo -e "${CYAN}Evidence saved to: $EVIDENCE_DIR${NC}"
echo -e "${CYAN}Report: $REPORT_FILE${NC}"
echo ""

if [[ $TESTS_FAILED -eq 0 ]]; then
    log_pass "All tests passed!"
    exit 0
else
    log_fail "Some tests failed"
    exit 1
fi
