#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::{HashMap, RingBuf},
    programs::TracePointContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskIOEvent {
    pub pid: u32,
    pub comm: [u8; 16],
    pub device_major: u32,
    pub device_minor: u32,
    pub operation: u8, // 0=read, 1=write
    pub sector: u64,
    pub num_sectors: u32,
    pub bytes: u64,
    pub latency_ns: u64, // I/O latency
    pub timestamp: u64,
}

#[map]
static DISK_IO_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

// Store pending I/O requests to calculate latency
// Key: request pointer, Value: (timestamp, pid)
#[map]
static PENDING_IO: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);

#[tracepoint]
pub fn block_rq_issue(ctx: TracePointContext) -> u32 {
    // Read device info from tracepoint context
    // block_rq_issue format: dev (u32) at offset ~8, sector (u64) at offset ~16
    let dev: u32 = unsafe { ctx.read_at(8).unwrap_or(0) };
    let sector: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };

    // Use sector as key (unique per request)
    // Store timestamp for latency calculation
    let timestamp = unsafe { bpf_ktime_get_ns() };

    // Store the start time in the map
    unsafe {
        let _ = PENDING_IO.insert(&sector, &timestamp, 0);
    }

    0
}

#[tracepoint]
pub fn block_rq_complete(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read request details from tracepoint context
    // block_rq_complete format: dev (u32) at ~8, sector (u64) at ~16, nr_sector (u32) at ~24
    let dev: u32 = unsafe { ctx.read_at(8).unwrap_or(0) };
    let sector: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };
    let nr_sectors: u32 = unsafe { ctx.read_at(24).unwrap_or(0) };

    // Extract device major and minor numbers
    let device_major = dev >> 20; // Top 12 bits
    let device_minor = dev & 0xFFFFF; // Bottom 20 bits

    // Calculate bytes transferred (sectors * 512)
    let bytes = (nr_sectors as u64) * 512;

    // Get current timestamp
    let current_time = unsafe { bpf_ktime_get_ns() };

    // Calculate latency by looking up start time
    let start_time = unsafe { PENDING_IO.get(&sector).copied().unwrap_or(0) };
    let latency = if start_time > 0 { current_time.saturating_sub(start_time) } else { 0 };

    // Try to read operation type from context (offset may vary)
    // 0 = read, 1 = write
    let operation: u8 = unsafe { ctx.read_at(28).unwrap_or(0) };

    let event = DiskIOEvent {
        pid,
        comm,
        device_major,
        device_minor,
        operation,
        sector,
        num_sectors: nr_sectors,
        bytes,
        latency_ns: latency,
        timestamp: current_time,
    };

    if let Some(mut buf) = DISK_IO_EVENTS.reserve::<DiskIOEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    // Clean up the pending request
    unsafe {
        let _ = PENDING_IO.remove(&sector);
    }

    0
}

#[tracepoint]
pub fn block_bio_queue(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read bio details
    let dev: u32 = unsafe { ctx.read_at(8).unwrap_or(0) };
    let sector: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };
    let nr_sectors: u32 = unsafe { ctx.read_at(24).unwrap_or(0) };

    let device_major = dev >> 20;
    let device_minor = dev & 0xFFFFF;

    let bytes = (nr_sectors as u64) * 512;
    let timestamp = unsafe { bpf_ktime_get_ns() };

    // Bio queue events don't have latency (not completed yet)
    let event = DiskIOEvent {
        pid,
        comm,
        device_major,
        device_minor,
        operation: 0, // Unknown at queue time
        sector,
        num_sectors: nr_sectors,
        bytes,
        latency_ns: 0, // Not completed yet
        timestamp,
    };

    if let Some(mut buf) = DISK_IO_EVENTS.reserve::<DiskIOEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
