# DOTS Family Mode eBPF Integration

This directory configures eBPF (Extended Berkeley Packet Filter) capabilities for the DOTS Family Mode parental control system. The eBPF infrastructure enables efficient kernel-level monitoring of processes, network connections, and filesystem activity.

## eBPF Architecture

### Overview

DOTS Family Mode implements a **three-pillar eBPF monitoring system** using the Aya library:

1. **Process Monitor**: Track process creation/termination via tracepoints
2. **Network Monitor**: Monitor TCP/UDP connections and socket state changes  
3. **Filesystem Monitor**: Track file access on sensitive directories

### Implementation Source

The eBPF implementation is shared from the parent `dots-detection` project:
- Location: `../../../dots-detection/src/ebpf/`
- Integration: `crates/dots-family-daemon/src/ebpf/` (symlinked or copied)
- Library: Aya v0.12 with async Tokio support

### Monitoring Capabilities

#### Process Monitoring (`ProcessMonitorEbpf`)
- **Hook Points**: `tracepoint:sched:sched_process_exec`, `tracepoint:sched:sched_process_exit`
- **Data Collected**: PID, PPID, UID, GID, command name, filename, timestamps
- **Use Case**: Real-time application launching detection for policy enforcement
- **Fallback**: `/proc/<pid>/stat` and `/proc/<pid>/cmdline` parsing

#### Network Monitoring (`NetworkMonitorEbpf`)
- **Hook Points**: `tracepoint:net:netif_receive_skb`, `sock:inet_sock_set_state`
- **Data Collected**: Connection events, addresses, ports, protocols, process association
- **Use Case**: Network activity tracking and internet usage monitoring
- **Fallback**: `ss -tuna` command output parsing

#### Filesystem Monitoring (`FilesystemMonitorEbpf`)
- **Hook Points**: `tracepoint:syscalls:sys_enter_openat`, filesystem tracepoints
- **Path Filters**: `/etc`, `/root`, `/home`, `/boot`, `/var/log` (configurable)
- **Data Collected**: File operations (open/read/write/delete/chmod), PIDs, filenames
- **Use Case**: Sensitive file access monitoring and tamper detection
- **Fallback**: `lsof -n` command enumeration

## Configuration Files

### `sysctl-ebpf.conf`
Kernel configuration for eBPF functionality:
```bash
# Enable controlled unprivileged eBPF for monitoring
kernel.unprivileged_bpf_disabled = 0

# Enable eBPF JIT compilation for performance
net.core.bpf_jit_enable = 1
net.core.bpf_jit_harden = 1
net.core.bpf_jit_kallsyms = 1

# Enable BPF statistics
kernel.bpf_stats_enabled = 1
```

### Required Capabilities
Configured in systemd service files:
- **`CAP_SYS_ADMIN`**: General eBPF operations and tracepoint attachment
- **`CAP_NET_ADMIN`**: Network monitoring and socket state tracking
- **`CAP_SYS_PTRACE`**: Process monitoring and introspection
- **`CAP_DAC_READ_SEARCH`**: Filesystem access for monitoring

## Installation Scripts

### `configure-ebpf-capabilities.sh`
Production installation script (requires root):
- Validates kernel eBPF support
- Installs sysctl configuration to `/etc/sysctl.d/`
- Checks required capabilities availability
- Configures debugfs mounting
- Applies kernel parameters

Usage:
```bash
sudo ./configure-ebpf-capabilities.sh
```

### `test-ebpf-capabilities.sh`  
Development testing script (unprivileged):
- Tests basic eBPF support without requiring root
- Validates capability requirements
- Checks systemd service integration
- Generates compatibility report

Usage:
```bash
./test-ebpf-capabilities.sh
```

## Security Model

### Privilege Requirements

**Root-Level Monitoring (Daemon)**:
- Runs as root user for eBPF attachment
- Full capability set for comprehensive monitoring
- Systemd security hardening with minimal filesystem access

**User-Level Components (Monitor)**:
- Limited capabilities for specific monitoring tasks
- No direct eBPF access (communicates via daemon DBus)
- Process-specific monitoring scope

### Data Protection

- **Kernel → Userspace**: eBPF maps provide secure data transfer
- **Userspace → Database**: SQLCipher encryption for activity storage
- **Inter-Process**: DBus with policy-enforced access control
- **Tamper Resistance**: eBPF programs protected against user modification

## Integration with DOTS Family Services

### Monitoring Service Integration
The `MonitoringService` in `dots-family-daemon` orchestrates eBPF monitoring:

```rust
// 10-second collection intervals
impl MonitoringService {
    async fn collect_activity_data(&self) -> Result<ActivityData> {
        let process_data = self.process_monitor.collect_recent().await?;
        let network_data = self.network_monitor.collect_recent().await?; 
        let filesystem_data = self.filesystem_monitor.collect_recent().await?;
        // Combine and report via DBus
    }
}
```

### Policy Enforcement
eBPF data drives real-time policy decisions:
- **Application Launching**: Process events trigger policy checks
- **Network Usage**: Connection events enforce internet restrictions
- **File Access**: Filesystem events protect sensitive data
- **Time Limits**: Activity aggregation tracks screen time

### Graceful Degradation
When eBPF is unavailable, fallback mechanisms provide reduced functionality:
- Process monitoring via `/proc` filesystem polling
- Network monitoring via `netstat`/`ss` commands
- Filesystem monitoring via `lsof` enumeration

## Development and Testing

### Prerequisites
- Linux kernel 4.4+ with eBPF support
- `CONFIG_BPF=y`, `CONFIG_BPF_SYSCALL=y` in kernel
- debugfs mounted at `/sys/kernel/debug`
- bpftool for debugging (optional)

### Development Environment
```bash
# Check eBPF readiness
./test-ebpf-capabilities.sh

# Configure for development (requires root)
sudo ./configure-ebpf-capabilities.sh

# Verify eBPF programs load
sudo bpftool prog list
sudo bpftool map list
```

### Integration Testing
```bash
# Test monitoring collection
cargo test --package dots-family-daemon ebpf_integration

# Test fallback mechanisms  
DOTS_DISABLE_EBPF=1 cargo test monitoring_fallback

# Performance testing
cargo test --package dots-family-daemon --release monitoring_performance
```

## Troubleshooting

### Common Issues

**"Permission denied" when loading eBPF**:
- Verify running as root or with sufficient capabilities
- Check that systemd service has `CapabilityBoundingSet` configured
- Ensure debugfs is mounted

**"Invalid program" errors**:
- Verify kernel supports required eBPF features
- Check kernel version compatibility (4.4+ required)
- Update to newer kernel if eBPF features missing

**High CPU usage**:
- Monitor eBPF program efficiency with `bpftool prog show`
- Consider reducing collection frequency from 10-second intervals
- Check if fallback mode is being used unintentionally

**Missing events**:
- Verify tracepoints exist: `ls /sys/kernel/debug/tracing/events/`
- Check eBPF program attachment: `bpftool prog list`
- Validate BPF map updates: `bpftool map dump`

### Performance Optimization

- **JIT Compilation**: Enable `net.core.bpf_jit_enable = 1` for performance
- **Map Sizing**: Configure appropriate BPF map sizes for activity volume
- **Filter Efficiency**: Use eBPF filtering to reduce userspace data transfer
- **Batch Collection**: Collect events in batches rather than individual notifications

## Related Documentation

- **Parent Project**: `../../../dots-detection/src/ebpf/` - Core eBPF implementation
- **Systemd Services**: `../systemd/` - Service files with eBPF capabilities
- **DBus Policies**: `../dbus-policies/` - Secure communication configuration
- **Database Schema**: `../migrations/` - Activity storage schema
- **Architecture**: `../docs/MONITORING.md` - Overall monitoring design

## Future Enhancements

- **XDP Integration**: Network packet filtering at driver level
- **LSM Hooks**: Linux Security Module integration for enhanced security
- **Custom Tracepoints**: Application-specific monitoring points
- **Machine Learning**: Behavioral analysis of collected activity data
- **Real-Time Alerting**: Immediate policy violation notifications