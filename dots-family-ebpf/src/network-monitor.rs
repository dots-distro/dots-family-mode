#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkEvent {
    pub event_type: u32,
    pub pid: u32,
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
}

#[map]
static NETWORK_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[kprobe]
pub fn tcp_connect(_ctx: ProbeContext) -> u32 {
    let event =
        NetworkEvent { event_type: 1, pid: 0, src_addr: 0, dst_addr: 0, src_port: 0, dst_port: 0 };

    if let Some(mut buf) = NETWORK_EVENTS.reserve::<NetworkEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
