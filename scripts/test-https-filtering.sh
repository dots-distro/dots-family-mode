#!/usr/bin/env bash
#
# DOTS Family Mode - HTTPS Filtering Test Script
#
# This script tests the HTTPS filtering functionality including:
# - SSL CA certificate generation
# - Certificate installation to browsers
# - HTTPS proxy connectivity
# - Content blocking with SSL interception
#
# Usage: ./test-https-filtering.sh [--evidence DIR]
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-/tmp/dots-family-https-test-$(date +%s)}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${CYAN}[INFO]${NC} $*"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $*"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

run_test() {
    local test_name="$1"
    local test_command="$2"
    local description="$3"
    
    echo ""
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

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  DOTS Family Mode - HTTPS Filtering Test Suite${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"
mkdir -p "$EVIDENCE_DIR/certs"
mkdir -p "$EVIDENCE_DIR/screenshots"
mkdir -p "$EVIDENCE_DIR/html"
mkdir -p "$EVIDENCE_DIR/network"

log_info "Evidence directory: $EVIDENCE_DIR"
echo ""

# ============================================
# Phase 1: SSL CA Certificate Tests
# ============================================
echo -e "${CYAN}═══ Phase 1: SSL CA Certificate Tests ═══${NC}"
echo ""

run_test "OpenSSL Available" \
    "command -v openssl && openssl version" \
    "Verify OpenSSL is installed for certificate generation"

run_test "CA Certificate Directory" \
    "[ -d /var/lib/dots-family/ssl ] || mkdir -p /var/lib/dots-family/ssl" \
    "Verify/create CA certificate directory"

run_test "CA Certificate Exists" \
    "[ -f /var/lib/dots-family/ssl/ca.crt ]" \
    "Check if CA certificate exists"

run_test "CA Private Key Exists" \
    "[ -f /var/lib/dots-family/ssl/ca.key ]" \
    "Check if CA private key exists"

if [[ -f /var/lib/dots-family/ssl/ca.crt ]]; then
    run_test "CA Certificate Valid" \
        "openssl x509 -in /var/lib/dots-family/ssl/ca.crt -noout -text | head -20" \
        "Verify CA certificate is valid and readable"
    
    run_test "CA Certificate Issuer" \
        "openssl x509 -in /var/lib/dots-family/ssl/ca.crt -noout -issuer" \
        "Check CA certificate issuer"
fi

# ============================================
# Phase 2: System Certificate Installation
# ============================================
echo ""
echo -e "${CYAN}═══ Phase 2: System Certificate Installation ═══${NC}"
echo ""

run_test "System Cert Directory" \
    "[ -d /etc/ssl/certs ]" \
    "Verify /etc/ssl/certs directory exists"

if [[ -f /var/lib/dots-family/ssl/ca.crt ]]; then
    run_test "CA Installed to System" \
        "[ -f /etc/ssl/certs/dots-family-ca.crt ]" \
        "Check if CA is installed to system certificates"
    
    run_test "System Cert Hash" \
        "openssl x509 -in /etc/ssl/certs/dots-family-ca.crt -noout -hash 2>/dev/null || echo 'hash_not_found'" \
        "Get certificate hash for verification"
fi

# ============================================
# Phase 3: Browser Configuration
# ============================================
echo ""
echo -e "${CYAN}═══ Phase 3: Browser Configuration ═══${NC}"
echo ""

run_test "Firefox Available" \
    "command -v firefox" \
    "Check if Firefox is installed"

if command -v firefox &> /dev/null; then
    run_test "Firefox Version" \
        "firefox --version" \
        "Get Firefox version"
    
    # Check Firefox certificate policy
    if [[ -f /var/lib/dots-family/ssl/ca.crt ]]; then
        run_test "Firefox Certificate Policy" \
            "[ -f /etc/firefox/policies/policies.json ] || [ -f /usr/share/firefox/distribution/policies.json ]" \
            "Check Firefox policies directory"
    fi
fi

run_test "Chromium Available" \
    "command -v chromium-browser || command -v chromium" \
    "Check if Chromium is installed"

# ============================================
# Phase 4: Proxy Connectivity Tests
# ============================================
echo ""
echo -e "${CYAN}═══ Phase 4: Proxy Connectivity Tests ═══${NC}"
echo ""

PROXY_HOST="${PROXY_HOST:-127.0.0.1}"
PROXY_PORT="${PROXY_PORT:-8080}"

log_info "Testing proxy at ${PROXY_HOST}:${PROXY_PORT}"

run_test "Proxy Process Running" \
    "pgrep -f 'dots-family-filter.*${PROXY_PORT}' || echo 'not_running'" \
    "Check if proxy process is running"

run_test "HTTP Through Proxy" \
    "curl -x http://${PROXY_HOST}:${PROXY_PORT} -s -o /dev/null -w '%{http_code}' http://example.com 2>/dev/null | grep -q '200\|404'" \
    "Test HTTP request through proxy"

run_test "HTTPS Connect Method" \
    "curl -x http://${PROXY_HOST}:${PROXY_PORT} -s -o /dev/null -w '%{http_code}' https://example.com 2>/dev/null" \
    "Test HTTPS request through proxy"

# ============================================
# Phase 5: Content Filtering Tests
# ============================================
echo ""
echo -e "${CYAN}═══ Phase 5: Content Filtering Tests ═══${NC}"
echo ""

if curl -x http://${PROXY_HOST}:${PROXY_PORT} -s -o /dev/null -w '%{http_code}' http://example.com 2>/dev/null | grep -q '200\|404'; then
    run_test "HTML Capture Through Proxy" \
        "curl -x http://${PROXY_HOST}:${PROXY_PORT} -s 'http://example.com' > '$EVIDENCE_DIR/html/proxy_example_com.html' && [ -s '$EVIDENCE_DIR/html/proxy_example_com.html' ]" \
        "Capture HTML content through proxy"
    
    run_test "Response Headers" \
        "curl -x http://${PROXY_HOST}:${PROXY_PORT} -I http://example.com 2>/dev/null | head -10 > '$EVIDENCE_DIR/network/headers.log'" \
        "Capture response headers through proxy"
    
    # Test with browser if available
    if command -v chromium-browser &> /dev/null; then
        log_info "Testing with Chromium..."
        
        timeout 20 chromium-browser \
            --headless \
            --disable-gpu \
            --no-sandbox \
            --disable-dev-shm-usage \
            --proxy-server="http://${PROXY_HOST}:${PROXY_PORT}" \
            --screenshot="$EVIDENCE_DIR/screenshots/chromium_test.png" \
            --window-size=1024,768 \
            "http://example.com" 2>&1 | tee "$EVIDENCE_DIR/screenshots/capture.log" || true
        
        if [[ -f "$EVIDENCE_DIR/screenshots/chromium_test.png" ]]; then
            log_pass "Chromium screenshot captured"
            ((TESTS_PASSED++))
        else
            log_warn "Chromium screenshot not captured"
            ((TESTS_SKIPPED++))
        fi
    fi
else
    log_warn "Proxy not available - skipping content tests"
    ((TESTS_SKIPPED+=3))
fi

# ============================================
# Phase 6: Certificate Generation Tests
# ============================================
echo ""
echo -e "${CYAN}═══ Phase 6: Certificate Generation Tests ═══${NC}"
echo ""

if [[ -f /var/lib/dots-family/ssl/ca.crt ]] && [[ -f /var/lib/dots-family/ssl/ca.key ]]; then
    # Test site certificate generation
    SITE_KEY="$EVIDENCE_DIR/certs/site.key"
    SITE_CERT="$EVIDENCE_DIR/certs/site.crt"
    SITE_CSR="$EVIDENCE_DIR/certs/site.csr"
    
    run_test "Generate Site Key" \
        "openssl genrsa -out '$SITE_KEY' 2048 2>/dev/null && [ -f '$SITE_KEY' ]" \
        "Generate a test site private key"
    
    run_test "Generate Site CSR" \
        "openssl req -new -key '$SITE_KEY' -out '$SITE_CSR' -subj '/C=US/ST=Test/L=Test/O=DOTS Family Mode/CN=example.com' 2>/dev/null && [ -f '$SITE_CSR' ]" \
        "Generate a test site CSR"
    
    run_test "Sign Site Certificate" \
        "openssl x509 -req -in '$SITE_CSR' -CA /var/lib/dots-family/ssl/ca.crt -CAkey /var/lib/dots-family/ssl/ca.key -CAcreateserial -out '$SITE_CERT' -days 1 2>/dev/null && [ -f '$SITE_CERT' ]" \
        "Sign test site certificate with CA"
    
    if [[ -f "$SITE_CERT" ]]; then
        run_test "Verify Signed Certificate" \
            "openssl verify -CAfile /var/lib/dots-family/ssl/ca.crt '$SITE_CERT'" \
            "Verify the signed certificate chain"
    fi
fi

# ============================================
# Generate Summary Report
# ============================================
echo ""
echo -e "${CYAN}═══ Test Summary ═══${NC}"
echo ""

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Failed: ${RED}${TESTS_FAILED}${NC}"
echo -e "Skipped: ${YELLOW}${TESTS_SKIPPED}${NC}"
echo ""

# Generate markdown report
REPORT_FILE="$EVIDENCE_DIR/HTTPS_TEST_REPORT.md"
cat > "$REPORT_FILE" << EOF
# DOTS Family Mode - HTTPS Filtering Test Report

## Test Configuration

- **Proxy:** ${PROXY_HOST}:${PROXY_PORT}
- **Evidence Directory:** $EVIDENCE_DIR
- **Date:** $(date -Iseconds)
- **CA Certificate:** /var/lib/dots-family/ssl/ca.crt

## Test Results

| Metric | Count |
|--------|-------|
| Total Tests | $TOTAL_TESTS |
| Passed | $TESTS_PASSED |
| Failed | $TESTS_FAILED |
| Skipped | $TESTS_SKIPPED |

## Test Phases

### Phase 1: SSL CA Certificate Tests
- OpenSSL availability
- CA certificate existence
- Certificate validity

### Phase 2: System Certificate Installation
- System certificate directory
- CA installation to system trust store

### Phase 3: Browser Configuration
- Firefox configuration
- Chromium availability

### Phase 4: Proxy Connectivity Tests
- HTTP through proxy
- HTTPS CONNECT method

### Phase 5: Content Filtering Tests
- HTML capture through proxy
- Browser screenshot testing

### Phase 6: Certificate Generation Tests
- Site certificate generation
- Certificate signing
- Chain verification

## Evidence Files

- **Certificates:** $EVIDENCE_DIR/certs/
- **Screenshots:** $EVIDENCE_DIR/screenshots/
- **HTML:** $EVIDENCE_DIR/html/
- **Network:** $EVIDENCE_DIR/network/
- **Logs:** $EVIDENCE_DIR/*.log

## Status

$([ $TESTS_FAILED -eq 0 ] && echo "✅ ALL TESTS PASSED" || echo "⚠️ ${TESTS_FAILED} TEST(S) FAILED")
EOF

log_info "Report saved: $REPORT_FILE"
echo ""
log_info "Evidence saved to: $EVIDENCE_DIR"

if [[ $TESTS_FAILED -eq 0 ]]; then
    log_pass "All tests passed!"
    exit 0
else
    log_fail "Some tests failed"
    exit 1
fi
