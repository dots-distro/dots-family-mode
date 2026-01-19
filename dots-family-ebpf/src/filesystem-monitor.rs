#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_get_current_uid_gid},
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FilesystemEvent {
    pub event_type: u32, // 1 = open, 2 = read, 3 = write, 4 = close
    pub pid: u32,
    pub fd: u32,
    pub filename: [u8; 255],    // Full file path
    pub bytes_transferred: u64, // For file I/O operations
    pub operation: u8,          // 0 = read, 1 = write, 2 = exec
}

#[map]
static FS_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[kprobe]
pub fn trace_do_sys_open(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();

    // Get file descriptor from context
    let fd = ctx.arg(0); // The file descriptor being opened

    // Get filename from context using helper function
    let filename = {
        let mut name = [0u8; 255];
        let mut i = 0;

        unsafe {
            let filename_ptr = ctx.arg(1); // Filename is second argument
            if !filename_ptr.is_null() {
                let arg_len = bpf_probe_read_user_str(filename_ptr, &mut name[i], 254);
                if arg_len > 0 && i + arg_len < 255 {
                    i += arg_len + 1;
                }
            }
        }
    };

    let event = FilesystemEvent {
        event_type: 1, // Open event
        pid: pid_tgid.pid,
        fd,
        filename,
        bytes_transferred: 0,
        operation: 0, // Read operation
    };

    if let Some(mut buf) = FS_EVENTS.reserve::<FilesystemEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[kprobe]
pub fn trace_do_sys_read(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();

    // Get file descriptor from context
    let fd = ctx.arg(0);
    let count = ctx.arg(1); // Number of bytes to read
    let pos = ctx.arg(2); // Position in file

    let event = FilesystemEvent {
        event_type: 2, // Read event
        pid: pid_tgid.pid,
        fd,
        filename: [0; 255], // Process reading from file, not filename directly
        bytes_transferred: count as u64,
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
    let pid_tgid = bpf_get_current_pid_tgid();

    // Get file descriptor from context
    let fd = ctx.arg(0);
    let count = ctx.arg(1); // Number of bytes to write
    let pos = ctx.arg(2); // Position in file

    let event = FilesystemEvent {
        event_type: 3, // Write event
        pid: pid_tgid.pid,
        fd,
        filename: [0; 255], // Process writing to file, not filename directly
        bytes_transferred: count as u64,
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
    let pid_tgid = bpf_get_current_pid_tgid();

    // Get file descriptor from context
    let fd = ctx.arg(0);

    let event = FilesystemEvent {
        event_type: 4, // Close event
        pid: pid_tgid.pid,
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
