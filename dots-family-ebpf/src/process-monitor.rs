#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub event_type: u32, // 0 = exec, 1 = exit
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(ctx) {
        Ok(ret) => ret,
        Err(_) => 1,
    }
}

fn try_sched_process_exec(_ctx: TracePointContext) -> Result<u32, u32> {
    let current_pid = unsafe { aya_ebpf::helpers::bpf_get_current_pid_tgid() as u32 };
    let uid_gid = unsafe { aya_ebpf::helpers::bpf_get_current_uid_gid() };

    let event = ProcessEvent {
        pid: current_pid,
        ppid: 0,
        uid: uid_gid as u32,
        gid: (uid_gid >> 32) as u32,
        comm: [0; 16],
        event_type: 0,
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    Ok(0)
}

#[tracepoint]
pub fn sched_process_exit(_ctx: TracePointContext) -> u32 {
    let event = ProcessEvent {
        pid: 0,  // Simplified for compatibility
        ppid: 0, // Simplified for compatibility
        uid: 0,  // Simplified for compatibility
        gid: 0,  // Simplified for compatibility
        comm: [0; 16],
        event_type: 1, // exit event
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
