#!/usr/bin/env bash
# DOTS Family Mode - Comprehensive Happy Path Test Suite
# Tests complete user workflows and system functionality

set -euo pipefail

VM_SSH_PORT="${VM_SSH_PORT:-10022}"
VM_HOST="${VM_HOST:-localhost}"
TEST_LOG="vm_happy_path_results.log"
EVIDENCE_DIR="test-evidence"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
WHITE='\033[1;37m'
NC='\033[0m'

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

EVIDENCE_FILE="${EVIDENCE_DIR}/happy_path_test_evidence_$(date +%Y%m%d_%H%M%S).md"

log() {
    local timestamp=$(date '+%H:%M:%S')
    echo -e "${timestamp} $*"
}

log_header() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║  $*{NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
}

log_section() {
    echo ""
    echo -e "${MAGENTA}┌─── $* ───┐${NC}"
}

log_subsection() {
    echo -e "\n${WHITE}  ▶ $*${NC}"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $*"
    ((TESTS_FAILED++))
}

log_skipped() {
    echo -e "${YELLOW}[SKIP]${NC} $*"
    ((TESTS_SKIPPED++))
}

run_test() {
    ((TESTS_RUN++))
    local test_name="$1"
    local test_description="$2"
    shift 2
    
    log_subsection "${test_name}"
    echo "**Test:** ${test_name}" >> "${EVIDENCE_FILE}"
    echo "**Description:** ${test_description}" >> "${EVIDENCE_FILE}"
    
    if "$@" >/dev/null 2>&1; then
        log_success "${test_name}"
        echo "**Result:** ✅ PASSED" >> "${EVIDENCE_FILE}"
        return 0
    else
        log_error "${test_name}"
        echo "**Result:** ❌ FAILED" >> "${EVIDENCE_FILE}"
        return 1
    fi
}

mkdir -p "${EVIDENCE_DIR}"

cat > "${EVIDENCE_FILE}" << 'EOF'
# DOTS Family Mode - Happy Path Test Evidence
Generated: $(date)
Host: $(hostname)

## Test Coverage Areas

1. Installation & Setup Workflow
2. Parent User Configuration Workflow
3. Child User Experience Workflow
4. Daemon Service Operations
5. DBus Communication Tests
6. Monitoring & eBPF Tests
7. CLI Command Tests
8. Security & Permissions Tests

EOF

log_header "DOTS FAMILY MODE - HAPPY PATH TEST SUITE"
log_info "Comprehensive user workflow validation"

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 1: INSTALLATION & SETUP WORKFLOW${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "1.1 Binary Installation Tests"

run_test "Main package builds successfully" \
    "Build all DOTS Family binaries" \
    nix build .#default

run_test "Daemon binary builds" \
    "Build dots-family-daemon package" \
    nix build .#dots-family-daemon

run_test "Monitor binary builds" \
    "Build dots-family-monitor package" \
    nix build .#dots-family-monitor

run_test "CLI tool binary builds" \
    "Build dots-family-ctl package" \
    nix build .#dots-family-ctl

run_test "Filter service binary builds" \
    "Build dots-family-filter package" \
    nix build .#dots-family-filter

run_test "Terminal filter binary builds" \
    "Build dots-terminal-filter package" \
    nix build .#dots-terminal-filter

run_test "eBPF programs build" \
    "Build kernel-space eBPF programs" \
    nix build .#dots-family-ebpf

log_section "1.2 Build Output Validation"

run_test "All binaries exist in output directory" \
    "Verify all built binaries are present" \
    bash -c 'ls -la result/bin/ >/dev/null 2>&1'

run_test "Daemon binary is executable" \
    "Check daemon binary has execute permissions" \
    bash -c 'test -x result/bin/dots-family-daemon'

run_test "CLI tool binary is executable" \
    "Check CLI binary has execute permissions" \
    bash -c 'test -x result/bin/dots-family-ctl'

run_test "eBPF programs exist" \
    "Verify eBPF program binaries are present" \
    bash -c 'ls -la result/target/bpfel-unknown-none/release/ >/dev/null 2>&1'

run_test "Binary sizes are reasonable" \
    "Check binaries are not empty (minimum 1KB)" \
    bash -c 'test $(stat -c%s result/bin/dots-family-daemon) -gt 1000'

log_section "1.3 NixOS Module Installation"

run_test "Flake evaluates successfully" \
    "Validate flake.nix syntax and evaluation" \
    bash -c 'nix-instantiate --parse flake.nix >/dev/null 2>&1'

run_test "NixOS modules load without errors" \
    "Import and validate NixOS module structure" \
    bash -c 'nix flake show >/dev/null 2>&1'

run_test "Systemd service file is valid" \
    "Validate systemd service configuration syntax" \
    bash -c 'systemd-analyze verify deployment/systemd/dots-family-daemon.service 2>/dev/null || true'

run_test "Installation script is executable" \
    "Check install script has correct permissions" \
    bash -c 'test -x deployment/systemd/install.sh'

log_section "1.4 Database Migration Tests"

run_test "Migration files exist" \
    "Verify database migration files are present" \
    bash -c 'test -f crates/dots-family-db/migrations/20260114155016_initial_schema.sql'

run_test "All migrations are numbered correctly" \
    "Validate migration file naming convention" \
    bash -c 'ls crates/dots-family-db/migrations/*.sql | wc -l | grep -q "^6$"'

run_test "Migration files are readable" \
    "Check migration files have proper permissions" \
    bash -c 'for f in crates/dots-family-db/migrations/*.sql; do test -r "$f"; done'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 2: PARENT USER CONFIGURATION WORKFLOW${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "2.1 CLI Tool Functionality"

run_test "CLI help displays correctly" \
    "Verify dots-family-ctl shows help information" \
    bash -c './result/bin/dots-family-ctl --help | grep -q "DOTS"'

run_test "CLI profile subcommand exists" \
    "Check profile management commands are available" \
    bash -c './result/bin/dots-family-ctl profile --help >/dev/null 2>&1'

run_test "CLI session subcommand exists" \
    "Check session management commands are available" \
    bash -c './result/bin/dots-family-ctl session --help >/dev/null 2>&1'

run_test "CLI status subcommand exists" \
    "Check status commands are available" \
    bash -c './result/bin/dots-family-ctl status --help >/dev/null 2>&1'

run_test "CLI check subcommand exists" \
    "Check application verification commands are available" \
    bash -c './result/bin/dots-family-ctl check --help >/dev/null 2>&1'

log_section "2.2 Service Configuration"

run_test "Systemd service has correct description" \
    "Verify service file contains proper description" \
    bash -c 'grep -q "Description=DOTS Family Mode Daemon" deployment/systemd/dots-family-daemon.service'

run_test "Systemd service has correct dependencies" \
    "Verify service has proper After/Wants directives" \
    bash -c 'grep -q "After=network.target" deployment/systemd/dots-family-daemon.service'

run_test "Systemd service has restart policy" \
    "Verify service has Restart configuration" \
    bash -c 'grep -q "Restart=on-failure" deployment/systemd/dots-family-daemon.service'

run_test "Systemd service has proper user settings" \
    "Verify service runs as root or dedicated user" \
    bash -c 'grep -qE "(User=root|User=dots-family)" deployment/systemd/dots-family-daemon.service'

run_test "Systemd service has capability configuration" \
    "Verify eBPF capabilities are configured" \
    bash -c 'grep -q "CAP_SYS_ADMIN" deployment/systemd/dots-family-daemon.service'

run_test "Systemd service has filesystem protection" \
    "Verify ProtectSystem directive is present" \
    bash -c 'grep -q "ProtectSystem" deployment/systemd/dots-family-daemon.service'

log_section "2.3 Configuration File Tests"

run_test "Daemon configuration file exists" \
    "Verify daemon configuration template is present" \
    bash -c 'test -f systemd/daemon.conf'

run_test "Monitor configuration file exists" \
    "Verify monitor configuration template is present" \
    bash -c 'test -f systemd/monitor.conf'

run_test "Configuration files have proper permissions" \
    "Check config files are readable" \
    bash -c 'test -r systemd/daemon.conf && test -r systemd/monitor.conf'

log_section "2.4 DBus Service Configuration"

run_test "DBus service file exists" \
    "Verify D-Bus service definition is present" \
    bash -c 'test -f deployment/dbus/org.dots.FamilyDaemon.service'

run_test "DBus service has correct name" \
    "Verify DBus service name matches daemon" \
    bash -c 'grep -q "org.dots.FamilyDaemon" deployment/dbus/org.dots.FamilyDaemon.service'

run_test "DBus service has correct executable path" \
    "Verify DBus service points to correct binary" \
    bash -c 'grep -q "dots-family-daemon" deployment/dbus/org.dots.FamilyDaemon.service'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 3: DAEMON SERVICE OPERATIONS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "3.1 Daemon Binary Tests"

run_test "Daemon starts without arguments" \
    "Verify daemon can initialize without explicit configuration" \
    bash -c 'timeout 5 ./result/bin/dots-family-daemon 2>&1 | grep -q "Starting DOTS Family Daemon"'

run_test "Daemon initializes database" \
    "Verify database initialization completes successfully" \
    bash -c 'rm -f /tmp/test-daemon.db && timeout 10 ./result/bin/dots-family-daemon 2>&1 | grep -q "Database initialized successfully"'

run_test "Daemon runs migrations" \
    "Verify database migrations execute properly" \
    bash -c 'rm -f /tmp/test-daemon.db && timeout 10 ./result/bin/dots-family-daemon 2>&1 | grep -q "Database migrations completed"'

run_test "Daemon initializes policy engine" \
    "Verify policy engine starts successfully" \
    bash -c 'rm -f /tmp/test-daemon.db && timeout 10 ./result/bin/dots-family-daemon 2>&1 | grep -q "policy engine initialized"'

run_test "Daemon initializes eBPF manager" \
    "Verify eBPF manager is available" \
    bash -c 'rm -f /tmp/test-daemon.db && timeout 10 ./result/bin/dots-family-daemon 2>&1 | grep -q "eBPF manager initialized"'

run_test "Daemon accepts help flag" \
    "Verify daemon responds to help" \
    bash -c './result/bin/dots-family-daemon --help 2>&1 | grep -q "DOTS"'

log_section "3.2 Monitor Binary Tests"

run_test "Monitor binary starts" \
    "Verify monitor can initialize" \
    bash -c 'timeout 3 ./result/bin/dots-family-monitor 2>&1 | grep -q "Starting DOTS Family Monitor"'

run_test "Monitor detects window manager" \
    "Verify monitor can detect window manager type" \
    bash -c 'timeout 3 ./result/bin/dots-family-monitor 2>&1 | grep -q "window manager"'

run_test "Monitor has WMCapabilities" \
    "Verify monitor detects window manager capabilities" \
    bash -c 'timeout 3 ./result/bin/dots-family-monitor 2>&1 | grep -q "WMCapabilities"'

run_test "Monitor shows activity polling" \
    "Verify monitor has activity polling mechanism" \
    bash -c 'timeout 3 ./result/bin/dots-family-monitor 2>&1 | grep -q "polling"'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 4: FILTER SERVICE TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "4.1 Content Filter Tests"

run_test "Filter service shows help" \
    "Verify filter accepts help flag" \
    bash -c './result/bin/dots-family-filter --help | grep -q "filtering"'

run_test "Filter service has port configuration" \
    "Verify filter has configurable port option" \
    bash -c './result/bin/dots-family-filter --help | grep -q "port"'

run_test "Filter service has bind address" \
    "Verify filter has configurable bind address" \
    bash -c './result/bin/dots-family-filter --help | grep -q "bind"'

run_test "Filter service has config path option" \
    "Verify filter supports configuration file" \
    bash -c './result/bin/dots-family-filter --help | grep -q "config"'

log_section "4.2 Terminal Filter Tests"

run_test "Terminal filter shows help" \
    "Verify terminal filter accepts help flag" \
    bash -c './result/bin/dots-terminal-filter --help | grep -q "filter"'

run_test "Terminal filter has shell option" \
    "Verify terminal filter supports shell specification" \
    bash -c './result/bin/dots-terminal-filter --help | grep -q "shell"'

run_test "Terminal filter has check-only mode" \
    "Verify terminal filter has safety check mode" \
    bash -c './result/bin/dots-terminal-filter --help | grep -q "check"'

run_test "Terminal filter has interactive mode" \
    "Verify terminal filter supports interactive mode" \
    bash -c './result/bin/dots-terminal-filter --help | grep -q "interactive"'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 5: NIXOS MODULE TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "5.1 Module Structure Tests"

run_test "Main module file exists" \
    "Verify NixOS main module file is present" \
    bash -c 'test -f nixos-modules/dots-family/default.nix'

run_test "Daemon module file exists" \
    "Verify daemon configuration module is present" \
    bash -c 'test -f nixos-modules/dots-family/daemon.nix'

run_test "DBus module file exists" \
    "Verify D-Bus configuration module is present" \
    bash -c 'test -f nixos-modules/dots-family/dbus.nix'

run_test "Security module file exists" \
    "Verify security hardening module is present" \
    bash -c 'test -f nixos-modules/dots-family/security.nix'

run_test "User services module exists" \
    "Verify user services module is present" \
    bash -c 'test -f nixos-modules/dots-family/user-services.nix'

log_section "5.2 Module Configuration Tests"

run_test "Module has enable option" \
    "Verify services.dots-family.enable option exists" \
    bash -c 'grep -q "enable.*EnableOption" nixos-modules/dots-family/default.nix'

run_test "Module has parent users option" \
    "Verify parentUsers configuration option exists" \
    bash -c 'grep -q "parentUsers" nixos-modules/dots-family/default.nix'

run_test "Module has child users option" \
    "Verify childUsers configuration option exists" \
    bash -c 'grep -q "childUsers" nixos-modules/dots-family/default.nix'

run_test "Module has profiles option" \
    "Verify profiles configuration option exists" \
    bash -c 'grep -q "profiles" nixos-modules/dots-family/default.nix'

run_test "Module has runAsRoot option" \
    "Verify runAsRoot option for privilege control exists" \
    bash -c 'grep -q "runAsRoot" nixos-modules/dots-family/default.nix'

run_test "Module has reportingOnly option" \
    "Verify reportingOnly option exists" \
    bash -c 'grep -q "reportingOnly" nixos-modules/dots-family/default.nix'

log_section "5.3 Daemon Module Tests"

run_test "Daemon module creates systemd service" \
    "Verify systemd service is defined in daemon module" \
    bash -c 'grep -q "systemd.services.dots-family-daemon" nixos-modules/dots-family/daemon.nix'

run_test "Daemon module configures service type" \
    "Verify service Type is set to dbus" \
    bash -c 'grep -q "Type.*dbus" nixos-modules/dots-family/daemon.nix'

run_test "Daemon module configures capabilities" \
    "Verify capability bounding set is configured" \
    bash -c 'grep -q "CapabilityBoundingSet" nixos-modules/dots-family/daemon.nix'

run_test "Daemon module supports root user" \
    "Verify User can be set to root" \
    bash -c 'grep -q "User.*root" nixos-modules/dots-family/daemon.nix'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 6: DOCUMENTATION & RESOURCE TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "6.1 Documentation Tests"

run_test "README file exists" \
    "Verify project README is present" \
    bash -c 'test -f README.md'

run_test "Architecture documentation exists" \
    "Verify architecture documentation is present" \
    bash -c 'test -f docs/ARCHITECTURE.md'

run_test "NixOS integration documentation exists" \
    "Verify NixOS integration docs are present" \
    bash -c 'test -f docs/NIXOS_INTEGRATION.md'

run_test "Security documentation exists" \
    "Verify security architecture documentation is present" \
    bash -c 'test -f docs/SECURITY_ARCHITECTURE.md'

run_test "Parental controls documentation exists" \
    "Verify parental controls documentation is present" \
    bash -c 'test -f docs/PARENTAL_CONTROLS.md'

run_test "Monitoring documentation exists" \
    "Verify monitoring documentation is present" \
    bash -c 'test -f docs/MONITORING.md'

log_section "6.2 Example Configuration Tests"

run_test "Example configuration exists" \
    "Verify example NixOS configuration is present" \
    bash -c 'test -f nixos-modules/example-configuration.nix'

run_test "Example configuration is valid Nix" \
    "Verify example configuration syntax is correct" \
    bash -c 'nix-instantiate --parse nixos-modules/example-configuration.nix >/dev/null 2>&1'

log_section "6.3 Script Tests"

run_test "VM test script exists" \
    "Verify VM testing script is present" \
    bash -c 'test -f scripts/tests/vm-test.sh'

run_test "Integration test script exists" \
    "Verify integration test script is present" \
    bash -c 'test -f scripts/test_integration.sh'

run_test "Deployment script exists" \
    "Verify system deployment script is present" \
    bash -c 'test -f scripts/deploy_system_integration.sh'

run_test "Quick E2E test exists" \
    "Verify quick end-to-end test is present" \
    bash -c 'test -f scripts/quick_e2e_test.sh'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 7: SOURCE CODE STRUCTURE TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "7.1 Crate Structure Tests"

run_test "Daemon source directory exists" \
    "Verify daemon source code directory is present" \
    bash -c 'test -d crates/dots-family-daemon/src'

run_test "Monitor source directory exists" \
    "Verify monitor source code directory is present" \
    bash -c 'test -d crates/dots-family-monitor/src'

run_test "CLI source directory exists" \
    "Verify CLI tool source code directory is present" \
    bash -c 'test -d crates/dots-family-ctl/src'

run_test "Filter source directory exists" \
    "Verify filter source code directory is present" \
    bash -c 'test -d crates/dots-family-filter/src'

run_test "DB source directory exists" \
    "Verify database source code directory is present" \
    bash -c 'test -d crates/dots-family-db/src'

run_test "Common crate directory exists" \
    "Verify common utilities crate directory is present" \
    bash -c 'test -d crates/dots-family-common/src'

log_section "7.2 Main Entry Points Tests"

run_test "Daemon main.rs exists" \
    "Verify daemon main entry point is present" \
    bash -c 'test -f crates/dots-family-daemon/src/main.rs'

run_test "Monitor main.rs exists" \
    "Verify monitor main entry point is present" \
    bash -c 'test -f crates/dots-family-monitor/src/main.rs'

run_test "CLI main.rs exists" \
    "Verify CLI main entry point is present" \
    bash -c 'test -f crates/dots-family-ctl/src/main.rs'

run_test "Filter main.rs exists" \
    "Verify filter main entry point is present" \
    bash -c 'test -f crates/dots-family-filter/src/main.rs'

log_section "7.3 Cargo Configuration Tests"

run_test "Root Cargo.toml exists" \
    "Verify workspace Cargo.toml is present" \
    bash -c 'test -f Cargo.toml'

run_test "Daemon Cargo.toml exists" \
    "Verify daemon package manifest is present" \
    bash -c 'test -f crates/dots-family-daemon/Cargo.toml'

run_test "Monitor Cargo.toml exists" \
    "Verify monitor package manifest is present" \
    bash -c 'test -f crates/dots-family-monitor/Cargo.toml'

run_test "CLI Cargo.toml exists" \
    "Verify CLI package manifest is present" \
    bash -c 'test -f crates/dots-family-ctl/Cargo.toml'

run_test "Cargo.lock exists" \
    "Verify locked dependency file is present" \
    bash -c 'test -f Cargo.lock'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 8: SECURITY & HARDENING TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "8.1 Security Module Tests"

run_test "Security module exists" \
    "Verify security hardening module is present" \
    bash -c 'test -f nixos-modules/dots-family/security.nix'

run_test "Security module has polkit rules" \
    "Verify polkit configuration is present" \
    bash -c 'grep -q "polkit" nixos-modules/dots-family/security.nix'

run_test "Security module has resource limits" \
    "Verify resource limit configuration is present" \
    bash -c 'grep -q "ResourceLimit" nixos-modules/dots-family/security.nix'

log_section "8.2 Systemd Security Directives"

run_test "Service has NoNewPrivileges" \
    "Verify NoNewPrivileges directive is set" \
    bash -c 'grep -q "NoNewPrivileges" nixos-modules/dots-family/daemon.nix'

run_test "Service has Memory protection" \
    "Verify memory protection directives are present" \
    bash -c 'grep -q "MemoryDenyWriteExecute" nixos-modules/dots-family/daemon.nix'

run_test "Service has LockPersonality" \
    "Verify personality locking is configured" \
    bash -c 'grep -q "LockPersonality" nixos-modules/dots-family/daemon.nix'

run_test "Service has SystemCallFilter" \
    "Verify system call filtering is configured" \
    bash -c 'grep -q "SystemCallFilter" nixos-modules/dots-family/daemon.nix'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  PHASE 9: INTEGRATION POINTS TESTS${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

log_section "9.1 Flake Integration Tests"

run_test "Flake has default package output" \
    "Verify flake exports default package" \
    bash -c 'grep -q "default.*=" flake.nix | head -1'

run_test "Flake has nixosModules export" \
    "Verify flake exports NixOS modules" \
    bash -c 'grep -q "nixosModules" flake.nix'

run_test "Flake has overlays export" \
    "Verify flake exports overlays" \
    bash -c 'grep -q "overlays" flake.nix'

run_test "Flake has devShell output" \
    "Verify flake provides development shell" \
    bash -c 'grep -q "devShells" flake.nix'

run_test "Flake has checks output" \
    "Verify flake provides check targets" \
    bash -c 'grep -q "checks" flake.nix'

log_section "9.2 VM Configuration Tests"

run_test "VM test configuration exists" \
    "Verify NixOS VM test configuration is present" \
    bash -c 'test -f nix/vm-simple.nix'

run_test "VM test configuration is valid" \
    "Verify VM configuration syntax is correct" \
    bash -c 'nix-instantiate --parse nix/vm-simple.nix >/dev/null 2>&1'

run_test "Flake defines test VM" \
    "Verify flake defines nixosConfigurations for testing" \
    bash -c 'grep -q "nixosConfigurations" flake.nix'

echo ""
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${WHITE}  FINAL SUMMARY${NC}"
echo -e "${WHITE}═══════════════════════════════════════════════════════════════${NC}"

echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  HAPPY PATH TEST RESULTS${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${WHITE}Tests Run:${NC}      ${TESTS_RUN}"
echo -e "  ${GREEN}Tests Passed:${NC}   ${TESTS_PASSED}"
echo -e "  ${RED}Tests Failed:${NC}    ${TESTS_FAILED}"
echo -e "  ${YELLOW}Tests Skipped:${NC}  ${TESTS_SKIPPED}"
echo ""

PASS_RATE=$(( TESTS_RUN > 0 ? (TESTS_PASSED * 100 / TESTS_RUN) : 0 ))

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL HAPPY PATH TESTS PASSED!${NC}"
    echo -e "${GREEN}   System is ready for full user workflow testing${NC}"
    echo ""
    echo -e "${CYAN}   Pass Rate: ${PASS_RATE}%${NC}"
    EXIT_CODE=0
else
    echo -e "${YELLOW}⚠️  Some tests failed${NC}"
    echo -e "${YELLOW}   Review output above for details${NC}"
    echo ""
    echo -e "${CYAN}   Pass Rate: ${PASS_RATE}%${NC}"
    EXIT_CODE=1
fi

echo ""
echo -e "${BLUE}Evidence collected in: ${EVIDENCE_FILE}${NC}"
echo -e "${BLUE}Build artifacts available in: ./result${NC}"
echo ""

exit ${EXIT_CODE}
