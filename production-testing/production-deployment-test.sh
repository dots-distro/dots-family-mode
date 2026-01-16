#!/run/current-system/sw/bin/bash

# DOTS Family Mode Production Deployment Test Suite
# Comprehensive testing framework for production deployment validation

set -euo pipefail

# Test configuration
TEST_REPORT="/tmp/dots-family-production-test-$(date +%Y%m%d-%H%M%S).txt"
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
CRITICAL_FAILURES=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test result tracking
declare -a FAILED_TESTS=()
declare -a CRITICAL_FAILED_TESTS=()

log_test() {
    local status="$1"
    local test_name="$2"
    local message="$3"
    local is_critical="${4:-false}"
    local timestamp=$(date +'%Y-%m-%d %H:%M:%S')
    
    case "$status" in
        "PASS")
            echo -e "${GREEN}[PASS]${NC} $test_name: $message" | tee -a "$TEST_REPORT"
            ((TESTS_PASSED++))
            ;;
        "FAIL")
            echo -e "${RED}[FAIL]${NC} $test_name: $message" | tee -a "$TEST_REPORT"
            ((TESTS_FAILED++))
            FAILED_TESTS+=("$test_name")
            if [[ "$is_critical" == "true" ]]; then
                ((CRITICAL_FAILURES++))
                CRITICAL_FAILED_TESTS+=("$test_name")
            fi
            ;;
        "SKIP")
            echo -e "${YELLOW}[SKIP]${NC} $test_name: $message" | tee -a "$TEST_REPORT"
            ((TESTS_SKIPPED++))
            ;;
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $test_name: $message" | tee -a "$TEST_REPORT"
            ;;
    esac
}

test_system_prerequisites() {
    echo "=== System Prerequisites Tests ===" | tee -a "$TEST_REPORT"
    
    # Test kernel version
    local kernel_version=$(uname -r)
    local kernel_major=$(echo "$kernel_version" | cut -d. -f1)
    local kernel_minor=$(echo "$kernel_version" | cut -d. -f2)
    
    if [[ $kernel_major -gt 4 ]] || ([[ $kernel_major -eq 4 ]] && [[ $kernel_minor -ge 4 ]]); then
        log_test "PASS" "kernel_version" "Kernel $kernel_version supports eBPF"
    else
        log_test "FAIL" "kernel_version" "Kernel $kernel_version too old for eBPF (need 4.4+)" "true"
    fi
    
    # Test memory
    local memory_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    local memory_gb=$((memory_kb / 1024 / 1024))
    
    if [[ $memory_gb -ge 2 ]]; then
        log_test "PASS" "memory_requirement" "${memory_gb}GB memory available"
    else
        log_test "FAIL" "memory_requirement" "Only ${memory_gb}GB memory (need 2GB+)" "true"
    fi
    
    # Test systemd version
    if systemctl --version >/dev/null 2>&1; then
        local systemd_version=$(systemctl --version | head -1 | awk '{print $2}')
        if [[ $systemd_version -ge 245 ]]; then
            log_test "PASS" "systemd_version" "systemd $systemd_version supports enhanced security"
        else
            log_test "FAIL" "systemd_version" "systemd $systemd_version lacks security features (need 245+)"
        fi
    else
        log_test "FAIL" "systemd_version" "systemd not available" "true"
    fi
    
    # Test DBus
    if systemctl is-active dbus.service >/dev/null 2>&1; then
        log_test "PASS" "dbus_service" "DBus service is active"
    else
        log_test "FAIL" "dbus_service" "DBus service not running" "true"
    fi
    
    # Test debugfs mount
    if mountpoint -q /sys/kernel/debug 2>/dev/null; then
        log_test "PASS" "debugfs_mount" "debugfs mounted for eBPF"
    else
        log_test "FAIL" "debugfs_mount" "debugfs not mounted (eBPF may not work)"
    fi
}

test_package_installation() {
    echo -e "\n=== Package Installation Tests ===" | tee -a "$TEST_REPORT"
    
    # Test DOTS Family binaries
    local binaries=("dots-family-daemon" "dots-family-monitor" "dots-family-ctl")
    
    for binary in "${binaries[@]}"; do
        if command -v "$binary" >/dev/null 2>&1; then
            log_test "PASS" "binary_$binary" "Binary $binary is available"
            
            # Test binary execution
            if "$binary" --version >/dev/null 2>&1 || "$binary" --help >/dev/null 2>&1; then
                log_test "PASS" "binary_${binary}_exec" "Binary $binary executes successfully"
            else
                log_test "FAIL" "binary_${binary}_exec" "Binary $binary fails to execute"
            fi
        else
            log_test "FAIL" "binary_$binary" "Binary $binary not found" "true"
        fi
    done
    
    # Test optional tools
    local optional_tools=("bpftool" "apparmor_parser" "auditctl")
    
    for tool in "${optional_tools[@]}"; do
        if command -v "$tool" >/dev/null 2>&1; then
            log_test "PASS" "tool_$tool" "Tool $tool is available"
        else
            log_test "SKIP" "tool_$tool" "Tool $tool not available (optional)"
        fi
    done
}

test_service_configuration() {
    echo -e "\n=== Service Configuration Tests ===" | tee -a "$TEST_REPORT"
    
    local services=(
        "dots-family-daemon.service"
        "dots-family-monitor@.service" 
        "dots-family-filter.service"
        "dots-family.target"
    )
    
    for service in "${services[@]}"; do
        # Test service file exists
        if systemctl cat "$service" >/dev/null 2>&1; then
            log_test "PASS" "service_${service%.*}_exists" "Service $service configuration exists"
            
            # Test security hardening in service
            local service_config=$(systemctl cat "$service" 2>/dev/null)
            
            # Check for security features
            if echo "$service_config" | grep -q "ProtectSystem="; then
                log_test "PASS" "service_${service%.*}_protect_system" "ProtectSystem configured"
            else
                log_test "FAIL" "service_${service%.*}_protect_system" "ProtectSystem not configured"
            fi
            
            if echo "$service_config" | grep -q "CapabilityBoundingSet="; then
                log_test "PASS" "service_${service%.*}_capabilities" "Capability restrictions configured"
            else
                log_test "FAIL" "service_${service%.*}_capabilities" "No capability restrictions"
            fi
            
            # Test service can load
            if systemd-analyze verify "$service" >/dev/null 2>&1; then
                log_test "PASS" "service_${service%.*}_valid" "Service configuration is valid"
            else
                log_test "FAIL" "service_${service%.*}_valid" "Service configuration invalid" "true"
            fi
            
        else
            log_test "FAIL" "service_${service%.*}_exists" "Service $service not found" "true"
        fi
    done
}

test_dbus_configuration() {
    echo -e "\n=== DBus Configuration Tests ===" | tee -a "$TEST_REPORT"
    
    # Test system bus policy
    if [[ -f "/etc/dbus-1/system.d/org.dots.FamilyDaemon.conf" ]]; then
        log_test "PASS" "dbus_system_policy" "System DBus policy file exists"
        
        # Test policy syntax
        if xmllint --noout /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf 2>/dev/null; then
            log_test "PASS" "dbus_system_policy_syntax" "System DBus policy syntax valid"
        elif python3 -c "import xml.etree.ElementTree; xml.etree.ElementTree.parse('/etc/dbus-1/system.d/org.dots.FamilyDaemon.conf')" 2>/dev/null; then
            log_test "PASS" "dbus_system_policy_syntax" "System DBus policy syntax valid (python)"
        else
            log_test "FAIL" "dbus_system_policy_syntax" "System DBus policy syntax invalid"
        fi
        
        # Test for security features
        if grep -q "context=\"default\"" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf && \
           grep -q "<deny" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf; then
            log_test "PASS" "dbus_system_policy_security" "Default deny policy configured"
        else
            log_test "FAIL" "dbus_system_policy_security" "No default deny policy"
        fi
    else
        log_test "FAIL" "dbus_system_policy" "System DBus policy file missing" "true"
    fi
    
    # Test session bus policy
    if [[ -f "/etc/dbus-1/session.d/org.dots.FamilyMonitor.conf" ]]; then
        log_test "PASS" "dbus_session_policy" "Session DBus policy file exists"
    else
        log_test "FAIL" "dbus_session_policy" "Session DBus policy file missing"
    fi
    
    # Test service activation file
    if [[ -f "/usr/share/dbus-1/system-services/org.dots.FamilyDaemon.service" ]] || \
       [[ -f "/usr/share/dbus-1/services/org.dots.FamilyDaemon.service" ]]; then
        log_test "PASS" "dbus_activation" "DBus service activation configured"
    else
        log_test "FAIL" "dbus_activation" "DBus service activation missing"
    fi
}

test_user_groups() {
    echo -e "\n=== User Group Tests ===" | tee -a "$TEST_REPORT"
    
    local required_groups=("dots-family" "dots-parents")
    
    for group in "${required_groups[@]}"; do
        if getent group "$group" >/dev/null 2>&1; then
            log_test "PASS" "group_$group" "Group $group exists"
        else
            log_test "FAIL" "group_$group" "Group $group missing" "true"
        fi
    done
    
    # Test monitor user
    if id dots-monitor >/dev/null 2>&1; then
        log_test "PASS" "user_monitor" "Monitor user exists"
    else
        log_test "FAIL" "user_monitor" "Monitor user missing"
    fi
}

test_database_setup() {
    echo -e "\n=== Database Configuration Tests ===" | tee -a "$TEST_REPORT"
    
    # Test database directory
    local db_dir="/var/lib/dots-family"
    if [[ -d "$db_dir" ]]; then
        log_test "PASS" "database_directory" "Database directory exists"
        
        # Test permissions
        local perms=$(stat -c "%a" "$db_dir")
        if [[ "$perms" == "750" ]]; then
            log_test "PASS" "database_directory_perms" "Database directory permissions correct (750)"
        else
            log_test "FAIL" "database_directory_perms" "Database directory permissions incorrect: $perms"
        fi
        
        # Test ownership
        local owner=$(stat -c "%U:%G" "$db_dir")
        if [[ "$owner" =~ ^(root|dots-family):(root|dots-family|dots-family-parents)$ ]]; then
            log_test "PASS" "database_directory_owner" "Database directory ownership correct: $owner"
        else
            log_test "FAIL" "database_directory_owner" "Database directory ownership incorrect: $owner"
        fi
    else
        log_test "FAIL" "database_directory" "Database directory missing" "true"
    fi
    
    # Test SQLite availability
    if command -v sqlite3 >/dev/null 2>&1; then
        log_test "PASS" "sqlite_available" "SQLite3 is available"
        
        # Test SQLCipher (if available)
        if sqlite3 ":memory:" "PRAGMA cipher_version;" 2>/dev/null | grep -q "^[0-9]"; then
            log_test "PASS" "sqlcipher_available" "SQLCipher encryption available"
        else
            log_test "SKIP" "sqlcipher_available" "SQLCipher not available (using standard SQLite)"
        fi
    else
        log_test "FAIL" "sqlite_available" "SQLite3 not available" "true"
    fi
}

test_security_hardening() {
    echo -e "\n=== Security Hardening Tests ===" | tee -a "$TEST_REPORT"
    
    # Test AppArmor
    if command -v aa-status >/dev/null 2>&1; then
        if aa-enabled 2>/dev/null; then
            log_test "PASS" "apparmor_enabled" "AppArmor is enabled"
            
            # Test for DOTS Family profiles
            local aa_profiles=$(aa-status 2>/dev/null | grep dots-family | wc -l || echo 0)
            if [[ $aa_profiles -gt 0 ]]; then
                log_test "PASS" "apparmor_profiles" "$aa_profiles DOTS Family AppArmor profiles loaded"
            else
                log_test "SKIP" "apparmor_profiles" "No DOTS Family AppArmor profiles (optional)"
            fi
        else
            log_test "SKIP" "apparmor_enabled" "AppArmor not enabled (optional)"
        fi
    else
        log_test "SKIP" "apparmor_enabled" "AppArmor not available (optional)"
    fi
    
    # Test audit system
    if systemctl is-active auditd.service >/dev/null 2>&1; then
        log_test "PASS" "audit_enabled" "Audit daemon is active"
        
        if auditctl -l 2>/dev/null | grep -q dots-family; then
            log_test "PASS" "audit_rules" "DOTS Family audit rules configured"
        else
            log_test "SKIP" "audit_rules" "No DOTS Family audit rules (optional)"
        fi
    else
        log_test "SKIP" "audit_enabled" "Audit daemon not active (optional)"
    fi
    
    # Test sysctl security parameters
    local security_params=(
        "kernel.dmesg_restrict:1"
        "kernel.yama.ptrace_scope:1,2"
        "net.ipv4.ip_forward:0"
    )
    
    for param_check in "${security_params[@]}"; do
        local param="${param_check%%:*}"
        local expected="${param_check##*:}"
        local actual=$(sysctl -n "$param" 2>/dev/null || echo "unknown")
        
        if [[ "$expected" =~ $actual ]]; then
            log_test "PASS" "sysctl_$param" "$param correctly configured ($actual)"
        elif [[ "$actual" == "unknown" ]]; then
            log_test "SKIP" "sysctl_$param" "$param not available"
        else
            log_test "FAIL" "sysctl_$param" "$param incorrectly set: $actual (expected $expected)"
        fi
    done
}

test_ebpf_functionality() {
    echo -e "\n=== eBPF Functionality Tests ===" | tee -a "$TEST_REPORT"
    
    # Test eBPF syscall availability
    if [[ -f "/proc/sys/kernel/unprivileged_bpf_disabled" ]]; then
        log_test "PASS" "ebpf_syscall" "eBPF syscall interface available"
        
        local bpf_status=$(cat /proc/sys/kernel/unprivileged_bpf_disabled)
        log_test "INFO" "ebpf_status" "Unprivileged eBPF status: $bpf_status"
    else
        log_test "FAIL" "ebpf_syscall" "eBPF syscall interface not available"
    fi
    
    # Test JIT compilation
    if [[ -f "/proc/sys/net/core/bpf_jit_enable" ]]; then
        local jit_status=$(cat /proc/sys/net/core/bpf_jit_enable)
        if [[ $jit_status -ge 1 ]]; then
            log_test "PASS" "ebpf_jit" "eBPF JIT compilation enabled ($jit_status)"
        else
            log_test "FAIL" "ebpf_jit" "eBPF JIT compilation disabled"
        fi
    else
        log_test "FAIL" "ebpf_jit" "eBPF JIT interface not available"
    fi
    
    # Test bpftool
    if command -v bpftool >/dev/null 2>&1; then
        log_test "PASS" "bpftool_available" "bpftool is available"
        
        # Test listing programs (requires root)
        if [[ $EUID -eq 0 ]]; then
            if bpftool prog list >/dev/null 2>&1; then
                local prog_count=$(bpftool prog list 2>/dev/null | wc -l)
                log_test "PASS" "bpftool_functional" "bpftool functional ($prog_count programs)"
            else
                log_test "FAIL" "bpftool_functional" "bpftool cannot list programs"
            fi
        else
            log_test "SKIP" "bpftool_functional" "bpftool test requires root privileges"
        fi
    else
        log_test "SKIP" "bpftool_available" "bpftool not available (optional)"
    fi
}

test_network_configuration() {
    echo -e "\n=== Network Configuration Tests ===" | tee -a "$TEST_REPORT"
    
    # Test network interfaces
    local interfaces=$(ip link show | grep "^[0-9]" | wc -l)
    log_test "INFO" "network_interfaces" "$interfaces network interfaces detected"
    
    # Test localhost connectivity
    if ping -c 1 127.0.0.1 >/dev/null 2>&1; then
        log_test "PASS" "localhost_connectivity" "Localhost connectivity works"
    else
        log_test "FAIL" "localhost_connectivity" "Localhost connectivity broken" "true"
    fi
    
    # Test DNS resolution
    if nslookup localhost >/dev/null 2>&1 || getent hosts localhost >/dev/null 2>&1; then
        log_test "PASS" "dns_resolution" "DNS resolution functional"
    else
        log_test "FAIL" "dns_resolution" "DNS resolution not working"
    fi
    
    # Test firewall configuration
    if command -v ufw >/dev/null 2>&1; then
        local ufw_status=$(ufw status 2>/dev/null | head -1 || echo "unknown")
        log_test "INFO" "firewall_ufw" "UFW firewall: $ufw_status"
    elif command -v firewalld >/dev/null 2>&1; then
        if systemctl is-active firewalld.service >/dev/null 2>&1; then
            log_test "INFO" "firewall_firewalld" "firewalld is active"
        else
            log_test "INFO" "firewall_firewalld" "firewalld is inactive"
        fi
    else
        log_test "INFO" "firewall_status" "No common firewall detected"
    fi
}

test_service_startup() {
    echo -e "\n=== Service Startup Tests ===" | tee -a "$TEST_REPORT"
    
    # Test daemon service startup simulation
    if [[ $EUID -eq 0 ]]; then
        # Actually test service startup
        if systemctl start dots-family-daemon.service 2>/dev/null; then
            log_test "PASS" "daemon_startup" "Daemon service starts successfully"
            
            # Test service status
            if systemctl is-active dots-family-daemon.service >/dev/null 2>&1; then
                log_test "PASS" "daemon_active" "Daemon service is active"
            else
                log_test "FAIL" "daemon_active" "Daemon service not active after start"
            fi
            
            # Stop service for cleanup
            systemctl stop dots-family-daemon.service 2>/dev/null || true
        else
            log_test "FAIL" "daemon_startup" "Daemon service failed to start" "true"
        fi
    else
        # Simulate startup testing without root
        if systemd-analyze verify dots-family-daemon.service 2>/dev/null; then
            log_test "PASS" "daemon_startup_sim" "Daemon service configuration valid (simulated)"
        else
            log_test "FAIL" "daemon_startup_sim" "Daemon service configuration invalid"
        fi
        
        log_test "SKIP" "daemon_startup" "Service startup test requires root privileges"
    fi
}

run_integration_tests() {
    echo -e "\n=== Integration Tests ===" | tee -a "$TEST_REPORT"
    
    # Test CLI tool basic functionality
    if command -v dots-family-ctl >/dev/null 2>&1; then
        # Test help output
        if dots-family-ctl --help >/dev/null 2>&1; then
            log_test "PASS" "cli_help" "CLI help output works"
        else
            log_test "FAIL" "cli_help" "CLI help output failed"
        fi
        
        # Test status command (may fail if daemon not running)
        if dots-family-ctl status >/dev/null 2>&1; then
            log_test "PASS" "cli_status" "CLI status command works"
        else
            log_test "SKIP" "cli_status" "CLI status command failed (daemon may not be running)"
        fi
    else
        log_test "SKIP" "cli_help" "CLI tool not available for testing"
        log_test "SKIP" "cli_status" "CLI tool not available for testing"
    fi
}

generate_test_report() {
    echo -e "\n=== PRODUCTION DEPLOYMENT TEST SUMMARY ===" | tee -a "$TEST_REPORT"
    echo "Test completed: $(date)" | tee -a "$TEST_REPORT"
    echo "Report file: $TEST_REPORT" | tee -a "$TEST_REPORT"
    echo "" | tee -a "$TEST_REPORT"
    echo "Test Results:" | tee -a "$TEST_REPORT"
    echo "  Passed:  $TESTS_PASSED" | tee -a "$TEST_REPORT"
    echo "  Failed:  $TESTS_FAILED" | tee -a "$TEST_REPORT"
    echo "  Skipped: $TESTS_SKIPPED" | tee -a "$TEST_REPORT"
    echo "  Critical Failures: $CRITICAL_FAILURES" | tee -a "$TEST_REPORT"
    echo "" | tee -a "$TEST_REPORT"
    
    local total_tests=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
    local success_rate=0
    if [[ $total_tests -gt 0 ]]; then
        success_rate=$(( (TESTS_PASSED * 100) / total_tests ))
    fi
    
    echo "Success Rate: $success_rate%" | tee -a "$TEST_REPORT"
    echo "" | tee -a "$TEST_REPORT"
    
    if [[ $CRITICAL_FAILURES -gt 0 ]]; then
        echo -e "${RED}DEPLOYMENT STATUS: CRITICAL FAILURES DETECTED${NC}" | tee -a "$TEST_REPORT"
        echo "The following critical tests failed:" | tee -a "$TEST_REPORT"
        for test in "${CRITICAL_FAILED_TESTS[@]}"; do
            echo "  - $test" | tee -a "$TEST_REPORT"
        done
        echo "" | tee -a "$TEST_REPORT"
        echo "DO NOT DEPLOY TO PRODUCTION until these issues are resolved." | tee -a "$TEST_REPORT"
        return 3
    elif [[ $TESTS_FAILED -gt 5 ]]; then
        echo -e "${RED}DEPLOYMENT STATUS: TOO MANY FAILURES${NC}" | tee -a "$TEST_REPORT"
        echo "Consider addressing failures before production deployment." | tee -a "$TEST_REPORT"
        return 2
    elif [[ $success_rate -lt 80 ]]; then
        echo -e "${YELLOW}DEPLOYMENT STATUS: LOW SUCCESS RATE${NC}" | tee -a "$TEST_REPORT"
        echo "Consider improving test coverage and addressing failures." | tee -a "$TEST_REPORT"
        return 1
    else
        echo -e "${GREEN}DEPLOYMENT STATUS: READY FOR PRODUCTION${NC}" | tee -a "$TEST_REPORT"
        echo "All critical tests passed. System is ready for deployment." | tee -a "$TEST_REPORT"
        return 0
    fi
}

main() {
    echo "DOTS Family Mode Production Deployment Tests" | tee "$TEST_REPORT"
    echo "============================================" | tee -a "$TEST_REPORT"
    echo "Started: $(date)" | tee -a "$TEST_REPORT"
    echo "System: $(uname -a)" | tee -a "$TEST_REPORT"
    if [[ $EUID -eq 0 ]]; then
        echo "Running as: root (full testing enabled)" | tee -a "$TEST_REPORT"
    else
        echo "Running as: $(whoami) (limited testing)" | tee -a "$TEST_REPORT"
    fi
    echo "" | tee -a "$TEST_REPORT"
    
    # Run all test suites
    test_system_prerequisites
    test_package_installation
    test_service_configuration
    test_dbus_configuration
    test_user_groups
    test_database_setup
    test_security_hardening
    test_ebpf_functionality
    test_network_configuration
    test_service_startup
    run_integration_tests
    
    # Generate final report
    generate_test_report
    local exit_code=$?
    
    echo ""
    echo "Full test report saved to: $TEST_REPORT"
    
    if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
        echo ""
        echo "Failed tests summary:"
        for test in "${FAILED_TESTS[@]}"; do
            echo "  - $test"
        done
    fi
    
    return $exit_code
}

# Parse command line arguments
FORCE_ROOT=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --force-root)
            FORCE_ROOT=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --force-root  Allow running as non-root (limited testing)"
            echo "  --verbose     Enable verbose output"
            echo "  --help        Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check for root privileges
if [[ $EUID -ne 0 ]] && [[ "$FORCE_ROOT" != "true" ]]; then
    echo "Warning: Running as non-root user. Some tests will be skipped."
    echo "For complete testing, run as root or use --force-root flag."
    echo ""
fi

main "$@"