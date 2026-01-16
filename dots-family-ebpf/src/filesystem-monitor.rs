#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FilesystemEvent {
    pub event_type: u32,
    pub pid: u32,
    pub fd: u32,
    pub filename: [u8; 64],
}

#[map]
static FS_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[kprobe]
pub fn trace_do_sys_open(_ctx: ProbeContext) -> u32 {
    let event = FilesystemEvent {
        event_type: 1, // EVENT_OPEN
        pid: 0,        // Simplified for compatibility
        fd: 0,
        filename: [0; 64],
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
