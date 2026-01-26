#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_probe_read_kernel, bpf_probe_read_kernel_str_bytes,
    },
    macros::{map, tracepoint},
    maps::{PerCpuArray, RingBuf},
    programs::TracePointContext,
    EbpfContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub cmdline: [u8; 256], // Reduced from 512 to fit eBPF stack limit
    pub event_type: u32,
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

// Per-CPU buffer to avoid stack overflow (eBPF has 512 byte stack limit)
// We use this to temporarily store the cmdline before copying to the event
#[map]
static CMDLINE_BUF: PerCpuArray<[u8; 256]> = PerCpuArray::with_max_entries(1, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    // Extract PID and TGID (thread group ID, which is the process ID)
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32; // TGID (actual process ID)
    let tid = (pid_tgid & 0xFFFFFFFF) as u32; // TID (thread ID)

    // Extract UID and GID
    let uid_gid = unsafe { bpf_get_current_uid_gid() };
    let uid = (uid_gid >> 32) as u32;
    let gid = (uid_gid & 0xFFFFFFFF) as u32;

    // Extract process name (comm)
    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Phase 2: Extract PPID from tracepoint context
    // The sched_process_exec tracepoint provides parent_pid in its context
    // We attempt to read it from offset 12 bytes (after common fields)
    // Format: filename (ptr), pid (4), old_pid (4)
    // If this fails, we'll fall back to 0 (same as Phase 1)
    let ppid = unsafe { ctx.read_at::<i32>(12).unwrap_or(0) as u32 };

    // Phase 2: Extract executable path from filename field
    // Use per-CPU buffer to avoid eBPF stack limit (512 bytes)
    // We access the buffer, populate it, then copy directly into the event
    if let Some(cmdline_buf) = CMDLINE_BUF.get_ptr_mut(0) {
        let cmdline_buf = unsafe { &mut *cmdline_buf };
        // Initialize buffer to zeros
        *cmdline_buf = [0u8; 256];

        // Try to read the filename pointer from the tracepoint context
        // The filename is at offset 8 in the tracepoint (after common_pid at offset 4)
        if let Ok(filename_ptr) = unsafe { ctx.read_at::<u64>(8) } {
            // The filename_ptr is actually encoded as __data_loc type:
            // high 16 bits = size, low 16 bits = offset from context start
            let offset = (filename_ptr & 0xFFFF) as usize;
            let size = ((filename_ptr >> 16) & 0xFFFF) as usize;

            // Limit size to our buffer (256 bytes - 1 for null terminator)
            let read_size = if size > 255 { 255 } else { size };

            // Calculate the actual pointer: context base + offset
            let ctx_ptr = ctx.as_ptr() as usize;
            let actual_filename_ptr = (ctx_ptr + offset) as *const u8;

            // Read the filename string from kernel memory
            let _ = unsafe {
                bpf_probe_read_kernel_str_bytes(actual_filename_ptr, &mut cmdline_buf[..read_size])
            };
        }

        // Build event directly with the per-CPU buffer content
        let event = ProcessEvent {
            pid,
            ppid,
            uid,
            gid,
            comm,
            cmdline: *cmdline_buf, // Copy from per-CPU buffer
            event_type: 0,         // exec event
        };

        if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
            buf.write(event);
            buf.submit(0);
        }
    }

    0
}

#[tracepoint]
pub fn sched_process_exit(_ctx: TracePointContext) -> u32 {
    // Extract PID and TGID
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32;

    // Extract UID and GID
    let uid_gid = unsafe { bpf_get_current_uid_gid() };
    let uid = (uid_gid >> 32) as u32;
    let gid = (uid_gid & 0xFFFFFFFF) as u32;

    // Extract process name
    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    let event = ProcessEvent {
        pid,
        ppid: 0, // Not critical for exit events
        uid,
        gid,
        comm,
        cmdline: [0; 256], // Not needed for exit events
        event_type: 1,     // exit event
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
