# eBPF Monitors Quick Reference

Quick reference guide for all DOTS Family Mode eBPF monitoring programs.

## Monitor Overview

| Monitor | Size | Type | Probes | Phase | Purpose |
|---------|------|------|--------|-------|---------|
| process-monitor | 4.8K | tracepoint | 2 | Phase 2 | Process lifecycle tracking |
| filesystem-monitor | 6.8K | kprobe | 4 | Phase 2 | File operations monitoring |
| network-monitor | 5.5K | kprobe | 3 | Phase 3 | TCP connections and bandwidth |
| memory-monitor | 5.7K | tracepoint | 4 | Phase 3 | Memory allocation tracking |
| disk-io-monitor | 4.6K | tracepoint | 3 | Phase 3 | Block device I/O and latency |
| **TOTAL** | **27.4K** | - | **16** | - | Comprehensive system monitoring |

## Binary Locations

All prebuilt eBPF binaries are located in:
```
prebuilt-ebpf/
├── process-monitor
├── filesystem-monitor
├── network-monitor
├── memory-monitor
└── disk-io-monitor
```

## Process Monitor

**File:** `crates/dots-family-ebpf/src/process-monitor.rs`  
**Binary:** `prebuilt-ebpf/process-monitor`  
**Size:** 4.8K

### Probe Points
- `sched_process_exec` - Process execution
- `sched_process_exit` - Process termination

### Event Structure
```rust
struct ProcessEvent {
    pid: u32,           // Process ID
    ppid: u32,          // Parent process ID
    uid: u32,           // User ID
    comm: [u8; 16],     // Command name (first 16 chars)
    event_type: u8,     // 0=exec, 1=exit
}
```

### Usage Example
```bash
# Load program
sudo bpftool prog load prebuilt-ebpf/process-monitor /sys/fs/bpf/process-monitor

# View events (from userspace)
# Events sent to perf ring buffer for daemon consumption
```

### Key Features
- Tracks all process creation and termination
- Captures parent-child relationships
- Essential for application control

---

## Filesystem Monitor

**File:** `crates/dots-family-ebpf/src/filesystem-monitor.rs`  
**Binary:** `prebuilt-ebpf/filesystem-monitor`  
**Size:** 6.8K

### Probe Points
- `vfs_open` - File open operations
- `vfs_read` - File read operations
- `vfs_write` - File write operations
- `vfs_unlink` - File deletion operations

### Event Structure
```rust
struct FilesystemEvent {
    pid: u32,           // Process ID
    uid: u32,           // User ID
    operation: u8,      // 0=open, 1=read, 2=write, 3=unlink
    filename: [u8; 256], // Full path (up to 256 chars)
}
```

### Usage Example
```bash
# Load program
sudo bpftool prog load prebuilt-ebpf/filesystem-monitor /sys/fs/bpf/filesystem-monitor

# Monitor specific operations
# Configure daemon to filter by path patterns
```

### Key Features
- Tracks all VFS layer operations
- Full pathname extraction (256 bytes)
- Critical for content filtering

---

## Network Monitor

**File:** `crates/dots-family-ebpf/src/network-monitor.rs`  
**Binary:** `prebuilt-ebpf/network-monitor`  
**Size:** 5.5K

### Probe Points
- `tcp_v4_connect` - TCP connection attempts
- `tcp_sendmsg` - Outbound data transmission
- `tcp_recvmsg` - Inbound data reception

### Event Structure
```rust
struct NetworkEvent {
    pid: u32,           // Process ID
    uid: u32,           // User ID
    saddr: u32,         // Source IPv4 address
    daddr: u32,         // Destination IPv4 address
    sport: u16,         // Source port
    dport: u16,         // Destination port
    event_type: u8,     // 0=connect, 1=send, 2=recv
    bytes: u64,         // Bytes transferred (for send/recv)
}
```

### Usage Example
```bash
# Load program
sudo bpftool prog load prebuilt-ebpf/network-monitor /sys/fs/bpf/network-monitor

# Track bandwidth per process
# Monitor connection destinations
```

### Key Features
- TCP connection tracking
- Bandwidth monitoring (TX/RX)
- Essential for web filtering
- IPv4 only (no IPv6 support yet)

---

## Memory Monitor

**File:** `crates/dots-family-ebpf/src/memory-monitor.rs`  
**Binary:** `prebuilt-ebpf/memory-monitor`  
**Size:** 5.7K

### Probe Points
- `kmem_kmalloc` - Kernel memory allocations
- `kmem_kfree` - Kernel memory frees
- `kmem_mm_page_alloc` - Page allocations
- `kmem_mm_page_free` - Page frees

### Event Structure
```rust
struct MemoryEvent {
    pid: u32,           // Process ID (0 for kernel)
    size: u64,          // Allocation size in bytes
    event_type: u8,     // 0=kmalloc, 1=kfree, 2=page_alloc, 3=page_free
    order: u32,         // Page order (for page events)
}
```

### Usage Example
```bash
# Load program
sudo bpftool prog load prebuilt-ebpf/memory-monitor /sys/fs/bpf/memory-monitor

# Track memory pressure
# Identify memory-intensive processes
```

### Key Features
- Kernel-level allocation tracking
- Page order to bytes conversion: `size = (2^order) * 4096`
- System resource monitoring
- Useful for detecting resource abuse

---

## Disk I/O Monitor

**File:** `crates/dots-family-ebpf/src/disk-io-monitor.rs`  
**Binary:** `prebuilt-ebpf/disk-io-monitor`  
**Size:** 4.6K

### Probe Points
- `block_rq_issue` - I/O request issued
- `block_rq_complete` - I/O request completed
- `block_bio_queue` - Bio queued to device

### Event Structure
```rust
struct DiskIoEvent {
    pid: u32,           // Process ID
    dev: u32,           // Device ID
    sector: u64,        // Disk sector
    nr_sector: u32,     // Number of sectors
    latency: u64,       // I/O latency in nanoseconds (for complete)
    event_type: u8,     // 0=issue, 1=complete, 2=bio_queue
}
```

### Usage Example
```bash
# Load program
sudo bpftool prog load prebuilt-ebpf/disk-io-monitor /sys/fs/bpf/disk-io-monitor

# Monitor I/O latency
# Track per-process disk usage
```

### Key Features
- Stateful latency tracking (uses HashMap)
- Sector-level granularity
- Device-specific monitoring
- Performance analysis

---

## Build Instructions

### Prerequisites
```bash
# Install rustup nightly (NOT Nix rust)
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

### Build All Monitors
```bash
cd crates/dots-family-ebpf

# Use rustup nightly explicitly
export PATH="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH"

# Build for eBPF target
cargo +nightly build --release --target bpfel-unknown-none -Z build-std=core

# Copy binaries to prebuilt directory
cp target/bpfel-unknown-none/release/*-monitor ../../prebuilt-ebpf/
```

### Build Individual Monitor
```bash
cd crates/dots-family-ebpf

# Build specific binary
cargo +nightly build --release --bin network-monitor \
  --target bpfel-unknown-none -Z build-std=core

# Copy to prebuilt
cp target/bpfel-unknown-none/release/network-monitor ../../prebuilt-ebpf/
```

### Verify Binary
```bash
cd prebuilt-ebpf

# Check ELF format
file network-monitor
# Output: network-monitor: ELF 64-bit LSB relocatable, eBPF, version 1 (SYSV)

# Check sections
readelf -S network-monitor | grep -E "tracepoint|kprobe"

# Check symbols
readelf -s network-monitor | grep -E "tcp_|vfs_|kmem_|block_"
```

---

## Loading and Management

### Load eBPF Program
```bash
# Basic load
sudo bpftool prog load prebuilt-ebpf/network-monitor /sys/fs/bpf/network-monitor

# Load with map pinning
sudo bpftool prog load prebuilt-ebpf/disk-io-monitor /sys/fs/bpf/disk-io-monitor \
  map name PENDING_IO pinned /sys/fs/bpf/pending_io_map
```

### List Loaded Programs
```bash
# List all eBPF programs
sudo bpftool prog list

# Show specific program
sudo bpftool prog show name network_monitor

# Dump program instructions
sudo bpftool prog dump xlated name network_monitor
```

### Manage Maps
```bash
# List all maps
sudo bpftool map list

# Show map contents
sudo bpftool map dump name PENDING_IO

# Update map entry
sudo bpftool map update name EVENTS_MAP key 0 value 1
```

### Unload Programs
```bash
# Remove pinned program
sudo rm /sys/fs/bpf/network-monitor

# Or use bpftool
sudo bpftool prog detach /sys/fs/bpf/network-monitor
```

---

## Integration with Daemon

### Event Flow
```
eBPF Kernel Program
    ↓ (perf ring buffer)
Userspace Daemon (Rust + aya)
    ↓ (process events)
Policy Engine
    ↓ (enforce rules)
Action (allow/block/log)
```

### Daemon Integration Points

1. **Load Programs at Startup**
   ```rust
   // In daemon initialization
   let process_monitor = Bpf::load_file("prebuilt-ebpf/process-monitor")?;
   process_monitor.attach()?;
   ```

2. **Consume Events**
   ```rust
   // Set up event handlers
   let perf_array = PerfEventArray::try_from(bpf.map("EVENTS")?)?;
   
   for cpu in 0..num_cpus {
       let mut buf = perf_array.open(cpu, None)?;
       // Read events from buf
   }
   ```

3. **Apply Policy**
   ```rust
   // Process event and check policy
   match event.event_type {
       ProcessEvent::Exec => check_app_allowed(event.comm),
       NetworkEvent::Connect => check_domain_allowed(event.daddr),
       // ... etc
   }
   ```

### Database Schema Extensions

See `docs/PHASE3_INTEGRATION_PLAN.md` for:
- New activity log tables
- Metrics aggregation tables
- Index recommendations
- Migration scripts

---

## Performance Considerations

### Overhead
- Process Monitor: ~1-2% CPU (process-heavy workloads)
- Filesystem Monitor: ~2-5% CPU (I/O-heavy workloads)
- Network Monitor: ~1-3% CPU (network-heavy workloads)
- Memory Monitor: ~3-7% CPU (allocation-heavy workloads)
- Disk I/O Monitor: ~2-4% CPU (I/O-heavy workloads)

### Memory Usage
- Each monitor: ~50-100KB resident memory
- HashMap overhead: ~1MB per 10,000 entries (disk-io-monitor)
- Ring buffer: 128KB per CPU (configurable)

### Optimization Tips
1. Use appropriate ring buffer sizes
2. Filter events in kernel space when possible
3. Batch userspace processing
4. Use indexes on database tables
5. Aggregate metrics periodically

---

## Troubleshooting

### Program Won't Load
```bash
# Check kernel version (need 5.10+)
uname -r

# Check BTF support
ls -la /sys/kernel/btf/vmlinux

# Enable verbose logging
sudo bpftool -d prog load prebuilt-ebpf/network-monitor /sys/fs/bpf/network-monitor
```

### No Events Received
```bash
# Check program is attached
sudo bpftool prog list | grep monitor

# Check map statistics
sudo bpftool map show

# Verify probe points exist
sudo cat /sys/kernel/debug/tracing/available_events | grep -E "sched|kmem|block"
sudo cat /sys/kernel/debug/tracing/available_filter_functions | grep -E "tcp_|vfs_"
```

### High CPU Usage
```bash
# Check event rate
sudo bpftool prog show | grep -A3 network_monitor

# Reduce ring buffer size
# Modify daemon configuration to use smaller buffers

# Add kernel-side filtering
# Modify eBPF programs to filter more events in kernel
```

### Permission Errors
```bash
# Ensure CAP_BPF and CAP_PERFMON
sudo setcap cap_bpf,cap_perfmon+ep /path/to/daemon

# Or run as root
sudo /path/to/daemon
```

---

## Testing

### Unit Tests
```bash
# Run all eBPF crate tests
cd crates/dots-family-ebpf
cargo test --workspace --lib --bins

# Test specific monitor
cargo test --bin network-monitor
```

### Integration Tests
See `docs/PHASE3_INTEGRATION_PLAN.md` for comprehensive testing strategy:
- Phase 1: Binary loading tests
- Phase 2: Event generation tests
- Phase 3: Userspace integration tests
- Phase 4: Policy enforcement tests
- Phase 5: Performance testing
- Phase 6: End-to-end testing

### Manual Testing
```bash
# Test process monitor
ls -la  # Should generate exec event
sleep 5 & # Should generate exec + exit events

# Test filesystem monitor
echo "test" > /tmp/test.txt  # write event
cat /tmp/test.txt            # read event
rm /tmp/test.txt             # unlink event

# Test network monitor
curl https://example.com     # connect + send/recv events

# Test memory monitor
# Memory allocations happen automatically
stress-ng --vm 1 --vm-bytes 100M --timeout 10s

# Test disk I/O monitor
dd if=/dev/zero of=/tmp/test.dat bs=1M count=100
rm /tmp/test.dat
```

---

## Advanced Patterns

### HashMap for Stateful Tracking
```rust
#[map]
static PENDING_IO: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);

// Store state on entry
unsafe { PENDING_IO.insert(&key, &value, 0); }

// Retrieve state on exit
let start = unsafe { PENDING_IO.get(&key).copied().unwrap_or(0) };
```

### PerCpuArray for Large Buffers
```rust
#[map]
static BUFFER: PerCpuArray<[u8; 256]> = PerCpuArray::with_max_entries(1, 0);

// Avoid stack overflow (512-byte limit)
let buf = unsafe { BUFFER.get_ptr_mut(0).ok_or(0)?.as_mut() };
```

### Reading Struct Fields at Offsets
```rust
// Read at specific offset (no BTF/CO-RE)
let pid: u32 = unsafe { ctx.read_at(0).unwrap_or(0) };
let size: u64 = unsafe { ctx.read_at(8).unwrap_or(0) };
let order: u32 = unsafe { ctx.read_at(16).unwrap_or(0) };
```

---

## References

- **EBPF_ENHANCEMENTS.md** - Full Phase 1-3 implementation history
- **PHASE3_INTEGRATION_PLAN.md** - Integration testing guide
- **DEVELOPMENT.md** - Development environment setup
- **ARCHITECTURE.md** - System architecture overview

---

**Last Updated:** January 26, 2026  
**eBPF Status:** Phase 3 Complete (5 monitors, 16 probes, 27.4KB)  
**Next Phase:** Userspace Integration Testing
