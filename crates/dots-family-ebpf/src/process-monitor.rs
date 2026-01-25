#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_probe_read_kernel,
    },
    macros::{map, tracepoint},
    maps::RingBuf,
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
    pub cmdline: [u8; 512],
    pub event_type: u32,
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

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

    // Note: cmdline extraction requires reading from task_struct->mm->arg_start
    // which needs more complex BTF/CO-RE support - deferred to later phase

    let event = ProcessEvent {
        pid,
        ppid,
        uid,
        gid,
        comm,
        cmdline: [0; 512], // TODO: Read from task_struct->mm->arg_start via bpf_probe_read_user
        event_type: 0,     // exec event
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
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
        cmdline: [0; 512], // Not needed for exit events
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
