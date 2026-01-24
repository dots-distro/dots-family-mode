#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkEvent {
    pub event_type: u32, // 1 = connect, 2 = bind, 3 = accept, 4 = close
    pub pid: u32,
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,    // TCP = 6, UDP = 17
    pub bytes_sent: u64, // For data transfer events
    pub bytes_received: u64,
}

#[map]
static NETWORK_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[kprobe]
pub fn tcp_connect(_ctx: ProbeContext) -> u32 {
    // Get pid from current process
    // bpf_get_current_pid_tgid() returns u64 where high 32 bits = tgid, low 32 bits = pid
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid & 0xFFFFFFFF) as u32;

    // Simplified event - in a real implementation, we'd extract socket info from kernel structs
    // For now, just log that a connection attempt was made
    let event = NetworkEvent {
        event_type: 1, // TCP connect
        pid,
        src_addr: 0, // Would need to read from socket struct
        dst_addr: 0, // Would need to read from socket struct
        src_port: 0, // Would need to read from socket struct
        dst_port: 0, // Would need to read from socket struct
        protocol: 6, // TCP
        bytes_sent: 0,
        bytes_received: 0,
    };

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
