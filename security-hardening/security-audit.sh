#!/run/current-system/sw/bin/bash

# DOTS Family Security Audit Tool
# Comprehensive security assessment and compliance checking

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

AUDIT_REPORT="/tmp/dots-family-security-audit-$(date +%Y%m%d-%H%M%S).txt"
CRITICAL_ISSUES=0
HIGH_ISSUES=0
MEDIUM_ISSUES=0
LOW_ISSUES=0

log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date +'%Y-%m-%d %H:%M:%S')
    
    case "$level" in
        "CRITICAL")
            echo -e "${RED}[CRITICAL]${NC} $message" | tee -a "$AUDIT_REPORT"
            ((CRITICAL_ISSUES++))
            ;;
        "HIGH")
            echo -e "${RED}[HIGH]${NC} $message" | tee -a "$AUDIT_REPORT"
            ((HIGH_ISSUES++))
            ;;
        "MEDIUM")
            echo -e "${YELLOW}[MEDIUM]${NC} $message" | tee -a "$AUDIT_REPORT"
            ((MEDIUM_ISSUES++))
            ;;
        "LOW")
            echo -e "${YELLOW}[LOW]${NC} $message" | tee -a "$AUDIT_REPORT"
            ((LOW_ISSUES++))
            ;;
        "PASS")
            echo -e "${GREEN}[PASS]${NC} $message" | tee -a "$AUDIT_REPORT"
            ;;
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $message" | tee -a "$AUDIT_REPORT"
            ;;
    esac
}

audit_systemd_hardening() {
    echo "=== Systemd Service Hardening Audit ===" | tee -a "$AUDIT_REPORT"
    
    local services=("dots-family-daemon.service" "dots-family-monitor@.service" "dots-family-filter.service")
    
    for service in "${services[@]}"; do
        if systemctl list-unit-files | grep -q "$service"; then
            # Check security features
            local config=$(systemctl show "$service" 2>/dev/null || echo "")
            
            # Critical security checks
            if echo "$config" | grep -q "ProtectSystem=strict"; then
                log "PASS" "$service: ProtectSystem=strict enabled"
            else
                log "HIGH" "$service: ProtectSystem not set to strict"
            fi
            
            if echo "$config" | grep -q "PrivateTmp=yes"; then
                log "PASS" "$service: PrivateTmp enabled"
            else
                log "MEDIUM" "$service: PrivateTmp not enabled"
            fi
            
            if echo "$config" | grep -q "NoNewPrivileges=yes"; then
                log "PASS" "$service: NoNewPrivileges enabled"
            else
                log "HIGH" "$service: NoNewPrivileges not enabled"
            fi
            
            # Check capability restrictions
            local caps=$(echo "$config" | grep "CapabilityBoundingSet=" || echo "none")
            if [[ "$caps" != "none" ]]; then
                log "PASS" "$service: Capability restrictions configured"
            else
                log "CRITICAL" "$service: No capability restrictions"
            fi
            
        else
            log "LOW" "$service: Service not installed"
        fi
    done
}

audit_dbus_security() {
    echo -e "\n=== DBus Security Policy Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check system bus policy
    if [[ -f "/etc/dbus-1/system.d/org.dots.FamilyDaemon.conf" ]]; then
        log "PASS" "System DBus policy file exists"
        
        # Check for default deny policy
        if grep -q "context=\"default\"" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf && \
           grep -q "<deny" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf; then
            log "PASS" "Default deny policy configured"
        else
            log "CRITICAL" "No default deny policy found"
        fi
        
        # Check for role-based access
        if grep -q "group=\"dots-parents\"" /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf; then
            log "PASS" "Role-based access control configured"
        else
            log "HIGH" "No role-based access control"
        fi
        
    else
        log "CRITICAL" "System DBus policy file missing"
    fi
    
    # Check session bus policy
    if [[ -f "/etc/dbus-1/session.d/org.dots.FamilyMonitor.conf" ]]; then
        log "PASS" "Session DBus policy file exists"
    else
        log "HIGH" "Session DBus policy file missing"
    fi
}

audit_user_groups() {
    echo -e "\n=== User Group Security Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check required groups exist
    for group in dots-family dots-parents; do
        if getent group "$group" >/dev/null 2>&1; then
            log "PASS" "Group $group exists"
            
            # Check group membership
            local members=$(getent group "$group" | cut -d: -f4)
            if [[ -n "$members" ]]; then
                log "INFO" "Group $group members: $members"
                
                # Validate parent group is not empty for production
                if [[ "$group" == "dots-parents" ]]; then
                    log "PASS" "Parent group has members"
                fi
            else
                if [[ "$group" == "dots-parents" ]]; then
                    log "CRITICAL" "Parent group has no members"
                else
                    log "INFO" "Group $group is empty"
                fi
            fi
        else
            log "CRITICAL" "Required group $group missing"
        fi
    done
    
    # Check for unauthorized sudo access
    if [[ -f "/etc/sudoers.d/dots-family" ]]; then
        log "INFO" "Custom sudo rules found, reviewing..."
        # Check for overly permissive rules
        if grep -q "NOPASSWD.*ALL" /etc/sudoers.d/dots-family 2>/dev/null; then
            log "HIGH" "Overly permissive sudo rules detected"
        else
            log "PASS" "Sudo rules appear restricted"
        fi
    fi
}

audit_file_permissions() {
    echo -e "\n=== File Permissions Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Database directory permissions
    if [[ -d "/var/lib/dots-family" ]]; then
        local perm=$(stat -c "%a" /var/lib/dots-family)
        if [[ "$perm" == "750" ]]; then
            log "PASS" "Database directory permissions correct (750)"
        else
            log "HIGH" "Database directory permissions incorrect: $perm (should be 750)"
        fi
        
        # Database file permissions
        for db_file in /var/lib/dots-family/*.db; do
            if [[ -f "$db_file" ]]; then
                local db_perm=$(stat -c "%a" "$db_file")
                if [[ "$db_perm" == "640" ]]; then
                    log "PASS" "Database file permissions correct (640)"
                else
                    log "HIGH" "Database file $db_file permissions incorrect: $db_perm (should be 640)"
                fi
            fi
        done
    else
        log "MEDIUM" "Database directory does not exist"
    fi
    
    # Configuration file permissions
    if [[ -d "/etc/dots-family" ]]; then
        find /etc/dots-family -type f -exec stat -c "%n %a" {} \; | while read file perm; do
            if [[ "$perm" -le "644" ]]; then
                log "PASS" "Config file $file permissions OK ($perm)"
            else
                log "MEDIUM" "Config file $file overly permissive: $perm"
            fi
        done
    fi
    
    # Log directory permissions
    if [[ -d "/var/log/dots-family" ]]; then
        local log_perm=$(stat -c "%a" /var/log/dots-family)
        if [[ "$log_perm" == "750" ]]; then
            log "PASS" "Log directory permissions correct (750)"
        else
            log "MEDIUM" "Log directory permissions: $log_perm (should be 750)"
        fi
    fi
}

audit_ebpf_security() {
    echo -e "\n=== eBPF Security Configuration Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check eBPF sysctl settings
    local unprivileged_bpf=$(sysctl -n kernel.unprivileged_bpf_disabled 2>/dev/null || echo "unknown")
    case "$unprivileged_bpf" in
        "1"|"2")
            log "PASS" "Unprivileged eBPF disabled ($unprivileged_bpf)"
            ;;
        "0")
            log "MEDIUM" "Unprivileged eBPF enabled (may be needed for monitoring)"
            ;;
        *)
            log "MEDIUM" "eBPF status unknown"
            ;;
    esac
    
    # Check JIT hardening
    local jit_harden=$(sysctl -n net.core.bpf_jit_harden 2>/dev/null || echo "unknown")
    if [[ "$jit_harden" -ge "1" ]]; then
        log "PASS" "eBPF JIT hardening enabled ($jit_harden)"
    else
        log "MEDIUM" "eBPF JIT hardening not optimal: $jit_harden"
    fi
    
    # Check for loaded eBPF programs
    if command -v bpftool >/dev/null 2>&1; then
        local bpf_progs=$(bpftool prog list 2>/dev/null | wc -l || echo 0)
        log "INFO" "$bpf_progs eBPF programs currently loaded"
        
        # Check for DOTS Family programs
        local dots_progs=$(bpftool prog list 2>/dev/null | grep -c dots-family || echo 0)
        if [[ "$dots_progs" -gt 0 ]]; then
            log "PASS" "$dots_progs DOTS Family eBPF programs active"
        else
            log "LOW" "No DOTS Family eBPF programs detected"
        fi
    else
        log "LOW" "bpftool not available for eBPF inspection"
    fi
}

audit_apparmor_profiles() {
    echo -e "\n=== AppArmor Security Profile Audit ===" | tee -a "$AUDIT_REPORT"
    
    if command -v aa-status >/dev/null 2>&1; then
        # Check if AppArmor is active
        if aa-enabled 2>/dev/null; then
            log "PASS" "AppArmor is active"
            
            # Check for DOTS Family profiles
            local loaded_profiles=$(aa-status 2>/dev/null | grep dots-family | wc -l || echo 0)
            if [[ "$loaded_profiles" -gt 0 ]]; then
                log "PASS" "$loaded_profiles DOTS Family AppArmor profiles loaded"
                
                # List profile status
                aa-status 2>/dev/null | grep dots-family | while read profile; do
                    log "INFO" "Profile: $profile"
                done
            else
                log "MEDIUM" "No DOTS Family AppArmor profiles loaded"
            fi
            
            # Check for complain mode profiles
            local complain_profiles=$(aa-status 2>/dev/null | grep "complain" | grep dots-family | wc -l || echo 0)
            if [[ "$complain_profiles" -gt 0 ]]; then
                log "MEDIUM" "$complain_profiles DOTS Family profiles in complain mode"
            fi
            
        else
            log "MEDIUM" "AppArmor not active"
        fi
    else
        log "LOW" "AppArmor not available"
    fi
}

audit_network_security() {
    echo -e "\n=== Network Security Configuration Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check critical sysctl parameters
    local params=(
        "net.ipv4.ip_forward:0"
        "net.ipv4.conf.all.accept_redirects:0"
        "net.ipv4.conf.all.send_redirects:0"
        "net.ipv6.conf.all.forwarding:0"
    )
    
    for param_check in "${params[@]}"; do
        local param="${param_check%%:*}"
        local expected="${param_check##*:}"
        local actual=$(sysctl -n "$param" 2>/dev/null || echo "unknown")
        
        if [[ "$actual" == "$expected" ]]; then
            log "PASS" "$param correctly set to $actual"
        elif [[ "$actual" == "unknown" ]]; then
            log "LOW" "$param not available"
        else
            log "MEDIUM" "$param set to $actual (expected $expected)"
        fi
    done
    
    # Check for listening services
    if command -v ss >/dev/null 2>&1; then
        local listening=$(ss -tlnp | grep dots-family | wc -l || echo 0)
        log "INFO" "$listening DOTS Family services listening on network"
    fi
}

audit_logging_security() {
    echo -e "\n=== Logging and Monitoring Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check audit daemon
    if systemctl is-active auditd.service >/dev/null 2>&1; then
        log "PASS" "Audit daemon is active"
        
        # Check for DOTS Family audit rules
        if auditctl -l 2>/dev/null | grep -q dots-family; then
            log "PASS" "DOTS Family audit rules configured"
        else
            log "MEDIUM" "No DOTS Family audit rules found"
        fi
    else
        log "MEDIUM" "Audit daemon not active"
    fi
    
    # Check log rotation
    if [[ -f "/etc/logrotate.d/dots-family" ]]; then
        log "PASS" "Log rotation configured"
    else
        log "LOW" "Log rotation not configured"
    fi
    
    # Check security monitoring
    if systemctl list-timers | grep -q dots-family-security-monitor; then
        log "PASS" "Security monitoring timer active"
    else
        log "LOW" "Security monitoring not configured"
    fi
}

audit_process_security() {
    echo -e "\n=== Process Security Audit ===" | tee -a "$AUDIT_REPORT"
    
    # Check if daemon is running as correct user
    if pgrep -f dots-family-daemon >/dev/null; then
        local daemon_user=$(ps -o user= -p $(pgrep -f dots-family-daemon) | head -1)
        if [[ "$daemon_user" == "root" ]]; then
            log "PASS" "Daemon running as root (required for eBPF)"
        else
            log "HIGH" "Daemon running as $daemon_user (should be root for eBPF)"
        fi
    else
        log "CRITICAL" "DOTS Family daemon not running"
    fi
    
    # Check for security features in running processes
    if [[ -d "/proc/$(pgrep -f dots-family-daemon)" ]]; then
        local proc_dir="/proc/$(pgrep -f dots-family-daemon)"
        
        # Check NoNewPrivs
        if grep -q "NoNewPrivs.*1" "$proc_dir/status" 2>/dev/null; then
            log "PASS" "NoNewPrivileges active for daemon"
        else
            log "MEDIUM" "NoNewPrivileges not detected"
        fi
        
        # Check capability bounding set
        local caps=$(grep "CapBnd" "$proc_dir/status" 2>/dev/null | awk '{print $2}' || echo "unknown")
        if [[ "$caps" != "0000003fffffffff" ]] && [[ "$caps" != "unknown" ]]; then
            log "PASS" "Capability bounding set restricted"
        else
            log "MEDIUM" "Capability bounding set check inconclusive"
        fi
    fi
}

generate_summary_report() {
    echo -e "\n=== SECURITY AUDIT SUMMARY ===" | tee -a "$AUDIT_REPORT"
    echo "Audit completed: $(date)" | tee -a "$AUDIT_REPORT"
    echo "Report file: $AUDIT_REPORT" | tee -a "$AUDIT_REPORT"
    echo "" | tee -a "$AUDIT_REPORT"
    echo "Issue Summary:" | tee -a "$AUDIT_REPORT"
    echo "  Critical: $CRITICAL_ISSUES" | tee -a "$AUDIT_REPORT"
    echo "  High:     $HIGH_ISSUES" | tee -a "$AUDIT_REPORT"
    echo "  Medium:   $MEDIUM_ISSUES" | tee -a "$AUDIT_REPORT"
    echo "  Low:      $LOW_ISSUES" | tee -a "$AUDIT_REPORT"
    echo "" | tee -a "$AUDIT_REPORT"
    
    local total_issues=$((CRITICAL_ISSUES + HIGH_ISSUES + MEDIUM_ISSUES + LOW_ISSUES))
    
    if [[ $CRITICAL_ISSUES -gt 0 ]]; then
        echo -e "${RED}SECURITY STATUS: CRITICAL ISSUES DETECTED${NC}" | tee -a "$AUDIT_REPORT"
        echo "Immediate action required before production deployment" | tee -a "$AUDIT_REPORT"
        return 3
    elif [[ $HIGH_ISSUES -gt 0 ]]; then
        echo -e "${RED}SECURITY STATUS: HIGH RISK${NC}" | tee -a "$AUDIT_REPORT"
        echo "Address high-priority issues before production" | tee -a "$AUDIT_REPORT"
        return 2
    elif [[ $MEDIUM_ISSUES -gt 3 ]]; then
        echo -e "${YELLOW}SECURITY STATUS: MODERATE RISK${NC}" | tee -a "$AUDIT_REPORT"
        echo "Consider addressing medium-priority issues" | tee -a "$AUDIT_REPORT"
        return 1
    else
        echo -e "${GREEN}SECURITY STATUS: ACCEPTABLE${NC}" | tee -a "$AUDIT_REPORT"
        echo "System meets security requirements for production" | tee -a "$AUDIT_REPORT"
        return 0
    fi
}

main() {
    echo "DOTS Family Mode Security Audit" | tee "$AUDIT_REPORT"
    echo "===============================" | tee -a "$AUDIT_REPORT"
    echo "Started: $(date)" | tee -a "$AUDIT_REPORT"
    echo "System: $(uname -a)" | tee -a "$AUDIT_REPORT"
    echo "" | tee -a "$AUDIT_REPORT"
    
    audit_systemd_hardening
    audit_dbus_security
    audit_user_groups
    audit_file_permissions
    audit_ebpf_security
    audit_apparmor_profiles
    audit_network_security
    audit_logging_security
    audit_process_security
    
    generate_summary_report
    local exit_code=$?
    
    echo ""
    echo "Full audit report saved to: $AUDIT_REPORT"
    
    return $exit_code
}

main "$@"