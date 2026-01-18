# DOTS Family Mode - Production Deployment Checklist

## Pre-Deployment Requirements

### System Requirements
- [ ] NixOS or Nix package manager installed
- [ ] systemd service manager available  
- [ ] Linux kernel 5.8+ with eBPF support
- [ ] CAP_BPF and CAP_SYS_ADMIN capabilities available
- [ ] SQLite and SQLCipher libraries installed

### Security Requirements
- [ ] Root access for system service installation
- [ ] Parent authentication configured
- [ ] Database encryption key configured
- [ ] Network firewall configured (if applicable)
- [ ] Audit logging destination configured

## Pre-Deployment Testing

### Build and Test
```bash
# 1. Clone and enter project
git clone <repository-url>
cd dots-family-mode
nix develop

# 2. Run full test suite
cargo test --workspace

# 3. Test eBPF compilation
cd dots-family-ebpf && nix build
ls target/bpfel-unknown-none/release/

# 4. Test integration
./scripts/deploy_system_integration.sh development
```

### Security Validation
```bash
# Test security components
cargo test -p dots-family-common security

# Validate systemd service configuration
systemd-analyze verify systemd/dots-family-daemon.service

# Test privilege restrictions
sudo ./scripts/deploy_system_integration.sh production --dry-run
```

## Production Deployment Steps

### 1. System Preparation
```bash
# Install as root
sudo su

# Ensure system requirements
systemctl --version
uname -r  # Check kernel version >= 5.8
```

### 2. Deploy System Services
```bash
# Run production deployment script
sudo ./scripts/deploy_system_integration.sh production
```

This script will:
- [x] Build release binaries with optimizations
- [x] Install to system locations (/usr/bin, /etc, /var)
- [x] Configure D-Bus system bus permissions
- [x] Install and enable systemd services
- [x] Set up security hardening (capabilities, isolation)
- [x] Create required directories with proper permissions

### 3. Configuration
```bash
# Copy production configuration
cp configs/production/daemon.conf /etc/dots-family/

# Set parent password (interactive)
dots-family-ctl setup-parent

# Create initial child profiles
dots-family-ctl profile create alice 8-12
dots-family-ctl profile create bob 13-17
```

### 4. Service Management
```bash
# Start services
systemctl start dots-family-daemon.service
systemctl start dots-family-monitor@<username>.service

# Enable automatic startup
systemctl enable dots-family-daemon.service
systemctl enable dots-family-maintenance.timer

# Verify service status
systemctl status dots-family-daemon.service
journalctl -u dots-family-daemon.service -f
```

## Post-Deployment Validation

### Functionality Tests
```bash
# Test CLI administration
dots-family-ctl status
dots-family-ctl profile list
dots-family-ctl check firefox

# Test D-Bus communication
busctl --system list | grep dots
busctl --system introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon

# Test eBPF loading (requires root)
sudo journalctl -u dots-family-daemon.service | grep -i ebpf
```

### Security Validation
```bash
# Verify service isolation
systemctl show dots-family-daemon.service | grep -E "(Protect|Restrict|Private)"

# Check capability restrictions
cat /proc/$(pgrep dots-family-daemon)/status | grep Cap

# Verify database encryption
sqlite3 /var/lib/dots-family/family.db ".tables"  # Should fail if encrypted

# Test rate limiting
for i in {1..6}; do dots-family-ctl authenticate wrong-password; done
```

### Performance Monitoring
```bash
# Monitor resource usage
systemctl status dots-family-daemon.service
top -p $(pgrep dots-family-daemon)

# Check eBPF program status
sudo bpftool prog list | grep dots-family

# Monitor activity logging
tail -f /var/log/dots-family/daemon.log
```

## Maintenance Tasks

### Regular Maintenance
```bash
# Database backup (automated via systemd timer)
systemctl status dots-family-maintenance.timer

# Log rotation (handled by systemd)
journalctl --disk-usage

# Profile updates
dots-family-ctl profile update alice --daily-limit 3h
```

### Troubleshooting
```bash
# Check service health
systemctl is-active dots-family-daemon.service
dots-family-ctl status

# View detailed logs
journalctl -u dots-family-daemon.service --since "1 hour ago"

# Test D-Bus connectivity
busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon GetSystemStatus

# Reset configuration (emergency)
systemctl stop dots-family-daemon.service
sudo rm -f /var/lib/dots-family/family.db
systemctl start dots-family-daemon.service
```

## Security Considerations

### Data Protection
- Database encrypted with parent password-derived key
- Session tokens expire after 15 minutes
- Rate limiting prevents brute force attacks
- Audit logging tracks all administrative actions

### System Isolation
- Service runs with minimal capabilities
- Filesystem access restricted to necessary paths
- Network access limited to localhost
- System call filtering prevents privilege escalation

### Child Safety
- Monitoring is transparent and age-appropriate
- Emergency override available for safety situations
- No keystroke logging or screenshot capture
- Configurable privacy levels by age group

## Rollback Procedure

If deployment fails or causes issues:

```bash
# Stop services
sudo systemctl stop dots-family-daemon.service
sudo systemctl disable dots-family-daemon.service

# Remove system files
sudo rm -f /usr/bin/dots-family-*
sudo rm -f /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf
sudo rm -f /etc/systemd/system/dots-family-*.service
sudo rm -f /etc/systemd/system/dots-family-*.timer

# Reload system configuration
sudo systemctl daemon-reload
sudo systemctl reload dbus.service

# Remove data (if needed)
sudo rm -rf /var/lib/dots-family
sudo rm -rf /etc/dots-family
sudo rm -rf /var/log/dots-family
```

## Support and Documentation

- **System Logs**: `journalctl -u dots-family-daemon.service`
- **Configuration**: `/etc/dots-family/daemon.conf`
- **CLI Help**: `dots-family-ctl --help`
- **D-Bus Introspection**: `busctl --system introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon`
- **Architecture Documentation**: `docs/ARCHITECTURE.md`
- **Security Documentation**: `docs/SECURITY_ARCHITECTURE.md`