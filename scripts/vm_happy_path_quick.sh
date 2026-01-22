#!/usr/bin/env bash
# DOTS Family Mode - Happy Path Quick Test Suite

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0

test_result() {
    local test_name="$1"
    local test_command="$2"
    
    echo -e "${BLUE}[TEST]${NC} ${test_name}"
    
    if eval "$test_command" >/dev/null 2>&1; then
        echo -e "${GREEN}[PASS]${NC} ${test_name}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}[FAIL]${NC} ${test_name}"
        ((TESTS_FAILED++))
    fi
}

echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  DOTS FAMILY MODE - HAPPY PATH TEST SUITE                   ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${CYAN}═══ PHASE 1: Binary Installation Tests ═══${NC}"

test_result "Main package builds" "nix build .#default >/dev/null 2>&1"
test_result "Daemon package builds" "nix build .#dots-family-daemon >/dev/null 2>&1"
test_result "Monitor package builds" "nix build .#dots-family-monitor >/dev/null 2>&1"
test_result "CLI tool builds" "nix build .#dots-family-ctl >/dev/null 2>&1"
test_result "Filter service builds" "nix build .#dots-family-filter >/dev/null 2>&1"
test_result "Terminal filter builds" "nix build .#dots-terminal-filter >/dev/null 2>&1"
test_result "eBPF programs build" "nix build .#dots-family-ebpf >/dev/null 2>&1"
test_result "All binaries exist" "test -f result/bin/dots-family-daemon && test -f result/bin/dots-family-monitor"
test_result "eBPF programs exist" "test -f result/target/bpfel-unknown-none/release/process-monitor"

echo ""
echo -e "${CYAN}═══ PHASE 2: Binary Functionality Tests ═══${NC}"

test_result "Daemon starts" "timeout 5 ./result/bin/dots-family-daemon 2>&1 | grep -q 'Starting DOTS Family'"
test_result "Monitor starts" "timeout 3 ./result/bin/dots-family-monitor 2>&1 | grep -q 'Starting DOTS Family'"
test_result "CLI help works" "./result/bin/dots-family-ctl --help | grep -q 'DOTS'"
test_result "Filter help works" "./result/bin/dots-family-filter --help | grep -q 'filter'"
test_result "Terminal filter help works" "./result/bin/dots-terminal-filter --help | grep -q 'filter'"

echo ""
echo -e "${CYAN}═══ PHASE 3: Configuration Tests ═══${NC}"

test_result "Flake syntax valid" "nix-instantiate --parse flake.nix >/dev/null 2>&1"
test_result "README exists" "test -f README.md"
test_result "Systemd service exists" "test -f systemd/dots-family-daemon.service"
test_result "Install script exists" "test -f systemd/install.sh"
test_result "DBus service exists" "test -f dbus/org.dots.FamilyDaemon.service"
test_result "Daemon module exists" "test -f nixos-modules/dots-family/daemon.nix"
test_result "Default module exists" "test -f nixos-modules/dots-family/default.nix"
test_result "VM config exists" "test -f nix/vm-simple.nix"
test_result "Migrations exist" "ls crates/dots-family-db/migrations/*.sql | wc -l | grep -q '^6'"

echo ""
echo -e "${CYAN}═══ PHASE 4: Source Structure Tests ═══${NC}"

test_result "Daemon source dir exists" "test -d crates/dots-family-daemon/src"
test_result "Monitor source dir exists" "test -d crates/dots-family-monitor/src"
test_result "CLI source dir exists" "test -d crates/dots-family-ctl/src"
test_result "Filter source dir exists" "test -d crates/dots-family-filter/src"
test_result "DB source dir exists" "test -d crates/dots-family-db/src"
test_result "Common crate exists" "test -d crates/dots-family-common/src"
test_result "Main Cargo.toml exists" "test -f Cargo.toml"
test_result "Cargo.lock exists" "test -f Cargo.lock"

echo ""
echo -e "${CYAN}═══ PHASE 5: Documentation Tests ═══${NC}"

test_result "Architecture docs exist" "test -f docs/ARCHITECTURE.md"
test_result "NixOS docs exist" "test -f docs/NIXOS_INTEGRATION.md"
test_result "Security docs exist" "test -f docs/SECURITY_ARCHITECTURE.md"
test_result "Parental controls docs exist" "test -f docs/PARENTAL_CONTROLS.md"
test_result "Monitoring docs exist" "test -f docs/MONITORING.md"
test_result "Example config exists" "test -f nixos-modules/example-configuration.nix"

echo ""
echo -e "${CYAN}═══ PHASE 6: Security Configuration Tests ═══${NC}"

test_result "Systemd has User config" "grep -q 'User=' systemd/dots-family-daemon.service"
test_result "Systemd has Restart config" "grep -q 'Restart=' systemd/dots-family-daemon.service"
test_result "Systemd has CAP_SYS_ADMIN" "grep -q 'CAP_SYS_ADMIN' systemd/dots-family-daemon.service"
test_result "Systemd has ProtectSystem" "grep -q 'ProtectSystem' systemd/dots-family-daemon.service"
test_result "Daemon module has runAsRoot" "grep -q 'runAsRoot' nixos-modules/dots-family/default.nix"
test_result "Daemon module has profiles" "grep -q 'profiles' nixos-modules/dots-family/default.nix"

echo ""
echo -e "${CYAN}═══ PHASE 7: Flake Integration Tests ═══${NC}"

test_result "Flake has default output" "grep -q 'default.*=' flake.nix"
test_result "Flake has nixosModules" "grep -q 'nixosModules' flake.nix"
test_result "Flake has overlays" "grep -q 'overlays' flake.nix"
test_result "Flake has devShells" "grep -q 'devShells' flake.nix"
test_result "Flake has checks" "grep -q 'checks' flake.nix"
test_result "Flake has nixosConfigurations" "grep -q 'nixosConfigurations' flake.nix"

echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  TEST SUMMARY                                                ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
TOTAL=$((TESTS_PASSED + TESTS_FAILED))
PASS_RATE=$(( TESTS_PASSED * 100 / TOTAL ))

echo -e "  ${GREEN}Passed:${NC} ${TESTS_PASSED}"
echo -e "  ${RED}Failed:${NC} ${TESTS_FAILED}"
echo -e "  Total:  ${TOTAL}"
echo -e "  ${CYAN}Pass Rate:${NC} ${PASS_RATE}%"
echo ""

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some tests failed${NC}"
    exit 1
fi
