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
pub fn tcp_connect(ctx: ProbeContext) -> u32 {
    // Get socket information from context
    let sock = ctx.sock();
    if sock.is_null() {
        return 0;
    }

    unsafe {
        // Extract connection details using safe helper
        let src_addr = bpf_probe_read_user_str(&sock.__sk_common.skc_rcv_saddr);
        let dst_addr = bpf_probe_read_user_str(&sock.__sk_common.skc_daddr);
        let src_port = bpf_probe_read(&sock.__sk_common.skc_num, 2);
        let dst_port = bpf_probe_read(&sock.__sk_common.skc_dport, 2);

        let event = NetworkEvent {
            event_type: 1, // TCP connect
            pid: bpf_get_current_pid_tgid().pid,
            src_addr: u32::from_ne_bytes(src_addr),
            dst_addr: u32::from_ne_bytes(dst_addr),
            src_port: u16::from_be(src_port),
            dst_port: u16::from_be(dst_port),
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
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
