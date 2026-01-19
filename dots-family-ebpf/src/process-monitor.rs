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
    pub cmdline: [u8; 512],
    pub event_type: u32,
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(_ctx: TracePointContext) -> u32 {
    let event = ProcessEvent {
        pid: 0,
        ppid: 0,
        uid: 0,
        gid: 0,
        comm: [0; 16],
        cmdline: [0; 512],
        event_type: 0,
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[tracepoint]
pub fn sched_process_exit(_ctx: TracePointContext) -> u32 {
    let event = ProcessEvent {
        pid: 0,
        ppid: 0,
        uid: 0,
        gid: 0,
        comm: [0; 16],
        cmdline: [0; 512],
        event_type: 1,
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
