# DOTS Family Mode Security Hardening

This directory contains comprehensive security hardening and production tuning tools for DOTS Family Mode deployment.

## Overview

The security hardening framework provides multiple layers of protection:

1. **systemd Service Hardening** - Filesystem isolation, capability restrictions, resource limits
2. **AppArmor Profiles** - Application-level mandatory access controls
3. **Audit Logging** - Comprehensive security event monitoring
4. **Network Security** - Protocol restrictions and filtering
5. **Performance Tuning** - Optimized parameters for production workloads
6. **Continuous Monitoring** - Automated security health checks

## Security Tools

### Production Security Hardening
**File:** `harden-production-security.sh`

Complete security hardening for production deployment:
- Enhanced systemd service configurations
- AppArmor profile installation and activation
- Audit rule configuration
- Security monitoring setup
- Performance optimization

**Usage:**
```bash
# Apply all security hardening measures
sudo ./harden-production-security.sh

# Review applied configurations
journalctl -t dots-family-security
```

**Features:**
- **systemd Hardening**: Additional security overrides for all services
- **AppArmor Profiles**: Mandatory access control for daemon, monitor, and CLI
- **Audit Rules**: Security event logging for compliance
- **Resource Limits**: Memory and process restrictions
- **Security Monitoring**: Hourly automated security health checks

### Security Audit Tool
**File:** `security-audit.sh`

Comprehensive security assessment and compliance checking:
- systemd service configuration audit
- DBus policy validation
- User group and permission verification
- eBPF security configuration review
- AppArmor profile status
- Network security assessment

**Usage:**
```bash
# Run comprehensive security audit
./security-audit.sh

# Review generated report
cat /tmp/dots-family-security-audit-*.txt
```

**Audit Categories:**
- **Critical Issues**: Immediate security risks requiring action
- **High Issues**: Important security gaps to address before production
- **Medium Issues**: Security improvements to consider
- **Low Issues**: Minor security enhancements

### Production Tuning
**File:** `production-tuning.conf`

Optimized kernel parameters for production deployment:
- Database and filesystem performance
- Memory management for monitoring workloads
- Network optimization for eBPF traffic
- Process scheduler tuning

**Integration:**
```bash
# Apply tuning parameters
sudo cp production-tuning.conf /etc/sysctl.d/99-dots-family-tuning.conf
sudo sysctl -p /etc/sysctl.d/99-dots-family-tuning.conf
```

## Security Architecture

### Multi-Layer Security Model

```
┌─────────────────────────────────────────────────────┐
│                 User Space                          │
├─────────────────┬───────────────┬───────────────────┤
│   AppArmor      │    systemd    │   File Permissions│
│   Profiles      │   Hardening   │   & Capabilities  │
├─────────────────┼───────────────┼───────────────────┤
│            DBus Security Policies                   │
├─────────────────────────────────────────────────────┤
│              Audit Logging                          │
├─────────────────────────────────────────────────────┤
│                Kernel Space                         │
│            eBPF + Network Security                  │
└─────────────────────────────────────────────────────┘
```

### systemd Security Features

**Daemon Hardening:**
- `ProtectSystem=strict` - Read-only filesystem
- `PrivateTmp=true` - Isolated temporary files
- `NoNewPrivileges=true` - Prevent privilege escalation
- `CapabilityBoundingSet` - Minimal capabilities for eBPF
- `SystemCallFilter` - Restricted syscall access
- `MemoryMax=512M` - Memory limits

**Monitor Hardening:**
- `PrivateDevices=true` - Device isolation
- `RestrictRealtime=true` - Prevent realtime access
- `RestrictSUIDSGID=true` - Block SUID/SGID execution
- `MemoryMax=256M` - Resource constraints

**Filter Hardening:**
- `RestrictAddressFamilies` - Network protocol restrictions
- `IPAddressDeny=any` with selective allow rules
- `TasksMax=100` - Process limits

### AppArmor Profiles

**Daemon Profile** (`/etc/apparmor.d/dots-family.daemon`):
- DBus communication permissions
- eBPF and tracing access
- Configuration and database access
- Deny sensitive system locations

**Monitor Profile** (`/etc/apparmor.d/dots-family.monitor`):
- Wayland/X11 compositor access
- User configuration access
- Process monitoring permissions
- DBus session communication

**CLI Profile** (`/etc/apparmor.d/dots-family.ctl`):
- Read-only database access
- User configuration reading
- DBus system communication
- Deny administrative modifications

### Audit Configuration

**Security Events Monitored:**
- Configuration file changes (`/var/lib/dots-family/`, `/etc/dots-family/`)
- Service management operations
- Binary execution tracking
- User group modifications
- eBPF configuration changes
- Privilege escalation attempts
- Capability modifications

**Log Locations:**
- `/var/log/audit/audit.log` - System audit events
- `/var/log/dots-family/security-monitor.log` - Automated monitoring
- `journalctl -t dots-family-security` - Security alerts

## Production Deployment

### Prerequisites
- Root access for security configuration
- systemd 245+ (for enhanced security features)
- AppArmor 3.0+ (optional but recommended)
- auditd for compliance logging
- 4GB+ RAM for optimal performance

### Deployment Steps

1. **Initial Security Hardening:**
   ```bash
   sudo ./harden-production-security.sh
   ```

2. **Security Audit:**
   ```bash
   ./security-audit.sh
   # Address any critical or high-priority issues
   ```

3. **Performance Tuning:**
   ```bash
   sudo cp production-tuning.conf /etc/sysctl.d/99-dots-family-tuning.conf
   sudo sysctl -p /etc/sysctl.d/99-dots-family-tuning.conf
   ```

4. **Reboot and Validation:**
   ```bash
   sudo reboot
   # After reboot:
   ./security-audit.sh
   ```

### Ongoing Security Maintenance

**Automated Monitoring:**
- Security monitor runs hourly via systemd timer
- Critical alerts logged to system journal
- Monthly audit reports recommended

**Manual Security Checks:**
```bash
# Weekly security audit
./security-audit.sh

# Monitor security events
journalctl -t dots-family-security -f

# Check AppArmor denials
sudo dmesg | grep -i apparmor

# Review audit logs
sudo ausearch -k dots-family-privesc
```

**Update Procedures:**
1. Test security updates in development environment
2. Run security audit before and after updates
3. Review audit logs for new denial patterns
4. Update security baselines as needed

## Integration with NixOS

The security hardening integrates with the NixOS module:

**NixOS Configuration:**
```nix
services.dots-family = {
  enable = true;
  # Security hardening automatically applied
};

# Additional security options
security.apparmor.enable = true;
services.auditd.enable = true;
```

**Enhanced Integration:**
- systemd overrides applied automatically
- AppArmor profiles integrated with NixOS security
- Audit rules configured declaratively
- Performance tuning applied via boot parameters

## Troubleshooting

### Common Security Issues

**AppArmor Denials:**
```bash
# Check for denials
sudo dmesg | grep -i "apparmor.*denied"

# Put profiles in complain mode for debugging
sudo aa-complain /etc/apparmor.d/dots-family.daemon
```

**systemd Security Failures:**
```bash
# Check service status
systemctl status dots-family-daemon.service

# Review security restrictions
systemd-analyze security dots-family-daemon.service
```

**Audit Configuration:**
```bash
# Check audit rules
sudo auditctl -l | grep dots-family

# Test audit logging
sudo ausearch -k dots-family-config
```

### Performance Issues

**Memory Usage:**
```bash
# Monitor service memory usage
systemctl status dots-family-daemon.service
systemd-cgtop

# Adjust memory limits in service overrides
sudo systemctl edit dots-family-daemon.service
```

**eBPF Performance:**
```bash
# Monitor eBPF program efficiency
sudo bpftool prog show | grep dots-family
sudo bpftool prog profile
```

## Security Compliance

The security hardening framework addresses:

- **CIS Controls** - Critical security controls implementation
- **NIST Cybersecurity Framework** - Security function coverage
- **ISO 27001** - Information security management
- **SOC 2** - Security monitoring and logging requirements

**Compliance Features:**
- Comprehensive audit logging
- Access control documentation
- Security monitoring automation
- Incident detection and response
- Regular security assessments

For specific compliance requirements, customize audit rules and monitoring scripts according to your organization's security policies.