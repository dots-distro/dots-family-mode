# eBPF Monitoring Enhancements

## Implementation Status

### Phase 1: Basic Framework (✅ COMPLETED - Session 7)
- PID, UID, GID extraction using kernel helpers
- Process name (comm) extraction
- Ring buffer event submission
- Basic network and filesystem event capture

### Phase 2: Enhanced Data Extraction (✅ COMPLETED - Sessions 8 & 10)
- ✅ **PPID extraction** (process-monitor.rs) - Parent process ID from tracepoint context
- ✅ **Executable path extraction** (process-monitor.rs) - Full binary path from sched_process_exec filename field
- ✅ **Socket address/port extraction** (network-monitor.rs) - Source/destination IP and ports from struct sock
- ✅ **Filename extraction** (filesystem-monitor.rs) - Full file paths from user space memory

**Binary Size Changes:**
- process-monitor: 1.6K → 4.8K (+200%)
- network-monitor: 1.5K → 2.6K (+73%)
- filesystem-monitor: 2.4K → 6.8K (+183%)

### Phase 3: Advanced Metrics (🔄 PLANNED)
- CPU time tracking per process
- Memory usage monitoring (RSS, VMS, shared)
- Disk I/O metrics (bytes read/written, latency)
- Enhanced network metrics (bandwidth, connections)
- Process scheduling and latency tracking

---

## Current Implementation Details

### Network Monitor (`network-monitor.rs`)
- **Features**: TCP connect events with socket information
- **Data Extracted**: 
  - Process ID (PID) and name (comm)
  - Source and destination IP addresses
  - Source and destination ports
  - Protocol identification (TCP = 6)
- **Method**: Read struct sock fields at approximate offsets (24-34 bytes)
- **Limitations**: Offsets may vary by kernel version (no BTF/CO-RE yet)

### Process Monitor (`process-monitor.rs`)
- **Features**: Process exec/exit events with full context
- **Data Extracted**:
  - Process ID (PID), Parent Process ID (PPID)
  - User ID (UID), Group ID (GID)
  - Process name (comm) - 16 bytes
  - Executable path (cmdline) - 256 bytes
- **Method**: 
  - PPID from tracepoint context offset 12
  - Executable path from __data_loc encoded pointer at offset 8
  - Per-CPU buffer to avoid stack overflow
- **Limitations**: Cmdline reduced to 256 bytes due to eBPF 512-byte stack limit

### Filesystem Monitor (`filesystem-monitor.rs`)
- **Features**: File open/read/write/close events with filenames
- **Data Extracted**:
  - Process ID (PID) and name (comm)
  - File descriptor (FD)
  - Full filename/path - 255 bytes
  - Bytes transferred for I/O operations
  - Operation type (read/write/exec)
- **Method**: 
  - Filename from second argument (ctx.arg::<u64>(1))
  - Read from user space via bpf_probe_read_user_str_bytes
  - Per-CPU buffer for stack safety
- **Limitations**: Only captures open events with filename, not all I/O

---

## Phase 3: Proposed Advanced Enhancements

### 1. Memory Usage Monitoring

Add a new `memory-monitor.rs` eBPF program to track memory consumption:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryEvent {
    pub pid: u32,
    pub event_type: u32,  // 1=alloc, 2=free, 3=mmap, 4=munmap
    pub size: u64,
    pub rss_bytes: u64,         // Resident set size
    pub vms_bytes: u64,         // Virtual memory size
    pub shared_bytes: u64,      // Shared memory
    pub timestamp: u64,
}
```

**Tracepoints to attach:**
- `kmem:kmalloc` - Memory allocation
- `kmem:kfree` - Memory free
- `kmem:mm_page_alloc` - Page allocation
- `kmem:mm_page_free` - Page free
- `syscalls:sys_enter_mmap` - Memory mapping
- `syscalls:sys_exit_munmap` - Memory unmapping

**Benefits:**
- Track per-process memory usage over time
- Identify memory-intensive applications
- Detect memory leaks
- Enforce memory limits for child profiles

**Implementation Strategy:**
```rust
#[tracepoint]
pub fn kmalloc(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid & 0xFFFFFFFF) as u32;
    
    // Read size from tracepoint context
    let size: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };
    
    // Read memory stats from /proc/[pid]/status via BPF helpers
    let rss = bpf_probe_read_kernel(&task.mm.rss_stat);
    
    let event = MemoryEvent {
        pid,
        event_type: 1,
        size,
        rss_bytes: rss * PAGE_SIZE,
        vms_bytes: 0, // Calculate from task struct
        shared_bytes: 0,
        timestamp: bpf_ktime_get_ns(),
    };
    
    if let Some(mut buf) = MEMORY_EVENTS.reserve::<MemoryEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    0
}
```

### 2. Disk I/O Monitoring

Add a new `disk-io-monitor.rs` eBPF program to track disk operations:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskIOEvent {
    pub pid: u32,
    pub device_major: u32,
    pub device_minor: u32,
    pub operation: u8,      // 0=read, 1=write
    pub sector: u64,
    pub num_sectors: u32,
    pub bytes: u64,
    pub latency_ns: u64,    // I/O latency
    pub timestamp: u64,
}
```

**Tracepoints to attach:**
- `block:block_rq_issue` - Block I/O request issued
- `block:block_rq_complete` - Block I/O request completed
- `block:block_bio_queue` - Block I/O queued
- `block:block_bio_complete` - Block I/O completed

**Benefits:**
- Track per-process disk I/O
- Identify disk-intensive applications
- Measure I/O latency
- Detect excessive disk usage

**Implementation Strategy:**
```rust
// Store pending requests in a map
#[map]
static PENDING_IO: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);

#[tracepoint]
pub fn block_rq_issue(ctx: TracePointContext) -> u32 {
    let request_ptr = ctx.arg::<u64>(0).unwrap_or(0);
    let timestamp = bpf_ktime_get_ns();
    
    // Store request start time
    PENDING_IO.insert(&request_ptr, &timestamp, 0).ok();
    
    0
}

#[tracepoint]
pub fn block_rq_complete(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid & 0xFFFFFFFF) as u32;
    
    let request_ptr = ctx.arg::<u64>(0).unwrap_or(0);
    let current_time = bpf_ktime_get_ns();
    
    // Calculate latency
    let start_time = PENDING_IO.get(&request_ptr).copied().unwrap_or(0);
    let latency = if start_time > 0 {
        current_time - start_time
    } else {
        0
    };
    
    // Read request details from context
    let bytes: u64 = ctx.read_at(32).unwrap_or(0);
    let operation: u8 = ctx.read_at(40).unwrap_or(0);
    
    let event = DiskIOEvent {
        pid,
        device_major: 0,  // Read from bio struct
        device_minor: 0,
        operation,
        sector: 0,
        num_sectors: 0,
        bytes,
        latency_ns: latency,
        timestamp: current_time,
    };
    
    if let Some(mut buf) = DISK_IO_EVENTS.reserve::<DiskIOEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    // Clean up
    PENDING_IO.remove(&request_ptr).ok();
    
    0
}
```

### 3. Enhanced Network Monitoring

Improve existing `network-monitor.rs` with real data extraction:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkEvent {
    pub event_type: u32,
    pub pid: u32,
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub comm: [u8; 16],      // Process name
    pub timestamp: u64,
}

#[kprobe]
pub fn tcp_connect(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid & 0xFFFFFFFF) as u32;
    
    // Read socket structure from kernel
    // struct sock *sk is first argument
    let sk_ptr = ctx.arg::<u64>(0).unwrap_or(0);
    if sk_ptr == 0 {
        return 0;
    }
    
    // Extract socket information
    // offset based on kernel struct sock definition
    let src_addr: u32 = unsafe {
        bpf_probe_read_kernel(&*(sk_ptr as *const u32).add(12))
            .unwrap_or(0)
    };
    
    let dst_addr: u32 = unsafe {
        bpf_probe_read_kernel(&*(sk_ptr as *const u32).add(16))
            .unwrap_or(0)
    };
    
    let src_port: u16 = unsafe {
        bpf_probe_read_kernel(&*(sk_ptr as *const u16).add(20))
            .unwrap_or(0)
    };
    
    let dst_port: u16 = unsafe {
        bpf_probe_read_kernel(&*(sk_ptr as *const u16).add(22))
            .unwrap_or(0)
    };
    
    // Get process name
    let mut comm = [0u8; 16];
    bpf_get_current_comm(&mut comm).ok();
    
    let event = NetworkEvent {
        event_type: 1,
        pid,
        src_addr: u32::from_be(src_addr),
        dst_addr: u32::from_be(dst_addr),
        src_port: u16::from_be(src_port),
        dst_port: u16::from_be(dst_port),
        protocol: 6, // TCP
        bytes_sent: 0,
        bytes_received: 0,
        comm,
        timestamp: bpf_ktime_get_ns(),
    };
    
    if let Some(mut buf) = NETWORK_EVENTS.reserve::<NetworkEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    0
}
```

**Additional Network Enhancements:**
- Track UDP connections via `udp_sendmsg`
- Monitor DNS queries
- Capture SNI from TLS connections
- Track bandwidth per process
- Detect port scanning

### 4. Enhanced Process Monitoring

Improve existing `process-monitor.rs` with actual data:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub cmdline: [u8; 512],
    pub event_type: u32,
    pub exit_code: i32,      // For exit events
    pub timestamp: u64,
    pub cpu_time_us: u64,    // CPU time used
}

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    let pid = ctx.read_at::<u32>(8).unwrap_or(0);
    let ppid = ctx.read_at::<u32>(12).unwrap_or(0);
    
    // Get current task struct
    let task = bpf_get_current_task() as *const task_struct;
    
    let uid = unsafe {
        bpf_probe_read_kernel(&(*task).cred.uid.val).unwrap_or(0)
    };
    
    let gid = unsafe {
        bpf_probe_read_kernel(&(*task).cred.gid.val).unwrap_or(0)
    };
    
    let mut comm = [0u8; 16];
    bpf_get_current_comm(&mut comm).ok();
    
    // Read command line arguments
    let mut cmdline = [0u8; 512];
    // Read from task->mm->arg_start
    
    let event = ProcessEvent {
        pid,
        ppid,
        uid,
        gid,
        comm,
        cmdline,
        event_type: 0, // exec
        exit_code: 0,
        timestamp: bpf_ktime_get_ns(),
        cpu_time_us: 0,
    };
    
    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    0
}
```

### 5. Enhanced Filesystem Monitoring

Improve existing `filesystem-monitor.rs` with filenames:

```rust
#[kprobe]
pub fn trace_do_sys_open(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid & 0xFFFFFFFF) as u32;
    
    // Get filename pointer from syscall argument
    let filename_ptr = ctx.arg::<*const u8>(0).unwrap_or(core::ptr::null());
    
    let mut filename = [0u8; 255];
    if !filename_ptr.is_null() {
        // Read filename from user space
        bpf_probe_read_user_str(&mut filename, filename_ptr).ok();
    }
    
    let fd = ctx.ret().unwrap_or(0) as u32;
    
    let event = FilesystemEvent {
        event_type: 1,
        pid,
        fd,
        filename,
        bytes_transferred: 0,
        operation: 0,
    };
    
    if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    0
}
```

## Implementation Roadmap

### Phase 1: Enhanced Data Extraction (Week 1)
- [ ] Fix network monitor to extract real socket data
- [ ] Fix process monitor to extract task struct data
- [ ] Fix filesystem monitor to extract filenames
- [ ] Add process names to all events
- [ ] Add timestamps to all events

### Phase 2: Memory Monitoring (Week 2)
- [ ] Create `memory-monitor.rs` stub
- [ ] Implement kmalloc/kfree tracking
- [ ] Implement mmap/munmap tracking
- [ ] Add RSS/VMS stat extraction
- [ ] Create user-space consumer in daemon
- [ ] Add memory usage to profile limits

### Phase 3: Disk I/O Monitoring (Week 3)
- [ ] Create `disk-io-monitor.rs` stub
- [ ] Implement block I/O tracking
- [ ] Add latency measurement
- [ ] Create per-process I/O aggregation
- [ ] Add disk I/O to activity reports

### Phase 4: Advanced Network Features (Week 4)
- [ ] Add UDP monitoring
- [ ] Implement DNS query tracking
- [ ] Add bandwidth tracking
- [ ] Implement connection state tracking
- [ ] Add network usage to profile limits

### Phase 5: Integration & Testing (Week 5)
- [ ] Update monitoring service to consume new events
- [ ] Add new metrics to database schema
- [ ] Create dashboard visualizations
- [ ] Performance testing and optimization
- [ ] Documentation updates

## Technical Challenges

### 1. Kernel Structure Offsets
**Challenge**: Kernel structures change between versions
**Solution**: Use BTF (BPF Type Format) with CO-RE (Compile Once, Run Everywhere)

```rust
use aya_ebpf::cty::*;
use aya_ebpf::helpers::*;

// Use CO-RE to access kernel structs
#[repr(C)]
struct sock {
    __sk_common: sock_common,
    // ... other fields
}

// BTF-enabled field access
let src_addr = BPF_CORE_READ(&sk, __sk_common.skc_rcv_saddr);
```

### 2. Ring Buffer Overflow
**Challenge**: High-frequency events can overflow ring buffers
**Solution**: 
- Implement sampling (e.g., 1 in 100 events)
- Use larger ring buffers (1-4MB)
- Implement per-CPU buffers
- Add event filtering in kernel space

### 3. Performance Impact
**Challenge**: eBPF overhead on system performance
**Solution**:
- Profile eBPF program execution time
- Optimize hot paths
- Use maps for aggregation instead of per-event reporting
- Implement adaptive sampling based on load

### 4. Security Implications
**Challenge**: eBPF programs run with kernel privileges
**Solution**:
- Thorough code review
- BPF verifier ensures safety
- Capability restrictions (CAP_BPF)
- Input validation on all kernel reads

## Testing Strategy

### Unit Tests
- Test event structure serialization
- Test ring buffer operations
- Mock kernel structures for testing

### Integration Tests
```bash
# Load eBPF programs
sudo bpftool prog load memory-monitor.o /sys/fs/bpf/memory-monitor

# Verify events
sudo bpftool map dump name MEMORY_EVENTS

# Performance testing
sudo perf stat -e bpf:bpf_prog_load ./test-workload
```

### VM Testing
Use the existing NixOS VM tests to verify:
- eBPF programs load correctly
- Events are captured
- No kernel panics
- Acceptable performance overhead (<5%)

## Metrics & Monitoring

Track eBPF program performance:
```bash
# Check program stats
sudo bpftool prog show

# Monitor map usage
sudo bpftool map show

# Check for dropped events
cat /sys/kernel/debug/tracing/trace_pipe
```

## Documentation

Update:
- [ ] `docs/MONITORING.md` - Add new metrics
- [ ] `docs/DEPLOYMENT.md` - Update kernel requirements
- [ ] `docs/ARCHITECTURE.md` - Document eBPF architecture
- [ ] Create `docs/EBPF_DEVELOPMENT.md` - Development guide

## References

- [eBPF Documentation](https://ebpf.io/what-is-ebpf/)
- [Aya Framework](https://aya-rs.dev/)
- [BPF CO-RE](https://nakryiko.com/posts/bpf-portability-and-co-re/)
- [Kernel Tracepoints](https://www.kernel.org/doc/html/latest/trace/tracepoints.html)
- [BPF Performance Tools](http://www.brendangregg.com/bpf-performance-tools-book.html)
