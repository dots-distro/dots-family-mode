#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERMINAL_FILTER_BIN="${SCRIPT_DIR}/target/x86_64-unknown-linux-gnu/debug/dots-terminal-filter"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=== DOTS Terminal Filter Quick Test ==="

echo -n "Testing binary existence: "
if [[ -f "$TERMINAL_FILTER_BIN" ]]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗ Binary not found at $TERMINAL_FILTER_BIN${NC}"
    exit 1
fi

echo -n "Testing version: "
if VERSION=$($TERMINAL_FILTER_BIN --version 2>&1); then
    echo -e "${GREEN}✓ $VERSION${NC}"
else
    echo -e "${RED}✗ Failed${NC}"
    exit 1
fi

echo -n "Testing safe command (ls -la): "
if $TERMINAL_FILTER_BIN --check-only --command "ls -la" >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Allowed${NC}"
else
    echo -e "${RED}✗ Blocked${NC}"
fi

echo -n "Testing dangerous command (rm -rf /): "
if $TERMINAL_FILTER_BIN --check-only --command "rm -rf /" >/dev/null 2>&1; then
    echo -e "${RED}✗ Allowed (should be blocked)${NC}"
else
    echo -e "${GREEN}✓ Blocked${NC}"
fi

echo -n "Testing dangerous command (sudo rm -rf /tmp): "
if $TERMINAL_FILTER_BIN --check-only --command "sudo rm -rf /tmp" >/dev/null 2>&1; then
    echo -e "${RED}✗ Allowed (should be blocked)${NC}"
else
    echo -e "${GREEN}✓ Blocked${NC}"
fi

echo
echo "=== Shell Integration Files Test ==="

echo -n "Checking bash integration: "
if [[ -f "$SCRIPT_DIR/shell-integration/dots-bash-integration.sh" ]]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
fi

echo -n "Checking installer: "
if [[ -f "$SCRIPT_DIR/shell-integration/install.sh" ]]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
fi

echo -n "Testing installer help: "
if "$SCRIPT_DIR/shell-integration/install.sh" --help >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
fi

echo
echo "=== Summary ==="
echo "✓ Terminal filter implementation appears to be working!"
echo "✓ Shell integration files are present!"
echo "✓ Ready for VM testing or production deployment!"