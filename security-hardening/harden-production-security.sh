#!/run/current-system/sw/bin/bash

# DOTS Family Mode Security Hardening Script
# Applies additional security measures for production deployment

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root (use sudo)"
        exit 1
    fi
}

harden_systemd_services() {
    log "Hardening systemd services..."
    
    # Create override directories
    mkdir -p /etc/systemd/system/dots-family-daemon.service.d
    mkdir -p /etc/systemd/system/dots-family-monitor@.service.d
    mkdir -p /etc/systemd/system/dots-family-filter.service.d
    
    # Additional hardening for daemon
    cat > /etc/systemd/system/dots-family-daemon.service.d/99-hardening.conf << 'EOF'
[Service]
# Additional security hardening
ProtectKernelLogs = true
ProtectClock = true
ProtectHostname = true
RestrictRealtime = true
RestrictSUIDSGID = true
RemoveIPC = true
SystemCallArchitectures = native

# Restrict system calls
SystemCallFilter = @system-service
SystemCallFilter = ~@debug @mount @cpu-emulation @obsolete @privileged @reboot @swap

# Additional filesystem restrictions
TemporaryFileSystem = /var/tmp
BindReadOnlyPaths = /usr/lib /usr/share
PrivateDevices = true

# Resource limits
LimitNOFILE = 1024
LimitNPROC = 100
MemoryMax = 512M
TasksMax = 50

# Network hardening
IPAddressDeny = any
IPAddressAllow = localhost
IPAddressAllow = link-local
IPAddressAllow = multicast
EOF

    # Additional hardening for monitor
    cat > /etc/systemd/system/dots-family-monitor@.service.d/99-hardening.conf << 'EOF'
[Service]
# Additional security hardening
ProtectKernelLogs = true
ProtectClock = true
ProtectHostname = true
RestrictRealtime = true
RestrictSUIDSGID = true
RemoveIPC = true

# System call restrictions
SystemCallFilter = @system-service
SystemCallFilter = ~@debug @mount @cpu-emulation @obsolete @privileged @reboot @swap

# Resource limits
LimitNOFILE = 512
LimitNPROC = 50
MemoryMax = 256M
TasksMax = 25

# Additional filesystem restrictions
PrivateDevices = true
ProtectKernelModules = true
ProtectKernelTunables = true
EOF

    # Additional hardening for filter
    cat > /etc/systemd/system/dots-family-filter.service.d/99-hardening.conf << 'EOF'
[Service]
# Additional security hardening
ProtectKernelLogs = true
ProtectClock = true
ProtectHostname = true
RestrictRealtime = true
RestrictSUIDSGID = true
RemoveIPC = true

# System call restrictions  
SystemCallFilter = @system-service @network-io
SystemCallFilter = ~@debug @mount @cpu-emulation @obsolete @reboot @swap

# Resource limits
LimitNOFILE = 2048
LimitNPROC = 100
MemoryMax = 1G
TasksMax = 100

# Network restrictions for proxy
RestrictAddressFamilies = AF_INET AF_INET6 AF_UNIX
EOF

    systemctl daemon-reload
    success "Systemd service hardening applied"
}

configure_apparmor_profiles() {
    log "Configuring AppArmor profiles..."
    
    if ! command -v apparmor_parser >/dev/null 2>&1; then
        warn "AppArmor not available, skipping profile installation"
        return 0
    fi
    
    mkdir -p /etc/apparmor.d/dots-family
    
    # AppArmor profile for daemon
    cat > /etc/apparmor.d/dots-family.daemon << 'EOF'
#include <tunables/global>

/usr/bin/dots-family-daemon {
  #include <abstractions/base>
  #include <abstractions/nameservice>
  #include <abstractions/dbus-session-strict>
  #include <abstractions/dbus-strict>

  # Binary execution
  /usr/bin/dots-family-daemon mr,

  # Configuration and data
  /var/lib/dots-family/ r,
  /var/lib/dots-family/** rw,
  /etc/dots-family/ r,
  /etc/dots-family/** r,

  # Logging
  /var/log/dots-family/ r,
  /var/log/dots-family/** rw,

  # System monitoring (eBPF)
  /sys/kernel/debug/tracing/ r,
  /sys/kernel/debug/tracing/** r,
  /proc/*/stat r,
  /proc/*/cmdline r,
  /proc/*/comm r,
  /proc/sys/kernel/random/uuid r,

  # Network for updates
  network inet stream,
  network inet dgram,

  # DBus
  dbus send
    bus=system
    peer=(name=org.freedesktop.DBus)
    interface=org.freedesktop.DBus,
    
  dbus receive
    bus=system
    peer=(name=org.freedesktop.DBus),

  # Temporary files
  /tmp/ r,
  owner /tmp/dots-family-** rw,
  /var/tmp/ r,
  owner /var/tmp/dots-family-** rw,

  # Deny sensitive locations
  deny /home/ r,
  deny /home/** rw,
  deny /root/ r,
  deny /root/** rw,
  deny /etc/shadow r,
  deny /etc/passwd w,
  deny /etc/sudoers* rw,
  deny /etc/ssh/ r,
  deny /etc/ssh/** rw,

  # Deny ptrace except for allowed monitoring
  deny ptrace,
  ptrace (read) peer=/usr/bin/dots-family-monitor,
}
EOF

    # AppArmor profile for monitor
    cat > /etc/apparmor.d/dots-family.monitor << 'EOF'
#include <tunables/global>

/usr/bin/dots-family-monitor {
  #include <abstractions/base>
  #include <abstractions/nameservice>
  #include <abstractions/dbus-session-strict>
  #include <abstractions/wayland>
  #include <abstractions/X>

  # Binary execution
  /usr/bin/dots-family-monitor mr,

  # User configuration
  owner @{HOME}/.config/dots-family/ r,
  owner @{HOME}/.config/dots-family/** rw,

  # Wayland compositor access
  /run/user/*/wayland-* rw,
  /tmp/.X11-unix/X* rw,

  # Process monitoring
  /proc/*/stat r,
  /proc/*/cmdline r,
  /proc/*/comm r,

  # DBus
  dbus send
    bus=session
    peer=(name=org.dots.FamilyMonitor),
    
  dbus send
    bus=system
    peer=(name=org.dots.FamilyDaemon),

  # Temporary files
  /tmp/ r,
  owner /tmp/dots-family-monitor-** rw,

  # Deny system modification
  deny /etc/** w,
  deny /var/lib/dots-family/** rw,
  deny /root/** rw,
}
EOF

    # AppArmor profile for CLI tool
    cat > /etc/apparmor.d/dots-family.ctl << 'EOF'
#include <tunables/global>

/usr/bin/dots-family-ctl {
  #include <abstractions/base>
  #include <abstractions/nameservice>
  #include <abstractions/dbus-session-strict>

  # Binary execution
  /usr/bin/dots-family-ctl mr,

  # User configuration
  owner @{HOME}/.config/dots-family/ r,
  owner @{HOME}/.config/dots-family/** r,

  # DBus communication
  dbus send
    bus=system
    peer=(name=org.dots.FamilyDaemon),

  # Read-only system access
  /var/lib/dots-family/ r,
  /var/lib/dots-family/*.db r,

  # Deny modification
  deny /var/lib/dots-family/** w,
  deny /etc/** w,
  deny /root/** rw,
}
EOF

    # Load profiles
    apparmor_parser -r /etc/apparmor.d/dots-family.* 2>/dev/null || warn "Failed to load some AppArmor profiles"
    success "AppArmor profiles configured"
}

setup_audit_logging() {
    log "Setting up audit logging..."
    
    # Create audit rules for DOTS Family components
    cat > /etc/audit/rules.d/99-dots-family.rules << 'EOF'
# DOTS Family Mode - Security Audit Rules

# Monitor daemon configuration changes
-w /var/lib/dots-family/ -p wa -k dots-family-config
-w /etc/dots-family/ -p wa -k dots-family-config

# Monitor service management
-w /etc/systemd/system/dots-family* -p wa -k dots-family-services
-w /usr/bin/dots-family-daemon -p x -k dots-family-execution
-w /usr/bin/dots-family-monitor -p x -k dots-family-execution
-w /usr/bin/dots-family-ctl -p x -k dots-family-execution

# Monitor DBus configuration
-w /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf -p wa -k dots-family-dbus
-w /etc/dbus-1/session.d/org.dots.FamilyMonitor.conf -p wa -k dots-family-dbus

# Monitor user group changes
-w /etc/group -p wa -k dots-family-groups
-w /etc/passwd -p wa -k dots-family-users

# Monitor eBPF configuration
-w /etc/sysctl.d/99-dots-family-ebpf.conf -p wa -k dots-family-ebpf
-w /sys/kernel/debug/tracing/ -p wa -k dots-family-ebpf

# Monitor for privilege escalation attempts
-a always,exit -F arch=b64 -S setuid,setgid,setreuid,setregid -F auid>=1000 -F auid!=4294967295 -k dots-family-privesc
-a always,exit -F arch=b32 -S setuid,setgid,setreuid,setregid -F auid>=1000 -F auid!=4294967295 -k dots-family-privesc

# Monitor capability changes
-a always,exit -F arch=b64 -S capset -k dots-family-capabilities
-a always,exit -F arch=b32 -S capset -k dots-family-capabilities
EOF

    # Restart auditd if running
    if systemctl is-active auditd.service >/dev/null 2>&1; then
        systemctl restart auditd.service || warn "Failed to restart auditd"
        success "Audit rules loaded"
    else
        warn "auditd not running, rules will be loaded on next boot"
    fi
}

configure_sysctl_security() {
    log "Configuring system security parameters..."
    
    cat > /etc/sysctl.d/99-dots-family-security.conf << 'EOF'
# DOTS Family Mode - Security Hardening

# Network security
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0
net.ipv4.conf.all.secure_redirects = 0
net.ipv4.conf.default.secure_redirects = 0
net.ipv4.conf.all.log_martians = 1
net.ipv4.conf.default.log_martians = 1
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# Process hardening
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 2
kernel.yama.ptrace_scope = 2
kernel.core_uses_pid = 1
kernel.core_pattern = /var/crash/core.%e.%p.%h.%t

# Memory protection
vm.mmap_min_addr = 65536
kernel.randomize_va_space = 2

# File system hardening
fs.protected_hardlinks = 1
fs.protected_symlinks = 1
fs.protected_fifos = 2
fs.protected_regular = 2
fs.suid_dumpable = 0

# Additional eBPF security (complement to earlier config)
kernel.unprivileged_bpf_disabled = 1  # More restrictive for production
net.core.bpf_jit_harden = 2  # Always enable hardening
EOF

    sysctl -p /etc/sysctl.d/99-dots-family-security.conf
    success "Security parameters configured"
}

setup_log_rotation() {
    log "Setting up log rotation..."
    
    cat > /etc/logrotate.d/dots-family << 'EOF'
/var/log/dots-family/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    copytruncate
    create 0640 dots-family dots-family-parents
    
    postrotate
        /bin/systemctl reload dots-family-daemon.service > /dev/null 2>&1 || true
    endscript
}

/var/log/dots-family/audit.log {
    daily
    missingok
    rotate 90
    compress
    delaycompress
    notifempty
    copytruncate
    create 0640 root root
}
EOF

    success "Log rotation configured"
}

create_monitoring_script() {
    log "Creating security monitoring script..."
    
    cat > /usr/local/bin/dots-family-security-monitor << 'EOF'
#!/bin/bash

# DOTS Family Security Monitor
# Regular security health checks

set -euo pipefail

LOG_FILE="/var/log/dots-family/security-monitor.log"
ALERT_THRESHOLD=3

log_message() {
    echo "$(date +'%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

check_service_integrity() {
    local errors=0
    
    # Check if services are running with correct users
    if ! systemctl is-active dots-family-daemon.service >/dev/null 2>&1; then
        log_message "ALERT: daemon service not running"
        ((errors++))
    fi
    
    # Check database permissions
    if [[ -f "/var/lib/dots-family/family.db" ]]; then
        local db_perms=$(stat -c "%a" /var/lib/dots-family/family.db)
        if [[ "$db_perms" != "640" ]]; then
            log_message "ALERT: Database permissions incorrect: $db_perms (should be 640)"
            ((errors++))
        fi
    fi
    
    # Check for unauthorized modifications
    if [[ -f "/etc/systemd/system/dots-family-daemon.service" ]]; then
        local service_hash=$(sha256sum /etc/systemd/system/dots-family-daemon.service | cut -d' ' -f1)
        local expected_hash_file="/var/lib/dots-family/.service-hashes"
        
        if [[ -f "$expected_hash_file" ]] && ! grep -q "$service_hash" "$expected_hash_file"; then
            log_message "ALERT: Service file may have been modified"
            ((errors++))
        fi
    fi
    
    return $errors
}

check_user_groups() {
    local errors=0
    
    # Verify critical groups exist
    for group in dots-family dots-parents; do
        if ! getent group "$group" >/dev/null 2>&1; then
            log_message "ALERT: Required group $group does not exist"
            ((errors++))
        fi
    done
    
    # Check for unauthorized users in parent group
    local parent_users=$(getent group dots-parents | cut -d: -f4)
    if [[ -n "$parent_users" ]]; then
        log_message "INFO: Parent group members: $parent_users"
    fi
    
    return $errors
}

check_ebpf_status() {
    local errors=0
    
    # Check eBPF programs
    if command -v bpftool >/dev/null 2>&1; then
        local bpf_count=$(bpftool prog list 2>/dev/null | grep -c dots-family || true)
        if [[ $bpf_count -eq 0 ]]; then
            log_message "WARNING: No DOTS Family eBPF programs loaded"
        else
            log_message "INFO: $bpf_count eBPF programs loaded"
        fi
    fi
    
    return $errors
}

check_audit_logs() {
    local errors=0
    
    # Check for suspicious audit events in last hour
    if command -v ausearch >/dev/null 2>&1; then
        local suspicious_events=$(ausearch -ts recent -k dots-family-privesc 2>/dev/null | wc -l || echo 0)
        if [[ $suspicious_events -gt 0 ]]; then
            log_message "ALERT: $suspicious_events privilege escalation attempts detected"
            ((errors++))
        fi
    fi
    
    return $errors
}

main() {
    log_message "Starting security health check"
    
    local total_errors=0
    
    check_service_integrity || ((total_errors += $?))
    check_user_groups || ((total_errors += $?))
    check_ebpf_status || ((total_errors += $?))
    check_audit_logs || ((total_errors += $?))
    
    if [[ $total_errors -ge $ALERT_THRESHOLD ]]; then
        log_message "CRITICAL: $total_errors security issues detected"
        # Send alert to administrators
        echo "DOTS Family security alert: $total_errors issues detected" | \
            logger -p auth.crit -t dots-family-security
        exit 1
    elif [[ $total_errors -gt 0 ]]; then
        log_message "WARNING: $total_errors minor security issues detected"
        exit 2
    else
        log_message "INFO: Security health check passed"
        exit 0
    fi
}

main "$@"
EOF

    chmod +x /usr/local/bin/dots-family-security-monitor
    
    # Create systemd timer for regular monitoring
    cat > /etc/systemd/system/dots-family-security-monitor.service << 'EOF'
[Unit]
Description=DOTS Family Security Monitor
Documentation=man:dots-family-security-monitor(1)

[Service]
Type=oneshot
ExecStart=/usr/local/bin/dots-family-security-monitor
User=root
Group=root
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadOnlyPaths=/var/lib/dots-family
StandardOutput=journal
StandardError=journal
EOF

    cat > /etc/systemd/system/dots-family-security-monitor.timer << 'EOF'
[Unit]
Description=Run DOTS Family Security Monitor hourly
Requires=dots-family-security-monitor.service

[Timer]
OnCalendar=hourly
Persistent=true
RandomizedDelaySec=300

[Install]
WantedBy=timers.target
EOF

    systemctl daemon-reload
    systemctl enable dots-family-security-monitor.timer
    systemctl start dots-family-security-monitor.timer
    
    success "Security monitoring configured"
}

create_security_baseline() {
    log "Creating security baseline..."
    
    mkdir -p /var/lib/dots-family
    
    # Store service file hashes for integrity checking
    if [[ -f "/etc/systemd/system/dots-family-daemon.service" ]]; then
        sha256sum /etc/systemd/system/dots-family-daemon.service > /var/lib/dots-family/.service-hashes
        sha256sum /etc/systemd/system/dots-family-monitor@.service >> /var/lib/dots-family/.service-hashes
        sha256sum /etc/systemd/system/dots-family-filter.service >> /var/lib/dots-family/.service-hashes
        success "Security baseline created"
    else
        warn "Service files not found, skipping baseline creation"
    fi
}

generate_security_report() {
    log "Generating security configuration report..."
    
    cat > /var/log/dots-family/security-report.txt << EOF
DOTS Family Mode Security Configuration Report
Generated: $(date)
Hostname: $(hostname)
Kernel: $(uname -r)

=== Service Hardening ===
$(systemctl show dots-family-daemon.service | grep -E "CapabilityBoundingSet|ProtectSystem|PrivateNetwork" || echo "Service not found")

=== AppArmor Status ===
$(aa-status 2>/dev/null | grep dots-family || echo "AppArmor not active or no profiles loaded")

=== Audit Configuration ===
$(auditctl -l 2>/dev/null | grep dots-family || echo "No audit rules active")

=== System Security Parameters ===
$(sysctl -a 2>/dev/null | grep -E "kernel.yama.ptrace_scope|kernel.dmesg_restrict|kernel.unprivileged_bpf_disabled" || echo "Parameters not set")

=== File Permissions ===
$(find /var/lib/dots-family -type f -exec ls -la {} \; 2>/dev/null || echo "Directory not found")

=== Group Membership ===
$(getent group dots-family dots-parents 2>/dev/null || echo "Groups not found")

=== eBPF Status ===
$(bpftool prog list 2>/dev/null | head -5 || echo "bpftool not available")

End of Report
EOF

    success "Security report generated at /var/log/dots-family/security-report.txt"
}

main() {
    log "DOTS Family Mode Security Hardening"
    log "==================================="
    
    check_root
    
    local errors=0
    
    harden_systemd_services || ((errors++))
    configure_apparmor_profiles || ((errors++))
    setup_audit_logging || ((errors++))
    configure_sysctl_security || ((errors++))
    setup_log_rotation || ((errors++))
    create_monitoring_script || ((errors++))
    create_security_baseline || ((errors++))
    generate_security_report || ((errors++))
    
    if [[ $errors -eq 0 ]]; then
        success "Security hardening completed successfully!"
        log ""
        log "Next steps:"
        log "1. Review security report: /var/log/dots-family/security-report.txt"
        log "2. Reboot to ensure all kernel parameters take effect"
        log "3. Monitor security alerts: journalctl -f -t dots-family-security"
        log "4. Run security monitor manually: /usr/local/bin/dots-family-security-monitor"
        log ""
        log "Security monitoring will run automatically every hour"
    else
        error "Security hardening completed with $errors errors"
        log "Please review the errors above before proceeding to production"
        exit 1
    fi
}

main "$@"