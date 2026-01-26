#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_probe_read_user_str_bytes},
    macros::{kprobe, map},
    maps::{PerCpuArray, RingBuf},
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FilesystemEvent {
    pub event_type: u32, // 1 = open, 2 = read, 3 = write, 4 = close
    pub pid: u32,
    pub comm: [u8; 16], // Process name
    pub fd: u32,
    pub filename: [u8; 255],    // Full file path
    pub bytes_transferred: u64, // For file I/O operations
    pub operation: u8,          // 0 = read, 1 = write, 2 = exec
}

#[map]
static FS_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

// Per-CPU buffer to avoid stack overflow
#[map]
static FILENAME_BUF: PerCpuArray<[u8; 255]> = PerCpuArray::with_max_entries(1, 0);

#[kprobe]
pub fn trace_do_sys_open(ctx: ProbeContext) -> u32 {
    // bpf_get_current_pid_tgid() returns u64 where high 32 bits = tgid, low 32 bits = pid
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32; // TGID (actual process ID)

    // Get process name
    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Get file descriptor from context (returns Option)
    let fd = ctx.arg::<u32>(0).unwrap_or(0);

    // Phase 2: Extract filename from user space
    // Use per-CPU buffer to avoid stack overflow
    if let Some(filename_buf) = FILENAME_BUF.get_ptr_mut(0) {
        let filename_buf = unsafe { &mut *filename_buf };
        *filename_buf = [0u8; 255];

        // The second argument is the filename pointer (char __user *)
        if let Some(filename_ptr) = ctx.arg::<u64>(1) {
            if filename_ptr != 0 {
                // Read filename from user space memory
                let _ = unsafe {
                    bpf_probe_read_user_str_bytes(filename_ptr as *const u8, filename_buf)
                };
            }
        }

        let event = FilesystemEvent {
            event_type: 1, // Open event
            pid,
            comm,
            fd,
            filename: *filename_buf,
            bytes_transferred: 0,
            operation: 0, // Read operation
        };

        if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
            buf.write(event);
            buf.submit(0);
        }
    }

    0
}

#[kprobe]
pub fn trace_do_sys_read(ctx: ProbeContext) -> u32 {
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32;

    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Get file descriptor from context
    let fd = ctx.arg::<u32>(0).unwrap_or(0);
    let count = ctx.arg::<u64>(2).unwrap_or(0); // Number of bytes to read

    let event = FilesystemEvent {
        event_type: 2, // Read event
        pid,
        comm,
        fd,
        filename: [0; 255], // Process reading from file, not filename directly
        bytes_transferred: count,
        operation: 0, // Read operation
    };

    if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[kprobe]
pub fn trace_do_sys_write(ctx: ProbeContext) -> u32 {
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32;

    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Get file descriptor from context
    let fd = ctx.arg::<u32>(0).unwrap_or(0);
    let count = ctx.arg::<u64>(2).unwrap_or(0); // Number of bytes to write

    let event = FilesystemEvent {
        event_type: 3, // Write event
        pid,
        comm,
        fd,
        filename: [0; 255], // Process writing to file, not filename directly
        bytes_transferred: count,
        operation: 1, // Write operation
    };

    if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[kprobe]
pub fn trace_do_sys_close(ctx: ProbeContext) -> u32 {
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32;

    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Get file descriptor from context
    let fd = ctx.arg::<u32>(0).unwrap_or(0);

    let event = FilesystemEvent {
        event_type: 4, // Close event
        pid,
        comm,
        fd,
        filename: [0; 255], // Process closing file, not filename directly
        bytes_transferred: 0,
        operation: 4, // Close operation
    };

    if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
