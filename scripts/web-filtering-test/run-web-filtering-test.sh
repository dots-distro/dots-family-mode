#!/usr/bin/env bash
# DOTS Family Mode - Web Filtering Test Runner
# 
# This script runs the Playwright-based web filtering tests.
# It can be executed in the VM or on a development system.
#
# Usage:
#   ./run-web-filtering-test.sh [--proxy HOST] [--port PORT] [--evidence DIR]
#
# Defaults:
#   --proxy 127.0.0.1
#   --port 8080
#   --evidence test-evidence/web-filtering

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-test-evidence/web-filtering}"
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

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
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
            echo "DOTS Family Mode - Web Filtering Test Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --proxy HOST        Proxy host (default: 127.0.0.1)"
            echo "  --port PORT         Proxy port (default: 8080)"
            echo "  --evidence DIR      Evidence directory (default: test-evidence/web-filtering)"
            echo "  --help, -h          Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  PROXY_HOST          Proxy host"
            echo "  PROXY_PORT          Proxy port"
            echo "  EVIDENCE_DIR        Evidence directory"
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
echo -e "${CYAN}  DOTS Family Mode - Web Filtering Test Suite${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Check if Node.js is available
if ! command -v node &> /dev/null; then
    log_fail "Node.js is not installed. Please install Node.js 18+ to run tests."
    exit 1
fi

# Check Node.js version
NODE_VERSION=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
if [[ "$NODE_VERSION" -lt 18 ]]; then
    log_fail "Node.js version 18+ required. Found: $(node --version)"
    exit 1
fi

log_info "Node.js version: $(node --version)"
log_info "Proxy: ${PROXY_HOST}:${PROXY_PORT}"
log_info "Evidence directory: ${EVIDENCE_DIR}"
echo ""

# Configure Playwright to use Nix-packaged browsers (from nix/vm-simple.nix)
# The playwright-driver.browsers package provides pre-built browsers in versioned folders:
# chromium-1181, chromium_headless_shell-1181, firefox-1489, webkit-2191

# Method 1: Check system profile (VM with nix-env or system package)
if [[ -d "/run/current-system/sw/share/playwright-driver/browsers" ]]; then
    export PLAYWRIGHT_BROWSERS_PATH="/run/current-system/sw/share/playwright-driver/browsers"
    log_info "Playwright browsers (system): ${PLAYWRIGHT_BROWSERS_PATH}"
    
# Method 2: Check user profile
elif [[ -d "$HOME/.nix-profile/share/playwright-driver/browsers" ]]; then
    export PLAYWRIGHT_BROWSERS_PATH="$HOME/.nix-profile/share/playwright-driver/browsers"
    log_info "Playwright browsers (user): ${PLAYWRIGHT_BROWSERS_PATH}"
    
# Method 3: Query Nix store for installed package (works in nix develop shell)
elif PLAYWRIGHT_PATH=$(nix-instantiate --eval -E 'with import <nixpkgs> {}; playwright-driver.browsers.outPath' 2>/dev/null) && [[ -n "$PLAYWRIGHT_PATH" && -d "$PLAYWRIGHT_PATH" ]]; then
    export PLAYWRIGHT_BROWSERS_PATH="$PLAYWRIGHT_PATH"
    log_info "Playwright browsers (nix store): ${PLAYWRIGHT_BROWSERS_PATH}"
    
# Method 4: Try to find browsers in common Nix store locations
else
    for nix_path in /nix/store/*-playwright-browsers /nix/store/*-playwright-driver.browsers; do
        if [[ -d "$nix_path" && -d "$nix_path/chromium_headless_shell"* ]]; then
            export PLAYWRIGHT_BROWSERS_PATH="$nix_path"
            log_info "Playwright browsers (found): ${PLAYWRIGHT_BROWSERS_PATH}"
            break
        fi
    done
fi

# Set download skip flag to prevent attempts to download browsers
export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD="1"

# Verify browsers are accessible
if [[ -n "${PLAYWRIGHT_BROWSERS_PATH:-}" && -d "$PLAYWRIGHT_BROWSERS_PATH" ]]; then
    log_info "Using browsers from: ${PLAYWRIGHT_BROWSERS_PATH}"
    ls -la "$PLAYWRIGHT_BROWSERS_PATH" 2>/dev/null || true
else
    log_warn "Playwright browsers not found - browser tests will be skipped"
    export PLAYWRIGHT_BROWSERS_PATH="${EVIDENCE_DIR}/.browsers"
fi

# Create evidence directory
mkdir -p "$EVIDENCE_DIR"
mkdir -p "$EVIDENCE_DIR/screenshots"
mkdir -p "$EVIDENCE_DIR/network"
mkdir -p "$EVIDENCE_DIR/html"

# Run the tests
log_info "Starting web filtering tests..."
echo ""

# Build command with arguments
NODE_ARGS=()
NODE_ARGS+=("$SCRIPT_DIR/web-filtering-test.js")
NODE_ARGS+=("--proxy-host=$PROXY_HOST")
NODE_ARGS+=("--proxy-port=$PROXY_PORT")
NODE_ARGS+=("--evidence-dir=$EVIDENCE_DIR")

# Execute tests
if node "${NODE_ARGS[@]}"; then
    echo ""
    log_pass "Web filtering tests completed successfully"
    echo ""
    log_info "Evidence saved to: $EVIDENCE_DIR"
    log_info "Report: $EVIDENCE_DIR/test_report.md"
    exit 0
else
    EXIT_CODE=$?
    echo ""
    log_fail "Web filtering tests failed (exit code: $EXIT_CODE)"
    echo ""
    log_info "Check evidence in: $EVIDENCE_DIR"
    exit $EXIT_CODE
fi
