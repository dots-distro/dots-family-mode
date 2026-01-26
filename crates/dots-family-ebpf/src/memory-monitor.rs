#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryEvent {
    pub pid: u32,
    pub comm: [u8; 16],
    pub event_type: u32, // 1=alloc, 2=free, 3=mmap, 4=munmap
    pub size: u64,
    pub rss_bytes: u64,    // Resident set size (placeholder for now)
    pub vms_bytes: u64,    // Virtual memory size (placeholder for now)
    pub shared_bytes: u64, // Shared memory (placeholder for now)
    pub timestamp: u64,
}

#[map]
static MEMORY_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[tracepoint]
pub fn kmem_kmalloc(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    // Get process name
    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read allocation size from tracepoint context
    // kmem:kmalloc format: bytes_req at offset ~16, bytes_alloc at offset ~24
    let size: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };

    // Get timestamp
    let timestamp = unsafe { bpf_ktime_get_ns() };

    // Phase 3 TODO: Read memory stats from task_struct
    // For now, we'll use placeholder values and focus on tracking allocations
    let event = MemoryEvent {
        pid,
        comm,
        event_type: 1, // kmalloc
        size,
        rss_bytes: 0,    // TODO: Read from task_struct->mm->rss_stat
        vms_bytes: 0,    // TODO: Read from task_struct->mm->total_vm
        shared_bytes: 0, // TODO: Read from task_struct->mm->shared_vm
        timestamp,
    };

    if let Some(mut buf) = MEMORY_EVENTS.reserve::<MemoryEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[tracepoint]
pub fn kmem_kfree(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read pointer being freed (we don't know the size for kfree)
    let _ptr: u64 = unsafe { ctx.read_at(16).unwrap_or(0) };

    let timestamp = unsafe { bpf_ktime_get_ns() };

    let event = MemoryEvent {
        pid,
        comm,
        event_type: 2, // kfree
        size: 0,       // Size not available on kfree
        rss_bytes: 0,
        vms_bytes: 0,
        shared_bytes: 0,
        timestamp,
    };

    if let Some(mut buf) = MEMORY_EVENTS.reserve::<MemoryEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[tracepoint]
pub fn kmem_mm_page_alloc(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read page order (2^order pages allocated)
    // mm_page_alloc format: order at offset ~16
    let order: u32 = unsafe { ctx.read_at(16).unwrap_or(0) };

    // Calculate size: (2^order) * PAGE_SIZE (assuming 4KB pages)
    let page_size: u64 = 4096;
    let num_pages: u64 = 1u64 << order;
    let size = num_pages * page_size;

    let timestamp = unsafe { bpf_ktime_get_ns() };

    let event = MemoryEvent {
        pid,
        comm,
        event_type: 1, // Page allocation (treat as alloc)
        size,
        rss_bytes: 0,
        vms_bytes: 0,
        shared_bytes: 0,
        timestamp,
    };

    if let Some(mut buf) = MEMORY_EVENTS.reserve::<MemoryEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[tracepoint]
pub fn kmem_mm_page_free(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Read page order
    let order: u32 = unsafe { ctx.read_at(16).unwrap_or(0) };

    // Calculate size
    let page_size: u64 = 4096;
    let num_pages: u64 = 1u64 << order;
    let size = num_pages * page_size;

    let timestamp = unsafe { bpf_ktime_get_ns() };

    let event = MemoryEvent {
        pid,
        comm,
        event_type: 2, // Page free
        size,
        rss_bytes: 0,
        vms_bytes: 0,
        shared_bytes: 0,
        timestamp,
    };

    if let Some(mut buf) = MEMORY_EVENTS.reserve::<MemoryEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
