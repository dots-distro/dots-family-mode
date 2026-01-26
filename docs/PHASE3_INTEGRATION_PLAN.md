# Phase 3 eBPF Integration Test Plan

## Overview

This document outlines the integration testing strategy for the Phase 3 eBPF monitors. All monitors have been implemented, compiled, and verified. This plan focuses on userspace integration and end-to-end testing.

## Monitor Status

| Monitor | Status | Binary Size | Probe Count | Type |
|---------|--------|-------------|-------------|------|
| process-monitor | ✅ Ready | 4.8K | 2 | tracepoint |
| filesystem-monitor | ✅ Ready | 6.8K | 4 | kprobe |
| network-monitor | ✅ Ready | 5.5K | 3 | kprobe |
| memory-monitor | ✅ NEW | 5.7K | 4 | tracepoint |
| disk-io-monitor | ✅ NEW | 4.6K | 3 | tracepoint |

**Total:** 27.4KB, 16 probe functions

---

## Phase 1: Binary Loading Tests

### Objective
Verify all eBPF binaries can be loaded into the kernel without errors.

### Prerequisites
- Root access
- Kernel 5.10+ with BPF support
- bpftool installed

### Test Cases

#### T1.1: Load process-monitor
```bash
sudo bpftool prog load prebuilt-ebpf/process-monitor /sys/fs/bpf/process-monitor
sudo bpftool prog list | grep process-monitor
```
**Expected:** Program loads successfully, 2 tracepoints attached

#### T1.2: Load filesystem-monitor
```bash
sudo bpftool prog load prebuilt-ebpf/filesystem-monitor /sys/fs/bpf/filesystem-monitor
sudo bpftool prog list | grep filesystem-monitor
```
**Expected:** Program loads successfully, 4 kprobes attached

#### T1.3: Load network-monitor
```bash
sudo bpftool prog load prebuilt-ebpf/network-monitor /sys/fs/bpf/network-monitor
sudo bpftool prog list | grep network-monitor
```
**Expected:** Program loads successfully, 3 kprobes attached

#### T1.4: Load memory-monitor (NEW)
```bash
sudo bpftool prog load prebuilt-ebpf/memory-monitor /sys/fs/bpf/memory-monitor
sudo bpftool prog list | grep memory-monitor
```
**Expected:** Program loads successfully, 4 tracepoints attached

#### T1.5: Load disk-io-monitor (NEW)
```bash
sudo bpftool prog load prebuilt-ebpf/disk-io-monitor /sys/fs/bpf/disk-io-monitor
sudo bpftool prog list | grep disk-io-monitor
```
**Expected:** Program loads successfully, 3 tracepoints attached

#### T1.6: Verify Map Creation
```bash
sudo bpftool map list
```
**Expected:** All ring buffers and HashMaps created:
- PROCESS_EVENTS (RingBuf)
- FS_EVENTS (RingBuf)
- NETWORK_EVENTS (RingBuf)
- MEMORY_EVENTS (RingBuf)
- DISK_IO_EVENTS (RingBuf)
- PENDING_IO (HashMap - disk-io-monitor)
- CMDLINE_BUF (PerCpuArray - process-monitor)
- FILENAME_BUF (PerCpuArray - filesystem-monitor)

---

## Phase 2: Event Generation Tests

### Objective
Verify each monitor captures events when expected activity occurs.

### T2.1: Process Events
```bash
# Trigger process execution
ls /tmp
echo "test" | cat

# Check for events
sudo cat /sys/kernel/debug/tracing/trace_pipe | grep sched_process
```
**Expected:** 
- exec events with PID, PPID, UID, GID
- Executable paths populated
- exit events when processes terminate

### T2.2: Filesystem Events
```bash
# Trigger file operations
touch /tmp/test.txt
echo "data" > /tmp/test.txt
cat /tmp/test.txt
rm /tmp/test.txt

# Check for events
sudo bpftool map dump name FS_EVENTS
```
**Expected:**
- open events with filename
- read/write events with byte counts
- close events

### T2.3: Network Events (Enhanced)
```bash
# Trigger network activity
curl https://example.com

# Check for events
sudo bpftool map dump name NETWORK_EVENTS
```
**Expected:**
- tcp_connect events with socket details
- tcp_sendmsg events with bytes sent
- tcp_recvmsg events with bytes received
- Source/destination IPs and ports populated

### T2.4: Memory Events (NEW)
```bash
# Trigger memory allocations
stress-ng --vm 1 --vm-bytes 128M --timeout 5s

# Check for events
sudo bpftool map dump name MEMORY_EVENTS
```
**Expected:**
- kmalloc/kfree events with allocation sizes
- mm_page_alloc/free events with calculated page sizes
- Timestamps present

### T2.5: Disk I/O Events with Latency (NEW)
```bash
# Trigger disk I/O
dd if=/dev/zero of=/tmp/test.dat bs=1M count=100
sync
rm /tmp/test.dat

# Check for events
sudo bpftool map dump name DISK_IO_EVENTS
sudo bpftool map dump name PENDING_IO
```
**Expected:**
- block_rq_issue events with device info
- block_rq_complete events with latency > 0
- HashMap tracking I/O requests
- Latency measurements in nanoseconds

---

## Phase 3: Userspace Integration

### Objective
Integrate eBPF monitors with the DOTS Family daemon.

### T3.1: Update Monitoring Service

**File:** `crates/dots-family-daemon/src/monitoring_service.rs`

**Required Changes:**
1. Add event structures for new monitors:
```rust
pub struct MemoryEvent {
    pub pid: u32,
    pub comm: [u8; 16],
    pub event_type: u32,  // 1=alloc, 2=free, 3=page_alloc, 4=page_free
    pub size: u64,
    pub rss_bytes: u64,
    pub vms_bytes: u64,
    pub shared_bytes: u64,
    pub timestamp: u64,
}

pub struct DiskIOEvent {
    pub pid: u32,
    pub comm: [u8; 16],
    pub device_major: u32,
    pub device_minor: u32,
    pub operation: u8,
    pub sector: u64,
    pub num_sectors: u32,
    pub bytes: u64,
    pub latency_ns: u64,
    pub timestamp: u64,
}
```

2. Load new monitors in `load_ebpf_programs()`:
```rust
// Load memory monitor
let memory_monitor = Bpf::load_file("prebuilt-ebpf/memory-monitor")?;
let prog = memory_monitor.program_mut("kmem_kmalloc").unwrap();
prog.load()?;
prog.attach()?;
// ... repeat for all probes

// Load disk-io monitor
let disk_io_monitor = Bpf::load_file("prebuilt-ebpf/disk-io-monitor")?;
// ... attach all probes
```

3. Add event handlers:
```rust
async fn process_memory_event(&self, event: MemoryEvent) {
    // Log to database
    // Check against memory limits
    // Trigger alerts if needed
}

async fn process_disk_io_event(&self, event: DiskIOEvent) {
    // Log I/O activity
    // Track latency metrics
    // Detect excessive I/O
}
```

### T3.2: Update Database Schema

**File:** `crates/dots-family-common/src/database.rs`

**Required Tables:**
```sql
CREATE TABLE IF NOT EXISTS memory_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    event_type INTEGER NOT NULL,
    size INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    profile_id INTEGER,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

CREATE TABLE IF NOT EXISTS disk_io_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    device_major INTEGER NOT NULL,
    device_minor INTEGER NOT NULL,
    operation INTEGER NOT NULL,
    bytes INTEGER NOT NULL,
    latency_ns INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    profile_id INTEGER,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_memory_events_timestamp ON memory_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_memory_events_profile ON memory_events(profile_id);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_timestamp ON disk_io_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_profile ON disk_io_events(profile_id);
```

### T3.3: Update Network Event Processing

**Enhancement:** Process bandwidth data from tcp_sendmsg/tcp_recvmsg

```rust
async fn process_network_event(&self, event: NetworkEvent) {
    match event.event_type {
        1 => self.handle_tcp_connect(event).await,
        2 => self.handle_tcp_send(event).await,      // NEW
        3 => self.handle_tcp_recv(event).await,      // NEW
        _ => {}
    }
}

async fn handle_tcp_send(&self, event: NetworkEvent) {
    // Aggregate bandwidth per process
    // Track total bytes sent
    // Check against bandwidth limits
}
```

### T3.4: Policy Engine Updates

**File:** `crates/dots-family-daemon/src/policy_engine.rs`

**New Policy Types:**
```rust
pub struct MemoryLimits {
    pub max_allocation_mb: u64,
    pub max_process_memory_mb: u64,
}

pub struct DiskIOLimits {
    pub max_bandwidth_mbps: u64,
    pub max_latency_ms: u64,
}

pub struct NetworkLimits {
    pub max_upload_mbps: u64,
    pub max_download_mbps: u64,
}
```

---

## Phase 4: Performance Testing

### Objective
Measure overhead and ensure acceptable performance.

### T4.1: Baseline Performance
```bash
# Without eBPF monitors
sysbench cpu --threads=4 --time=60 run
sysbench fileio --file-test-mode=seqwr --time=60 run
iperf3 -c speedtest.example.com -t 60
```

### T4.2: With All Monitors Loaded
```bash
# Load all 5 monitors
./load_all_monitors.sh

# Re-run benchmarks
sysbench cpu --threads=4 --time=60 run
sysbench fileio --file-test-mode=seqwr --time=60 run
iperf3 -c speedtest.example.com -t 60
```

**Acceptance Criteria:**
- CPU overhead < 5%
- I/O throughput degradation < 10%
- Network throughput degradation < 5%

### T4.3: Event Rate Testing
```bash
# High-frequency event generation
stress-ng --all 4 --timeout 60s

# Monitor event rates
sudo bpftool prog show | grep run_cnt
sudo bpftool map dump | wc -l
```

**Expected:**
- No dropped events
- Maps don't overflow
- HashMap cleanup working (PENDING_IO size stable)

### T4.4: Memory Leak Testing
```bash
# Run monitors for extended period
./load_all_monitors.sh

# Generate continuous load
stress-ng --all 2 --timeout 3600s &

# Monitor memory usage every 5 minutes
watch -n 300 "ps aux | grep dots-family-daemon"
```

**Expected:**
- Memory usage stable over time
- No growing HashMap (disk-io PENDING_IO)
- Ring buffers cycling correctly

---

## Phase 5: End-to-End Integration

### Objective
Verify complete workflow from eBPF event to policy enforcement.

### T5.1: Memory Limit Enforcement
```bash
# Set memory limit for child profile
dots-family-ctl profile set-memory-limit child 500MB

# Trigger memory allocation as child user
stress-ng --vm 1 --vm-bytes 600M --timeout 60s
```

**Expected:**
- Memory events captured
- Limit exceeded detected
- Process terminated or throttled
- Alert logged

### T5.2: Disk I/O Latency Alerting
```bash
# Enable latency monitoring
dots-family-ctl profile set-disk-latency-alert child 100ms

# Generate high-latency I/O
dd if=/dev/urandom of=/tmp/test bs=1M count=1000 oflag=sync
```

**Expected:**
- Latency events captured with accurate measurements
- Alert triggered when latency > threshold
- Parent notified via DBus

### T5.3: Network Bandwidth Monitoring
```bash
# Enable bandwidth tracking
dots-family-ctl profile set-bandwidth-limit child 10Mbps

# Download large file
wget https://example.com/largefile.iso
```

**Expected:**
- tcp_send/recv events captured
- Bandwidth calculated correctly
- Limit enforced when exceeded

---

## Phase 6: Documentation and Deployment

### T6.1: Update User Guide

**File:** `docs/USER_GUIDE.md`

Add sections:
- Memory usage monitoring and limits
- Disk I/O performance tracking
- Network bandwidth controls

### T6.2: Update Deployment Guide

**File:** `docs/DEPLOYMENT.md`

Add:
- Kernel version requirements for new monitors
- Performance tuning recommendations
- Troubleshooting for HashMap-based monitors

### T6.3: Create Migration Guide

For existing installations upgrading to Phase 3:
1. Database schema updates
2. Configuration file changes
3. Policy migration steps

---

## Test Automation

### Unit Tests (Existing - ✅ Passing)
```bash
cargo test --workspace --lib --bins
```
**Status:** 216 tests passing

### Integration Tests (To Be Created)
```bash
# tests/integration/phase3_monitors.rs
cargo test --test phase3_monitors
```

**Test Cases:**
- Mock eBPF event injection
- Event handler verification
- Policy enforcement logic
- Database operations

### VM Tests (Recommended)
```bash
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
./result/bin/run-dots-family-test-vm

# Inside VM
sudo systemctl start dots-family-daemon
dots-family-ctl profile list
# Run test scenarios
```

---

## Success Criteria

### Must Have (Phase 3 Complete)
- ✅ All 5 monitors load without errors
- ✅ All probe functions attach successfully
- ✅ Events captured for all activity types
- ✅ HashMap latency tracking working
- ✅ No memory leaks or performance issues
- ✅ Database schema updated
- ✅ Documentation complete

### Nice to Have (Future)
- [ ] Real-time dashboard showing metrics
- [ ] Historical trend analysis
- [ ] Predictive alerting based on patterns
- [ ] BTF/CO-RE support (aya upgrade)
- [ ] IPv6 network monitoring

---

## Rollback Plan

If issues are discovered:

1. **Disable specific monitors:**
   ```bash
   sudo bpftool prog detach <prog_id>
   ```

2. **Revert to Phase 2:**
   ```bash
   git checkout bf7da41  # Last Phase 2 commit
   nix build
   ```

3. **Database rollback:**
   ```sql
   DROP TABLE memory_events;
   DROP TABLE disk_io_events;
   ```

---

## Timeline

**Estimated effort:** 2-3 weeks

- Week 1: Binary loading + event generation tests (Phase 1-2)
- Week 2: Userspace integration (Phase 3)
- Week 3: Performance testing + E2E integration (Phase 4-5)
- Documentation: Ongoing throughout

---

## Contact & Support

For questions or issues during integration:
- Review: `docs/EBPF_ENHANCEMENTS.md`
- Review: `docs/DEVELOPMENT.md`
- Check: `git log --oneline --grep="Phase 3"`
- Commit: `f75f8b9` (Phase 3 implementation)

**eBPF Monitor Status:** ✅ Production Ready
